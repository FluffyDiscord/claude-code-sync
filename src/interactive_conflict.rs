use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Confirm, Select};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

use crate::conflict::{Conflict, ConflictResolution};
use crate::parser::ConversationSession;

#[derive(Debug, Clone)]
pub enum ResolutionAction {
    KeepLocal,
    KeepRemote,
    KeepBoth,
    ViewDetails,
}

impl std::fmt::Display for ResolutionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionAction::KeepLocal => write!(f, "local"),
            ResolutionAction::KeepRemote => write!(f, "remote"),
            ResolutionAction::KeepBoth => write!(f, "both (remote saved as a copy)"),
            ResolutionAction::ViewDetails => write!(f, "details"),
        }
    }
}

#[derive(Debug)]
pub struct ResolutionResult {
    pub keep_local: Vec<Conflict>,
    pub keep_remote: Vec<Conflict>,
    pub keep_both: Vec<Conflict>,
}

impl Default for ResolutionResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ResolutionResult {
    pub fn new() -> Self {
        ResolutionResult {
            keep_local: Vec::new(),
            keep_remote: Vec::new(),
            keep_both: Vec::new(),
        }
    }
}

pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

pub fn ask_confirmation(question: &str, rerun_hint: &str, assume_yes: bool) -> bool {
    if assume_yes {
        return true;
    }

    let interactive = is_interactive();
    if !interactive {
        println!("  {} {}", "!".yellow(), rerun_hint);
        return false;
    }

    Confirm::new(question)
        .with_default(false)
        .prompt()
        .unwrap_or(false)
}

fn display_conflict_details(conflict: &Conflict) {
    println!("\n{}", "=".repeat(80).cyan());
    println!("{}", "Conflict Details".bold().cyan());
    println!("{}", "=".repeat(80).cyan());

    println!("\n{} {}", "Session ID:".bold(), conflict.session_id.cyan());

    println!(
        "\n{} {}",
        "Local File:".bold().green(),
        conflict.local_file.display()
    );
    println!(
        "  {} messages",
        conflict.local_message_count.to_string().green()
    );
    if let Some(ts) = &conflict.local_timestamp {
        println!("  Last updated: {}", ts.dimmed());
    }
    println!("  Content hash: {}", conflict.local_hash[..16].dimmed());

    println!(
        "\n{} {}",
        "Remote File:".bold().yellow(),
        conflict.remote_file.display()
    );
    println!(
        "  {} messages",
        conflict.remote_message_count.to_string().yellow()
    );
    if let Some(ts) = &conflict.remote_timestamp {
        println!("  Last updated: {}", ts.dimmed());
    }
    println!("  Content hash: {}", conflict.remote_hash[..16].dimmed());

    let msg_diff = conflict.remote_message_count as i32 - conflict.local_message_count as i32;
    if msg_diff > 0 {
        println!(
            "\n{} Remote has {} more messages",
            "→".yellow(),
            msg_diff.to_string().yellow().bold()
        );
    } else if msg_diff < 0 {
        println!(
            "\n{} Local has {} more messages",
            "→".green(),
            (-msg_diff).to_string().green().bold()
        );
    } else {
        println!(
            "\n{} Both have the same number of messages, but content differs",
            "→".cyan()
        );
    }

    println!("{}", "=".repeat(80).cyan());
}

fn list_menu_options() -> Vec<ResolutionAction> {
    vec![
        ResolutionAction::KeepLocal,
        ResolutionAction::KeepRemote,
        ResolutionAction::KeepBoth,
        ResolutionAction::ViewDetails,
    ]
}

fn format_menu_header(conflict: &Conflict) -> String {
    let description = conflict.description();
    match &conflict.merge_error {
        Some(reason) => format!("{description}\n  Automatic merge failed: {reason}"),
        None => description,
    }
}

