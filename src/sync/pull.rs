use anyhow::{Context, Result};
use colored::Colorize;
use indicatif::ProgressIterator;
use inquire::Confirm;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::conflict::ConflictDetector;
use crate::filter::FilterConfig;
use crate::git::{ConflictChoice, ConflictedFile, GitRepo};
use crate::history::{
    ConversationSummary, OperationHistory, OperationRecord, OperationType, SyncOperation,
};
use crate::interactive_conflict;
use crate::later_timestamps::keep_later_timestamps;
use crate::parser::ConversationSession;
use crate::report::{save_conflict_report, ConflictReport};
use crate::undo::Snapshot;

use super::discovery::{claude_home_dir, claude_projects_dir, discover_sessions, warn_large_files};
use super::state::SyncState;
use super::MAX_CONVERSATIONS_TO_DISPLAY;

fn local_destination(
    remote_session: &ConversationSession,
    remote_projects_dir: &Path,
    claude_dir: &Path,
    filter: &FilterConfig,
) -> Option<PathBuf> {
    let remote_relative = remote_session
        .file_path
        .strip_prefix(remote_projects_dir)
        .ok()?;
    let (repo_project_dir, inside_project) =
        crate::project_map::split_project_path(remote_relative)?;
    let local_project_dir =
        crate::project_map::local_project_dir(filter, claude_dir, repo_project_dir)?;

    Some(local_project_dir.join(inside_project))
}

fn settle_conflict(
    file: &ConflictedFile,
    can_ask: bool,
    filter: &FilterConfig,
) -> Result<ConflictChoice> {
    let later_dates_merged = match (&file.local, &file.remote) {
        (Some(local), Some(remote)) => keep_later_timestamps(local, remote),
        _ => None,
    };
    if let Some(merged) = later_dates_merged {
        println!("  {} {}: kept the later dates", "✓".green(), file.path);
        return Ok(ConflictChoice::WriteMerged(merged));
    }

    if !can_ask {
        return Ok(ConflictChoice::AbortMerge);
    }
    crate::merge_tool::resolve_conflict(&filter.merge_tool, filter.prefer_merge_tool, file)
}

