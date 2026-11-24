use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};

use crate::parser::{ConversationEntry, ConversationSession};

#[derive(Debug, Clone)]
pub struct MessageNode {
    pub entry: ConversationEntry,

    pub children: Vec<MessageNode>,
}

impl MessageNode {
    fn new(entry: ConversationEntry) -> Self {
        MessageNode {
            entry,
            children: Vec::new(),
        }
    }

    fn add_child(&mut self, child: MessageNode) {
        self.children.push(child);
    }

    fn collect_entries(&self) -> Vec<ConversationEntry> {
        let mut entries = Vec::new();
        let mut stack: Vec<&MessageNode> = vec![self];

        while let Some(node) = stack.pop() {
            entries.push(node.entry.clone());

            let mut sorted_children: Vec<&MessageNode> = node.children.iter().collect();
            sorted_children.sort_by(|a, b| {
                let a_ts = a.entry.timestamp.as_ref();
                let b_ts = b.entry.timestamp.as_ref();
                b_ts.cmp(&a_ts)
            });

            for child in sorted_children {
                stack.push(child);
            }
        }

        entries
    }
}

#[derive(Debug)]
pub struct MergeResult {
    pub merged_entries: Vec<ConversationEntry>,

    pub stats: MergeStats,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MergeStats {
    pub local_messages: usize,

    pub remote_messages: usize,

    pub merged_messages: usize,

    pub duplicates_removed: usize,

    pub edits_resolved: usize,

    pub branches_detected: usize,

    pub timestamp_merged: usize,
}

pub struct SmartMerger<'a> {
    local: &'a [ConversationEntry],
    remote: &'a [ConversationEntry],
    stats: MergeStats,
}

fn count_messages(entries: &[ConversationEntry]) -> usize {
    entries
        .iter()
        .filter(|entry| crate::parser::is_message(entry))
        .count()
}

impl<'a> SmartMerger<'a> {
    pub fn new(local: &'a [ConversationEntry], remote: &'a [ConversationEntry]) -> Self {
        SmartMerger {
            local,
            remote,
            stats: MergeStats::default(),
        }
    }

    pub fn merge(&mut self) -> Result<MergeResult> {
        self.stats.local_messages = count_messages(self.local);
        self.stats.remote_messages = count_messages(self.remote);

        let local_map = self.build_uuid_map(self.local);
        let remote_map = self.build_uuid_map(self.remote);

        let resolved_edits = self.detect_and_resolve_edits(&local_map, &remote_map)?;

        let (local_uuid_entries, local_non_uuid): (Vec<_>, Vec<_>) =
            self.local.iter().partition(|e| e.uuid.is_some());

        let (remote_uuid_entries, remote_non_uuid): (Vec<_>, Vec<_>) =
            self.remote.iter().partition(|e| e.uuid.is_some());

        let mut all_uuid_entries: Vec<&ConversationEntry> = Vec::new();
        all_uuid_entries.extend(local_uuid_entries);
        all_uuid_entries.extend(remote_uuid_entries);

        let merged_roots = self.build_unified_tree(&all_uuid_entries, &resolved_edits)?;

        let mut merged_entries = Vec::new();
        for root in &merged_roots {
            merged_entries.extend(root.collect_entries());
        }

        let local_vec: Vec<_> = local_non_uuid.into_iter().cloned().collect();
        let remote_vec: Vec<_> = remote_non_uuid.into_iter().cloned().collect();
        let non_uuid_merged = self.merge_by_timestamp(&local_vec, &remote_vec);

        self.stats.timestamp_merged = non_uuid_merged.len();

        merged_entries.extend(non_uuid_merged);
        merged_entries.sort_by(|a, b| {
            let a_ts = a.timestamp.as_ref();
            let b_ts = b.timestamp.as_ref();
            a_ts.cmp(&b_ts)
        });

        self.stats.merged_messages = merged_entries.len();

        Ok(MergeResult {
            merged_entries,
            stats: self.stats.clone(),
        })
    }

    fn build_uuid_map(&self, entries: &[ConversationEntry]) -> HashMap<String, ConversationEntry> {
        entries
            .iter()
            .filter_map(|e| e.uuid.as_ref().map(|uuid| (uuid.clone(), e.clone())))
            .collect()
    }

