pub mod discovery;
mod init;
mod pull;
pub mod push;
mod remote;
mod state;
mod status;
mod summary_cache;

pub use init::{init_from_onboarding, init_sync_repo, print_map_project_hint};
pub use pull::pull_history;
pub use push::push_history;
pub use remote::{remove_remote, set_remote, show_remote};
pub use state::{MultiRepoState, RepoConfig, SyncState};
pub use status::show_status;

use anyhow::Result;
use colored::Colorize;

use crate::artifacts::engine::{descriptor, ArtifactChangeKind, ArtifactReport, CategoryCounts};

const MAX_CONVERSATIONS_TO_DISPLAY: usize = 10;

pub(crate) fn limit_checkout(
    repo: &crate::git::GitRepo,
    filter: &crate::filter::FilterConfig,
) -> Result<()> {
    let directories = filter
        .mapped_projects_only
        .then(|| crate::project_map::checkout_directories(filter));
    let limited = repo.limit_checkout(directories.as_deref());
    let Err(error) = limited else {
        return Ok(());
    };
    if !filter.mapped_projects_only {
        return Err(error.context("Failed to check out every project in the sync repository"));
    }
    log::warn!("Kept the sync repository's current checkout: {error:#}");
    Ok(())
}

pub(crate) fn print_artifact_changes(report: &ArtifactReport) {
    let has_no_changes = report.changes.is_empty();
    if has_no_changes {
        return;
    }

    println!("\n{}", "Changed Artifacts:".bold());

    for counts in &report.counts {
        let mut changes: Vec<_> = report
            .changes
            .iter()
            .filter(|change| change.category == counts.category)
            .collect();
        let change_count = changes.len();
        if change_count == 0 {
            continue;
        }
        changes.sort_by(|left, right| left.path.cmp(&right.path));

        let category_name = descriptor(counts.category).name;
        println!(
            "\n  {} {}/  {}",
            "Category:".bold(),
            category_name.cyan(),
            describe_change_counts(counts).dimmed()
        );

        for change in changes.iter().take(MAX_CONVERSATIONS_TO_DISPLAY) {
            let kind_label = match change.kind {
                ArtifactChangeKind::Added => "ADD".green(),
                ArtifactChangeKind::Modified => "MOD".cyan(),
                ArtifactChangeKind::Deleted => "DEL".red(),
            };
            println!("    {} {}", kind_label, change.path.display());
        }

        if change_count > MAX_CONVERSATIONS_TO_DISPLAY {
            let hidden_count = change_count - MAX_CONVERSATIONS_TO_DISPLAY;
            println!(
                "    {}",
                format!("... and {hidden_count} more files").dimmed()
            );
        }
    }
}

fn describe_change_counts(counts: &CategoryCounts) -> String {
    let labelled_counts = [
        (counts.added, "added"),
        (counts.modified, "modified"),
        (counts.deleted, "deleted"),
    ];

    labelled_counts
        .iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, label)| format!("{count} {label}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn commit_sync_attributes(
    repo: &crate::git::GitRepo,
    repo_path: &std::path::Path,
) -> Result<()> {
    if repo.has_changes()? {
        return Ok(());
    }
    if !crate::git::attributes::ensure_sync_attributes(repo_path)? {
        return Ok(());
    }

    repo.stage_all()?;
    repo.stage_renormalized()?;
    repo.commit("Store text files with LF and merge append-only files")?;

    Ok(())
}

