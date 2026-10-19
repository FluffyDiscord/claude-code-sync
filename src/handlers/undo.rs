use anyhow::{Context, Result};
use colored::Colorize;
use inquire::Confirm;

use crate::interactive_conflict;
use crate::sync;
use crate::undo;

pub fn handle_undo_pull(preview_only: bool, verbosity: crate::VerbosityLevel) -> Result<()> {
    let undo_verbosity = match verbosity {
        crate::VerbosityLevel::Quiet => undo::VerbosityLevel::Quiet,
        crate::VerbosityLevel::Normal => undo::VerbosityLevel::Normal,
        crate::VerbosityLevel::Verbose => undo::VerbosityLevel::Verbose,
    };

    if verbosity != crate::VerbosityLevel::Quiet {
        println!("{}", "Preparing to undo last pull operation...".cyan());
    }

    let preview = undo::preview_undo_pull(None).context("Failed to preview undo operation")?;
    preview.display(undo_verbosity);

    if preview_only {
        if verbosity != crate::VerbosityLevel::Quiet {
            println!("\n{}", "Preview only - no changes made.".yellow());
        }
        return Ok(());
    }

    let is_interactive = interactive_conflict::is_interactive();

    if is_interactive {
        let confirm = Confirm::new("Restore files to before the pull?")
            .with_default(false)
            .prompt()
            .context("Failed to get confirmation")?;

        if !confirm {
            println!("\n{}", "Undo operation cancelled.".yellow());
            return Ok(());
        }
    }

    if verbosity != crate::VerbosityLevel::Quiet {
        println!("\n{}", "Undoing last pull operation...".cyan());
    }

    let summary = undo::undo_pull(None, None).context("Failed to undo pull operation")?;

    if verbosity == crate::VerbosityLevel::Quiet {
        println!("Pull undone successfully");
    } else {
        println!("\n{}", "SUCCESS".green().bold());
        println!("{summary}");
    }

    Ok(())
}

pub fn handle_undo_push(preview_only: bool, verbosity: crate::VerbosityLevel) -> Result<()> {
    let undo_verbosity = match verbosity {
        crate::VerbosityLevel::Quiet => undo::VerbosityLevel::Quiet,
        crate::VerbosityLevel::Normal => undo::VerbosityLevel::Normal,
        crate::VerbosityLevel::Verbose => undo::VerbosityLevel::Verbose,
    };

    if verbosity != crate::VerbosityLevel::Quiet {
        println!("{}", "Preparing to undo last push operation...".cyan());
    }

    let state = sync::SyncState::load()
        .context("Sync not initialized. Run 'claude-code-sync init' first.")?;

    let preview = undo::preview_undo_push(None).context("Failed to preview undo operation")?;
    preview.display(undo_verbosity);

    if preview_only {
        if verbosity != crate::VerbosityLevel::Quiet {
            println!("\n{}", "Preview only - no changes made.".yellow());
        }
        return Ok(());
    }

    let is_interactive = interactive_conflict::is_interactive();

    if is_interactive {
        let confirm = Confirm::new("Reset the sync repository to before the push?")
            .with_default(false)
            .prompt()
            .context("Failed to get confirmation")?;

        if !confirm {
            println!("\n{}", "Undo operation cancelled.".yellow());
            return Ok(());
        }
    }

    if verbosity != crate::VerbosityLevel::Quiet {
        println!("\n{}", "Undoing last push operation...".cyan());
    }

    let summary =
        undo::undo_push(&state.sync_repo_path, None).context("Failed to undo push operation")?;

    if verbosity == crate::VerbosityLevel::Quiet {
        println!("Push undone successfully");
    } else {
        println!("\n{}", "SUCCESS".green().bold());
        println!("{summary}");
    }

    Ok(())
}
