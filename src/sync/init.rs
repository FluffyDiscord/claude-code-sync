use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

use crate::git::{self, GitRepo};

use super::state::{MultiRepoState, RepoConfig};

pub fn init_from_onboarding(
    repo_path: &Path,
    remote_url: Option<&str>,
    is_cloned: bool,
) -> Result<()> {
    crate::config::ConfigManager::ensure_config_dir()?;

    let repo = if repo_path.exists() && git::is_repo(repo_path) {
        GitRepo::open(repo_path)?
    } else {
        GitRepo::init(repo_path)?
    };

    git::attributes::ensure_sync_attributes(repo_path)?;

    let has_remote = if let Some(url) = remote_url {
        if !repo.has_remote("origin") {
            repo.add_remote("origin", url)?;
        }
        true
    } else {
        false
    };

    let repo_name = "default".to_string();
    let repo_config = RepoConfig {
        name: repo_name.clone(),
        sync_repo_path: repo_path.to_path_buf(),
        has_remote,
        is_cloned_repo: is_cloned,
        remote_url: remote_url.map(String::from),
        description: None,
    };

    let mut repos = HashMap::new();
    repos.insert(repo_name.clone(), repo_config);
    let state = MultiRepoState {
        version: 2,
        active_repo: repo_name,
        repos,
    };
    state.save()?;

    Ok(())
}

pub fn print_map_project_hint() {
    println!(
        "{} map each project to sync on this machine: claude-code-sync config --map-project <path>",
        "Next:".cyan().bold()
    );
}

pub fn init_sync_repo(repo_path: &Path, remote_url: Option<&str>) -> Result<()> {
    crate::config::ConfigManager::ensure_config_dir()?;

    println!(
        "{}",
        "Initializing Claude Code sync repository...".cyan().bold()
    );

    let repo = if repo_path.exists() && git::is_repo(repo_path) {
        println!(
            "  {} existing repository at {}",
            "Using".green(),
            repo_path.display()
        );
        GitRepo::open(repo_path)?
    } else {
        println!(
            "  {} new repository at {}",
            "Creating".green(),
            repo_path.display()
        );
        GitRepo::init(repo_path)?
    };

    git::attributes::ensure_sync_attributes(repo_path)?;

    let has_remote = if let Some(url) = remote_url {
        if !repo.has_remote("origin") {
            repo.add_remote("origin", url)?;
            println!("  {} remote 'origin' -> {}", "Added".green(), url);
        } else {
            println!("  {} Remote 'origin' already exists", "Note:".yellow());
        }
        true
    } else {
        false
    };

    let repo_name = "default".to_string();
    let repo_config = RepoConfig {
        name: repo_name.clone(),
        sync_repo_path: repo_path.to_path_buf(),
        has_remote,
        is_cloned_repo: false,
        remote_url: remote_url.map(String::from),
        description: None,
    };

    let mut repos = HashMap::new();
    repos.insert(repo_name.clone(), repo_config);
    let state = MultiRepoState {
        version: 2,
        active_repo: repo_name,
        repos,
    };
    state.save()?;

    let filter_config_path = crate::config::ConfigManager::filter_config_path()?;
    if !filter_config_path.exists() {
        crate::filter::FilterConfig::default().save()?;
    }

    println!(
        "{}",
        "Sync repository initialized successfully!".green().bold()
    );
    print_map_project_hint();

    Ok(())
}
