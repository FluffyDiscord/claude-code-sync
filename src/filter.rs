use anyhow::{bail, Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_older_than_days: Option<u32>,

    #[serde(default)]
    pub include_patterns: Vec<String>,

    #[serde(default)]
    pub exclude_patterns: Vec<String>,

    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: u64,

    #[serde(default)]
    pub exclude_attachments: bool,

    #[serde(default)]
    pub enable_lfs: bool,

    #[serde(default = "default_lfs_patterns")]
    pub lfs_patterns: Vec<String>,

    #[serde(default = "default_sync_subdirectory")]
    pub sync_subdirectory: String,

    #[serde(default)]
    pub use_project_name_only: bool,

    #[serde(default)]
    pub sync_artifacts: crate::artifacts::registry::ArtifactToggles,

    #[serde(default)]
    pub project_map: crate::project_map::ProjectMap,

    #[serde(default)]
    pub mapped_projects_only: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purge_older_than_days: Option<u32>,

    #[serde(default)]
    pub purge_after_sync: bool,

    #[serde(default)]
    pub warn_each_skipped_file: bool,

    #[serde(default)]
    pub merge_tool: String,

    #[serde(default)]
    pub prefer_merge_tool: bool,
}

fn default_lfs_patterns() -> Vec<String> {
    vec!["*.jsonl".to_string()]
}

fn default_max_file_size() -> u64 {
    10 * 1024 * 1024
}

fn default_sync_subdirectory() -> String {
    "projects".to_string()
}

impl Default for FilterConfig {
    fn default() -> Self {
        FilterConfig {
            exclude_older_than_days: None,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            max_file_size_bytes: default_max_file_size(),
            exclude_attachments: false,
            enable_lfs: false,
            lfs_patterns: default_lfs_patterns(),
            sync_subdirectory: default_sync_subdirectory(),
            use_project_name_only: false,
            sync_artifacts: Default::default(),
            project_map: Default::default(),
            mapped_projects_only: false,
            purge_older_than_days: None,
            purge_after_sync: false,
            warn_each_skipped_file: false,
            merge_tool: String::new(),
            prefer_merge_tool: false,
        }
    }
}

impl FilterConfig {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let mut config: FilterConfig =
            toml::from_str(&content).context("Failed to parse config file")?;

