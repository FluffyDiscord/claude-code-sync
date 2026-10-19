use serde_json::Value;

fn line_key(line: &str) -> String {
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(line) {
        if map.contains_key("timestamp") || map.contains_key("display") {
            return format!(
                "{}\u{1f}{}\u{1f}{}",
                map.get("timestamp")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                map.get("project").map(Value::to_string).unwrap_or_default(),
                map.get("display").map(Value::to_string).unwrap_or_default(),
            );
        }
    }
    format!("raw\u{1f}{line}")
}

fn line_timestamp(line: &str) -> Option<i64> {
    let value: Value = serde_json::from_str(line).ok()?;
    match value.get("timestamp")? {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.timestamp_millis()),
        _ => None,
    }
}

pub fn merge_history_lines(dest: &str, incoming: &str) -> (String, usize) {
    let mut seen = std::collections::HashSet::new();
    let mut lines: Vec<&str> = Vec::new();
    let mut added = 0usize;

    for line in dest.lines().filter(|l| !l.trim().is_empty()) {
        if seen.insert(line_key(line)) {
            lines.push(line);
        }
    }
    for line in incoming.lines().filter(|l| !l.trim().is_empty()) {
        if seen.insert(line_key(line)) {
            lines.push(line);
            added += 1;
        }
    }

    let mut carried = i64::MIN;
    let keyed: Vec<(i64, &str)> = lines
        .iter()
        .map(|line| {
            if let Some(ts) = line_timestamp(line) {
                carried = ts;
            }
            (carried, *line)
        })
        .collect();
    let mut sorted = keyed;
    sorted.sort_by_key(|(ts, _)| *ts);

    let mut merged = sorted
        .iter()
        .map(|(_, line)| *line)
        .collect::<Vec<_>>()
        .join("\n");
    if !merged.is_empty() {
        merged.push('\n');
    }

    (merged, added)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(ts: u64, project: &str, display: &str) -> String {
        format!(
            r#"{{"display":"{display}","pastedContents":{{}},"timestamp":{ts},"project":"{project}","sessionId":"s-{ts}"}}"#
        )
    }

    #[test]
    fn test_union_dedups_shared_lines() {
        let a = format!(
            "{}\n{}\n",
            line(1000, "/p1", "first"),
            line(2000, "/p1", "second")
        );
        let b = format!(
            "{}\n{}\n",
            line(2000, "/p1", "second"),
            line(3000, "/p2", "third")
        );

        let (merged, added) = merge_history_lines(&a, &b);
        assert_eq!(added, 1, "only the genuinely new line counts");
        assert_eq!(merged.lines().count(), 3);
        assert_eq!(merged.matches("second").count(), 1, "no duplicate");
    }

    #[test]
    fn test_union_orders_chronologically() {
        let a = format!(
            "{}\n{}\n",
            line(1000, "/p", "one"),
            line(3000, "/p", "three")
        );
        let b = format!("{}\n", line(2000, "/p", "two"));

        let (merged, added) = merge_history_lines(&a, &b);
        assert_eq!(added, 1);
        let order: Vec<_> = merged
            .lines()
            .map(|l| {
                serde_json::from_str::<Value>(l).unwrap()["timestamp"]
                    .as_u64()
                    .unwrap()
            })
            .collect();
        assert_eq!(order, vec![1000, 2000, 3000]);
    }

    #[test]
    fn test_union_is_idempotent() {
        let a = format!("{}\n{}\n", line(1000, "/p", "one"), line(2000, "/p", "two"));
        let b = format!("{}\n", line(1500, "/q", "mid"));

        let (merged_once, _) = merge_history_lines(&a, &b);
        let (merged_twice, added_again) = merge_history_lines(&merged_once, &b);
        assert_eq!(added_again, 0, "re-merging the same input adds nothing");
        assert_eq!(merged_once, merged_twice);
    }

    #[test]
    fn test_malformed_lines_kept_once_via_raw_fallback() {
        let junk = "not json at all";
        let a = format!("{}\n{junk}\n", line(1000, "/p", "one"));
        let b = format!("{junk}\n{}\n", line(2000, "/p", "two"));

        let (merged, added) = merge_history_lines(&a, &b);
        assert_eq!(added, 1, "junk line dedups by raw content");
        assert_eq!(merged.matches(junk).count(), 1);
        assert_eq!(merged.lines().count(), 3);
    }

    #[test]
    fn test_same_display_different_timestamp_not_deduped() {
        let a = format!("{}\n", line(1000, "/p", "run tests"));
        let b = format!("{}\n", line(2000, "/p", "run tests"));

        let (merged, added) = merge_history_lines(&a, &b);
        assert_eq!(added, 1);
        assert_eq!(merged.lines().count(), 2);
    }

    #[test]
    fn test_empty_sides() {
        let a = format!("{}\n", line(1000, "/p", "one"));

        let (merged, added) = merge_history_lines(&a, "");
        assert_eq!(added, 0);
        assert_eq!(merged, a);

        let (merged, added) = merge_history_lines("", &a);
        assert_eq!(added, 1);
        assert_eq!(merged, a);

        let (merged, added) = merge_history_lines("", "");
        assert_eq!(added, 0);
        assert_eq!(merged, "");
    }

    #[test]
    fn test_output_ends_with_newline_when_nonempty() {
        let a = line(1000, "/p", "one");
        let (merged, _) = merge_history_lines(&a, "");
        assert!(
            merged.ends_with('\n'),
            "JSONL output must be newline-terminated"
        );
    }
}
