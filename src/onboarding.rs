use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select, Text};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::ConfigManager;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitConfig {
    pub repo_path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,

    #[serde(default)]
    pub clone: bool,

    #[serde(default)]
    pub exclude_attachments: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_older_than_days: Option<u32>,

    #[serde(default)]
    pub enable_lfs: bool,

    #[serde(default = "default_sync_subdirectory")]
    pub sync_subdirectory: String,

    #[serde(default)]
    pub use_project_name_only: bool,

    #[serde(default)]
    pub sync_artifacts: crate::artifacts::registry::ArtifactToggles,
}

fn default_sync_subdirectory() -> String {
    "projects".to_string()
}

impl InitConfig {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: InitConfig =
            toml::from_str(&content).context("Failed to parse init config file")?;

        config.validate()?;

        Ok(config)
    }

    pub fn load_default() -> Result<Option<Self>> {
        if let Ok(path) = std::env::var("CLAUDE_CODE_SYNC_INIT_CONFIG") {
            let path = PathBuf::from(&path);
            if path.exists() {
                log::info!(
                    "Loading init config from CLAUDE_CODE_SYNC_INIT_CONFIG: {}",
                    path.display()
                );
                return Ok(Some(Self::load(&path)?));
            }
        }

        if let Some(home) = dirs::home_dir() {
            let home_config = home.join(".claude-code-sync-init.toml");
            if home_config.exists() {
                log::info!("Loading init config from: {}", home_config.display());
                return Ok(Some(Self::load(&home_config)?));
            }
        }

        if let Ok(config_dir) = ConfigManager::config_dir() {
            let config_path = config_dir.join("init.toml");
            if config_path.exists() {
                log::info!("Loading init config from: {}", config_path.display());
                return Ok(Some(Self::load(&config_path)?));
            }
        }

        Ok(None)
    }

    fn validate(&self) -> Result<()> {
        if let Some(ref url) = self.remote_url {
            if !is_valid_git_url(url) {
                return Err(anyhow::anyhow!(
                    "Invalid git URL '{}'. Must start with 'https://', 'http://', 'git@', or 'ssh://'",
                    url
                ));
            }
        }

        if self.clone && self.remote_url.is_none() {
            return Err(anyhow::anyhow!(
                "clone = true requires remote_url to be set"
            ));
        }

        Ok(())
    }

    pub fn to_onboarding_config(&self) -> Result<OnboardingConfig> {
        let repo_path = expand_tilde(&self.repo_path)?;

        Ok(OnboardingConfig {
            repo_path,
            remote_url: self.remote_url.clone(),
            is_cloned: self.clone,
            exclude_attachments: self.exclude_attachments,
            exclude_older_than_days: self.exclude_older_than_days,
            sync_artifacts: self.sync_artifacts.clone(),
        })
    }
}

#[derive(Debug, Clone)]
enum RepoType {
    Remote,
    Local,
}

impl std::fmt::Display for RepoType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoType::Remote => write!(f, "Remote Git Repository (clone from GitHub/GitLab/etc.)"),
            RepoType::Local => write!(f, "Local Directory (new or existing)"),
        }
    }
}

#[derive(Debug, Clone)]
enum CloneLocation {
    Default,
    Custom,
}

impl std::fmt::Display for CloneLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CloneLocation::Default => {
                let default_path = ConfigManager::default_repo_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "~/.claude-code-sync/repo/".to_string());
                write!(f, "Default ({default_path})")
            }
            CloneLocation::Custom => write!(f, "Custom path"),
        }
    }
}

#[derive(Debug)]
pub struct OnboardingConfig {
    pub repo_path: PathBuf,

    pub remote_url: Option<String>,

    pub is_cloned: bool,

    pub exclude_attachments: bool,

    pub exclude_older_than_days: Option<u32>,

    pub sync_artifacts: crate::artifacts::registry::ArtifactToggles,
}