        if config.max_file_size_bytes == 0 {
            let default = default_max_file_size();
            eprintln!(
                "{} max_file_size_bytes is 0 in {}, which would exclude every file. \
                 Falling back to {} MB. Run `claude-code-sync config` to set it.",
                "!".yellow(),
                config_path.display(),
                default / (1024 * 1024),
            );
            config.max_file_size_bytes = default;
        }

        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create config directory: {}", parent.display())
            })?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

        Ok(())
    }

    fn config_path() -> Result<PathBuf> {
        crate::config::ConfigManager::filter_config_path()
    }

    pub fn should_include(&self, file_path: &Path) -> bool {
        if self.exclude_attachments {
            if let Some(ext) = file_path.extension() {
                if ext != "jsonl" {
                    return false;
                }
            }
        }

        if let Ok(metadata) = fs::metadata(file_path) {
            if metadata.len() > self.max_file_size_bytes {
                return false;
            }
        }

        let path_str = file_path.to_string_lossy();

        if !self.exclude_patterns.is_empty() {
            for pattern in &self.exclude_patterns {
                if glob_match(pattern, &path_str) {
                    return false;
                }
            }
        }

        if !self.include_patterns.is_empty() {
            let mut matches_include = false;
            for pattern in &self.include_patterns {
                if glob_match(pattern, &path_str) {
                    matches_include = true;
                    break;
                }
            }
            if !matches_include {
                return false;
            }
        }

        if let Some(max_days) = self.exclude_older_than_days {
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    let age = std::time::SystemTime::now()
                        .duration_since(modified)
                        .unwrap_or_default();

                    let max_age = std::time::Duration::from_secs((max_days as u64) * 24 * 60 * 60);
                    if age > max_age {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub fn validate(&self) -> Result<()> {
        if self.sync_subdirectory == crate::artifacts::registry::ARTIFACTS_SUBDIR {
            bail!(
                "sync_subdirectory cannot be '{}': that directory is reserved \
                 for artifact sync",
                crate::artifacts::registry::ARTIFACTS_SUBDIR
            );
        }
        let subdirectory = Path::new(&self.sync_subdirectory);
        let escapes = subdirectory.is_absolute()
            || subdirectory
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir));
        if escapes || self.sync_subdirectory == "." {
            bail!(
                "sync_subdirectory must be a directory inside the repository, got '{}'",
                self.sync_subdirectory
            );
        }
        if self.max_file_size_bytes == 0 {
            bail!(
                "max_file_size_bytes cannot be 0: every file would be filtered out \
                 and nothing would ever sync"
            );
        }
        crate::project_map::validate(&self.project_map)?;
        Ok(())
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern.contains('*') {
        let parts: Vec<_> = pattern.split('*').collect();
        if parts.len() == 2 {
            text.starts_with(parts[0]) && text.ends_with(parts[1])
        } else {
            let mut pos = 0;
            for (i, part) in parts.iter().enumerate() {
                if part.is_empty() {
                    continue;
                }
                if i == 0 {
                    if !text[pos..].starts_with(part) {
                        return false;
                    }
                    pos += part.len();
                } else if i == parts.len() - 1 {
                    return text[pos..].ends_with(part);
                } else if let Some(idx) = text[pos..].find(part) {
                    pos += idx + part.len();
                } else {
                    return false;
                }
            }
            true
        }
    } else {
        text.contains(pattern)
    }
}

pub fn update_project_map(add: &[String], remove: &[String]) -> Result<()> {
    let mut config = FilterConfig::load()?;

    for id in remove {
        if config.project_map.remove(id.trim()).is_none() {
            println!("{}", format!("No mapping named {id:?}").yellow());
        }
    }
    for entry in add {
        crate::project_map::add_mapping(&mut config.project_map, entry)?;
    }

    config.validate()?;
    config.save()?;

    println!("{}", "Project map:".green());
    for (id, path) in &config.project_map {
        println!("  {} -> {}", id.cyan(), path.display());
    }
    Ok(())
}

pub fn configure_purge(older_than_days: Option<u32>, after_sync: Option<bool>) -> Result<()> {
    let mut config = FilterConfig::load()?;

    if let Some(days) = older_than_days {
        if days < crate::purge::MINIMUM_RETENTION_DAYS {
            bail!(
                "purge_older_than_days must be at least {} days",
                crate::purge::MINIMUM_RETENTION_DAYS
            );
        }
        config.purge_older_than_days = Some(days);
        println!(
            "{}",
            format!("Purge transcripts older than {days} days").green()
        );
    }
    if let Some(after_sync) = after_sync {
        config.purge_after_sync = after_sync;
        println!(
            "{}",
            format!(
                "Purge as part of sync: {}",
                if after_sync { "enabled" } else { "disabled" }
            )
            .green()
        );
    }

    config.validate()?;
    config.save()?;
    Ok(())
}

pub fn set_warn_each_skipped_file(each_file: bool) -> Result<()> {
    let mut config = FilterConfig::load()?;
    config.warn_each_skipped_file = each_file;
    config.validate()?;
    config.save()?;

    println!(
        "{}",
        format!(
            "Skipped files: {}",
            if each_file {
                "one warning each"
            } else {
                "one combined warning"
            }
        )
        .green()
    );
    Ok(())
}

pub fn set_mapped_projects_only(mapped_only: bool) -> Result<()> {
    let mut config = FilterConfig::load()?;
    config.mapped_projects_only = mapped_only;
    config.validate()?;
    config.save()?;

    println!(
        "{}",
        format!("Projects synced: {}", projects_synced_label(mapped_only)).green()
    );
    let nothing_is_mapped = config.project_map.is_empty();
    let syncs_nothing = mapped_only && nothing_is_mapped;
    if syncs_nothing {
        println!(
            "{}",
            "No project is mapped yet, so nothing syncs. Map one: \
             claude-code-sync config --map-project <path>"
                .yellow()
        );
    }
    Ok(())
}

fn projects_synced_label(mapped_only: bool) -> &'static str {
    if mapped_only {
        "mapped only"
    } else {
        "all"
    }
}

pub fn set_merge_tool(command: &str) -> Result<()> {
    let mut config = FilterConfig::load()?;
    config.merge_tool = command.trim().to_string();
    config.validate()?;
    config.save()?;

    if config.merge_tool.is_empty() {
        println!("{}", "Merge tool cleared".green());
    } else {
        println!("{}", format!("Merge tool: {}", config.merge_tool).green());
    }
    Ok(())
}

