#![allow(dead_code)]

use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct ConfigEnv {
    temp_dir: TempDir,
}

impl ConfigEnv {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("failed to create temp config dir");
        std::env::set_var("CLAUDE_CODE_SYNC_CONFIG_DIR", temp_dir.path());
        Self { temp_dir }
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.temp_dir.path().join(name)
    }

    pub fn config_dir(&self) -> PathBuf {
        let dir = self.temp_dir.path().join("claude-code-sync");
        std::fs::create_dir_all(&dir).expect("failed to create config dir");
        dir
    }

    pub fn write_state_json(&self, contents: &str) -> PathBuf {
        let path = self.config_dir().join("state.json");
        std::fs::write(&path, contents).expect("failed to write state.json");
        path
    }
}

impl Default for ConfigEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ConfigEnv {
    fn drop(&mut self) {
        std::env::remove_var("CLAUDE_CODE_SYNC_CONFIG_DIR");
    }
}
