use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncState {
    pub sync_repo_path: PathBuf,

    pub has_remote: bool,

    #[serde(default)]
    pub is_cloned_repo: bool,
}

impl SyncState {
    pub fn load() -> Result<Self> {
        let state_path = Self::state_file_path()?;

        if !state_path.exists() {
            return Err(anyhow!(
                "Sync not initialized. Run 'claude-code-sync init' first."
            ));
        }

        let content = fs::read_to_string(&state_path).context("Failed to read sync state")?;

        if let Ok(multi_state) = serde_json::from_str::<MultiRepoState>(&content) {
            if multi_state.version >= 2 {
                if let Some(active) = multi_state.repos.get(&multi_state.active_repo) {
                    return Ok(SyncState {
                        sync_repo_path: active.sync_repo_path.clone(),
                        has_remote: active.has_remote,
                        is_cloned_repo: active.is_cloned_repo,
                    });
                } else {
                    return Err(anyhow!(
                        "Active repository '{}' not found in state",
                        multi_state.active_repo
                    ));
                }
            }
        }

        let state: SyncState =
            serde_json::from_str(&content).context("Failed to parse sync state")?;

        Ok(state)
    }

    pub(crate) fn save(&self) -> Result<()> {
        let state_path = Self::state_file_path()?;

        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize sync state")?;

        fs::write(&state_path, content).context("Failed to write sync state")?;

        Ok(())
    }

    fn state_file_path() -> Result<PathBuf> {
        crate::config::ConfigManager::state_file_path()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RepoConfig {
    pub name: String,

    pub sync_repo_path: PathBuf,

    pub has_remote: bool,

    #[serde(default)]
    pub is_cloned_repo: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MultiRepoState {
    #[serde(default = "default_version")]
    pub version: u32,

    pub active_repo: String,

    pub repos: HashMap<String, RepoConfig>,
}

fn default_version() -> u32 {
    2
}

#[allow(dead_code)]
impl MultiRepoState {
    pub fn active(&self) -> Option<&RepoConfig> {
        self.repos.get(&self.active_repo)
    }

    pub fn active_mut(&mut self) -> Option<&mut RepoConfig> {
        self.repos.get_mut(&self.active_repo)
    }

    pub fn load() -> Result<Self> {
        let state_path = SyncState::state_file_path()?;

        if !state_path.exists() {
            return Err(anyhow!(
                "Sync not initialized. Run 'claude-code-sync init' first."
            ));
        }

        let content = fs::read_to_string(&state_path).context("Failed to read sync state")?;

        if let Ok(state) = serde_json::from_str::<MultiRepoState>(&content) {
            if state.version >= 2 {
                return Ok(state);
            }
        }

        let legacy: SyncState = serde_json::from_str(&content)
            .context("Failed to parse sync state (neither v1 nor v2 format)")?;

        let migrated = Self::migrate_from_v1(legacy)?;

        migrated.save()?;

        log::info!("Migrated state.json from v1 to v2 format");

        Ok(migrated)
    }

    fn migrate_from_v1(legacy: SyncState) -> Result<Self> {
        let repo_name = "default".to_string();

        let repo_config = RepoConfig {
            name: repo_name.clone(),
            sync_repo_path: legacy.sync_repo_path,
            has_remote: legacy.has_remote,
            is_cloned_repo: legacy.is_cloned_repo,
            remote_url: None,
            description: Some("Migrated from single-repo configuration".to_string()),
        };

        let mut repos = HashMap::new();
        repos.insert(repo_name.clone(), repo_config);

        Ok(MultiRepoState {
            version: 2,
            active_repo: repo_name,
            repos,
        })
    }

    pub fn save(&self) -> Result<()> {
        let state_path = SyncState::state_file_path()?;

        if let Some(parent) = state_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize multi-repo state")?;

        fs::write(&state_path, content).context("Failed to write multi-repo state")?;

        Ok(())
    }

    pub fn has_repo(&self, name: &str) -> bool {
        self.repos.contains_key(name)
    }

    pub fn add_repo(&mut self, config: RepoConfig) -> Result<()> {
        if self.repos.contains_key(&config.name) {
            return Err(anyhow!("Repository '{}' already exists", config.name));
        }
        self.repos.insert(config.name.clone(), config);
        Ok(())
    }

    pub fn remove_repo(&mut self, name: &str) -> Result<()> {
        if name == self.active_repo {
            return Err(anyhow!(
                "Cannot remove active repository '{}'. Switch to another repo first.",
                name
            ));
        }
        if self.repos.remove(name).is_none() {
            return Err(anyhow!("Repository '{}' not found", name));
        }
        Ok(())
    }

    pub fn switch_active(&mut self, name: &str) -> Result<()> {
        if !self.repos.contains_key(name) {
            return Err(anyhow!("Repository '{}' not found", name));
        }
        self.active_repo = name.to_string();
        Ok(())
    }

    pub fn repo_names(&self) -> Vec<&String> {
        self.repos.keys().collect()
    }
}
