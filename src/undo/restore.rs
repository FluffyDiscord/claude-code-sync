use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
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

fn ensure_inside_base(path: &Path, target: &Path, allowed_base: &Path) -> Result<()> {
    if target.starts_with(allowed_base) {
        return Ok(());
    }
    Err(anyhow!(
        "Security: Path traversal detected. Path {} is outside allowed directory {}",
        path.display(),
        allowed_base.display()
    ))
}

impl Snapshot {
    pub fn restore_with_base(&self, allowed_base_dir: Option<&Path>) -> Result<()> {
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

        let mut targets: HashMap<&str, PathBuf> = HashMap::new();
        for path_str in self.deleted_files.iter().chain(self.files.keys()) {
            let path = Path::new(path_str);
            let target = resolve_without_creating(path)?;
            ensure_inside_base(path, &target, &allowed_base)?;
            targets.insert(path_str, target);
        }

        for path_str in self.list_files_changed_since_pull() {
            eprintln!("Warning: Kept {path_str}: it changed after the pull");
        }

        for path_str in self.list_created_files_to_remove() {
            fs::remove_file(&path_str)
                .with_context(|| format!("Failed to delete file: {path_str}"))?;
        }

        for path_str in self.list_files_to_restore() {
            let target = &targets[path_str.as_str()];
            let content = &self.files[&path_str];

            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }

            fs::write(target, content)
                .with_context(|| format!("Failed to restore file: {}", target.display()))?;
        }

        Ok(())
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
    fn test_restore_rejects_a_deletion_outside_the_base() {
        let temp_dir = tempdir().unwrap();
        let allowed = temp_dir.path().join("allowed");
        fs::create_dir_all(&allowed).unwrap();
        let outside = create_test_file(temp_dir.path(), "outside.txt", "keep me");

        let mut snapshot = metadata_only_snapshot(
            &Uuid::new_v4().to_string(),
            OperationType::Pull,
            Duration::zero(),
        );
        snapshot
            .deleted_files
            .push(outside.to_string_lossy().to_string());

        let err = snapshot
            .restore_with_base(Some(&allowed))
            .expect_err("restore must reject a deletion outside the allowed base")
            .to_string();
        assert!(err.contains("Security"), "unexpected error: {err}");
        assert!(outside.exists());
    }

    #[test]
    fn test_restore_checks_every_path_before_deleting_anything() {
        let temp_dir = tempdir().unwrap();
        let allowed = temp_dir.path().join("allowed");
        fs::create_dir_all(&allowed).unwrap();
        let created = create_test_file(&allowed, "created.txt", "pulled");
        let outside = temp_dir.path().join("outside.txt");

        let mut snapshot = metadata_only_snapshot(
            &Uuid::new_v4().to_string(),
            OperationType::Pull,
            Duration::zero(),
        );
        snapshot
            .deleted_files
            .push(created.to_string_lossy().to_string());
        snapshot
            .files
            .insert(outside.to_string_lossy().to_string(), b"old".to_vec());

        let err = snapshot
            .restore_with_base(Some(&allowed))
            .expect_err("restore must reject a path outside the allowed base")
            .to_string();
        assert!(err.contains("Security"), "unexpected error: {err}");
        assert!(
            created.exists(),
            "a rejected restore must not delete anything first"
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
