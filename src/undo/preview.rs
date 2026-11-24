use anyhow::{anyhow, Result};
use std::path::PathBuf;

use super::snapshot::Snapshot;
use crate::history::{OperationHistory, OperationType};

#[derive(Debug)]
pub struct UndoPreview {
    pub operation_type: OperationType,
    pub operation_timestamp: time::OffsetDateTime,
    pub branch: Option<String>,
    pub affected_files: Vec<String>,
    pub conversation_count: usize,
    pub commit_hash: Option<String>,
    pub snapshot_timestamp: Option<time::OffsetDateTime>,
    pub files_to_delete: Vec<String>,
    pub files_changed_since_pull: Vec<String>,
}

const NORMAL_FILE_LIST_LIMIT: usize = 10;

fn print_file_list(title: &str, files: &[String], limit: usize) {
    use colored::Colorize;

    if files.is_empty() {
        return;
    }
    println!("\n{}", title.bold());
    for file in files.iter().take(limit) {
        println!("  • {}", file.dimmed());
    }
    if files.len() > limit {
        println!(
            "  ... and {} more files",
            (files.len() - limit).to_string().dimmed()
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbosityLevel {
    Quiet,
    Normal,
    Verbose,
}

impl UndoPreview {
    pub fn display(&self, verbosity: VerbosityLevel) {
        use colored::Colorize;

        match verbosity {
            VerbosityLevel::Quiet => {
                let op_type = match self.operation_type {
                    OperationType::Pull => "Pull",
                    OperationType::Push => "Push",
                };
                println!(
                    "Undo {}: {} conversations affected",
                    op_type, self.conversation_count
                );
                if !self.affected_files.is_empty() {
                    println!("  {} files will be restored", self.affected_files.len());
                }
                if !self.files_to_delete.is_empty() {
                    println!("  {} files will be removed", self.files_to_delete.len());
                }
                if !self.files_changed_since_pull.is_empty() {
                    println!(
                        "  {} files changed after the pull stay as they are",
                        self.files_changed_since_pull.len()
                    );
                }
            }

            VerbosityLevel::Normal => {
                println!("\n{}", "=".repeat(80).yellow());
                println!("{}", "Undo Preview".bold().yellow());
                println!("{}", "=".repeat(80).yellow());

                let op_type = match self.operation_type {
                    OperationType::Pull => "PULL".green(),
                    OperationType::Push => "PUSH".blue(),
                };

                println!("\n{} {}", "Operation:".bold(), op_type);
                println!(
                    "{} {}",
                    "Performed:".bold(),
                    crate::datetime::format_utc(
                        self.operation_timestamp,
                        crate::datetime::DATE_SECONDS_UTC
                    )
                    .cyan()
                );

                if let Some(branch) = &self.branch {
                    println!("{} {}", "Branch:".bold(), branch.cyan());
                }

                if let Some(commit) = &self.commit_hash {
                    let short_hash = if commit.len() >= 8 {
                        &commit[..8]
                    } else {
                        commit.as_str()
                    };
                    println!("{} {}", "Will reset to:".bold(), short_hash.yellow());
                }

                println!(
                    "\n{} {}",
                    "Conversations affected:".bold(),
                    self.conversation_count.to_string().yellow()
                );

                print_file_list(
                    "Files to be restored:",
                    &self.affected_files,
                    NORMAL_FILE_LIST_LIMIT,
                );
                print_file_list(
                    "Files the pull created, to be removed:",
                    &self.files_to_delete,
                    NORMAL_FILE_LIST_LIMIT,
                );
                print_file_list(
                    "Files changed after the pull, kept as they are:",
                    &self.files_changed_since_pull,
                    NORMAL_FILE_LIST_LIMIT,
                );

                if let Some(snapshot_ts) = self.snapshot_timestamp {
                    println!(
                        "\n{} {}",
                        "Snapshot taken:".bold(),
                        crate::datetime::format_utc(snapshot_ts, crate::datetime::DATE_SECONDS_UTC)
                            .dimmed()
                    );
                }

                println!("{}", "=".repeat(80).yellow());
            }

            VerbosityLevel::Verbose => {
                println!("\n{}", "=".repeat(80).yellow());
                println!("{}", "Undo Preview (Verbose Mode)".bold().yellow());
                println!("{}", "=".repeat(80).yellow());

                let op_type = match self.operation_type {
                    OperationType::Pull => "PULL".green(),
                    OperationType::Push => "PUSH".blue(),
                };

                println!("\n{} {}", "Operation Type:".bold(), op_type);
                println!(
                    "{} {}",
                    "Performed at:".bold(),
                    crate::datetime::format_utc(
                        self.operation_timestamp,
                        crate::datetime::DATE_SECONDS_UTC
                    )
                    .cyan()
                );

                if let Some(branch) = &self.branch {
                    println!("{} {}", "Branch:".bold(), branch.cyan());
                }

                if let Some(commit) = &self.commit_hash {
                    let short_hash = if commit.len() >= 8 {
                        &commit[..8]
                    } else {
                        commit.as_str()
                    };
                    println!(
                        "{} {} (full: {})",
                        "Will reset to:".bold(),
                        short_hash.yellow(),
                        commit.dimmed()
                    );
                }

                println!(
                    "\n{} {}",
                    "Total conversations affected:".bold(),
                    self.conversation_count.to_string().yellow()
                );

                if !self.affected_files.is_empty() {
                    println!(
                        "\n{} ({} total)",
                        "Files to be restored:".bold(),
                        self.affected_files.len()
                    );
                    for (idx, file) in self.affected_files.iter().enumerate() {
                        println!("  {}. {}", idx + 1, file);

                        if let Ok(metadata) = std::fs::metadata(file) {
                            let size_kb = metadata.len() as f64 / 1024.0;
                            println!("     {} {:.1} KB", "Size:".dimmed(), size_kb);
                        }
                    }
                }
                print_file_list(
                    "Files the pull created, to be removed:",
                    &self.files_to_delete,
                    usize::MAX,
                );
                print_file_list(
                    "Files changed after the pull, kept as they are:",
                    &self.files_changed_since_pull,
                    usize::MAX,
                );

                if let Some(snapshot_ts) = self.snapshot_timestamp {
                    println!(
                        "\n{} {}",
                        "Snapshot created:".bold(),
                        crate::datetime::format_utc(snapshot_ts, crate::datetime::DATE_SECONDS_UTC)
                            .cyan()
                    );

                    let time_diff = time::OffsetDateTime::now_utc() - snapshot_ts;
                    let days = time_diff.whole_days();
                    let hours = time_diff.whole_hours() % 24;
                    let mins = time_diff.whole_minutes() % 60;
                    println!(
                        "  {} {} days, {} hours, {} minutes ago",
                        "Age:".dimmed(),
                        days,
                        hours,
                        mins
                    );
                }

                println!("{}", "=".repeat(80).yellow());
            }
        }
    }
}

pub fn preview_undo_pull(history_path: Option<PathBuf>) -> Result<UndoPreview> {
    let history = OperationHistory::from_path(history_path)?;

    let last_pull = history
        .get_last_operation_by_type(OperationType::Pull)
        .ok_or_else(|| anyhow!("No pull operation found in history to undo"))?;

    let snapshot_path = last_pull.snapshot_path.as_ref().ok_or_else(|| {
        anyhow!(
            "No snapshot found for last pull operation. \
                Cannot undo without a snapshot."
        )
    })?;

    if !snapshot_path.exists() {
        return Err(anyhow!(
            "Snapshot file not found: {}. \
            The snapshot may have been deleted.",
            snapshot_path.display()
        ));
    }

    let snapshot = Snapshot::load_from_disk(snapshot_path)?;

    let affected_files = snapshot.list_files_to_restore();

    Ok(UndoPreview {
        operation_type: OperationType::Pull,
        operation_timestamp: last_pull.timestamp,
        branch: last_pull.branch.clone(),
        affected_files,
        conversation_count: last_pull.affected_conversations.len(),
        commit_hash: None,
        snapshot_timestamp: Some(snapshot.timestamp),
        files_to_delete: snapshot.list_created_files_to_remove(),
        files_changed_since_pull: snapshot.list_files_changed_since_pull(),
    })
}

pub fn preview_undo_push(history_path: Option<PathBuf>) -> Result<UndoPreview> {
    let history = OperationHistory::from_path(history_path)?;

    let last_push = history
        .get_last_operation_by_type(OperationType::Push)
        .ok_or_else(|| anyhow!("No push operation found in history to undo"))?;

    if let Some(ref hash) = last_push.commit_hash {
        return Ok(UndoPreview {
            operation_type: OperationType::Push,
            operation_timestamp: last_push.timestamp,
            branch: last_push.branch.clone(),
            affected_files: Vec::new(),
            conversation_count: last_push.affected_conversations.len(),
            commit_hash: Some(hash.clone()),
            snapshot_timestamp: None,
            files_to_delete: Vec::new(),
            files_changed_since_pull: Vec::new(),
        });
    }

    let snapshot_path = last_push.snapshot_path.as_ref().ok_or_else(|| {
        anyhow!(
            "No commit hash or snapshot found for last push operation. \
                Cannot undo."
        )
    })?;

    if !snapshot_path.exists() {
        return Err(anyhow!(
            "Snapshot file not found: {}. \
            The snapshot may have been deleted.",
            snapshot_path.display()
        ));
    }

    let snapshot = Snapshot::load_from_disk(snapshot_path)?;

    Ok(UndoPreview {
        operation_type: OperationType::Push,
        operation_timestamp: last_push.timestamp,
        branch: snapshot.branch.clone(),
        affected_files: Vec::new(),
        conversation_count: last_push.affected_conversations.len(),
        commit_hash: snapshot.git_commit_hash.clone(),
        snapshot_timestamp: Some(snapshot.timestamp),
        files_to_delete: Vec::new(),
        files_changed_since_pull: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::{OperationRecord, OperationType};
    use tempfile::TempDir;

    #[test]
    fn test_preview_undo_push_with_commit_hash_only() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("operation-history.json");

        let mut record =
            OperationRecord::new(OperationType::Push, Some("main".to_string()), vec![]);
        record.commit_hash = Some("abcdef1234567890".to_string());
        assert!(record.snapshot_path.is_none());

        let mut history = OperationHistory::from_path(Some(history_path.clone())).unwrap();
        history.add_operation(record).unwrap();

        let preview = preview_undo_push(Some(history_path)).unwrap();
        assert_eq!(preview.commit_hash.as_deref(), Some("abcdef1234567890"));
        assert_eq!(preview.branch.as_deref(), Some("main"));
        assert!(preview.affected_files.is_empty());
        assert!(preview.snapshot_timestamp.is_none());
    }
}
