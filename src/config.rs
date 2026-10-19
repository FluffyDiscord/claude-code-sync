use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct ConfigManager;

impl ConfigManager {
    pub fn config_dir() -> Result<PathBuf> {
        if let Ok(override_dir) = std::env::var("CLAUDE_CODE_SYNC_CONFIG_DIR") {
            if !override_dir.is_empty() {
                return Ok(PathBuf::from(override_dir).join("claude-code-sync"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
                Ok(PathBuf::from(xdg_config).join("claude-code-sync"))
            } else {
                let home = dirs::home_dir().context("Failed to get home directory")?;
                Ok(home.join(".config").join("claude-code-sync"))
            }
        }

        #[cfg(target_os = "macos")]
        {
            let home = dirs::home_dir().context("Failed to get home directory")?;
            Ok(home
                .join("Library")
                .join("Application Support")
                .join("claude-code-sync"))
        }

        #[cfg(target_os = "windows")]
        {
            Ok(dirs::config_dir()
                .context("Failed to get Windows config directory")?
                .join("claude-code-sync"))
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            let home = dirs::home_dir().context("Failed to get home directory")?;
            Ok(home.join(".claude-code-sync"))
        }
    }

    pub fn state_file_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("state.json"))
    }

    pub fn filter_config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn operation_history_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("operation-history.json"))
    }

    pub fn session_summaries_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("session-summaries.json"))
    }

    pub fn snapshots_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("snapshots"))
    }

    pub fn default_repo_dir() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("repo"))
    }

    #[allow(dead_code)]
    pub fn conflict_report_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("latest-conflict-report.json"))
    }

    pub fn log_file_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("claude-code-sync.log"))
    }

    pub fn ensure_config_dir() -> Result<PathBuf> {
        let config_dir = Self::config_dir()?;
        std::fs::create_dir_all(&config_dir).with_context(|| {
            format!(
                "Failed to create config directory: {}",
                config_dir.display()
            )
        })?;
        Ok(config_dir)
    }

    #[allow(dead_code)]
    pub fn ensure_snapshots_dir() -> Result<PathBuf> {
        let snapshots_dir = Self::snapshots_dir()?;
        std::fs::create_dir_all(&snapshots_dir).with_context(|| {
            format!(
                "Failed to create snapshots directory: {}",
                snapshots_dir.display()
            )
        })?;
        Ok(snapshots_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_paths() {
        let config_dir = ConfigManager::config_dir().unwrap();
        assert!(config_dir.to_string_lossy().contains("claude-code-sync"));

        let state_path = ConfigManager::state_file_path().unwrap();
        assert!(state_path.to_string_lossy().contains("state.json"));

        let filter_path = ConfigManager::filter_config_path().unwrap();
        assert!(filter_path.to_string_lossy().contains("config.toml"));

        let history_path = ConfigManager::operation_history_path().unwrap();
        assert!(history_path
            .to_string_lossy()
            .contains("operation-history.json"));

        let snapshots = ConfigManager::snapshots_dir().unwrap();
        assert!(snapshots.to_string_lossy().contains("snapshots"));

        let repo = ConfigManager::default_repo_dir().unwrap();
        assert!(repo.to_string_lossy().contains("repo"));

        let conflict = ConfigManager::conflict_report_path().unwrap();
        assert!(conflict
            .to_string_lossy()
            .contains("latest-conflict-report.json"));

        let log = ConfigManager::log_file_path().unwrap();
        assert!(log.to_string_lossy().contains("claude-code-sync.log"));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_xdg_config_home_respected() {
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/test-xdg-config");
        let config_dir = ConfigManager::config_dir().unwrap();
        assert!(config_dir
            .to_string_lossy()
            .contains("/tmp/test-xdg-config/claude-code-sync"));
        std::env::remove_var("XDG_CONFIG_HOME");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_library_path() {
        if std::env::var("CLAUDE_CODE_SYNC_CONFIG_DIR").is_err() {
            let config_dir = ConfigManager::config_dir().unwrap();
            assert!(config_dir
                .to_string_lossy()
                .contains("Library/Application Support/claude-code-sync"));
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_config_dir_override_env_all_platforms() {
        std::env::set_var("CLAUDE_CODE_SYNC_CONFIG_DIR", "/tmp/ccs-override-test");
        let config_dir = ConfigManager::config_dir().unwrap();
        std::env::remove_var("CLAUDE_CODE_SYNC_CONFIG_DIR");
        assert_eq!(
            config_dir,
            PathBuf::from("/tmp/ccs-override-test/claude-code-sync")
        );
    }
}
