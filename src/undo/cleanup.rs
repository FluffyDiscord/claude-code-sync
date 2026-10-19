use anyhow::Result;
use log::warn;
use std::fs;
use std::path::Path;

use super::snapshot::Snapshot;
use crate::history::OperationType;

pub struct SnapshotCleanupConfig {
    pub max_count_per_type: usize,
    pub max_age_days: i64,
}

impl Default for SnapshotCleanupConfig {
    fn default() -> Self {
        Self {
            max_count_per_type: 5,
            max_age_days: 7,
        }
    }
}

pub fn cleanup_old_snapshots_with_dir(
    config: Option<SnapshotCleanupConfig>,
    dry_run: bool,
    snapshots_dir: Option<&Path>,
) -> Result<usize> {
    let config = config.unwrap_or_default();
    let snapshots_dir = if let Some(dir) = snapshots_dir {
        dir.to_path_buf()
    } else {
        Snapshot::snapshots_dir()?
    };

    if !snapshots_dir.exists() {
        return Ok(0);
    }

    let mut pull_snapshots: Vec<(std::path::PathBuf, chrono::DateTime<chrono::Utc>)> = Vec::new();
    let mut push_snapshots: Vec<(std::path::PathBuf, chrono::DateTime<chrono::Utc>)> = Vec::new();

    for entry in fs::read_dir(&snapshots_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(snapshot) = serde_json::from_str::<Snapshot>(&content) {
                match snapshot.operation_type {
                    OperationType::Pull => pull_snapshots.push((path, snapshot.timestamp)),
                    OperationType::Push => push_snapshots.push((path, snapshot.timestamp)),
                }
            }
        }
    }

    pull_snapshots.sort_by_key(|s| std::cmp::Reverse(s.1));
    push_snapshots.sort_by_key(|s| std::cmp::Reverse(s.1));

    let now = chrono::Utc::now();
    let age_threshold = now - chrono::Duration::days(config.max_age_days);

    let mut to_delete = Vec::new();

    for (idx, (path, timestamp)) in pull_snapshots.iter().enumerate() {
        let within_count_limit = idx < config.max_count_per_type;
        let within_age_limit = *timestamp >= age_threshold;

        if !within_count_limit && !within_age_limit {
            to_delete.push(path.clone());
        }
    }

    for (idx, (path, timestamp)) in push_snapshots.iter().enumerate() {
        let within_count_limit = idx < config.max_count_per_type;
        let within_age_limit = *timestamp >= age_threshold;

        if !within_count_limit && !within_age_limit {
            to_delete.push(path.clone());
        }
    }

    let deleted_count = to_delete.len();

    if dry_run {
        println!("Would delete {} snapshots:", deleted_count);
        for path in &to_delete {
            println!("  - {}", path.display());
        }
    } else {
        for path in &to_delete {
            if let Err(e) = fs::remove_file(path) {
                warn!("Failed to delete snapshot {}: {}", path.display(), e);
            }
        }
    }

    Ok(deleted_count)
}

pub fn cleanup_old_snapshots(
    config: Option<SnapshotCleanupConfig>,
    dry_run: bool,
) -> Result<usize> {
    cleanup_old_snapshots_with_dir(config, dry_run, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::undo::test_support::metadata_only_snapshot;
    use chrono::Duration;
    use tempfile::tempdir;

    fn snapshots_aged_by_day(dir: &Path, count: i64, operation_type: OperationType) {
        fs::create_dir_all(dir).unwrap();
        for i in 0..count {
            let id = format!("{operation_type:?}_{i}").to_lowercase();
            metadata_only_snapshot(&id, operation_type, Duration::days(i))
                .save_to_disk(Some(dir))
                .unwrap();
        }
    }

    #[test]
    fn test_cleanup_snapshots_respects_count_limit() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        snapshots_aged_by_day(&snapshots_dir, 10, OperationType::Pull);

        let config = SnapshotCleanupConfig {
            max_count_per_type: 5,
            max_age_days: 0,
        };

        let deleted =
            cleanup_old_snapshots_with_dir(Some(config), false, Some(&snapshots_dir)).unwrap();
        assert_eq!(deleted, 5, "Should delete 5 old snapshots");

        let remaining = fs::read_dir(&snapshots_dir).unwrap().count();
        assert_eq!(remaining, 5, "Should have 5 snapshots remaining");
    }

    #[test]
    fn test_cleanup_snapshots_respects_age_limit() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        fs::create_dir_all(&snapshots_dir).unwrap();

        for i in 0..10 {
            metadata_only_snapshot(
                &format!("snapshot_{i}"),
                OperationType::Pull,
                Duration::days(5 + i * 10),
            )
            .save_to_disk(Some(&snapshots_dir))
            .unwrap();
        }

        let config = SnapshotCleanupConfig {
            max_count_per_type: 0,
            max_age_days: 50,
        };

        let deleted =
            cleanup_old_snapshots_with_dir(Some(config), false, Some(&snapshots_dir)).unwrap();
        assert_eq!(deleted, 5, "Should delete 5 old snapshots");

        let remaining = fs::read_dir(&snapshots_dir).unwrap().count();
        assert_eq!(remaining, 5, "Should have 5 snapshots remaining");
    }

    #[test]
    fn test_cleanup_snapshots_separates_operation_types() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        fs::create_dir_all(&snapshots_dir).unwrap();

        for i in 0..10 {
            metadata_only_snapshot(&format!("pull_{i}"), OperationType::Pull, Duration::days(i))
                .save_to_disk(Some(&snapshots_dir))
                .unwrap();

            let mut push = metadata_only_snapshot(
                &format!("push_{i}"),
                OperationType::Push,
                Duration::days(i),
            );
            push.git_commit_hash = Some(format!("hash_{i}"));
            push.branch = Some("main".to_string());
            push.save_to_disk(Some(&snapshots_dir)).unwrap();
        }

        let config = SnapshotCleanupConfig {
            max_count_per_type: 3,
            max_age_days: 0,
        };

        let deleted =
            cleanup_old_snapshots_with_dir(Some(config), false, Some(&snapshots_dir)).unwrap();

        assert_eq!(deleted, 14, "Should delete 14 old snapshots");

        let remaining = fs::read_dir(&snapshots_dir).unwrap().count();
        assert_eq!(
            remaining, 6,
            "Should have 6 snapshots remaining (3 per type)"
        );
    }

    #[test]
    fn test_cleanup_snapshots_dry_run() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        snapshots_aged_by_day(&snapshots_dir, 10, OperationType::Pull);

        let config = SnapshotCleanupConfig {
            max_count_per_type: 3,
            max_age_days: 0,
        };

        let deleted =
            cleanup_old_snapshots_with_dir(Some(config), true, Some(&snapshots_dir)).unwrap();
        assert_eq!(deleted, 7, "Should report 7 snapshots would be deleted");

        let remaining = fs::read_dir(&snapshots_dir).unwrap().count();
        assert_eq!(
            remaining, 10,
            "All snapshots should still exist after dry run"
        );
    }
}