pub fn run_onboarding() -> Result<OnboardingConfig> {
    println!(
        "\n{}",
        "⚙️  First time setup detected. Let's configure claude-code-sync!"
            .cyan()
            .bold()
    );
    println!();

    let repo_type = Select::new("Repository type:", vec![RepoType::Remote, RepoType::Local])
        .prompt()
        .context("Failed to get repository type")?;

    let (repo_path, remote_url, is_cloned) = match repo_type {
        RepoType::Remote => {
            let url = Text::new("Enter remote repository URL:")
                .with_placeholder("git@github.com:user/claude-history.git or https://github.com/user/claude-history.git")
                .with_help_message("The git repository URL for syncing your Claude Code conversations")
                .prompt()
                .context("Failed to get remote URL")?;

            if !is_valid_git_url(&url) {
                return Err(anyhow::anyhow!(
                    "Invalid git URL. Must start with 'https://', 'http://', or 'git@'"
                ));
            }

            let clone_loc = Select::new(
                "Clone location:",
                vec![CloneLocation::Default, CloneLocation::Custom],
            )
            .without_filtering()
            .with_help_message("Use arrow keys to select, Enter to confirm")
            .prompt()
            .context("Failed to get clone location")?;

            let path = match clone_loc {
                CloneLocation::Default => ConfigManager::default_repo_dir()?,
                CloneLocation::Custom => {
                    let custom_path = Text::new("Enter custom clone path:")
                        .with_placeholder("~/Documents/claude-code-sync-repo")
                        .prompt()
                        .context("Failed to get custom path")?;

                    expand_tilde(&custom_path)?
                }
            };

            (path, Some(url), true)
        }
        RepoType::Local => {
            let path_str = Text::new("Enter local repository path:")
                .with_placeholder("~/claude-code-sync-repo")
                .with_help_message(
                    "Path to a new or existing git repository on your local filesystem",
                )
                .prompt()
                .context("Failed to get local path")?;

            let path = expand_tilde(&path_str)?;

            let add_remote =
                Confirm::new("Do you want to add a remote repository for backup/sync?")
                    .with_default(false)
                    .prompt()
                    .context("Failed to get remote preference")?;

            let remote = if add_remote {
                let url = Text::new("Enter remote repository URL:")
                    .with_placeholder("git@github.com:user/claude-history.git")
                    .prompt()
                    .context("Failed to get remote URL")?;

                if !is_valid_git_url(&url) {
                    return Err(anyhow::anyhow!(
                        "Invalid git URL. Must start with 'https://', 'http://', or 'git@'"
                    ));
                }

                Some(url)
            } else {
                None
            };

            (path, remote, false)
        }
    };

    println!();

    let exclude_attachments = Confirm::new("Exclude file attachments (images, PDFs, etc.)?")
        .with_default(true)
        .with_help_message("Only sync .jsonl conversation files, excluding any attached files")
        .prompt()
        .context("Failed to get attachment preference")?;

    let exclude_old = Confirm::new("Exclude old conversations?")
        .with_default(false)
        .with_help_message("Only sync conversations modified within a certain time period")
        .prompt()
        .context("Failed to get old conversation preference")?;

    let exclude_older_than_days = if exclude_old {
        let days_str = Text::new("Exclude conversations older than (days):")
            .with_default("30")
            .with_help_message("Conversations not modified in this many days will be excluded")
            .prompt()
            .context("Failed to get days threshold")?;

        Some(days_str.parse::<u32>().context("Invalid number of days")?)
    } else {
        None
    };

    let sync_artifacts = prompt_artifact_categories()?;

    println!();
    println!("{}", "✓ Configuration complete!".green().bold());

    Ok(OnboardingConfig {
        repo_path,
        remote_url,
        is_cloned,
        exclude_attachments,
        exclude_older_than_days,
        sync_artifacts,
    })
}

fn prompt_artifact_categories() -> Result<crate::artifacts::registry::ArtifactToggles> {
    use crate::artifacts::registry::{toggleable, ArtifactToggles};

    let options: Vec<String> = toggleable()
        .map(|d| format!("{} — {}", d.name, d.description))
        .collect();
    let defaults = ArtifactToggles::all_but_hooks();
    let default_indices: Vec<usize> = toggleable()
        .enumerate()
        .filter(|(_, d)| defaults.is_enabled(d.id))
        .map(|(i, _)| i)
        .collect();

    let picked = MultiSelect::new(
        "Sync these Claude Code artifacts alongside conversation history?",
        options,
    )
    .with_default(&default_indices)
    .with_help_message(
        "Space toggles, enter confirms. Secrets (credentials, settings.local.json, \
         .env*, keys) are never synced regardless of selection.",
    )
    .prompt()
    .context("Failed to get artifact sync preference")?;

    let mut toggles = ArtifactToggles::default();
    for label in picked {
        let name = label.split(" — ").next().unwrap_or(&label);
        if let Some(desc) = crate::artifacts::registry::find_by_name(name) {
            toggles.set_enabled(desc.id, true);
        }
    }
    Ok(toggles)
}