pub fn sync_bidirectional(
    commit_message: Option<&str>,
    branch: Option<&str>,
    exclude_attachments: bool,
    interactive: bool,
    verbosity: crate::VerbosityLevel,
) -> Result<()> {
    use crate::VerbosityLevel;

    if verbosity != VerbosityLevel::Quiet {
        println!("{}", "=== Bidirectional Sync ===".bold().cyan());
        println!();
        println!("{}", "Step 1: Pulling remote changes...".bold());
    }

    pull_history(true, branch, interactive, verbosity)?;

    let filter = crate::filter::FilterConfig::load()?;
    if filter.purge_after_sync {
        let state = SyncState::load()?;
        crate::handlers::purge::purge_after_sync(&filter, &state.sync_repo_path)?;
    }

    if verbosity != VerbosityLevel::Quiet {
        println!();
        println!("{}", "Step 2: Pushing local changes...".bold());
    }

    push_history(
        commit_message,
        true,
        branch,
        exclude_attachments,
        interactive,
        verbosity,
    )?;

    if verbosity == VerbosityLevel::Quiet {
        println!("Sync complete");
    } else {
        println!();
        println!("{}", "=== Sync Complete ===".green().bold());
        println!(
            "  {} Your local and remote histories are now in sync",
            "✓".green()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::FilterConfig;
    use crate::git::GitRepo;
    use serial_test::serial;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn git_output(dir: &Path, args: &[&str]) -> Vec<u8> {
        let output = Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    #[test]
    fn adding_the_rules_converts_a_hook_stored_with_crlf_to_lf() {
        let repo_dir = TempDir::new().unwrap();
        let repo_path = repo_dir.path();
        let repo = GitRepo::init(repo_path).unwrap();
        git_output(repo_path, &["config", "user.name", "Old"]);
        git_output(repo_path, &["config", "user.email", "old@local"]);
        git_output(repo_path, &["config", "core.autocrlf", "false"]);
        std::fs::write(repo_path.join("hook.sh"), "#!/bin/sh\r\necho hi\r\n").unwrap();
        std::fs::write(repo_path.join(".gitattributes"), "*.jsonl merge=union\n").unwrap();
        repo.stage_all().unwrap();
        repo.commit("written by an older version").unwrap();

        commit_sync_attributes(&repo, repo_path).unwrap();

        let stored_hook = git_output(repo_path, &["show", "HEAD:hook.sh"]);
        let commit_count = git_output(repo_path, &["rev-list", "--count", "HEAD"]);
        assert_eq!(stored_hook, b"#!/bin/sh\necho hi\n");
        assert_eq!(commit_count, b"2\n");
    }

    #[test]
    #[serial]
    fn test_url_validation() {
        let temp_dir = TempDir::new().unwrap();
        let prev = std::env::var("CLAUDE_CODE_SYNC_CONFIG_DIR").ok();
        std::env::set_var("CLAUDE_CODE_SYNC_CONFIG_DIR", temp_dir.path());
        let repo_path = temp_dir.path().join("test-repo");

        GitRepo::init(&repo_path).unwrap();

        let state = SyncState {
            sync_repo_path: repo_path.clone(),
            has_remote: false,
            is_cloned_repo: false,
        };

        let _state_path = crate::config::ConfigManager::ensure_config_dir().unwrap();
        let state_file = crate::config::ConfigManager::state_file_path().unwrap();
        std::fs::write(&state_file, serde_json::to_string(&state).unwrap()).unwrap();

        let result = set_remote("origin", "https://github.com/user/repo.git");
        assert!(result.is_ok());

        let result = set_remote("origin", "http://gitlab.com/user/repo.git");
        assert!(result.is_ok());

        let result = set_remote("origin", "git@github.com:user/repo.git");
        assert!(result.is_ok());

        let result = set_remote("origin", "github.com/user/repo.git");
        assert!(result.is_err());
        if let Err(e) = result {
            let error_msg = e.to_string();
            assert!(error_msg.contains("Invalid URL format"));
        }

        std::fs::remove_file(&state_file).ok();
        match prev {
            Some(v) => std::env::set_var("CLAUDE_CODE_SYNC_CONFIG_DIR", v),
            None => std::env::remove_var("CLAUDE_CODE_SYNC_CONFIG_DIR"),
        }
    }

    #[test]
    fn test_filter_with_attachments() {
        let filter = FilterConfig {
            exclude_attachments: true,
            ..Default::default()
        };

        assert!(filter.should_include(Path::new("session.jsonl")));
        assert!(filter.should_include(Path::new("/path/to/session.jsonl")));

        assert!(!filter.should_include(Path::new("image.png")));
        assert!(!filter.should_include(Path::new("document.pdf")));
        assert!(!filter.should_include(Path::new("archive.zip")));
        assert!(!filter.should_include(Path::new("/path/to/file.jpg")));
    }

    #[test]
    fn test_filter_without_attachments_exclusion() {
        let filter = FilterConfig::default();

        assert!(filter.should_include(Path::new("session.jsonl")));
        assert!(filter.should_include(Path::new("image.png")));
        assert!(filter.should_include(Path::new("document.pdf")));
    }
}
