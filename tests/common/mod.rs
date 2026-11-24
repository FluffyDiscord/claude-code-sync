#![allow(dead_code)]

use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub fn every_project_filter() -> claude_code_sync::filter::FilterConfig {
    claude_code_sync::filter::FilterConfig {
        mapped_projects_only: false,
        ..Default::default()
    }
}

const DIRECTORY_ENV_VARS: [&str; 3] = [
    "HOME",
    "CLAUDE_CODE_SYNC_CLAUDE_DIR",
    "CLAUDE_CODE_SYNC_CONFIG_DIR",
];

pub struct EnvRestore {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvRestore {
    pub fn capture() -> Self {
        let saved = DIRECTORY_ENV_VARS
            .iter()
            .map(|name| (*name, std::env::var(name).ok()))
            .collect();
        Self { saved }
    }
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        for (name, value) in &self.saved {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

pub fn point_env_at(home: &Path, claude: &Path, config: &Path) {
    std::env::set_var("HOME", home);
    std::env::set_var("CLAUDE_CODE_SYNC_CLAUDE_DIR", claude);
    std::env::set_var("CLAUDE_CODE_SYNC_CONFIG_DIR", config);
}

pub fn make_unmergeable_behind_the_summary_cache(projects_dir: &Path, transcript: &Path) {
    claude_code_sync::sync::discovery::discover_sessions(projects_dir, &every_project_filter())
        .unwrap();
    let metadata = std::fs::metadata(transcript).unwrap();
    let modified = metadata.modified().unwrap();
    let mut garbage = vec![b'x'; metadata.len() as usize - 1];
    garbage.push(b'\n');
    std::fs::write(transcript, garbage).unwrap();
    std::fs::File::options()
        .write(true)
        .open(transcript)
        .unwrap()
        .set_modified(modified)
        .unwrap();
}

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