fn resolve_conflict_interactive(conflict: &Conflict) -> Result<ResolutionAction> {
    loop {
        println!("\n{}", "Conflict Detected!".yellow().bold());
        println!("  {}", format_menu_header(conflict).dimmed());

        let action = Select::new("Resolve:", list_menu_options())
            .prompt()
            .context("Failed to get resolution action")?;

        match action {
            ResolutionAction::ViewDetails => {
                display_conflict_details(conflict);
                continue;
            }
            _ => return Ok(action),
        }
    }
}

pub fn resolve_conflicts_interactive(conflicts: &mut [Conflict]) -> Result<ResolutionResult> {
    if conflicts.is_empty() {
        return Ok(ResolutionResult::new());
    }

    let total_conflicts = conflicts.len();

    println!(
        "\n{}",
        format!("Found {total_conflicts} conflicts to resolve")
            .yellow()
            .bold()
    );
    println!("{}", "Let's resolve them one by one...".cyan());

    let mut result = ResolutionResult::new();

    for (idx, conflict) in conflicts.iter_mut().enumerate() {
        println!(
            "\n{} Conflict {} of {}",
            ">>>".yellow().bold(),
            (idx + 1).to_string().cyan(),
            total_conflicts.to_string().cyan()
        );

        let action = resolve_conflict_interactive(conflict)?;
        record_choice(conflict, action, &mut result);
    }

    println!("\n{}", "=".repeat(80).green());
    println!("{}", "Resolution Summary".bold().green());
    println!("{}", "=".repeat(80).green());
    println!(
        "  Keep Local:  {}",
        result.keep_local.len().to_string().green()
    );
    println!(
        "  Keep Remote: {}",
        result.keep_remote.len().to_string().yellow()
    );
    println!(
        "  Keep Both:   {}",
        result.keep_both.len().to_string().cyan()
    );
    println!("{}", "=".repeat(80).green());

    let confirm = Confirm::new("Apply?")
        .with_default(true)
        .prompt()
        .context("Failed to get confirmation")?;

    if !confirm {
        return Err(anyhow::anyhow!(
            "Resolution cancelled by user. No changes were made."
        ));
    }

    Ok(result)
}

fn record_choice(conflict: &mut Conflict, action: ResolutionAction, result: &mut ResolutionResult) {
    match action {
        ResolutionAction::KeepLocal => {
            println!("  {} Keeping local version", "✓".green());
            conflict.resolution = ConflictResolution::KeepLocal;
            result.keep_local.push(conflict.clone());
        }
        ResolutionAction::KeepRemote => {
            println!(
                "  {} Keeping remote version (will overwrite local)",
                "✓".yellow()
            );
            conflict.resolution = ConflictResolution::KeepRemote;
            result.keep_remote.push(conflict.clone());
        }
        ResolutionAction::KeepBoth => {
            println!(
                "  {} Keeping both versions (remote will be saved with conflict suffix)",
                "✓".cyan()
            );
            result.keep_both.push(conflict.clone());
        }
        ResolutionAction::ViewDetails => unreachable!("ViewDetails should be handled in the loop"),
    }
}

pub fn apply_resolutions(
    result: &ResolutionResult,
    remote_sessions: &[ConversationSession],
    claude_dir: &Path,
    _remote_projects_dir: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let mut renames = Vec::new();

    for conflict in &result.keep_remote {
        if let Some(remote_session) = find_remote_session(conflict, remote_sessions) {
            remote_session
                .copy_to(&conflict.local_file)
                .with_context(|| {
                    format!(
                        "Failed to overwrite local file with remote: {}",
                        conflict.local_file.display()
                    )
                })?;

            println!(
                "  {} Overwrote local with remote: {}",
                "✓".yellow(),
                conflict.local_file.display()
            );
        }
    }

    for conflict in &result.keep_both {
        let saved_copy = save_new_remote_copy(conflict, remote_sessions)?;
        if let Some(renamed_path) = saved_copy {
            let relative_renamed = renamed_path
                .strip_prefix(claude_dir)
                .unwrap_or(&renamed_path);
            println!(
                "  {} Saved remote as: {}",
                "✓".cyan(),
                relative_renamed.display()
            );

            renames.push((conflict.remote_file.clone(), renamed_path));
        }
    }

    Ok(renames)
}

