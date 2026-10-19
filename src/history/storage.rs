use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::record::OperationRecord;
use super::types::OperationType;

const MAX_HISTORY_SIZE: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationHistory {
    pub operations: Vec<OperationRecord>,

    #[serde(skip)]
    path: Option<PathBuf>,
}

impl OperationHistory {
    fn new() -> Self {
        Self {
            operations: Vec::new(),
            path: None,
        }
    }

    fn history_file_path() -> Result<PathBuf> {
        crate::config::ConfigManager::operation_history_path()
    }

    pub fn from_path(path: Option<PathBuf>) -> Result<Self> {
        let bound_path = path.clone();

        let file_path = match path {
            Some(p) => p,
            None => Self::history_file_path()?,
        };

        if !file_path.exists() {
            let mut history = Self::new();
            history.path = bound_path;
            return Ok(history);
        }

        let content = fs::read_to_string(&file_path).with_context(|| {
            format!(
                "Failed to read operation history file from: {}",
                file_path.display()
            )
        })?;

        let mut history: OperationHistory = serde_json::from_str(&content).with_context(|| {
            format!(
                "Failed to parse operation history JSON from: {}",
                file_path.display()
            )
        })?;
        history.path = bound_path;

        Ok(history)
    }

    pub fn load() -> Result<Self> {
        Self::from_path(None)
    }

