use anyhow::{Context, Result};
use colored::Colorize;
use std::path::Path;

use crate::config;
use crate::filter;
use crate::git::GitRepo;
use crate::onboarding::{self, InitConfig};
use crate::sync;

pub fn is_initialized() -> Result<bool> {
    let state_path = config::ConfigManager::state_file_path()?;
    Ok(state_path.exists())
}

pub fn run_onboarding_flow() -> Result<()> {
    let onboarding_config =
        onboarding::run_onboarding().context("Onboarding cancelled or failed")?;

    if onboarding_config.is_cloned {
        if let Some(ref remote_url) = onboarding_config.remote_url {
            println!();
            println!("{}", "✓ Cloning repository...".cyan());

            GitRepo::clone(remote_url, &onboarding_config.repo_path)
                .context("Failed to clone repository")?;

            println!("{}", "✓ Repository cloned successfully!".green());
        }
    }

    sync::init_from_onboarding(
        &onboarding_config.repo_path,
        onboarding_config.remote_url.as_deref(),
        onboarding_config.is_cloned,
    )
    .context("Failed to initialize sync state")?;

    let filter_config = filter::FilterConfig {
        exclude_attachments: onboarding_config.exclude_attachments,
        exclude_older_than_days: onboarding_config.exclude_older_than_days,
        sync_artifacts: onboarding_config.sync_artifacts.clone(),
        ..Default::default()
    };
    filter_config
        .save()
        .context("Failed to save filter configuration")?;

    println!("{}", "✓ Ready to sync!".green().bold());
    println!();

    Ok(())
}

pub fn run_init_from_config<P: AsRef<Path>>(config_path: Option<P>) -> Result<()> {
    let init_config = if let Some(path) = config_path {
        log::info!("Loading init config from: {}", path.as_ref().display());
        InitConfig::load(path.as_ref())?
    } else {
        InitConfig::load_default()?.ok_or_else(|| anyhow::anyhow!("No init config file found"))?
    };

    println!("{}", "📄 Initializing from config file...".cyan().bold());

    let onboarding_config = init_config.to_onboarding_config()?;

    if onboarding_config.is_cloned {
        if let Some(ref remote_url) = onboarding_config.remote_url {
            println!("  {} {}", "Cloning from:".cyan(), remote_url);

            GitRepo::clone(remote_url, &onboarding_config.repo_path)
                .context("Failed to clone repository")?;

            println!("{}", "  ✓ Repository cloned".green());
        }
    }

    sync::init_from_onboarding(
        &onboarding_config.repo_path,
        onboarding_config.remote_url.as_deref(),
        onboarding_config.is_cloned,
    )
    .context("Failed to initialize sync state")?;

    let filter_config = filter::FilterConfig {
        exclude_attachments: init_config.exclude_attachments,
        exclude_older_than_days: init_config.exclude_older_than_days,
        enable_lfs: init_config.enable_lfs,
        sync_subdirectory: init_config.sync_subdirectory.clone(),
        sync_artifacts: init_config.sync_artifacts.clone(),
        ..Default::default()
    };
    filter_config
        .save()
        .context("Failed to save filter configuration")?;

    println!("{}", "✓ Initialization complete!".green().bold());
    println!(
        "  {} {}",
        "Repo:".cyan(),
        onboarding_config.repo_path.display()
    );
    if let Some(ref url) = onboarding_config.remote_url {
        println!("  {} {}", "Remote:".cyan(), url);
    }
    if init_config.enable_lfs {
        println!("  {} enabled", "LFS:".cyan());
    }
    println!();

    Ok(())
}

pub fn try_init_from_config() -> Result<bool> {
    match InitConfig::load_default()? {
        Some(_) => {
            run_init_from_config::<&Path>(None)?;
            Ok(true)
        }
        None => Ok(false),
    }
}
