use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::snapshot::Snapshot;
use crate::history::OperationType;

impl Snapshot {
    pub fn create_differential_with_dir<P, I>(
        operation_type: OperationType,
        file_paths: I,
        commit_hash: Option<&str>,
        snapshots_dir: Option<&Path>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        let base_snapshot = Self::find_latest_snapshot(operation_type, snapshots_dir)?;

        let snapshot_id = Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now();

        let mut current_files: HashMap<String, Vec<u8>> = HashMap::new();
        for path in file_paths {
            let path = path.as_ref();
            match fs::read(path) {
                Ok(content) => {
                    current_files.insert(path.to_string_lossy().to_string(), content);
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

        let (files, base_snapshot_id, deleted_files) = if let Some(base) = base_snapshot {
            let mut changed_files = HashMap::new();
            let mut deleted = Vec::new();

            let base_full_state = base.reconstruct_full_state_with_dir(snapshots_dir)?;

            for (path, content) in &current_files {
                if let Some(base_content) = base_full_state.get(path) {
                    if base_content != content {
                        changed_files.insert(path.clone(), content.clone());
                    }
                } else {
                    changed_files.insert(path.clone(), content.clone());
                }
            }

            for path in base_full_state.keys() {
                if !current_files.contains_key(path) {
                    deleted.push(path.clone());
                }
            }

            (changed_files, Some(base.snapshot_id), deleted)
        } else {
            (current_files, None, Vec::new())
        };

        Ok(Snapshot {
            snapshot_id,
            timestamp,
            operation_type,
            git_commit_hash: commit_hash.map(|s| s.to_string()),
            files,
            branch: None,
            base_snapshot_id,
            deleted_files,
        })
    }

    pub fn create_differential<P, I>(
        operation_type: OperationType,
        file_paths: I,
        commit_hash: Option<&str>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        Self::create_differential_with_dir(operation_type, file_paths, commit_hash, None)
    }

    pub fn create_differential_with_commit<P, I>(
        operation_type: OperationType,
        file_paths: I,
        commit_hash: Option<&str>,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        Self::create_differential(operation_type, file_paths, commit_hash)
    }

    pub(crate) fn find_latest_snapshot(
        operation_type: OperationType,
        custom_dir: Option<&Path>,
    ) -> Result<Option<Snapshot>> {
        let snapshots_dir = if let Some(dir) = custom_dir {
            dir.to_path_buf()
        } else {
            Self::snapshots_dir()?
        };

        if !snapshots_dir.exists() {
            return Ok(None);
        }

        let mut snapshots: Vec<(PathBuf, chrono::DateTime<chrono::Utc>)> = Vec::new();

        for entry in fs::read_dir(&snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(snapshot) = serde_json::from_str::<Snapshot>(&content) {
                    if snapshot.operation_type == operation_type {
                        snapshots.push((path, snapshot.timestamp));
                    }
                }
            }
        }

        snapshots.sort_by_key(|s| std::cmp::Reverse(s.1));

        if let Some((path, _)) = snapshots.first() {
            let snapshot = Self::load_from_disk(path)?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    pub fn reconstruct_full_state_with_dir(
        &self,
        snapshots_dir: Option<&Path>,
    ) -> Result<HashMap<String, Vec<u8>>> {
        let mut state = HashMap::new();

        if let Some(base_id) = &self.base_snapshot_id {
            let snapshots_dir = if let Some(dir) = snapshots_dir {
                dir.to_path_buf()
            } else {
                Self::snapshots_dir()?
            };
            let base_path = snapshots_dir.join(format!("{}.json", base_id));

            if !base_path.exists() {
                return Err(anyhow!(
                    "Base snapshot not found: {}. \
                    The snapshot chain is broken. Cannot restore differential snapshot.",
                    base_id
                ));
            }

            let base_snapshot = Self::load_from_disk(&base_path)?;
            state = base_snapshot.reconstruct_full_state_with_dir(Some(&snapshots_dir))?;
        }

        for (path, content) in &self.files {
            state.insert(path.clone(), content.clone());
        }

        for deleted_path in &self.deleted_files {
            state.remove(deleted_path);
        }

        Ok(state)
    }

    #[allow(dead_code)]
    pub(crate) fn reconstruct_full_state(&self) -> Result<HashMap<String, Vec<u8>>> {
        self.reconstruct_full_state_with_dir(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn differential(snapshots_dir: &Path, files: &[&PathBuf]) -> Snapshot {
        let snapshot = Snapshot::create_differential_with_dir(
            OperationType::Pull,
            files.iter().copied(),
            None,
            Some(snapshots_dir),
        )
        .unwrap();
        snapshot.save_to_disk(Some(snapshots_dir)).unwrap();
        snapshot
    }

    #[test]
    fn test_differential_snapshot_first_snapshot_is_full() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, b"content1").unwrap();
        fs::write(&file2, b"content2").unwrap();

        let snapshot = differential(&snapshots_dir, &[&file1, &file2]);

        assert!(
            snapshot.base_snapshot_id.is_none(),
            "First snapshot should not have a base"
        );
        assert_eq!(
            snapshot.files.len(),
            2,
            "First snapshot should contain all files"
        );
        assert!(
            snapshot.deleted_files.is_empty(),
            "First snapshot should have no deleted files"
        );
    }

    #[test]
    fn test_differential_snapshot_only_stores_changes() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let file3 = temp_dir.path().join("file3.txt");

        fs::write(&file1, b"content1").unwrap();
        fs::write(&file2, b"content2").unwrap();
        let snapshot1 = differential(&snapshots_dir, &[&file1, &file2]);

        fs::write(&file1, b"modified_content1").unwrap();
        fs::write(&file3, b"content3").unwrap();
        let snapshot2 = differential(&snapshots_dir, &[&file1, &file2, &file3]);

        assert_eq!(
            snapshot2.base_snapshot_id.as_ref().unwrap(),
            &snapshot1.snapshot_id,
            "Base should be the first snapshot"
        );

        assert_eq!(
            snapshot2.files.len(),
            2,
            "Should only contain changed and new files"
        );
        assert!(snapshot2
            .files
            .contains_key(&file1.to_string_lossy().to_string()));
        assert!(snapshot2
            .files
            .contains_key(&file3.to_string_lossy().to_string()));
        assert!(
            !snapshot2
                .files
                .contains_key(&file2.to_string_lossy().to_string()),
            "unchanged file must not be stored again"
        );
    }

    #[test]
    fn test_differential_snapshot_tracks_deletions() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, b"content1").unwrap();
        fs::write(&file2, b"content2").unwrap();
        differential(&snapshots_dir, &[&file1, &file2]);

        fs::remove_file(&file2).unwrap();
        let snapshot2 = differential(&snapshots_dir, &[&file1]);

        assert_eq!(snapshot2.deleted_files.len(), 1);
        assert!(snapshot2
            .deleted_files
            .contains(&file2.to_string_lossy().to_string()));
    }

    #[test]
    fn test_differential_snapshot_reconstruction() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");
        let file3 = temp_dir.path().join("file3.txt");

        fs::write(&file1, b"v1").unwrap();
        fs::write(&file2, b"v1").unwrap();
        differential(&snapshots_dir, &[&file1, &file2]);

        fs::write(&file1, b"v2").unwrap();
        fs::write(&file3, b"v2").unwrap();
        let snapshot2 = differential(&snapshots_dir, &[&file1, &file2, &file3]);

        let full_state = snapshot2
            .reconstruct_full_state_with_dir(Some(&snapshots_dir))
            .unwrap();

        assert_eq!(full_state.len(), 3);
        let at = |p: &PathBuf| full_state.get(&p.to_string_lossy().to_string()).unwrap();
        assert_eq!(at(&file1), b"v2", "changed file takes the new content");
        assert_eq!(at(&file2), b"v1", "unchanged file comes from the base");
        assert_eq!(at(&file3), b"v2", "new file comes from this snapshot");
    }

