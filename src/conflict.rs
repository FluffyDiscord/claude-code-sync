use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::merge;
use crate::parser::ConversationSession;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub session_id: String,

    pub local_file: PathBuf,

    pub remote_file: PathBuf,

    pub local_timestamp: Option<String>,

    pub remote_timestamp: Option<String>,

    pub local_message_count: usize,

    pub remote_message_count: usize,

    pub local_hash: String,

    pub remote_hash: String,

    pub resolution: ConflictResolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    SmartMerge { stats: merge::MergeStats },

    KeepBoth { renamed_remote_file: PathBuf },

    KeepLocal,

    KeepRemote,

    Pending,
}

impl Conflict {
    pub fn new(local: &ConversationSession, remote: &ConversationSession) -> Self {
        Conflict {
            session_id: local.session_id.clone(),
            local_file: local.file_path.clone(),
            remote_file: remote.file_path.clone(),
            local_timestamp: local.latest_timestamp().map(str::to_string),
            remote_timestamp: remote.latest_timestamp().map(str::to_string),
            local_message_count: local.message_count(),
            remote_message_count: remote.message_count(),
            local_hash: local.content_hash().to_string(),
            remote_hash: remote.content_hash().to_string(),
            resolution: ConflictResolution::Pending,
        }
    }

    pub fn smart_merge_into_local_file(
        &mut self,
        local_session: &ConversationSession,
        remote_session: &ConversationSession,
    ) -> Result<()> {
        let merge_result = merge::merge_conversations(local_session, remote_session)?;

        crate::parser::write_entries_to_file(&self.local_file, &merge_result.merged_entries)
            .with_context(|| {
                format!(
                    "Failed to write smart merged file: {}",
                    self.local_file.display()
                )
            })?;

        self.resolution = ConflictResolution::SmartMerge {
            stats: merge_result.stats,
        };

        Ok(())
    }

    pub fn preview_smart_merge(
        &mut self,
        local_session: &ConversationSession,
        remote_session: &ConversationSession,
    ) -> Result<()> {
        let merge_result = merge::merge_conversations(local_session, remote_session)?;

        self.resolution = ConflictResolution::SmartMerge {
            stats: merge_result.stats,
        };

        Ok(())
    }

    pub fn resolve_keep_both(&mut self, conflict_suffix: &str) -> Result<PathBuf> {
        let remote_file_name = self
            .remote_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let remote_file_ext = self
            .remote_file
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("jsonl");

        let parent = self.remote_file.parent().unwrap_or_else(|| Path::new("."));

        let new_name = format!("{remote_file_name}-{conflict_suffix}.{remote_file_ext}");
        let renamed_path = parent.join(new_name);

        self.resolution = ConflictResolution::KeepBoth {
            renamed_remote_file: renamed_path.clone(),
        };

        Ok(renamed_path)
    }

    pub fn description(&self) -> String {
        format!(
            "Session {} has diverged:\n  Local: {} messages, last update: {}\n  Remote: {} messages, last update: {}",
            self.session_id,
            self.local_message_count,
            self.local_timestamp.as_deref().unwrap_or("unknown"),
            self.remote_message_count,
            self.remote_timestamp.as_deref().unwrap_or("unknown")
        )
    }

    pub fn is_real_conflict(&self) -> bool {
        self.local_hash != self.remote_hash
    }
}

pub struct ConflictDetector {
    conflicts: Vec<Conflict>,
}

impl ConflictDetector {
    pub fn new() -> Self {
        ConflictDetector {
            conflicts: Vec::new(),
        }
    }

    pub fn detect(&mut self, pairs: &[(&ConversationSession, &ConversationSession)]) {
        for (local, remote) in pairs {
            if local.content_hash() == remote.content_hash() {
                continue;
            }

            let conflict = Conflict::new(local, remote);
            if conflict.is_real_conflict() {
                self.conflicts.push(conflict);
            }
        }
    }

    #[allow(dead_code)]
    pub fn resolve_all_keep_both(&mut self) -> Result<Vec<(PathBuf, PathBuf)>> {
        let mut renames = Vec::new();

        for conflict in &mut self.conflicts {
            let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            let conflict_suffix = format!("conflict-{timestamp}");

            let renamed_path = conflict.resolve_keep_both(&conflict_suffix)?;
            renames.push((conflict.remote_file.clone(), renamed_path));
        }

        Ok(renames)
    }

    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }

    pub fn conflicts_mut(&mut self) -> &mut [Conflict] {
        &mut self.conflicts
    }

    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    pub fn conflict_count(&self) -> usize {
        self.conflicts.len()
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ConversationEntry;

    fn create_test_session(
        dir: &tempfile::TempDir,
        session_id: &str,
        message_count: usize,
    ) -> ConversationSession {
        let mut entries = Vec::new();

        for i in 0..message_count {
            entries.push(ConversationEntry {
                entry_type: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                uuid: Some(format!("uuid-{i}")),
                parent_uuid: if i > 0 {
                    Some(format!("uuid-{}", i - 1))
                } else {
                    None
                },
                session_id: Some(session_id.to_string()),
                timestamp: Some(format!("2025-01-01T{i:02}:00:00Z")),
                message: None,
                cwd: None,
                version: None,
                git_branch: None,
                extra: serde_json::Value::Null,
            });
        }

        let path = dir.path().join(format!("{session_id}.jsonl"));
        crate::parser::write_entries_to_file(&path, &entries).unwrap();
        ConversationSession::from_file(&path).unwrap()
    }

    #[test]
    fn test_conflict_detection() {
        let local_dir = tempfile::TempDir::new().unwrap();
        let remote_dir = tempfile::TempDir::new().unwrap();
        let local_session = create_test_session(&local_dir, "session-1", 5);
        let remote_session = create_test_session(&remote_dir, "session-1", 6);

        let mut detector = ConflictDetector::new();
        detector.detect(&[(&local_session, &remote_session)]);

        assert!(detector.has_conflicts());
        assert_eq!(detector.conflict_count(), 1);

        let conflict = &detector.conflicts()[0];
        assert_eq!(conflict.session_id, "session-1");
        assert_eq!(conflict.local_message_count, 5);
        assert_eq!(conflict.remote_message_count, 6);
    }

    #[test]
    fn test_no_conflict_same_content() {
        let local_dir = tempfile::TempDir::new().unwrap();
        let remote_dir = tempfile::TempDir::new().unwrap();
        let local_session = create_test_session(&local_dir, "session-1", 5);
        let remote_session = create_test_session(&remote_dir, "session-1", 5);

        let mut detector = ConflictDetector::new();
        detector.detect(&[(&local_session, &remote_session)]);

        assert!(!detector.has_conflicts());
    }
}
