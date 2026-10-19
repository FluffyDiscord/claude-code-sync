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
        ) -> MessageNode {
            let mut processing_order: Vec<String> = Vec::new();
            let mut queue: std::collections::VecDeque<String> = std::collections::VecDeque::new();
            queue.push_back(root_uuid.to_string());

            while let Some(uuid) = queue.pop_front() {
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

        let root_uuids = parent_to_children.get(&None).cloned().unwrap_or_default();

        let mut roots = Vec::new();
        for root_uuid in root_uuids {
            let root_node =
                build_subtree_iterative(&root_uuid, &uuid_to_entry, &parent_to_children);
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

    #[allow(dead_code)]
    fn build_tree(
        &mut self,
        entries: &[&ConversationEntry],
        resolved_edits: &HashMap<String, ConversationEntry>,
    ) -> Result<Vec<MessageNode>> {
        let mut uuid_to_entry: HashMap<String, ConversationEntry> = HashMap::new();

        for entry in entries {
            if let Some(uuid) = &entry.uuid {
                let entry_to_use = resolved_edits.get(uuid).unwrap_or(*entry);
                uuid_to_entry.insert(uuid.clone(), entry_to_use.clone());
            }
        }

        let mut used_as_child = HashSet::new();

        let mut parent_to_children: HashMap<Option<String>, Vec<MessageNode>> = HashMap::new();

        for entry in uuid_to_entry.values() {
            let node = MessageNode::new(entry.clone());

            parent_to_children
                .entry(entry.parent_uuid.clone())
                .or_default()
                .push(node);

            if let Some(parent_uuid) = &entry.parent_uuid {
                used_as_child.insert(parent_uuid.clone());
            }
        }

        fn build_subtree(
            parent_uuid: Option<String>,
            parent_to_children: &HashMap<Option<String>, Vec<MessageNode>>,
        ) -> Vec<MessageNode> {
            if let Some(children) = parent_to_children.get(&parent_uuid) {
                children
                    .iter()
                    .map(|child| {
                        let mut node = child.clone();
                        let child_uuid = child.entry.uuid.clone();
                        node.children = build_subtree(child_uuid, parent_to_children);
                        node
                    })
                    .collect()
            } else {
                Vec::new()
            }
        }

        let roots = build_subtree(None, &parent_to_children);

        let mut all_roots = roots;
        for (parent_uuid, children) in parent_to_children.iter() {
            if let Some(parent) = parent_uuid {
                if !uuid_to_entry.contains_key(parent) {
                    all_roots.extend(children.iter().map(|child| {
                        let mut node = child.clone();
                        let child_uuid = child.entry.uuid.clone();
                        node.children = build_subtree(child_uuid, &parent_to_children);
                        node
                    }));
                }
            }
        }

        Ok(all_roots)
    }

    #[allow(dead_code)]
    fn merge_trees(
        &mut self,
        local_roots: Vec<MessageNode>,
        remote_roots: Vec<MessageNode>,
    ) -> Result<Vec<MessageNode>> {
        let mut merged_nodes: HashMap<String, MessageNode> = HashMap::new();

        fn collect_nodes_recursive(
            nodes: &[MessageNode],
            collected: &mut HashMap<String, MessageNode>,
        ) {
            for node in nodes {
                if let Some(uuid) = &node.entry.uuid {
                    collected.insert(uuid.clone(), node.clone());
                }
                collect_nodes_recursive(&node.children, collected);
            }
        }

        collect_nodes_recursive(&local_roots, &mut merged_nodes);

        for remote_root in remote_roots {
            self.merge_node_into(&remote_root, &mut merged_nodes);
        }

        self.count_branches(&merged_nodes);

        let mut roots: Vec<MessageNode> = merged_nodes
            .values()
            .filter(|node| node.entry.parent_uuid.is_none())
            .cloned()
            .collect();

        roots.sort_by(|a, b| {
            let a_ts = a.entry.timestamp.as_ref();
            let b_ts = b.entry.timestamp.as_ref();
            a_ts.cmp(&b_ts)
        });

        Ok(roots)
    }

    #[allow(dead_code)]
    #[allow(clippy::only_used_in_recursion)]
    fn merge_node_into(
        &mut self,
        node: &MessageNode,
        merged_nodes: &mut HashMap<String, MessageNode>,
    ) {
        if let Some(uuid) = &node.entry.uuid {
            let mut children_to_merge = Vec::new();

            if let Some(existing) = merged_nodes.get_mut(uuid) {
                for child in &node.children {
                    let child_exists = if let Some(child_uuid) = &child.entry.uuid {
                        existing
                            .children
                            .iter()
                            .any(|c| c.entry.uuid.as_ref() == Some(child_uuid))
                    } else {
                        false
                    };

                    if !child_exists {
                        existing.add_child(child.clone());
                    }

                    if child.entry.uuid.is_some() {
                        children_to_merge.push(child.clone());
                    }
                }
            } else {
                merged_nodes.insert(uuid.clone(), node.clone());

                for child in &node.children {
                    children_to_merge.push(child.clone());
                }
            }

            for child in &children_to_merge {
                self.merge_node_into(child, merged_nodes);
            }
        }
    }

    #[allow(dead_code)]
    fn count_branches(&mut self, nodes: &HashMap<String, MessageNode>) {
        for node in nodes.values() {
            if node.children.len() > 1 {
                self.stats.branches_detected += 1;
            }
        }
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