pub fn pull_history(
    fetch_remote: bool,
    branch: Option<&str>,
    interactive: bool,
    verbosity: crate::VerbosityLevel,
) -> Result<()> {
    use crate::VerbosityLevel;

    if verbosity != VerbosityLevel::Quiet {
        println!("{}", "Pulling Claude Code history...".cyan().bold());
    }

    let state = SyncState::load()?;
    let repo = GitRepo::open(&state.sync_repo_path)?;
    let filter = FilterConfig::load()?;
    let claude_dir = claude_projects_dir()?;

    let branch_name = branch
        .map(|s| s.to_string())
        .or_else(|| repo.current_branch().ok())
        .unwrap_or_else(|| "main".to_string());

    super::commit_sync_attributes(&repo, &state.sync_repo_path)?;

    if fetch_remote && state.has_remote {
        println!("  {} from remote...", "Fetching".cyan());

        let resolve_conflict = |file: &ConflictedFile| {
            let can_ask = interactive_conflict::is_interactive();
            settle_conflict(file, can_ask, &filter)
        };
        repo.pull("origin", &branch_name, &resolve_conflict)
            .context(
                "Nothing was merged into ~/.claude. Pull again in a terminal to choose \
                 a version of each file both machines changed, or fix the remote — or \
                 run `claude-code-sync pull --fetch-remote false` to merge only what is \
                 already in the local sync repository.",
            )?;
        println!("  {} Pulled from origin/{}", "✓".green(), branch_name);

        super::commit_sync_attributes(&repo, &state.sync_repo_path)?;
    }

    println!("  {} local sessions...", "Discovering".cyan());
    let local_sessions = discover_sessions(&claude_dir, &filter)?;
    println!(
        "  {} {} local sessions",
        "Found".green(),
        local_sessions.len()
    );

    let remote_projects_dir = state.sync_repo_path.join(&filter.sync_subdirectory);
    println!("  {} remote sessions...", "Discovering".cyan());
    let remote_sessions = discover_sessions(&remote_projects_dir, &filter)?;
    println!(
        "  {} {} remote sessions",
        "Found".green(),
        remote_sessions.len()
    );

    if verbosity != VerbosityLevel::Quiet {
        println!("  {} conflicts...", "Detecting".cyan());
    }
    let local_by_path: HashMap<&Path, &ConversationSession> = local_sessions
        .iter()
        .map(|session| (session.file_path.as_path(), session))
        .collect();
    let paired: Vec<(&ConversationSession, &ConversationSession)> = remote_sessions
        .iter()
        .filter_map(|remote| {
            let destination =
                local_destination(remote, &remote_projects_dir, &claude_dir, &filter)?;
            let local = local_by_path.get(destination.as_path())?;
            Some((*local, remote))
        })
        .collect();

    let mut detector = ConflictDetector::new();
    detector.detect(&paired);

    let mut artifact_plan =
        crate::artifacts::engine::plan_pull(&claude_home_dir()?, &state.sync_repo_path, &filter)?;
    artifact_plan.can_ask = interactive_conflict::is_interactive();

    let snapshot_path = if detector.has_conflicts() || !artifact_plan.is_empty() {
        let mut files_to_snapshot: Vec<PathBuf> = detector
            .conflicts()
            .iter()
            .map(|c| c.local_file.clone())
            .collect();
        files_to_snapshot.extend(artifact_plan.paths_to_snapshot());

        println!(
            "  {} snapshot of {} files to be modified...",
            "Creating".cyan(),
            files_to_snapshot.len()
        );

        warn_large_files(&files_to_snapshot);

        let mut snapshot = Snapshot::create(OperationType::Pull, files_to_snapshot.iter(), None)
            .context("Failed to create snapshot before pull")?;

        snapshot.deleted_files = artifact_plan.created_paths();

        let path = snapshot
            .save_to_disk(None)
            .context("Failed to save snapshot to disk")?;

        if verbosity != VerbosityLevel::Quiet {
            println!(
                "  {} Snapshot created: {} ({} files)",
                "✓".green(),
                path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string()),
                files_to_snapshot.len()
            );
        }

        Some(path)
    } else {
        println!("  {} No conflicts - skipping snapshot", "✓".green());
        None
    };

    if verbosity != VerbosityLevel::Quiet {
        println!();
        println!("{}", "Pull Summary:".bold().cyan());
        println!("  {} Local sessions: {}", "•".cyan(), local_sessions.len());
        println!(
            "  {} Remote sessions: {}",
            "•".cyan(),
            remote_sessions.len()
        );
        println!();
    }

    if verbosity == VerbosityLevel::Verbose {
        println!("{}", "Remote sessions to be pulled:".bold());
        for (idx, session) in remote_sessions.iter().enumerate().take(20) {
            let relative_path = session
                .file_path
                .strip_prefix(&remote_projects_dir)
                .unwrap_or(&session.file_path);

            println!(
                "  {}. {} ({} messages)",
                idx + 1,
                relative_path.display(),
                session.message_count()
            );
        }
        if remote_sessions.len() > 20 {
            println!("  ... and {} more", remote_sessions.len() - 20);
        }
        println!();
    }

    if interactive && interactive_conflict::is_interactive() {
        let confirm = Confirm::new("Pull?")
            .with_default(true)
            .prompt()
            .context("Failed to get confirmation")?;

        if !confirm {
            println!("\n{}", "Pull cancelled.".yellow());
            return Ok(());
        }
    }

    let mut affected_conversations: Vec<ConversationSummary> = Vec::new();

    if detector.has_conflicts() {
        println!(
            "  {} {} conflicts detected",
            "!".yellow(),
            detector.conflict_count()
        );

        println!("  {} smart merge...", "Attempting".cyan());

        let remote_by_path: HashMap<&Path, &ConversationSession> = remote_sessions
            .iter()
            .map(|session| (session.file_path.as_path(), session))
            .collect();

        let mut smart_merge_success_count = 0;
        let mut smart_merge_failed_conflicts = Vec::new();

        for conflict in detector.conflicts_mut() {
            if let (Some(local_session), Some(remote_session)) = (
                local_by_path.get(conflict.local_file.as_path()),
                remote_by_path.get(conflict.remote_file.as_path()),
            ) {
                match conflict.smart_merge_into_local_file(local_session, remote_session) {
                    Ok(()) => {
                        smart_merge_success_count += 1;
                        if let crate::conflict::ConflictResolution::SmartMerge { ref stats } =
                            conflict.resolution
                        {
                            println!(
                                "  {} Smart merged {} ({} local + {} remote = {} total, {} branches)",
                                "✓".green(),
                                conflict.session_id,
                                stats.local_messages,
                                stats.remote_messages,
                                stats.merged_messages,
                                stats.branches_detected
                            );
                        }
                    }
                    Err(e) => {
                        log::warn!("Smart merge failed for {}: {}", conflict.session_id, e);
                        log::info!("Falling back to manual resolution...");
                        smart_merge_failed_conflicts.push(conflict.clone());
                    }
                }
            }
        }

        println!(
            "  {} Successfully smart merged {}/{} conflicts",
            "✓".green(),
            smart_merge_success_count,
            detector.conflict_count()
        );

        let renames = if !smart_merge_failed_conflicts.is_empty() {
            println!(
                "  {} {} conflicts require manual resolution",
                "!".yellow(),
                smart_merge_failed_conflicts.len()
            );

            let use_interactive = crate::interactive_conflict::is_interactive();

            if use_interactive {
                println!(
                    "\n{} Running in interactive mode for remaining conflicts",
                    "→".cyan()
                );

                let resolution_result = crate::interactive_conflict::resolve_conflicts_interactive(
                    &mut smart_merge_failed_conflicts,
                )?;

                let renames = crate::interactive_conflict::apply_resolutions(
                    &resolution_result,
                    &remote_sessions,
                    &claude_dir,
                    &remote_projects_dir,
                )?;

                let report = ConflictReport::from_conflicts(detector.conflicts());
                save_conflict_report(&report)?;

                renames
            } else {
                println!(
                    "\n{} Using automatic conflict resolution (keep both versions)",
                    "→".cyan()
                );

                let mut renames = Vec::new();

                println!("\n{}", "Conflict Resolution:".yellow().bold());
                for conflict in &smart_merge_failed_conflicts {
                    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
                    let conflict_suffix = format!("conflict-{timestamp}");

                    if let Ok(renamed_path) = conflict.clone().resolve_keep_both(&conflict_suffix) {
                        let relative_renamed = renamed_path
                            .strip_prefix(&claude_dir)
                            .unwrap_or(&renamed_path);
                        println!(
                            "  {} remote version saved as: {}",
                            "→".yellow(),
                            relative_renamed.display().to_string().cyan()
                        );

                        if let Some(session) = remote_sessions
                            .iter()
                            .find(|s| s.session_id == conflict.session_id)
                        {
                            session.copy_to(&renamed_path)?;
                        }

                        renames.push((conflict.remote_file.clone(), renamed_path));
                    }
                }

                let report = ConflictReport::from_conflicts(detector.conflicts());
                save_conflict_report(&report)?;

                renames
            }
        } else {
            Vec::new()
        };

        for (_original_path, renamed_path) in &renames {
            let relative_path = renamed_path
                .strip_prefix(&claude_dir)
                .unwrap_or(renamed_path)
                .to_string_lossy()
                .to_string();

            if let Some(session) = remote_sessions.iter().find(|s| {
                let session_file = s.file_path.file_name();
                let renamed_file = renamed_path.file_name();
                session_file
                    .and_then(|f| f.to_str())
                    .and_then(|name| name.split('-').next())
                    == renamed_file
                        .and_then(|f| f.to_str())
                        .and_then(|name| name.split('-').next())
            }) {
                match ConversationSummary::new(
                    session.session_id.clone(),
                    relative_path.clone(),
                    session.latest_timestamp().map(str::to_string),
                    session.message_count(),
                    SyncOperation::Conflict,
                ) {
                    Ok(summary) => affected_conversations.push(summary),
                    Err(e) => log::warn!(
                        "Failed to create summary for conflict {}: {}",
                        relative_path,
                        e
                    ),
                }
            }
        }

        println!(
            "\n{} View details with: claude-code-sync report",
            "Hint:".cyan()
        );
    } else {
        println!("  {} No conflicts detected", "✓".green());
    }

    println!("  {} non-conflicting sessions...", "Merging".cyan());
    let mut merged_count = 0;
    let mut added_count = 0;
    let mut modified_count = 0;
    let mut unchanged_count = 0;
    let mut skipped_no_local_match = 0;
    let mut skipped_by_project = crate::project_map::SkippedByProject::new();

    let progress = crate::progress::bar(remote_sessions.len(), "Merging sessions");
    for remote_session in remote_sessions.iter().progress_with(progress) {
        let remote_relative = remote_session
            .file_path
            .strip_prefix(&remote_projects_dir)
            .unwrap_or(&remote_session.file_path);

        let destination =
            local_destination(remote_session, &remote_projects_dir, &claude_dir, &filter);
        let Some(dest_path) = destination else {
            match crate::project_map::split_project_path(remote_relative) {
                Some((repo_project_dir, _)) => {
                    skipped_no_local_match += 1;
                    skipped_by_project
                        .entry(repo_project_dir.to_string())
                        .or_default()
                        .push(remote_session.file_path.clone());
                }
                None => log::warn!(
                    "Skipping {} (not a transcript inside a project directory)",
                    remote_session.file_path.display()
                ),
            }
            continue;
        };

        if detector
            .conflicts()
            .iter()
            .any(|conflict| conflict.local_file == dest_path)
        {
            continue;
        }

        let relative_path_for_tracking = dest_path
            .strip_prefix(&claude_dir)
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|_| remote_relative.to_path_buf());

        let operation = if let Some(local) = local_by_path.get(dest_path.as_path()) {
            if local.content_hash() == remote_session.content_hash() {
                unchanged_count += 1;
                SyncOperation::Unchanged
            } else {
                modified_count += 1;
                SyncOperation::Modified
            }
        } else {
            added_count += 1;
            SyncOperation::Added
        };

        if operation != SyncOperation::Unchanged {
            remote_session.copy_to(&dest_path)?;
            merged_count += 1;
        }

        let relative_path_str = relative_path_for_tracking.to_string_lossy().to_string();
        match ConversationSummary::new(
            remote_session.session_id.clone(),
            relative_path_str.clone(),
            remote_session.latest_timestamp().map(str::to_string),
            remote_session.message_count(),
            operation,
        ) {
            Ok(summary) => affected_conversations.push(summary),
            Err(e) => log::warn!("Failed to create summary for {}: {}", relative_path_str, e),
        }
    }

    println!("  {} Merged {} sessions", "✓".green(), merged_count);

    crate::project_map::merge_skipped(&mut skipped_by_project, &artifact_plan.unmapped_projects);
    for line in crate::project_map::skipped_project_warnings(
        &skipped_by_project,
        filter.warn_each_skipped_file,
    ) {
        log::warn!("{line}");
    }

    let artifact_report = crate::artifacts::engine::apply_pull(&artifact_plan, interactive)?;
    crate::artifacts::engine::record_synced_hashes(
        &claude_home_dir()?,
        &state.sync_repo_path,
        &filter,
    )?;
    if !artifact_plan.is_empty() {
        println!(
            "  {} Artifacts: {} created, {} overwritten locally, {} deleted locally",
            "✓".green(),
            artifact_report.total_added(),
            artifact_report.total_modified(),
            artifact_report.total_deleted()
        );
        if artifact_report.total_modified() > 0 || artifact_report.total_deleted() > 0 {
            println!("    {}", "Undo with: claude-code-sync undo pull".dimmed());
        }
    }

    let mut operation_record = OperationRecord::new(
        OperationType::Pull,
        Some(branch_name.clone()),
        affected_conversations.clone(),
    );

    operation_record.snapshot_path = snapshot_path;
    operation_record.artifact_counts = artifact_report.counts.clone();

    let mut history = match OperationHistory::load() {
        Ok(h) => h,
        Err(e) => {
            log::warn!("Failed to load operation history: {}", e);
            log::info!("Creating new history...");
            OperationHistory::default()
        }
    };

    if let Err(e) = history.add_operation(operation_record) {
        log::warn!("Failed to save operation to history: {}", e);
        log::info!("Pull completed successfully, but history was not updated.");
    }

    println!("\n{}", "=== Pull Summary ===".bold().cyan());

    let conflict_count = detector.conflict_count();
    let stats_msg = format!(
        "  {} Added    {} Modified    {} Conflicts    {} Unchanged",
        format!("{added_count}").green(),
        format!("{modified_count}").cyan(),
        format!("{conflict_count}").yellow(),
        format!("{unchanged_count}").dimmed(),
    );
    println!("{stats_msg}");
    if skipped_no_local_match > 0 {
        println!(
            "  {} Skipped sessions (no local match): {}",
            "!".yellow(),
            skipped_no_local_match
        );
    }
    if !artifact_report.counts.is_empty() || artifact_plan.unchanged > 0 {
        println!(
            "  {} Artifacts: {} added, {} modified, {} deleted, {} unchanged",
            "•".cyan(),
            artifact_report.total_added(),
            artifact_report.total_modified(),
            artifact_report.total_deleted(),
            artifact_plan.unchanged
        );
    }
    println!();

    let mut by_project: HashMap<String, Vec<&ConversationSummary>> = HashMap::new();
    for conv in &affected_conversations {
        if conv.operation == SyncOperation::Unchanged {
            continue;
        }

        let project = conv
            .project_path
            .split('/')
            .next()
            .unwrap_or("unknown")
            .to_string();
        by_project.entry(project).or_default().push(conv);
    }

    if !by_project.is_empty() {
        println!("{}", "Affected Conversations:".bold());

        let mut projects: Vec<_> = by_project.keys().collect();
        projects.sort();

        for project in projects {
            let conversations = &by_project[project];
            println!("\n  {} {}/", "Project:".bold(), project.cyan());

            for conv in conversations.iter().take(MAX_CONVERSATIONS_TO_DISPLAY) {
                let operation_str = match conv.operation {
                    SyncOperation::Added => "ADD".green(),
                    SyncOperation::Modified => "MOD".cyan(),
                    SyncOperation::Conflict => "CONFLICT".yellow(),
                    SyncOperation::Unchanged => "---".dimmed(),
                };

                let timestamp_str = conv
                    .timestamp
                    .as_ref()
                    .and_then(|t| t.split('T').next())
                    .unwrap_or("unknown");

                println!(
                    "    {} {} ({}msg, {})",
                    operation_str,
                    conv.project_path,
                    conv.message_count,
                    timestamp_str.dimmed()
                );
            }

            if conversations.len() > MAX_CONVERSATIONS_TO_DISPLAY {
                println!(
                    "    {} ... and {} more conversations",
                    "...".dimmed(),
                    conversations.len() - MAX_CONVERSATIONS_TO_DISPLAY
                );
            }
        }
    }

    super::print_artifact_changes(&artifact_report);

    println!("\n{}", "Pull complete!".green().bold());

    if let Err(e) = crate::undo::cleanup_old_snapshots(None, false) {
        log::warn!("Failed to cleanup old snapshots: {}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conflicted(local: Option<&str>, remote: Option<&str>) -> ConflictedFile {
        ConflictedFile {
            path: "artifacts/plugins/known_marketplaces.json".to_string(),
            base: None,
            local: local.map(|text| text.as_bytes().to_vec()),
            remote: remote.map(|text| text.as_bytes().to_vec()),
        }
    }

    #[test]
    fn a_conflict_settles_without_asking_only_when_the_dates_alone_differ() {
        let filter = FilterConfig::default();
        let cases = [
            (
                "dates alone differ",
                conflicted(
                    Some("\"lastUpdated\": \"2026-10-01T06:00:01.741Z\""),
                    Some("\"lastUpdated\": \"2026-10-01T06:50:50.567Z\""),
                ),
                ConflictChoice::WriteMerged(
                    b"\"lastUpdated\": \"2026-10-01T06:50:50.567Z\"".to_vec(),
                ),
            ),
            (
                "content differs too",
                conflicted(
                    Some("a 2026-10-01T06:00:01Z"),
                    Some("b 2026-10-01T06:00:02Z"),
                ),
                ConflictChoice::AbortMerge,
            ),
            (
                "deleted on one side",
                conflicted(Some("a"), None),
                ConflictChoice::AbortMerge,
            ),
        ];

        for (name, file, expected) in cases {
            let choice = settle_conflict(&file, false, &filter).unwrap();
            assert_eq!(choice, expected, "{name}");
        }
    }
}