    fn detect_and_resolve_edits(
        &mut self,
        local_map: &HashMap<String, ConversationEntry>,
        remote_map: &HashMap<String, ConversationEntry>,
    ) -> Result<HashMap<String, ConversationEntry>> {
        let mut resolved = HashMap::new();

        let common_uuids: HashSet<_> = local_map
            .keys()
            .filter(|uuid| remote_map.contains_key(*uuid))
            .collect();

        for uuid in common_uuids {
            let local_entry = &local_map[uuid];
            let remote_entry = &remote_map[uuid];

            let local_json = serde_json::to_string(local_entry)?;
            let remote_json = serde_json::to_string(remote_entry)?;

            if local_json != remote_json {
                self.stats.edits_resolved += 1;

                let chosen = self.resolve_by_timestamp(local_entry, remote_entry);
                resolved.insert(uuid.clone(), chosen.clone());
            } else {
                resolved.insert(uuid.clone(), local_entry.clone());
            }
        }

        Ok(resolved)
    }

    fn resolve_by_timestamp<'b>(
        &self,
        local: &'b ConversationEntry,
        remote: &'b ConversationEntry,
    ) -> &'b ConversationEntry {
        match (&local.timestamp, &remote.timestamp) {
            (Some(local_ts), Some(remote_ts)) => {
                if remote_ts > local_ts {
                    remote
                } else {
                    local
                }
            }
            (Some(_), None) => local,
            (None, Some(_)) => remote,
            (None, None) => local,
        }
    }

    fn build_unified_tree(
        &mut self,
        all_entries: &[&ConversationEntry],
        resolved_edits: &HashMap<String, ConversationEntry>,
    ) -> Result<Vec<MessageNode>> {
        let mut uuid_to_entry: HashMap<String, ConversationEntry> = HashMap::new();

        for entry in all_entries {
            if let Some(uuid) = &entry.uuid {
                if !uuid_to_entry.contains_key(uuid) {
                    let entry_to_use = resolved_edits.get(uuid).unwrap_or(*entry);
                    uuid_to_entry.insert(uuid.clone(), entry_to_use.clone());
                }
            }
        }

        let mut parent_to_children: HashMap<Option<String>, Vec<String>> = HashMap::new();

        for (uuid, entry) in &uuid_to_entry {
            parent_to_children
                .entry(entry.parent_uuid.clone())
                .or_default()
                .push(uuid.clone());
        }

        fn build_subtree_iterative(
            root_uuid: &str,
            uuid_to_entry: &HashMap<String, ConversationEntry>,
            parent_to_children: &HashMap<Option<String>, Vec<String>>,
            visited: &mut HashSet<String>,
        ) -> MessageNode {
            let mut processing_order: Vec<String> = Vec::new();
            let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
            queue.push_back(root_uuid.to_string());

            while let Some(uuid) = queue.pop_front() {
                let first_visit = visited.insert(uuid.clone());
                if !first_visit {
                    continue;
                }
                processing_order.push(uuid.clone());
                if let Some(child_uuids) = parent_to_children.get(&Some(uuid)) {
                    for child_uuid in child_uuids {
                        queue.push_back(child_uuid.clone());
                    }
                }
            }

            let mut node_map: HashMap<String, MessageNode> = HashMap::new();

            for uuid in processing_order.iter().rev() {
                let entry = uuid_to_entry.get(uuid).unwrap().clone();
                let mut node = MessageNode::new(entry);

                if let Some(child_uuids) = parent_to_children.get(&Some(uuid.clone())) {
                    for child_uuid in child_uuids {
                        if let Some(child_node) = node_map.remove(child_uuid) {
                            node.add_child(child_node);
                        }
                    }
                }

                node_map.insert(uuid.clone(), node);
            }

            node_map.remove(root_uuid).unwrap()
        }

        let mut root_uuids: Vec<&String> = uuid_to_entry
            .iter()
            .filter(|(_, entry)| {
                entry
                    .parent_uuid
                    .as_ref()
                    .is_none_or(|parent| !uuid_to_entry.contains_key(parent))
            })
            .map(|(uuid, _)| uuid)
            .collect();
        root_uuids.sort_by_key(|uuid| (uuid_to_entry[*uuid].timestamp.as_ref(), *uuid));

        let mut unreached_uuids: Vec<&String> = uuid_to_entry.keys().collect();
        unreached_uuids.sort_by_key(|uuid| (uuid_to_entry[*uuid].timestamp.as_ref(), *uuid));

        let mut visited: HashSet<String> = HashSet::new();
        let mut roots = Vec::new();
        for root_uuid in root_uuids.into_iter().chain(unreached_uuids) {
            let already_visited = visited.contains(root_uuid);
            if already_visited {
                continue;
            }
            let root_node = build_subtree_iterative(
                root_uuid,
                &uuid_to_entry,
                &parent_to_children,
                &mut visited,
            );
            roots.push(root_node);
        }

        for entry in uuid_to_entry.values() {
            if let Some(children) = parent_to_children.get(&Some(entry.uuid.clone().unwrap())) {
                if children.len() > 1 {
                    self.stats.branches_detected += 1;
                }
            }
        }

        roots.sort_by(|a, b| {
            let a_ts = a.entry.timestamp.as_ref();
            let b_ts = b.entry.timestamp.as_ref();
            a_ts.cmp(&b_ts)
        });

        Ok(roots)
    }

    fn merge_by_timestamp(
        &mut self,
        local: &[ConversationEntry],
        remote: &[ConversationEntry],
    ) -> Vec<ConversationEntry> {
        let mut all_entries = local.to_owned();
        all_entries.extend(remote.to_owned());

        all_entries.sort_by(|a, b| {
            let a_ts = a.timestamp.as_ref();
            let b_ts = b.timestamp.as_ref();
            a_ts.cmp(&b_ts)
        });

        let mut seen = HashSet::new();
        let mut unique_entries = Vec::new();

        for entry in all_entries {
            if let Ok(json) = serde_json::to_string(&entry) {
                if seen.insert(json.clone()) {
                    unique_entries.push(entry);
                } else {
                    self.stats.duplicates_removed += 1;
                }
            }
        }

        unique_entries
    }
}