pub fn set_prefer_merge_tool(prefer: bool) -> Result<()> {
    let mut config = FilterConfig::load()?;
    config.prefer_merge_tool = prefer;
    config.validate()?;
    config.save()?;

    let starting_option = if prefer { "merge" } else { "the first option" };
    println!("{}", format!("Prompts start on: {starting_option}").green());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_config(
    exclude_older_than: Option<u32>,
    include_projects: Option<String>,
    exclude_projects: Option<String>,
    exclude_attachments: Option<bool>,
    enable_lfs: Option<bool>,
    lfs_patterns: Option<String>,
    sync_subdirectory: Option<String>,
    use_project_name_only: Option<bool>,
    enable_artifacts: Option<String>,
    disable_artifacts: Option<String>,
) -> Result<()> {
    let mut config = FilterConfig::load()?;

    if let Some(days) = exclude_older_than {
        config.exclude_older_than_days = Some(days);
        println!(
            "{}",
            format!("Set exclude_older_than_days to {days} days").green()
        );
    }

    if let Some(includes) = include_projects {
        config.include_patterns = includes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        println!(
            "{}",
            format!("Set include patterns: {:?}", config.include_patterns).green()
        );
    }

    if let Some(excludes) = exclude_projects {
        config.exclude_patterns = excludes
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        println!(
            "{}",
            format!("Set exclude patterns: {:?}", config.exclude_patterns).green()
        );
    }

    if let Some(exclude_att) = exclude_attachments {
        config.exclude_attachments = exclude_att;
        println!("{}", format!("Exclude attachments: {exclude_att}").green());
    }

    if let Some(lfs) = enable_lfs {
        config.enable_lfs = lfs;
        println!(
            "{}",
            format!("Git LFS: {}", if lfs { "enabled" } else { "disabled" }).green()
        );
    }

    if let Some(patterns) = lfs_patterns {
        config.lfs_patterns = patterns
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        println!(
            "{}",
            format!("Set LFS patterns: {:?}", config.lfs_patterns).green()
        );
    }

    if let Some(subdir) = sync_subdirectory {
        let subdir_trimmed = subdir.trim().to_string();
        if subdir_trimmed.is_empty() {
            bail!("Sync subdirectory cannot be empty");
        }
        config.sync_subdirectory = subdir_trimmed;
        println!(
            "{}",
            format!("Set sync subdirectory: {}", config.sync_subdirectory).green()
        );
    }

    if let Some(project_name_only) = use_project_name_only {
        config.use_project_name_only = project_name_only;
        println!(
            "{}",
            format!(
                "Use project name only: {}",
                if project_name_only {
                    "enabled (multi-device mode)"
                } else {
                    "disabled (full path mode)"
                }
            )
            .green()
        );
    }

    if let Some(names) = enable_artifacts {
        apply_artifact_toggles(&mut config, &names, true)?;
    }

    if let Some(names) = disable_artifacts {
        apply_artifact_toggles(&mut config, &names, false)?;
    }

    config.validate()?;

    config.save()?;
    println!("{}", "Configuration saved successfully!".green().bold());

    Ok(())
}

fn apply_artifact_toggles(config: &mut FilterConfig, names: &str, value: bool) -> Result<()> {
    use crate::artifacts::registry::{find_by_name, ArtifactToggles, CategoryId};

    let verb = if value { "Enabled" } else { "Disabled" };

    for name in names.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if name == "all" {
            if value {
                let hooks = config.sync_artifacts.is_enabled(CategoryId::Hooks);
                config.sync_artifacts = ArtifactToggles::all_but_hooks();
                config.sync_artifacts.set_enabled(CategoryId::Hooks, hooks);
                println!(
                    "{}",
                    "Enabled all artifact categories (hooks only by name: --enable-artifacts hooks)"
                        .green()
                );
            } else {
                config.sync_artifacts = ArtifactToggles::default();
                println!("{}", format!("{verb} all artifact categories").green());
            }
            continue;
        }
        if name == "attachments" {
            bail!("Attachments are controlled by --exclude-attachments, not an artifact toggle");
        }
        let Some(desc) = find_by_name(name) else {
            let valid: Vec<&str> = crate::artifacts::registry::toggleable()
                .map(|d| d.name)
                .collect::<Vec<_>>();
            bail!(
                "Unknown artifact category '{}'. Valid names: {}, all",
                name,
                valid.join(", ")
            );
        };
        config.sync_artifacts.set_enabled(desc.id, value);
        println!("{}", format!("{verb} artifact category: {name}").green());
        if value && desc.id == CategoryId::Hooks {
            println!(
                "{}",
                "Warning: hooks run code. Every machine syncing this repository will run \
                 whatever scripts it holds; enable this only for a repository you control."
                    .yellow()
                    .bold()
            );
        }
    }

    Ok(())
}

