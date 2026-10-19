use anyhow::{anyhow, Context, Result};
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use super::snapshot::Snapshot;

fn resolve_without_creating(path: &Path) -> Result<PathBuf> {
    if path.components().any(|c| c == Component::ParentDir) {
        return Err(anyhow!(
            "Security: Path traversal detected. Path {} contains '..'",
            path.display()
        ));
    }

    let mut existing = path;
    let mut tail: Vec<&OsStr> = Vec::new();

    while existing.symlink_metadata().is_err() {
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                tail.push(name);
                existing = parent;
            }
            _ => {
                return Err(anyhow!(
                    "Cannot resolve path for restore: {}",
                    path.display()
                ))
            }
        }
    }

    let mut resolved = existing.canonicalize().with_context(|| {
        format!(
            "Failed to canonicalize {} while restoring {}",
            existing.display(),
            path.display()
        )
    })?;

    for name in tail.iter().rev() {
        resolved.push(name);
    }

    Ok(resolved)
}

impl Snapshot {
    pub fn restore_with_base_and_snapshots(
        &self,
        allowed_base_dir: Option<&Path>,
        snapshots_dir: Option<&Path>,
    ) -> Result<()> {
        let allowed_base = if let Some(base) = allowed_base_dir {
            base.canonicalize().with_context(|| {
                format!("Failed to canonicalize base directory: {}", base.display())
            })?
        } else {
            let home_dir = dirs::home_dir().context("Failed to get home directory")?;
            home_dir
                .canonicalize()
                .context("Failed to canonicalize home directory")?
        };

        let all_files = self.reconstruct_full_state_with_dir(snapshots_dir)?;

        for deleted_path in &self.deleted_files {
            let path = PathBuf::from(deleted_path);

            if let Ok(canonical) = path.canonicalize() {
                if canonical.starts_with(&allowed_base) && path.exists() {
                    fs::remove_file(&path)
                        .with_context(|| format!("Failed to delete file: {}", path.display()))?;
                }
            }
        }

        for (path_str, content) in &all_files {
            let path = PathBuf::from(path_str);

            let target = resolve_without_creating(&path)?;

            if !target.starts_with(&allowed_base) {
                return Err(anyhow!(
                    "Security: Path traversal detected. Path {} is outside allowed directory {}",
                    path.display(),
                    allowed_base.display()
                ));
            }

            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }

            fs::write(&target, content)
                .with_context(|| format!("Failed to restore file: {}", target.display()))?;
        }

