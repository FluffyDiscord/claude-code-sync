use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::history::OperationType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: String,

    pub timestamp: chrono::DateTime<chrono::Utc>,

    pub operation_type: OperationType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit_hash: Option<String>,

    #[serde(with = "base64_map")]
    pub files: HashMap<String, Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_snapshot_id: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deleted_files: Vec<String>,
}

mod base64_map {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(map: &HashMap<String, Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let base64_map: HashMap<String, String> = map
            .iter()
            .map(|(k, v)| (k.clone(), STANDARD.encode(v)))
            .collect();
        base64_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let base64_map: HashMap<String, String> = HashMap::deserialize(deserializer)?;
        base64_map
            .into_iter()
            .map(|(k, v)| {
                STANDARD
                    .decode(&v)
                    .map(|bytes| (k, bytes))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

impl Snapshot {
    pub fn create<P, I>(
        operation_type: OperationType,
        file_paths: I,
        commit_hash: Option<&str>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        let snapshot_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now();
        let mut files = HashMap::new();

        for path in file_paths {
            let path = path.as_ref();

            match fs::read(path) {
                Ok(content) => {
                    files.insert(path.to_string_lossy().to_string(), content);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    continue;
                }
                Err(e) => {
                    return Err(e).with_context(|| {
                        format!("Failed to read file for snapshot: {}", path.display())
                    });
                }
            }
        }

        Ok(Snapshot {
            snapshot_id,
            timestamp,
            operation_type,
            git_commit_hash: commit_hash.map(|s| s.to_string()),
            files,
            branch: None,
            base_snapshot_id: None,
            deleted_files: Vec::new(),
        })
    }

    pub fn save_to_disk(&self, custom_path: Option<&Path>) -> Result<PathBuf> {
        let snapshot_dir = if let Some(path) = custom_path {
            path.to_path_buf()
        } else {
            Self::snapshots_dir()?
        };

        fs::create_dir_all(&snapshot_dir).with_context(|| {
            format!(
                "Failed to create snapshots directory: {}",
                snapshot_dir.display()
            )
        })?;

        let snapshot_path = snapshot_dir.join(format!("{}.json", self.snapshot_id));

        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize snapshot to JSON")?;

        fs::write(&snapshot_path, &json).with_context(|| {
            format!(
                "Failed to write snapshot to disk: {}",
                snapshot_path.display()
            )
        })?;

        let size_mb = json.len() as f64 / (1024.0 * 1024.0);
        let snapshot_type = if self.base_snapshot_id.is_some() {
            "differential"
        } else {
            "full"
        };

        log::debug!(
            "Created {} snapshot: {} ({:.1} MB, {} files)",
            snapshot_type,
            self.snapshot_id,
            size_mb,
            self.files.len()
        );

        if size_mb > 100.0 {
            log::debug!("Large snapshot size - consider cleaning up old conversation files");
        }

        Ok(snapshot_path)
    }

    pub fn load_from_disk<P: AsRef<Path>>(snapshot_path: P) -> Result<Self> {
        let snapshot_path = snapshot_path.as_ref();

        let metadata = fs::metadata(snapshot_path).with_context(|| {
            format!(
                "Failed to read snapshot file metadata: {}",
                snapshot_path.display()
            )
        })?;

        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

        if size_mb > 100.0 {
            println!(
                "    {} Loading large snapshot: {} ({:.1} MB) - This may take a moment...",
                "⚠".yellow(),
                snapshot_path.file_name().unwrap().to_string_lossy().cyan(),
                size_mb
            );
        } else {
            println!(
                "    {} snapshot: {} ({:.1} MB)",
                "Loading".dimmed(),
                snapshot_path.file_name().unwrap().to_string_lossy().cyan(),
                size_mb
            );
        }

        let content = fs::read_to_string(snapshot_path).with_context(|| {
            format!("Failed to read snapshot file: {}", snapshot_path.display())
        })?;

        let snapshot: Snapshot = serde_json::from_str(&content).with_context(|| {
            format!("Failed to parse snapshot JSON: {}", snapshot_path.display())
        })?;

        println!("    {} {} files", "Contains".dimmed(), snapshot.files.len());

        if snapshot.files.len() > 1000 {
            println!(
                "    {} Large number of files - this is a full (non-differential) snapshot",
                "Note:".yellow()
            );
        }

        Ok(snapshot)
    }

    pub(crate) fn snapshots_dir() -> Result<PathBuf> {
        crate::config::ConfigManager::snapshots_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::undo::test_support::{create_test_file, setup_test_repo};
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_create_and_save() {
        let temp_dir = tempdir().unwrap();
        let file1 = create_test_file(temp_dir.path(), "file1.txt", "content 1");
        let file2 = create_test_file(temp_dir.path(), "file2.txt", "content 2");

        let snapshot = Snapshot::create(OperationType::Pull, vec![&file1, &file2], None).unwrap();

        assert_eq!(snapshot.operation_type, OperationType::Pull);
        assert_eq!(snapshot.files.len(), 2);
        assert!(snapshot.git_commit_hash.is_none());

        let snapshots_dir = temp_dir.path().join("snapshots");
        let snapshot_path = snapshot.save_to_disk(Some(&snapshots_dir)).unwrap();
        assert!(snapshot_path.exists());
    }

    #[test]
    fn test_snapshot_with_commit_hash() {
        let (temp_dir, repo) = setup_test_repo();
        let file1 = temp_dir.path().join("test.txt");
        let commit_hash = repo.current_commit_hash().unwrap();

        let snapshot =
            Snapshot::create(OperationType::Push, vec![&file1], Some(&commit_hash)).unwrap();

        assert_eq!(snapshot.operation_type, OperationType::Push);
        let stored_hash = snapshot.git_commit_hash.unwrap();
        assert_eq!(stored_hash.len(), 40);
        assert_eq!(stored_hash, commit_hash);
    }

    #[test]
    fn test_snapshot_save_and_load() {
        let temp_dir = tempdir().unwrap();
        let file1 = create_test_file(temp_dir.path(), "file1.txt", "test content");

        let original = Snapshot::create(OperationType::Pull, vec![&file1], None).unwrap();

        let snapshots_dir = temp_dir.path().join("snapshots");
        let snapshot_path = original.save_to_disk(Some(&snapshots_dir)).unwrap();

        let loaded = Snapshot::load_from_disk(&snapshot_path).unwrap();

        assert_eq!(loaded.snapshot_id, original.snapshot_id);
        assert_eq!(loaded.operation_type, original.operation_type);
        assert_eq!(loaded.files.len(), original.files.len());
    }

    #[test]
    fn test_snapshot_handles_binary_files() {
        let temp_dir = tempdir().unwrap();
        let binary_file = temp_dir.path().join("binary.dat");

        let binary_content: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x01, 0x02, 0x03];
        fs::write(&binary_file, &binary_content).unwrap();

        let snapshot = Snapshot::create(OperationType::Pull, vec![&binary_file], None).unwrap();

        let key = binary_file.to_string_lossy().to_string();
        assert_eq!(snapshot.files.get(&key).unwrap(), &binary_content);

        let snapshots_dir = temp_dir.path().join("snapshots");
        let snapshot_path = snapshot.save_to_disk(Some(&snapshots_dir)).unwrap();

        let loaded = Snapshot::load_from_disk(&snapshot_path).unwrap();
        assert_eq!(loaded.files.get(&key).unwrap(), &binary_content);
    }

    #[test]
    fn test_snapshot_serialization_with_special_characters() {
        let temp_dir = tempdir().unwrap();
        let file_with_unicode = temp_dir.path().join("日本語.txt");
        fs::write(&file_with_unicode, "Hello 世界").unwrap();

        let snapshot =
            Snapshot::create(OperationType::Pull, vec![&file_with_unicode], None).unwrap();

        let snapshots_dir = temp_dir.path().join("snapshots");
        let snapshot_path = snapshot.save_to_disk(Some(&snapshots_dir)).unwrap();

        let loaded = Snapshot::load_from_disk(&snapshot_path).unwrap();

        let content = loaded.files.values().next().unwrap();
        assert_eq!(String::from_utf8_lossy(content), "Hello 世界");
    }

    #[test]
    fn test_base64_encoding_for_binary_data() {
        let temp_dir = tempdir().unwrap();

        let binary_file = temp_dir.path().join("binary.dat");
        let binary_data: Vec<u8> = (0..=255).collect();
        fs::write(&binary_file, &binary_data).unwrap();

        let snapshot = Snapshot::create(OperationType::Pull, vec![&binary_file], None).unwrap();

        let json = serde_json::to_string(&snapshot).unwrap();
        let _parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        let deserialized: Snapshot = serde_json::from_str(&json).unwrap();

        let key = binary_file.to_string_lossy().to_string();
        assert_eq!(
            snapshot.files.get(&key).unwrap(),
            deserialized.files.get(&key).unwrap()
        );
        assert_eq!(deserialized.files.get(&key).unwrap(), &binary_data);
    }

    #[test]
    fn test_empty_snapshot() {
        let snapshot = Snapshot::create::<PathBuf, _>(OperationType::Pull, vec![], None).unwrap();

        assert_eq!(snapshot.files.len(), 0);
        assert!(snapshot.git_commit_hash.is_none());

        let temp_dir = tempdir().unwrap();
        let snapshot_path = snapshot.save_to_disk(Some(temp_dir.path())).unwrap();

        let loaded = Snapshot::load_from_disk(&snapshot_path).unwrap();
        assert_eq!(loaded.files.len(), 0);

        loaded.restore().unwrap();
    }

    #[test]
    fn test_snapshot_create_handles_missing_files() {
        let temp_dir = tempdir().unwrap();

        let existing_file = create_test_file(temp_dir.path(), "exists.txt", "content");
        let missing_file = temp_dir.path().join("does_not_exist.txt");

        let snapshot = Snapshot::create(
            OperationType::Pull,
            vec![&existing_file, &missing_file],
            None,
        )
        .unwrap();

        assert_eq!(snapshot.files.len(), 1);
        assert!(snapshot
            .files
            .contains_key(&existing_file.to_string_lossy().to_string()));
        assert!(!snapshot
            .files
            .contains_key(&missing_file.to_string_lossy().to_string()));
    }
}