pub fn merge_conversations(
    local: &ConversationSession,
    remote: &ConversationSession,
) -> Result<MergeResult> {
    if local.session_id != remote.session_id {
        return Err(anyhow!(
            "Cannot merge conversations with different session IDs: {} vs {}",
            local.session_id,
            remote.session_id
        ));
    }

    let local_entries = local.load_entries()?;
    let remote_entries = remote.load_entries()?;

    let mut merger = SmartMerger::new(&local_entries, &remote_entries);
    merger.merge()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_entry(
        uuid: &str,
        parent_uuid: Option<&str>,
        timestamp: &str,
    ) -> ConversationEntry {
        ConversationEntry {
            entry_type: "user".to_string(),
            uuid: Some(uuid.to_string()),
            parent_uuid: parent_uuid.map(|s| s.to_string()),
            session_id: Some("test-session".to_string()),
            timestamp: Some(timestamp.to_string()),
            message: Some(json!({"text": format!("Message {}", uuid)})),
            cwd: None,
            version: None,
            git_branch: None,
            extra: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_merge_non_overlapping_messages() {
        let local_entries = vec![
            create_test_entry("1", None, "2025-01-01T00:00:00Z"),
            create_test_entry("2", Some("1"), "2025-01-01T00:01:00Z"),
        ];

        let remote_entries = vec![
            create_test_entry("1", None, "2025-01-01T00:00:00Z"),
            create_test_entry("2", Some("1"), "2025-01-01T00:01:00Z"),
            create_test_entry("3", Some("2"), "2025-01-01T00:02:00Z"),
            create_test_entry("4", Some("3"), "2025-01-01T00:03:00Z"),
        ];

        let result = SmartMerger::new(&local_entries, &remote_entries)
            .merge()
            .unwrap();

        assert_eq!(
            result.merged_entries.len(),
            4,
            "Should merge to 4 total messages"
        );
        assert_eq!(result.stats.merged_messages, 4);
        assert_eq!(result.stats.local_messages, 2);
        assert_eq!(result.stats.remote_messages, 4);
    }

    #[test]
    fn test_merge_with_branches() {
        let local_entries = vec![
            create_test_entry("1", None, "2025-01-01T00:00:00Z"),
            create_test_entry("2", Some("1"), "2025-01-01T00:01:00Z"),
            create_test_entry("3", Some("2"), "2025-01-01T00:02:00Z"),
        ];

        let remote_entries = vec![
            create_test_entry("1", None, "2025-01-01T00:00:00Z"),
            create_test_entry("2", Some("1"), "2025-01-01T00:01:00Z"),
            create_test_entry("4", Some("2"), "2025-01-01T00:02:30Z"),
        ];

        let result = SmartMerger::new(&local_entries, &remote_entries)
            .merge()
            .unwrap();

        assert!(
            result.stats.branches_detected > 0,
            "Should detect at least one branch"
        );

        assert_eq!(
            result.merged_entries.len(),
            4,
            "Should have all 4 unique messages"
        );

        let uuids: Vec<String> = result
            .merged_entries
            .iter()
            .filter_map(|e| e.uuid.clone())
            .collect();
        assert!(uuids.contains(&"1".to_string()));
        assert!(uuids.contains(&"2".to_string()));
        assert!(uuids.contains(&"3".to_string()));
        assert!(uuids.contains(&"4".to_string()));
    }

    #[test]
    fn test_merge_keeps_entries_whose_parent_is_missing_from_both_sides() {
        let local_entries = vec![
            create_test_entry("1", None, "2025-01-01T00:00:00Z"),
            create_test_entry("2", Some("1"), "2025-01-01T00:01:00Z"),
        ];

        let remote_entries = vec![
            create_test_entry("1", None, "2025-01-01T00:00:00Z"),
            create_test_entry("2", Some("1"), "2025-01-01T00:01:00Z"),
            create_test_entry("4", Some("compacted-away"), "2025-01-01T00:03:00Z"),
            create_test_entry("5", Some("4"), "2025-01-01T00:04:00Z"),
        ];

        let result = SmartMerger::new(&local_entries, &remote_entries)
            .merge()
            .unwrap();

        let uuids: Vec<String> = result
            .merged_entries
            .iter()
            .filter_map(|e| e.uuid.clone())
            .collect();
        assert_eq!(uuids, vec!["1", "2", "4", "5"]);
    }

    #[test]
    fn test_merge_keeps_entries_in_a_parent_cycle() {
        let local_entries = vec![
            create_test_entry("1", None, "2025-01-01T00:00:00Z"),
            create_test_entry("2", Some("1"), "2025-01-01T00:01:00Z"),
        ];

        let remote_entries = vec![
            create_test_entry("1", None, "2025-01-01T00:00:00Z"),
            create_test_entry("2", Some("1"), "2025-01-01T00:01:00Z"),
            create_test_entry("A", Some("B"), "2025-01-01T00:02:00Z"),
            create_test_entry("B", Some("A"), "2025-01-01T00:03:00Z"),
        ];

        let result = SmartMerger::new(&local_entries, &remote_entries)
            .merge()
            .unwrap();

        let uuids: Vec<String> = result
            .merged_entries
            .iter()
            .filter_map(|e| e.uuid.clone())
            .collect();
        assert_eq!(uuids, vec!["1", "2", "A", "B"]);
    }

    #[test]
    fn test_merge_orders_roots_with_equal_timestamps_by_uuid() {
        let local_entries: Vec<ConversationEntry> = ["r3", "r1", "r7", "r5"]
            .iter()
            .map(|uuid| create_test_entry(uuid, None, "2025-01-01T00:00:00Z"))
            .collect();
        let remote_entries: Vec<ConversationEntry> = ["r6", "r0", "r4", "r2"]
            .iter()
            .map(|uuid| create_test_entry(uuid, None, "2025-01-01T00:00:00Z"))
            .collect();

        let result = SmartMerger::new(&local_entries, &remote_entries)
            .merge()
            .unwrap();

        let uuids: Vec<String> = result
            .merged_entries
            .iter()
            .filter_map(|e| e.uuid.clone())
            .collect();
        assert_eq!(uuids, vec!["r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7"]);
    }

    #[test]
    fn test_edit_resolution_by_timestamp() {
        let mut local_entry = create_test_entry("1", None, "2025-01-01T00:00:00Z");
        local_entry.message = Some(json!({"text": "Local version"}));

        let mut remote_entry = create_test_entry("1", None, "2025-01-01T00:01:00Z");
        remote_entry.message = Some(json!({"text": "Remote version (newer)"}));

        let result = SmartMerger::new(&[local_entry], &[remote_entry])
            .merge()
            .unwrap();

        assert_eq!(result.stats.edits_resolved, 1);

        assert_eq!(result.merged_entries.len(), 1);
        assert_eq!(
            result.merged_entries[0].message,
            Some(json!({"text": "Remote version (newer)"}))
        );
    }
}
