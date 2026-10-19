use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub type TrackedPaths = BTreeSet<String>;

pub type SyncedHashes = BTreeMap<String, String>;

const TRACKED_FILE_NAME: &str = ".claude-code-sync-tracked.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct TrackedRecord {
    #[serde(default)]
    repos: BTreeMap<String, TrackedPaths>,
    #[serde(default)]
    synced_hashes: BTreeMap<String, SyncedHashes>,
}

pub fn record_path(claude_dir: &Path) -> PathBuf {
    claude_dir.join(TRACKED_FILE_NAME)
}

fn read_record(claude_dir: &Path) -> TrackedRecord {
    let Ok(text) = std::fs::read_to_string(record_path(claude_dir)) else {
        return TrackedRecord::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn repo_key(repo_root: &Path) -> String {
    repo_root.to_string_lossy().to_string()
}

pub fn load(claude_dir: &Path, repo_root: &Path) -> TrackedPaths {
    read_record(claude_dir)
        .repos
        .remove(&repo_key(repo_root))
        .unwrap_or_default()
}

pub fn forget(claude_dir: &Path, repo_root: &Path) -> Result<()> {
    let mut record = read_record(claude_dir);
    let key = repo_key(repo_root);
    let forgot_paths = record.repos.remove(&key).is_some();
    let forgot_hashes = record.synced_hashes.remove(&key).is_some();
    if !forgot_paths && !forgot_hashes {
        return Ok(());
    }
    write_record(claude_dir, &record)
}

pub fn save(claude_dir: &Path, repo_root: &Path, paths: TrackedPaths) -> Result<()> {
    let mut record = read_record(claude_dir);
    record.repos.insert(repo_key(repo_root), paths);
    write_record(claude_dir, &record)
}

pub fn load_synced_hashes(claude_dir: &Path, repo_root: &Path) -> SyncedHashes {
    read_record(claude_dir)
        .synced_hashes
        .remove(&repo_key(repo_root))
        .unwrap_or_default()
}

pub fn save_synced_hashes(claude_dir: &Path, repo_root: &Path, hashes: SyncedHashes) -> Result<()> {
    let mut record = read_record(claude_dir);
    record.synced_hashes.insert(repo_key(repo_root), hashes);
    write_record(claude_dir, &record)
}

pub fn hash_content(bytes: &[u8]) -> String {
    crate::self_update::sha256_hex(bytes)
}

fn write_record(claude_dir: &Path, record: &TrackedRecord) -> Result<()> {
    let path = record_path(claude_dir);
    std::fs::create_dir_all(claude_dir)?;
    let text = serde_json::to_string_pretty(record)?;
    std::fs::write(&path, text).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(entries: &[&str]) -> TrackedPaths {
        entries.iter().map(|e| (*e).to_string()).collect()
    }

    #[test]
    fn an_absent_record_tracks_nothing() {
        let claude = tempfile::tempdir().unwrap();
        assert!(load(claude.path(), Path::new("/repo")).is_empty());
    }

    #[test]
    fn a_corrupt_record_tracks_nothing_instead_of_failing() {
        let claude = tempfile::tempdir().unwrap();
        std::fs::write(record_path(claude.path()), "{ not json").unwrap();
        assert!(load(claude.path(), Path::new("/repo")).is_empty());
    }

    #[test]
    fn each_repository_is_tracked_separately() {
        let claude = tempfile::tempdir().unwrap();
        save(
            claude.path(),
            Path::new("/one"),
            paths(&["artifacts/skills/a.md"]),
        )
        .unwrap();
        save(
            claude.path(),
            Path::new("/two"),
            paths(&["artifacts/rules/b.md"]),
        )
        .unwrap();

        assert_eq!(
            load(claude.path(), Path::new("/one")),
            paths(&["artifacts/skills/a.md"])
        );
        assert_eq!(
            load(claude.path(), Path::new("/two")),
            paths(&["artifacts/rules/b.md"])
        );
    }

    #[test]
    fn forgetting_one_repository_leaves_the_others_alone() {
        let claude = tempfile::tempdir().unwrap();
        save(claude.path(), Path::new("/one"), paths(&["a"])).unwrap();
        save(claude.path(), Path::new("/two"), paths(&["b"])).unwrap();

        forget(claude.path(), Path::new("/one")).unwrap();

        assert!(load(claude.path(), Path::new("/one")).is_empty());
        assert_eq!(load(claude.path(), Path::new("/two")), paths(&["b"]));
        forget(claude.path(), Path::new("/never-synced")).unwrap();
    }

    #[test]
    fn saving_replaces_the_previous_set_for_that_repository() {
        let claude = tempfile::tempdir().unwrap();
        save(claude.path(), Path::new("/one"), paths(&["a", "b"])).unwrap();
        save(claude.path(), Path::new("/one"), paths(&["b"])).unwrap();
        assert_eq!(load(claude.path(), Path::new("/one")), paths(&["b"]));
    }
}
