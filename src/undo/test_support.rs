#![allow(dead_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Duration;
use tempfile::{tempdir, TempDir};

use super::Snapshot;
use crate::git::GitRepo;
use crate::history::{
    ConversationSummary, OperationHistory, OperationRecord, OperationType, SyncOperation,
};

pub(super) fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
}

pub(super) fn setup_test_repo() -> (TempDir, GitRepo) {
    let temp_dir = tempdir().unwrap();
    let repo = GitRepo::init(temp_dir.path()).unwrap();

    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "initial content").unwrap();
    repo.stage_all().unwrap();
    repo.commit("Initial commit").unwrap();

    (temp_dir, repo)
}

pub(super) fn metadata_only_snapshot(
    id: &str,
    operation_type: OperationType,
    age: Duration,
) -> Snapshot {
    Snapshot {
        snapshot_id: id.to_string(),
        timestamp: chrono::Utc::now() - age,
        operation_type,
        git_commit_hash: None,
        files: HashMap::new(),
        branch: None,
        base_snapshot_id: None,
        deleted_files: Vec::new(),
    }
}

pub(super) struct HistoryBuilder {
    path: PathBuf,
    history: OperationHistory,
}

impl HistoryBuilder {
    pub(super) fn new(history_path: &Path) -> Self {
        Self {
            path: history_path.to_path_buf(),
            history: OperationHistory::from_path(Some(history_path.to_path_buf())).unwrap(),
        }
    }

    pub(super) fn push(
        mut self,
        operation_type: OperationType,
        branch: &str,
        snapshot: Option<&Path>,
    ) -> Self {
        let sync_op = match operation_type {
            OperationType::Pull => SyncOperation::Modified,
            _ => SyncOperation::Added,
        };
        let summary = ConversationSummary::new(
            "test-session".to_string(),
            "test/path".to_string(),
            None,
            5,
            sync_op,
        )
        .unwrap();

        let mut record =
            OperationRecord::new(operation_type, Some(branch.to_string()), vec![summary]);
        record.snapshot_path = snapshot.map(Path::to_path_buf);

        self.history.add_operation(record).unwrap();
        self
    }

    pub(super) fn save(self) {
        self.history.save_to(Some(self.path)).unwrap();
    }
}

pub(super) fn operation_count(history_path: &Path) -> usize {
    OperationHistory::from_path(Some(history_path.to_path_buf()))
        .unwrap()
        .len()
}

pub(super) fn operation_types(history_path: &Path) -> Vec<OperationType> {
    OperationHistory::from_path(Some(history_path.to_path_buf()))
        .unwrap()
        .list_operations()
        .iter()
        .map(|op| op.operation_type)
        .collect()
}