    #[test]
    fn test_differential_snapshot_broken_chain() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        let file1 = temp_dir.path().join("file1.txt");

        fs::write(&file1, b"content1").unwrap();

        let mut snapshot = Snapshot::create_differential_with_dir(
            OperationType::Pull,
            vec![&file1],
            None,
            Some(&snapshots_dir),
        )
        .unwrap();

        snapshot.base_snapshot_id = Some("non-existent-base-id".to_string());
        snapshot.save_to_disk(Some(&snapshots_dir)).unwrap();

        let result = snapshot.reconstruct_full_state();
        assert!(result.is_err(), "Should fail when base snapshot is missing");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Base snapshot not found"));
    }

    #[test]
    fn test_differential_snapshot_long_chain() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        let file1 = temp_dir.path().join("file1.txt");

        let mut snapshots = Vec::new();
        for i in 1..=5 {
            fs::write(&file1, format!("version_{i}").as_bytes()).unwrap();
            snapshots.push(differential(&snapshots_dir, &[&file1]));
        }

        assert!(
            snapshots[0].base_snapshot_id.is_none(),
            "First should have no base"
        );
        for i in 1..5 {
            assert_eq!(
                snapshots[i].base_snapshot_id.as_ref().unwrap(),
                &snapshots[i - 1].snapshot_id,
                "Snapshot {i} should reference snapshot {}",
                i - 1
            );
        }

        let full_state = snapshots[4]
            .reconstruct_full_state_with_dir(Some(&snapshots_dir))
            .unwrap();
        assert_eq!(
            full_state
                .get(&file1.to_string_lossy().to_string())
                .unwrap(),
            b"version_5"
        );
    }
}
