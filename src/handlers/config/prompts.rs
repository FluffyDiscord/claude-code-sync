use anyhow::{Context, Result};
use colored::Colorize;
use inquire::MultiSelect;

use super::fields::{format_age_days, format_patterns, format_size_mb};
use crate::artifacts::registry::{find_by_name, toggleable, ArtifactToggles};
use crate::filter::FilterConfig;

pub(super) fn display_config_summary(config: &FilterConfig) {
    println!(
        "  {} {}",
        "Exclude older than:".cyan(),
        config
            .exclude_older_than_days
            .map(|d| format!("{d} days"))
            .unwrap_or_else(|| "Not set".dimmed().to_string())
    );

    println!(
        "  {} {}",
        "Include patterns:".cyan(),
        format_patterns(
            &config.include_patterns,
            &"None (all included)".dimmed().to_string()
        )
    );

    println!(
        "  {} {}",
        "Exclude patterns:".cyan(),
        format_patterns(&config.exclude_patterns, &"None".dimmed().to_string())
    );

    println!(
        "  {} {} MB",
        "Max file size:".cyan(),
        format_size_mb(config.max_file_size_bytes)
    );

    println!(
        "  {} {}",
        "Exclude attachments:".cyan(),
        if config.exclude_attachments {
            "Yes (only .jsonl files)".green().to_string()
        } else {
            "No (all files)".yellow().to_string()
        }
    );

    let enabled: Vec<&str> = toggleable()
        .filter(|d| config.sync_artifacts.is_enabled(d.id))
        .map(|d| d.name)
        .collect();
    println!(
        "  {} {}",
        "Artifact sync:".cyan(),
        if enabled.is_empty() {
            "All disabled".dimmed().to_string()
        } else {
            enabled.join(", ")
        }
    );
}

pub(super) fn current_age(config: &FilterConfig) -> String {
    format_age_days(config.exclude_older_than_days)
}

pub(super) fn prompt_artifact_toggle_selection(
    current: &ArtifactToggles,
) -> Result<ArtifactToggles> {
    let rows: Vec<_> = toggleable().collect();
    let options: Vec<String> = rows
        .iter()
        .map(|d| format!("{} — {}", d.name, d.description))
        .collect();
    let preselected: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, d)| current.is_enabled(d.id))
        .map(|(i, _)| i)
        .collect();

    let picked = MultiSelect::new("Artifact categories to sync:", options)
        .with_default(&preselected)
        .with_help_message(
            "Space toggles, Enter confirms. Secrets (credentials, settings.local.json, \
             .env*, keys) are never synced regardless of selection.",
        )
        .prompt()
        .context("Failed to get artifact category selection")?;

    let mut toggles = ArtifactToggles::default();
    for label in picked {
        let name = label.split(" — ").next().unwrap_or(&label);
        if let Some(desc) = find_by_name(name) {
            toggles.set_enabled(desc.id, true);
        }
    }
    Ok(toggles)
}
