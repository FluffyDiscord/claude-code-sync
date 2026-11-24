use anyhow::{anyhow, Context, Result};
use colored::Colorize;

use crate::git::GitRepo;

use super::state::{MultiRepoState, SyncState};

pub fn show_remote() -> Result<()> {
    let state = SyncState::load()?;
    let repo = GitRepo::open(&state.sync_repo_path)?;

    println!("{}", "=== Git Remote Configuration ===".bold().cyan());
    println!();

    println!(
        "{} {}",
        "Sync Directory:".bold(),
        state.sync_repo_path.display().to_string().cyan()
    );

    if let Ok(branch) = repo.current_branch() {
        println!("{} {}", "Current Branch:".bold(), branch.cyan());
    }

    println!();

    let remotes = repo.list_remotes()?;

    if remotes.is_empty() {
        println!("{}", "No remotes configured".yellow());
        println!("\n{} claude-code-sync remote set <url>", "Hint:".cyan());
        return Ok(());
    }

    for name in &remotes {
        println!("{} {}", "Remote:".bold(), name.cyan());

        if let Ok(url) = repo.get_remote_url(name) {
            println!("  URL: {url}");
        } else {
            println!("  URL: {}", "None".yellow());
        }

        println!();
    }

    Ok(())
}

pub fn set_remote(name: &str, url: &str) -> Result<()> {
    let state = SyncState::load()?;
    let repo = GitRepo::open(&state.sync_repo_path)?;

    if !url.starts_with("http://")
        && !url.starts_with("https://")
        && !url.starts_with("git@")
        && !url.starts_with("ssh://")
    {
        return Err(anyhow!(
            "Invalid URL format: {url}\n\
            \n\
            URL must start with:\n\
            - https:// (e.g., https://github.com/user/repo.git)\n\
            - http:// (e.g., http://gitlab.com/user/repo.git)\n\
            - git@ (e.g., git@github.com:user/repo.git)\n\
            - ssh:// (e.g., ssh://git@github.com/user/repo.git)"
        ));
    }

    let remote_exists = repo.has_remote(name);

    if remote_exists {
        repo.set_remote_url(name, url)
            .with_context(|| format!("Failed to update remote '{name}' URL"))?;

        println!(
            "{} Updated remote '{}' to: {}",
            "✓".green().bold(),
            name.cyan(),
            url
        );
    } else {
        repo.add_remote(name, url)
            .with_context(|| format!("Failed to create remote '{name}'"))?;

        println!(
            "{} Created remote '{}': {}",
            "✓".green().bold(),
            name.cyan(),
            url
        );
    }

    if name == "origin" {
        save_origin_url(Some(url))?;
    }

    println!("\n{} claude-code-sync push", "Next:".cyan());

    Ok(())
}

pub fn remove_remote(name: &str) -> Result<()> {
    let state = SyncState::load()?;
    let repo = GitRepo::open(&state.sync_repo_path)?;

    if !repo.has_remote(name) {
        return Err(anyhow!("Remote '{name}' not found"));
    }

    repo.remove_remote(name)
        .with_context(|| format!("Failed to remove remote '{name}'"))?;

    println!("{} Removed remote '{}'", "✓".green().bold(), name.cyan());

    if name == "origin" {
        save_origin_url(None)?;
    }

    Ok(())
}

fn save_origin_url(url: Option<&str>) -> Result<()> {
    let mut state = MultiRepoState::load()?;
    let active = state
        .repos
        .get_mut(&state.active_repo)
        .ok_or_else(|| anyhow!("Active repository not found in state"))?;

    active.has_remote = url.is_some();
    active.remote_url = url.map(str::to_string);

    state.save()
}