pub fn show_config() -> Result<()> {
    let config = FilterConfig::load()?;

    println!("{}", "Current Filter Configuration:".bold());
    println!(
        "  {}: {}",
        "Exclude older than".cyan(),
        config
            .exclude_older_than_days
            .map(|d| format!("{d} days"))
            .unwrap_or_else(|| "Not set".to_string())
    );
    println!(
        "  {}: {}",
        "Include patterns".cyan(),
        if config.include_patterns.is_empty() {
            "None (all included)".to_string()
        } else {
            config.include_patterns.join(", ")
        }
    );
    println!(
        "  {}: {}",
        "Exclude patterns".cyan(),
        if config.exclude_patterns.is_empty() {
            "None".to_string()
        } else {
            config.exclude_patterns.join(", ")
        }
    );
    println!(
        "  {}: {} bytes ({:.2} MB)",
        "Max file size".cyan(),
        config.max_file_size_bytes,
        config.max_file_size_bytes as f64 / (1024.0 * 1024.0)
    );
    println!(
        "  {}: {}",
        "Exclude attachments".cyan(),
        if config.exclude_attachments {
            "Yes (only .jsonl files)".green()
        } else {
            "No (all files)".yellow()
        }
    );
    println!(
        "  {}: {}",
        "Git LFS".cyan(),
        if config.enable_lfs {
            format!("Enabled (patterns: {})", config.lfs_patterns.join(", ")).green()
        } else {
            "Disabled".yellow()
        }
    );
    println!(
        "  {}: {}",
        "Sync subdirectory".cyan(),
        config.sync_subdirectory.green()
    );
    println!(
        "  {}: {}",
        "Use project name only".cyan(),
        if config.use_project_name_only {
            "Yes (multi-device mode)".green()
        } else {
            "No (full path mode)".yellow()
        }
    );

    println!(
        "  {}: {}",
        "Warn for each skipped file".cyan(),
        if config.warn_each_skipped_file {
            "Yes (a line per file)".green()
        } else {
            "No (one combined warning)".yellow()
        }
    );

    println!(
        "  {}: {}",
        "Merge tool".cyan(),
        if config.merge_tool.is_empty() {
            "none (terminal picker only)".yellow()
        } else {
            config.merge_tool.green()
        }
    );

    println!(
        "  {}: {}",
        "Prefer merge tool".cyan(),
        if config.prefer_merge_tool {
            "Yes (prompts start on merge)".green()
        } else {
            "No".yellow()
        }
    );

    println!(
        "  {}: {}",
        "Purge transcripts older than".cyan(),
        match config.purge_older_than_days {
            Some(days) => format!("{days} days").green(),
            None => "default (180 days, or Claude Code's own window if longer)".green(),
        }
    );
    println!(
        "  {}: {}",
        "Purge as part of sync".cyan(),
        if config.purge_after_sync {
            "Yes".green()
        } else {
            "No (run `claude-code-sync purge`)".yellow()
        }
    );

    println!("  {}:", "Project map".cyan());
    if config.project_map.is_empty() {
        println!(
            "    {}",
            "empty (projects sync under their encoded path)".yellow()
        );
    }
    for (id, path) in &config.project_map {
        println!("    {} -> {}", id.cyan(), path.display());
    }
    println!(
        "  {}: {}",
        "Projects synced".cyan(),
        projects_synced_label(config.mapped_projects_only).cyan()
    );

    println!("  {}:", "Artifact sync".cyan());
    for desc in crate::artifacts::registry::toggleable() {
        let state = if config.sync_artifacts.is_enabled(desc.id) {
            "enabled".green()
        } else {
            "disabled".yellow()
        };
        println!(
            "    {}: {} — {}",
            desc.name,
            state,
            desc.description.dimmed()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn enabling_all_artifacts_leaves_hooks_to_be_named() {
        use crate::artifacts::registry::CategoryId;

        let mut config = FilterConfig::default();
        apply_artifact_toggles(&mut config, "all", true).unwrap();
        assert!(config.sync_artifacts.is_enabled(CategoryId::Skills));
        assert!(!config.sync_artifacts.is_enabled(CategoryId::Hooks));

        apply_artifact_toggles(&mut config, "hooks", true).unwrap();
        assert!(config.sync_artifacts.is_enabled(CategoryId::Hooks));
        apply_artifact_toggles(&mut config, "all", true).unwrap();
        assert!(
            config.sync_artifacts.is_enabled(CategoryId::Hooks),
            "`all` does not switch hooks back off either"
        );
    }

    use super::*;

    #[test]
    fn warn_each_skipped_file_survives_a_toml_round_trip() {
        let config = FilterConfig {
            warn_each_skipped_file: true,
            ..Default::default()
        };

        let serialized = toml::to_string(&config).unwrap();
        let reloaded: FilterConfig = toml::from_str(&serialized).unwrap();

        assert!(reloaded.warn_each_skipped_file);
        assert!(
            !FilterConfig::default().warn_each_skipped_file,
            "combined warnings are the default"
        );
    }

    #[test]
    fn a_config_written_before_the_key_existed_still_loads() {
        let older_config: FilterConfig = toml::from_str("max_file_size_bytes = 1024").unwrap();

        assert!(!older_config.warn_each_skipped_file);
    }

    #[test]
    fn test_validate_rejects_zero_max_file_size() {
        let config = FilterConfig {
            max_file_size_bytes: 0,
            ..Default::default()
        };

        let err = config
            .validate()
            .expect_err("a 0-byte size limit must not validate")
            .to_string();
        assert!(err.contains("max_file_size_bytes"), "unexpected: {err}");
    }

    #[test]
    fn test_validate_accepts_a_positive_max_file_size() {
        let config = FilterConfig {
            max_file_size_bytes: 1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_zero_max_file_size_excludes_every_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("session.jsonl");
        fs::write(&file, b"x").unwrap();

        assert!(
            FilterConfig::default().should_include(&file),
            "sanity: the default config includes this file"
        );

        let broken = FilterConfig {
            max_file_size_bytes: 0,
            ..Default::default()
        };
        assert!(
            !broken.should_include(&file),
            "a 0-byte limit excludes every non-empty file — this is the damage"
        );
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*test*", "this is a test"));
        assert!(glob_match("test*", "testing"));
        assert!(glob_match("*test", "this is a test"));
        assert!(!glob_match("test*", "no match"));
    }

    #[test]
    fn test_filter_config_default() {
        let config = FilterConfig::default();
        assert_eq!(config.exclude_older_than_days, None);
        assert!(config.include_patterns.is_empty());
        assert!(config.exclude_patterns.is_empty());
        assert!(!config.exclude_attachments);
        assert_eq!(config.max_file_size_bytes, 10 * 1024 * 1024);
    }

    #[test]
    fn test_exclude_attachments_filter() {
        use std::path::PathBuf;

        let config_include_all = FilterConfig::default();

        assert!(config_include_all.should_include(&PathBuf::from("session.jsonl")));

        assert!(config_include_all.should_include(&PathBuf::from("image.png")));
        assert!(config_include_all.should_include(&PathBuf::from("document.pdf")));

        let config_exclude = FilterConfig {
            exclude_attachments: true,
            ..Default::default()
        };

        assert!(config_exclude.should_include(&PathBuf::from("session.jsonl")));

        assert!(!config_exclude.should_include(&PathBuf::from("image.png")));
        assert!(!config_exclude.should_include(&PathBuf::from("image.jpg")));
        assert!(!config_exclude.should_include(&PathBuf::from("document.pdf")));
        assert!(!config_exclude.should_include(&PathBuf::from("archive.zip")));
    }

    #[test]
    fn test_exclude_attachments_with_patterns() {
        use std::path::PathBuf;

        let config = FilterConfig {
            exclude_attachments: true,
            exclude_patterns: vec!["*test*".to_string()],
            ..Default::default()
        };

        assert!(!config.should_include(&PathBuf::from("image.png")));

        assert!(!config.should_include(&PathBuf::from("/path/test/session.jsonl")));

        assert!(config.should_include(&PathBuf::from("/path/prod/session.jsonl")));
    }

    #[test]
    fn test_filter_config_serialization() {
        let config = FilterConfig {
            exclude_attachments: true,
            exclude_older_than_days: Some(30),
            ..Default::default()
        };

        let serialized = toml::to_string(&config).unwrap();
        assert!(serialized.contains("exclude_attachments"));

        let deserialized: FilterConfig = toml::from_str(&serialized).unwrap();
        assert!(deserialized.exclude_attachments);
        assert_eq!(deserialized.exclude_older_than_days, Some(30));
    }
}