    pub fn save_to(&self, path: Option<PathBuf>) -> Result<()> {
        let file_path = match path {
            Some(p) => p,
            None => Self::history_file_path()?,
        };

        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create history directory: {}", parent.display())
            })?;
        }

        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize operation history")?;

        fs::write(&file_path, content).with_context(|| {
            format!(
                "Failed to write operation history file to: {}",
                file_path.display()
            )
        })?;

        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(self.path.clone())
    }

    pub fn add_operation(&mut self, record: OperationRecord) -> Result<()> {
        self.operations.insert(0, record);

        if self.operations.len() > MAX_HISTORY_SIZE {
            self.operations.truncate(MAX_HISTORY_SIZE);
        }

        self.save()?;

        Ok(())
    }

    pub fn get_last_operation(&self) -> Option<&OperationRecord> {
        self.operations.first()
    }

    pub fn get_last_operation_by_type(&self, op_type: OperationType) -> Option<&OperationRecord> {
        self.operations
            .iter()
            .find(|op| op.operation_type == op_type)
    }

    pub fn list_operations(&self) -> &[OperationRecord] {
        &self.operations
    }

    pub fn clear(&mut self) -> Result<()> {
        self.operations.clear();
        self.save()?;
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.operations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn remove_last_operation_by_type(
        &mut self,
        op_type: OperationType,
        path: Option<PathBuf>,
    ) -> Result<bool> {
        if let Some(index) = self
            .operations
            .iter()
            .position(|op| op.operation_type == op_type)
        {
            self.operations.remove(index);
            self.save_to(path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl Default for OperationHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::summary::ConversationSummary;
    use super::super::types::SyncOperation;
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("CLAUDE_CODE_SYNC_CONFIG_DIR", temp_dir.path());
        let history_path = temp_dir.path().join("operation-history.json");
        (temp_dir, history_path)
    }

    #[test]
    fn test_operation_history_new() {
        let history = OperationHistory::new();
        assert_eq!(history.len(), 0);
        assert!(history.is_empty());
        assert!(history.get_last_operation().is_none());
    }

    #[test]
    fn test_operation_history_add_operation() {
        let (_temp_dir, path) = setup_test_env();

        let mut history = OperationHistory::from_path(Some(path.clone())).unwrap();

        let record = OperationRecord::new(OperationType::Push, Some("main".to_string()), vec![]);

        history.add_operation(record).unwrap();

        let loaded = OperationHistory::from_path(Some(path)).unwrap();
        assert_eq!(loaded.len(), 1);
        assert!(!loaded.is_empty());

        let op = loaded.get_last_operation().unwrap();
        assert_eq!(op.operation_type, OperationType::Push);
        assert_eq!(op.branch, Some("main".to_string()));
    }

    #[test]
    fn test_operation_history_save_load_roundtrip() {
        let (_temp_dir, path) = setup_test_env();

        let mut history = OperationHistory::new();

        let conversations1 = vec![
            ConversationSummary::new(
                "session-1".to_string(),
                "path/1".to_string(),
                Some("2025-01-15T10:00:00Z".to_string()),
                5,
                SyncOperation::Added,
            )
            .unwrap(),
            ConversationSummary::new(
                "session-2".to_string(),
                "path/2".to_string(),
                None,
                10,
                SyncOperation::Modified,
            )
            .unwrap(),
        ];

        let record1 = OperationRecord::new(
            OperationType::Push,
            Some("main".to_string()),
            conversations1,
        );

        let conversations2 = vec![ConversationSummary::new(
            "session-3".to_string(),
            "path/3".to_string(),
            None,
            15,
            SyncOperation::Conflict,
        )
        .unwrap()];

        let record2 = OperationRecord::new(
            OperationType::Pull,
            Some("develop".to_string()),
            conversations2,
        );

        history.operations.push(record1);
        history.operations.insert(0, record2);

        history.save_to(Some(path.clone())).unwrap();

        assert!(path.exists());

        let loaded = OperationHistory::from_path(Some(path)).unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.operations.len(), history.operations.len());

        let last = loaded.get_last_operation().unwrap();
        assert_eq!(last.operation_type, OperationType::Pull);
        assert_eq!(last.branch, Some("develop".to_string()));
        assert_eq!(last.affected_conversations.len(), 1);
        assert_eq!(
            last.affected_conversations[0].operation,
            SyncOperation::Conflict
        );

        let first = &loaded.operations[1];
        assert_eq!(first.operation_type, OperationType::Push);
        assert_eq!(first.affected_conversations.len(), 2);
    }

    #[test]
    fn test_operation_history_rotation() {
        let mut history = OperationHistory::new();

        for i in 0..7 {
            let record =
                OperationRecord::new(OperationType::Push, Some(format!("branch-{i}")), vec![]);
            history.operations.insert(0, record);
        }

        if history.operations.len() > MAX_HISTORY_SIZE {
            history.operations.truncate(MAX_HISTORY_SIZE);
        }

        assert_eq!(history.len(), MAX_HISTORY_SIZE);

        let last = history.get_last_operation().unwrap();
        assert_eq!(last.branch, Some("branch-6".to_string()));
    }

    #[test]
    fn test_operation_history_get_last_operation() {
        let mut history = OperationHistory::new();

        let record1 = OperationRecord::new(OperationType::Pull, Some("main".to_string()), vec![]);

        let record2 =
            OperationRecord::new(OperationType::Push, Some("develop".to_string()), vec![]);

        history.operations.push(record1);
        history.operations.insert(0, record2);

        let last = history.get_last_operation().unwrap();
        assert_eq!(last.operation_type, OperationType::Push);
        assert_eq!(last.branch, Some("develop".to_string()));
    }

    #[test]
    fn test_operation_history_get_last_operation_by_type() {
        let mut history = OperationHistory::new();

        let record1 = OperationRecord::new(OperationType::Pull, Some("main".to_string()), vec![]);

        let record2 =
            OperationRecord::new(OperationType::Push, Some("develop".to_string()), vec![]);

        let record3 =
            OperationRecord::new(OperationType::Pull, Some("feature".to_string()), vec![]);

        history.operations.push(record1);
        history.operations.insert(0, record2);
        history.operations.insert(0, record3);

        let last_pull = history
            .get_last_operation_by_type(OperationType::Pull)
            .unwrap();
        assert_eq!(last_pull.branch, Some("feature".to_string()));

        let last_push = history
            .get_last_operation_by_type(OperationType::Push)
            .unwrap();
        assert_eq!(last_push.branch, Some("develop".to_string()));
    }

    #[test]
    fn test_operation_history_list_operations() {
        let mut history = OperationHistory::new();

        let record1 = OperationRecord::new(OperationType::Pull, None, vec![]);
        let record2 = OperationRecord::new(OperationType::Push, None, vec![]);

        history.operations.push(record1);
        history.operations.insert(0, record2);

        let operations = history.list_operations();
        assert_eq!(operations.len(), 2);
        assert_eq!(operations[0].operation_type, OperationType::Push);
        assert_eq!(operations[1].operation_type, OperationType::Pull);
    }

    #[test]
    fn test_operation_history_clear() {
        let (_temp_dir, path) = setup_test_env();

        let mut history = OperationHistory::new();

        let record = OperationRecord::new(OperationType::Push, None, vec![]);
        history.operations.push(record);

        assert_eq!(history.len(), 1);

        history.save_to(Some(path.clone())).unwrap();

        history.clear().unwrap();
        assert_eq!(history.len(), 0);
        assert!(history.is_empty());

        history.save_to(Some(path.clone())).unwrap();

        let loaded = OperationHistory::from_path(Some(path)).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_operation_history_serialization() {
        let conversations = vec![ConversationSummary::new(
            "session-1".to_string(),
            "path/1".to_string(),
            Some("2025-01-15T10:00:00Z".to_string()),
            5,
            SyncOperation::Added,
        )
        .unwrap()];

        let record =
            OperationRecord::new(OperationType::Push, Some("main".to_string()), conversations);

        let mut history = OperationHistory::new();
        history.operations.push(record);

        let json = serde_json::to_string(&history).unwrap();
        let deserialized: OperationHistory = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.len(), 1);
        let op = deserialized.get_last_operation().unwrap();
        assert_eq!(op.operation_type, OperationType::Push);
        assert_eq!(op.branch, Some("main".to_string()));
        assert_eq!(op.affected_conversations.len(), 1);
        assert_eq!(op.affected_conversations[0].operation, SyncOperation::Added);
    }

    #[test]
    fn test_operation_history_default() {
        let history = OperationHistory::default();
        assert!(history.is_empty());
    }

    #[test]
    fn test_empty_history_last_operation() {
        let history = OperationHistory::new();
        assert!(history.get_last_operation().is_none());
        assert!(history
            .get_last_operation_by_type(OperationType::Pull)
            .is_none());
        assert!(history
            .get_last_operation_by_type(OperationType::Push)
            .is_none());
    }

    #[test]
    fn test_error_messages_include_file_paths() {
        let (_temp_dir, path) = setup_test_env();

        fs::write(&path, "{ invalid json }").unwrap();

        let result = OperationHistory::from_path(Some(path.clone()));
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains(&path.display().to_string()));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let readonly_dir = _temp_dir.path().join("readonly");
            fs::create_dir(&readonly_dir).unwrap();
            let readonly_path = readonly_dir.join("history.json");

            let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&readonly_dir, perms).unwrap();

            let history = OperationHistory::new();
            let result = history.save_to(Some(readonly_path.clone()));
            if let Err(err) = result {
                let error_msg = err.to_string();
                assert!(
                    error_msg.contains("history")
                        || error_msg.contains(&readonly_path.display().to_string())
                );
            }
        }
    }

    #[test]
    fn test_from_path_creates_new_when_missing() {
        let (_temp_dir, path) = setup_test_env();

        assert!(!path.exists());

        let history = OperationHistory::from_path(Some(path)).unwrap();
        assert!(history.is_empty());
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_operation_history_maintains_order() {
        let mut history = OperationHistory::new();

        for i in 0..3 {
            let record = OperationRecord::new(
                if i % 2 == 0 {
                    OperationType::Pull
                } else {
                    OperationType::Push
                },
                Some(format!("branch-{i}")),
                vec![],
            );
            history.operations.insert(0, record);
        }

        let operations = history.list_operations();
        assert_eq!(operations[0].branch, Some("branch-2".to_string()));
        assert_eq!(operations[1].branch, Some("branch-1".to_string()));
        assert_eq!(operations[2].branch, Some("branch-0".to_string()));
    }

    #[test]
    fn test_remove_last_operation_by_type() {
        let (_temp_dir, path) = setup_test_env();
        let mut history = OperationHistory::new();

        let record1 =
            OperationRecord::new(OperationType::Pull, Some("branch-1".to_string()), vec![]);
        let record2 =
            OperationRecord::new(OperationType::Push, Some("branch-2".to_string()), vec![]);
        let record3 =
            OperationRecord::new(OperationType::Pull, Some("branch-3".to_string()), vec![]);

        history.operations.push(record1);
        history.operations.insert(0, record2);
        history.operations.insert(0, record3);

        assert_eq!(history.len(), 3);

        let removed = history
            .remove_last_operation_by_type(OperationType::Pull, Some(path.clone()))
            .unwrap();
        assert!(removed);
        assert_eq!(history.len(), 2);

        let operations = history.list_operations();
        assert_eq!(operations[0].branch, Some("branch-2".to_string()));
        assert_eq!(operations[1].branch, Some("branch-1".to_string()));

        let removed = history
            .remove_last_operation_by_type(OperationType::Push, Some(path.clone()))
            .unwrap();
        assert!(removed);
        assert_eq!(history.len(), 1);

        let operations = history.list_operations();
        assert_eq!(operations[0].branch, Some("branch-1".to_string()));

        let removed = history
            .remove_last_operation_by_type(OperationType::Push, Some(path.clone()))
            .unwrap();
        assert!(!removed);
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_remove_last_operation_by_type_persists() {
        let (_temp_dir, path) = setup_test_env();
        let mut history = OperationHistory::new();

        let record1 = OperationRecord::new(OperationType::Pull, Some("main".to_string()), vec![]);
        let record2 =
            OperationRecord::new(OperationType::Push, Some("develop".to_string()), vec![]);

        history.operations.push(record1);
        history.operations.insert(0, record2);

        history.save_to(Some(path.clone())).unwrap();

        let removed = history
            .remove_last_operation_by_type(OperationType::Push, Some(path.clone()))
            .unwrap();
        assert!(removed);

        let loaded = OperationHistory::from_path(Some(path)).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.operations[0].operation_type, OperationType::Pull);
    }

    #[test]
    fn test_max_history_size_constant() {
        assert_eq!(MAX_HISTORY_SIZE, 5);
    }
}
