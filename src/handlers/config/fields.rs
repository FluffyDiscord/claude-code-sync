use anyhow::{bail, Context, Result};

const BYTES_PER_MB: f64 = 1024.0 * 1024.0;

pub(super) fn parse_patterns(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub(super) fn parse_file_size_mb(input: &str) -> Result<u64> {
    let mb: f64 = input
        .trim()
        .parse()
        .context("Invalid number. Must be a positive number.")?;

    if !mb.is_finite() {
        bail!("Max file size must be a finite number of megabytes");
    }
    if mb <= 0.0 {
        bail!("Max file size must be greater than 0 MB (got {mb})");
    }

    Ok((mb * BYTES_PER_MB) as u64)
}

pub(super) fn format_size_mb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / BYTES_PER_MB)
}

pub(super) fn format_age_days(days: Option<u32>) -> String {
    days.map_or_else(|| "Not set".to_string(), |d| d.to_string())
}

pub(super) fn format_patterns(patterns: &[String], when_empty: &str) -> String {
    if patterns.is_empty() {
        when_empty.to_string()
    } else {
        patterns.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_patterns_splits_and_trims() {
        assert_eq!(
            parse_patterns("*work*, /home/user/x , *test*"),
            vec!["*work*", "/home/user/x", "*test*"]
        );
    }

    #[test]
    fn test_parse_patterns_drops_empty_segments() {
        assert_eq!(parse_patterns("a,,b,"), vec!["a", "b"]);
        assert_eq!(parse_patterns("  ,  ,  "), Vec::<String>::new());
    }

    #[test]
    fn test_parse_patterns_empty_input_yields_empty_list() {
        assert_eq!(parse_patterns(""), Vec::<String>::new());
    }

    #[test]
    fn test_parse_file_size_accepts_positive_values() {
        assert_eq!(parse_file_size_mb("10").unwrap(), 10 * 1024 * 1024);
        assert_eq!(parse_file_size_mb("0.5").unwrap(), 512 * 1024);
        assert_eq!(parse_file_size_mb("  2  ").unwrap(), 2 * 1024 * 1024);
    }

    #[test]
    fn test_parse_file_size_rejects_negative() {
        let err = parse_file_size_mb("-5").unwrap_err().to_string();
        assert!(err.contains("greater than 0"), "unexpected error: {err}");
    }

    #[test]
    fn test_parse_file_size_rejects_zero() {
        assert!(parse_file_size_mb("0").is_err());
    }

    #[test]
    fn test_parse_file_size_rejects_non_finite() {
        assert!(parse_file_size_mb("inf").is_err());
        assert!(parse_file_size_mb("NaN").is_err());
    }

    #[test]
    fn test_parse_file_size_rejects_garbage() {
        assert!(parse_file_size_mb("ten").is_err());
        assert!(parse_file_size_mb("").is_err());
    }

    #[test]
    fn test_format_size_mb_round_trips_with_parse() {
        let bytes = parse_file_size_mb("12.5").unwrap();
        assert_eq!(format_size_mb(bytes), "12.5");
    }

    #[test]
    fn test_format_age_days() {
        assert_eq!(format_age_days(Some(30)), "30");
        assert_eq!(format_age_days(None), "Not set");
    }

    #[test]
    fn test_format_patterns_uses_caller_label_when_empty() {
        assert_eq!(format_patterns(&[], "None"), "None");
        assert_eq!(
            format_patterns(&["a".to_string(), "b".to_string()], "None"),
            "a, b"
        );
    }
}