        Ok(())
    }

    pub fn restore_with_base(&self, allowed_base_dir: Option<&Path>) -> Result<()> {
        self.restore_with_base_and_snapshots(allowed_base_dir, None)
    }

    pub fn restore(&self) -> Result<()> {
        self.restore_with_base(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::OperationType;
    use crate::undo::test_support::{create_test_file, metadata_only_snapshot};
    use chrono::Duration;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn test_snapshot_restore() {
        let temp_dir = tempdir().unwrap();
        let file1 = create_test_file(temp_dir.path(), "file1.txt", "original content");

        let snapshot = Snapshot::create(OperationType::Pull, vec![&file1], None).unwrap();

        fs::write(&file1, "modified content").unwrap();
        assert_eq!(fs::read_to_string(&file1).unwrap(), "modified content");

        snapshot.restore_with_base(Some(temp_dir.path())).unwrap();

        assert_eq!(fs::read_to_string(&file1).unwrap(), "original content");
    }

    #[test]
    fn test_snapshot_restores_file_hierarchy() {
        let temp_dir = tempdir().unwrap();

        let nested_dir = temp_dir.path().join("dir1").join("dir2");
        fs::create_dir_all(&nested_dir).unwrap();
        let nested_file = nested_dir.join("deep.txt");
        fs::write(&nested_file, "deep content").unwrap();

        let snapshot = Snapshot::create(OperationType::Pull, vec![&nested_file], None).unwrap();

        fs::remove_dir_all(temp_dir.path().join("dir1")).unwrap();
        assert!(!nested_file.exists());

        snapshot.restore_with_base(Some(temp_dir.path())).unwrap();

        assert!(nested_file.exists());
        assert_eq!(fs::read_to_string(&nested_file).unwrap(), "deep content");
    }

    #[test]
    fn test_snapshot_path_traversal_protection() {
        let mut malicious_snapshot = metadata_only_snapshot(
            &Uuid::new_v4().to_string(),
            OperationType::Pull,
            Duration::zero(),
        );

        let home = dirs::home_dir().unwrap();
        let evil_path = home.join("..").join("..").join("etc").join("passwd");

        malicious_snapshot.files.insert(
            evil_path.to_string_lossy().to_string(),
            b"malicious content".to_vec(),
        );

        let result = malicious_snapshot.restore();

        if let Err(err) = result {
            let err_msg = err.to_string();
            assert!(
                err_msg.contains("Security") || err_msg.contains("outside home"),
                "Error message should indicate security issue: {err_msg}"
            );
        } else {
            assert!(
                !PathBuf::from("/etc/passwd").exists()
                    || !fs::read_to_string("/etc/passwd")
                        .unwrap_or_default()
                        .contains("malicious")
            );
        }
    }

    #[test]
    fn test_differential_snapshot_restore_with_deletions() {
        let temp_dir = tempdir().unwrap();
        let snapshots_dir = temp_dir.path().join("snapshots");
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, b"content1").unwrap();
        fs::write(&file2, b"content2").unwrap();

        let snapshot1 = Snapshot::create_differential_with_dir(
            OperationType::Pull,
            vec![&file1, &file2],
            None,
            Some(&snapshots_dir),
        )
        .unwrap();
        snapshot1.save_to_disk(Some(&snapshots_dir)).unwrap();

        fs::remove_file(&file2).unwrap();

        let snapshot2 = Snapshot::create_differential_with_dir(
            OperationType::Pull,
            vec![&file1],
            None,
            Some(&snapshots_dir),
        )
        .unwrap();
        snapshot2.save_to_disk(Some(&snapshots_dir)).unwrap();

        fs::write(&file2, b"should_be_deleted").unwrap();

        snapshot2
            .restore_with_base_and_snapshots(Some(temp_dir.path()), Some(&snapshots_dir))
            .unwrap();

        assert!(file1.exists(), "file1 should exist after restore");
        assert!(!file2.exists(), "file2 should be deleted after restore");
    }

    #[test]
    fn test_restore_rejects_traversal_without_creating_anything() {
        let temp_dir = tempdir().unwrap();
        let allowed = temp_dir.path().join("allowed");
        fs::create_dir_all(&allowed).unwrap();

        let outside = temp_dir
            .path()
            .join("outside")
            .join("deep")
            .join("evil.txt");

        let mut snapshot = metadata_only_snapshot(
            &Uuid::new_v4().to_string(),
            OperationType::Pull,
            Duration::zero(),
        );
        snapshot.files.insert(
            outside.to_string_lossy().to_string(),
            b"malicious content".to_vec(),
        );

        let err = snapshot
            .restore_with_base(Some(&allowed))
            .expect_err("restore must reject a path outside the allowed base")
            .to_string();
        assert!(
            err.contains("Security") || err.contains("outside"),
            "unexpected error: {err}"
        );

        assert!(!outside.exists(), "the rejected file must not be created");
        assert!(
            !outside.parent().unwrap().exists(),
            "the rejected file's parent directories must not be created either"
        );
    }

    #[test]
    fn test_restore_rejects_dotdot_in_a_path_that_does_not_exist() {
        let temp_dir = tempdir().unwrap();
        let allowed = temp_dir.path().join("allowed");
        fs::create_dir_all(&allowed).unwrap();

        let escape = allowed
            .join("nope")
            .join("..")
            .join("..")
            .join("escape.txt");

        let mut snapshot = metadata_only_snapshot(
            &Uuid::new_v4().to_string(),
            OperationType::Pull,
            Duration::zero(),
        );
        snapshot
            .files
            .insert(escape.to_string_lossy().to_string(), b"escaped".to_vec());

        let err = snapshot
            .restore_with_base(Some(&allowed))
            .expect_err("restore must reject '..' in an unresolvable tail")
            .to_string();
        assert!(err.contains("Security"), "unexpected error: {err}");

        assert!(!temp_dir.path().join("escape.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_restore_does_not_follow_a_dangling_symlink_out_of_the_base() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempdir().unwrap();
        let allowed = temp_dir.path().join("allowed");
        fs::create_dir_all(&allowed).unwrap();

        let target_outside = temp_dir.path().join("victim.txt");
        let link = allowed.join("innocent.txt");
        symlink(&target_outside, &link).unwrap();
        assert!(!link.exists(), "symlink should be dangling");
        assert!(link.symlink_metadata().is_ok(), "but it is still an entry");

        let mut snapshot = metadata_only_snapshot(
            &Uuid::new_v4().to_string(),
            OperationType::Pull,
            Duration::zero(),
        );
        snapshot
            .files
            .insert(link.to_string_lossy().to_string(), b"pwned".to_vec());

        let _ = snapshot.restore_with_base(Some(&allowed));

        assert!(
            !target_outside.exists(),
            "restore must not write through a dangling symlink to {}",
            target_outside.display()
        );
    }
}