const CONFLICT_SUFFIX_PREFIX: &str = "conflict-";

pub fn save_new_remote_copy(
    conflict: &Conflict,
    remote_sessions: &[ConversationSession],
) -> Result<Option<PathBuf>> {
    let Some(remote_session) = find_remote_session(conflict, remote_sessions) else {
        return Ok(None);
    };

    let already_saved = has_identical_copy(conflict, remote_session);
    if already_saved {
        return Ok(None);
    }

    let timestamp = crate::datetime::format_utc(
        time::OffsetDateTime::now_utc(),
        time::macros::format_description!("[year][month][day]-[hour][minute][second]"),
    );
    let conflict_suffix = format!("{CONFLICT_SUFFIX_PREFIX}{timestamp}");
    let renamed_path = conflict.clone().resolve_keep_both(&conflict_suffix)?;

    remote_session.copy_to(&renamed_path).with_context(|| {
        format!(
            "Failed to write remote conflict version: {}",
            renamed_path.display()
        )
    })?;

    Ok(Some(renamed_path))
}

fn find_remote_session<'a>(
    conflict: &Conflict,
    remote_sessions: &'a [ConversationSession],
) -> Option<&'a ConversationSession> {
    remote_sessions
        .iter()
        .find(|session| session.file_path == conflict.remote_file)
}

fn has_identical_copy(conflict: &Conflict, remote_session: &ConversationSession) -> bool {
    let Some(project_dir) = conflict.local_file.parent() else {
        return false;
    };
    let Some(local_stem) = conflict.local_file.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    let Ok(entries) = std::fs::read_dir(project_dir) else {
        return false;
    };

    let copy_prefix = format!("{local_stem}-{CONFLICT_SUFFIX_PREFIX}");
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&copy_prefix))
        })
        .any(|path| {
            ConversationSession::from_file(&path)
                .is_ok_and(|copy| copy.content_hash() == remote_session.content_hash())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_result() {
        let result = ResolutionResult::new();
        assert_eq!(result.keep_local.len(), 0);
        assert_eq!(result.keep_remote.len(), 0);
        assert_eq!(result.keep_both.len(), 0);
    }

    #[test]
    fn test_display_resolution_action() {
        let action = ResolutionAction::KeepLocal;
        assert_eq!(action.to_string(), "local");

        let action = ResolutionAction::KeepRemote;
        assert_eq!(action.to_string(), "remote");

        let action = ResolutionAction::KeepBoth;
        assert_eq!(action.to_string(), "both (remote saved as a copy)");
    }

    #[test]
    fn test_fallback_menu_offers_no_smart_merge_and_shows_why_it_failed() {
        let local_dir = tempfile::TempDir::new().unwrap();
        let remote_dir = tempfile::TempDir::new().unwrap();
        let local_path = local_dir.path().join("s1.jsonl");
        let remote_path = remote_dir.path().join("s1.jsonl");
        std::fs::write(
            &local_path,
            r#"{"type":"user","uuid":"1","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z"}
"#,
        )
        .unwrap();
        std::fs::write(
            &remote_path,
            r#"{"type":"user","uuid":"2","sessionId":"s1","timestamp":"2025-01-01T00:01:00Z"}
"#,
        )
        .unwrap();
        let local = ConversationSession::from_file(&local_path).unwrap();
        let remote = ConversationSession::from_file(&remote_path).unwrap();
        let mut conflict = Conflict::new(&local, &remote);
        conflict.merge_error = Some("Failed to create file: s1.jsonl".to_string());

        let labels: Vec<String> = list_menu_options()
            .iter()
            .map(ToString::to_string)
            .collect();

        assert!(
            !labels.iter().any(|label| label.contains("smart merge")),
            "the automatic merge already failed, so the menu can't offer it again: {labels:?}"
        );
        assert!(
            format_menu_header(&conflict).contains("Failed to create file: s1.jsonl"),
            "the menu shows why the automatic merge failed"
        );
    }
}