fn is_valid_git_url(url: &str) -> bool {
    url.starts_with("https://")
        || url.starts_with("http://")
        || url.starts_with("git@")
        || url.starts_with("ssh://")
}

fn expand_tilde(path: &str) -> Result<PathBuf> {
    if path.starts_with("~/") || path == "~" {
        let home = dirs::home_dir().context("Failed to get home directory")?;
        if path == "~" {
            Ok(home)
        } else {
            Ok(home.join(&path[2..]))
        }
    } else {
        Ok(PathBuf::from(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_git_url() {
        assert!(is_valid_git_url("https://github.com/user/repo.git"));
        assert!(is_valid_git_url("http://gitlab.com/user/repo.git"));
        assert!(is_valid_git_url("git@github.com:user/repo.git"));
        assert!(is_valid_git_url("ssh://git@github.com/user/repo.git"));
        assert!(!is_valid_git_url("invalid-url"));
        assert!(!is_valid_git_url("/local/path"));
    }

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().unwrap();

        let expanded = expand_tilde("~/test").unwrap();
        assert_eq!(expanded, home.join("test"));

        let expanded = expand_tilde("~").unwrap();
        assert_eq!(expanded, home);

        let expanded = expand_tilde("/absolute/path").unwrap();
        assert_eq!(expanded, PathBuf::from("/absolute/path"));
    }

    #[test]
    fn test_init_config_parse_minimal() {
        let toml = r#"
            repo_path = "/tmp/test-repo"
        "#;
        let config: InitConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.repo_path, "/tmp/test-repo");
        assert!(config.remote_url.is_none());
        assert!(!config.clone);
        assert!(!config.exclude_attachments);
        assert!(!config.enable_lfs);
        assert_eq!(config.sync_subdirectory, "projects");
    }

    #[test]
    fn test_init_config_parse_full() {
        let toml = r#"
            repo_path = "~/claude-sync"
            remote_url = "https://github.com/user/repo.git"
            clone = true
            exclude_attachments = true
            exclude_older_than_days = 30
            enable_lfs = true
            sync_subdirectory = "history"
        "#;
        let config: InitConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.repo_path, "~/claude-sync");
        assert_eq!(
            config.remote_url,
            Some("https://github.com/user/repo.git".to_string())
        );
        assert!(config.clone);
        assert!(config.exclude_attachments);
        assert_eq!(config.exclude_older_than_days, Some(30));
        assert!(config.enable_lfs);
        assert_eq!(config.sync_subdirectory, "history");
    }

    #[test]
    fn test_init_config_validate_clone_requires_remote() {
        let config = InitConfig {
            repo_path: "/tmp/test".to_string(),
            remote_url: None,
            clone: true,
            exclude_attachments: false,
            exclude_older_than_days: None,
            enable_lfs: false,
            sync_subdirectory: "projects".to_string(),
            use_project_name_only: false,
            sync_artifacts: Default::default(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_init_config_to_onboarding_config() {
        let config = InitConfig {
            repo_path: "/tmp/test".to_string(),
            remote_url: Some("https://github.com/user/repo.git".to_string()),
            clone: true,
            exclude_attachments: true,
            exclude_older_than_days: Some(30),
            enable_lfs: true,
            sync_subdirectory: "projects".to_string(),
            use_project_name_only: false,
            sync_artifacts: Default::default(),
        };
        let onboarding = config.to_onboarding_config().unwrap();
        assert_eq!(onboarding.repo_path, PathBuf::from("/tmp/test"));
        assert_eq!(
            onboarding.remote_url,
            Some("https://github.com/user/repo.git".to_string())
        );
        assert!(onboarding.is_cloned);
        assert!(onboarding.exclude_attachments);
        assert_eq!(onboarding.exclude_older_than_days, Some(30));
    }
}
