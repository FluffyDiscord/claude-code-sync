use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use walkdir::WalkDir;

use claude_code_sync::git::GitRepo;
use claude_code_sync::history::{
    ConversationSummary, OperationHistory, OperationType, SyncOperation,
};
use claude_code_sync::parser::ConversationSession;
use claude_code_sync::sync::SyncState;
use claude_code_sync::undo::{undo_pull, undo_push, Snapshot};

const TEST_DATA_DIR: &str = "data";

fn copy_test_data(dest_projects_dir: &Path) -> anyhow::Result<()> {
    let test_data = Path::new(TEST_DATA_DIR);

    for entry in fs::read_dir(test_data)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path.file_name().unwrap();
            let dest_dir = dest_projects_dir.join(dir_name);

            fs::create_dir_all(&dest_dir)?;

            for file_entry in WalkDir::new(&path).max_depth(1) {
                let file_entry = file_entry?;
                let file_path = file_entry.path();

                if file_path.is_file()
                    && file_path.extension().and_then(|s| s.to_str()) == Some("jsonl")
                {
                    let file_name = file_path.file_name().unwrap();
                    let dest_file = dest_dir.join(file_name);
                    fs::copy(file_path, dest_file)?;
                }
            }
        }
    }

    Ok(())
}

fn create_test_sync_state(sync_repo_path: &Path, state_dir: &Path) -> anyhow::Result<PathBuf> {
    let state = SyncState {
        sync_repo_path: sync_repo_path.to_path_buf(),
        has_remote: false,
        is_cloned_repo: false,
    };

    let state_file = state_dir.join("state.json");
    fs::create_dir_all(state_dir)?;

    let content = serde_json::to_string_pretty(&state)?;
    fs::write(&state_file, content)?;

    Ok(state_file)
}

fn count_jsonl_files(dir: &Path) -> usize {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("jsonl"))
        .count()
}

fn discover_test_sessions(base_path: &Path) -> anyhow::Result<Vec<ConversationSession>> {
    let mut sessions = Vec::new();

    for entry in WalkDir::new(base_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            match ConversationSession::from_file(path) {
                Ok(session) => sessions.push(session),
                Err(e) => log::warn!("Failed to parse {}: {}", path.display(), e),
            }
        }
    }

    Ok(sessions)
}

fn create_test_filter_config(config_dir: &Path) -> anyhow::Result<()> {
    use claude_code_sync::filter::FilterConfig;

    let filter = FilterConfig::default();
    let filter_path = config_dir.join("filter.json");

    fs::create_dir_all(config_dir)?;
    let content = serde_json::to_string_pretty(&filter)?;
    fs::write(&filter_path, content)?;

    Ok(())
}

#[test]
fn test_full_push_pull_cycle() {
    let sync_repo_dir = TempDir::new().unwrap();
    let claude_dir = TempDir::new().unwrap();
    let config_dir = TempDir::new().unwrap();

    let sync_repo_path = sync_repo_dir.path();
    let claude_projects_dir = claude_dir.path().join("projects");
    fs::create_dir_all(&claude_projects_dir).unwrap();

    copy_test_data(&claude_projects_dir).unwrap();

    let file_count = count_jsonl_files(&claude_projects_dir);
    assert!(
        file_count >= 5,
        "Expected at least 5 test files, found {file_count}"
    );

    let repo = GitRepo::init(sync_repo_path).unwrap();

    let readme_path = sync_repo_path.join("README.md");
    fs::write(&readme_path, "# Claude Sync Test Repo\n").unwrap();
    repo.stage_all().unwrap();
    repo.commit("Initial commit").unwrap();

    let _state_file = create_test_sync_state(sync_repo_path, config_dir.path()).unwrap();
    create_test_filter_config(config_dir.path()).unwrap();

    std::env::set_var("HOME", config_dir.path());

    let original_sessions = discover_test_sessions(&claude_projects_dir).unwrap();
    assert!(
        original_sessions.len() >= 5,
        "Should have at least 5 test sessions"
    );

    let projects_dir = sync_repo_path.join("projects");
    fs::create_dir_all(&projects_dir).unwrap();

    for session in &original_sessions {
        let relative_path = Path::new(&session.file_path)
            .strip_prefix(&claude_projects_dir)
            .unwrap_or(Path::new(&session.file_path));

        let dest_path = projects_dir.join(relative_path);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        session.copy_to(&dest_path).unwrap();
    }

    repo.stage_all().unwrap();
    let has_changes = repo.has_changes().unwrap();
    assert!(has_changes, "Should have changes to commit");

    repo.commit("Test push").unwrap();

    let history_path = config_dir
        .path()
        .join(".claude-code-sync")
        .join("operation-history.json");
    let history = OperationHistory::from_path(Some(history_path.clone())).unwrap();

    let mut conversations = Vec::new();
    for session in &original_sessions {
        let relative_path = Path::new(&session.file_path)
            .strip_prefix(&claude_projects_dir)
            .unwrap_or(Path::new(&session.file_path))
            .to_string_lossy()
            .to_string();

        let summary = ConversationSummary::new(
            session.session_id.clone(),
            relative_path,
            session.latest_timestamp().map(str::to_string),
            session.message_count(),
            SyncOperation::Added,
        )
        .unwrap();

        conversations.push(summary);
    }

    let push_record = claude_code_sync::history::OperationRecord::new(
        OperationType::Push,
        Some("main".to_string()),
        conversations,
    );

    let mut history_loaded = history;
    history_loaded.operations.insert(0, push_record);
    history_loaded.save_to(Some(history_path.clone())).unwrap();

    let history = OperationHistory::from_path(Some(history_path.clone())).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(
        history.get_last_operation().unwrap().operation_type,
        OperationType::Push
    );

    let sync_sessions = discover_test_sessions(&projects_dir).unwrap();
    assert_eq!(sync_sessions.len(), original_sessions.len());

    let modified_id = sync_sessions.first().unwrap().session_id.clone();
    {
        let first_session = sync_sessions.first().unwrap();
        let session_path = projects_dir.join(&first_session.file_path);
        let content = fs::read_to_string(&session_path).unwrap();
        let modified_content = format!("{}\n{{\"type\":\"user\",\"uuid\":\"test-uuid\",\"sessionId\":\"{}\",\"timestamp\":\"2025-10-18T00:00:00Z\"}}\n",
            content.trim(), modified_id);
        fs::write(&session_path, modified_content).unwrap();

        repo.stage_all().unwrap();
        repo.commit("Modified conversation").unwrap();
    }

    let machine2_dir = TempDir::new().unwrap();
    let machine2_projects = machine2_dir.path().join("projects");
    fs::create_dir_all(&machine2_projects).unwrap();

    for session in &original_sessions {
        let relative_path = Path::new(&session.file_path)
            .strip_prefix(&claude_projects_dir)
            .unwrap_or(Path::new(&session.file_path));

        let dest_path = machine2_projects.join(relative_path);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        session.copy_to(&dest_path).unwrap();
    }

    let sync_sessions_after_modify = discover_test_sessions(&projects_dir).unwrap();
    for session in &sync_sessions_after_modify {
        let relative_path = Path::new(&session.file_path)
            .strip_prefix(&projects_dir)
            .unwrap_or(Path::new(&session.file_path));

        let dest_path = machine2_projects.join(relative_path);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        session.copy_to(&dest_path).unwrap();
    }

    let machine2_sessions = discover_test_sessions(&machine2_projects).unwrap();
    assert_eq!(machine2_sessions.len(), sync_sessions_after_modify.len());

    let modified_id = &modified_id;
    let pulled = machine2_sessions
        .iter()
        .find(|session| &session.session_id == modified_id)
        .expect("the modified session reached the second machine");
    let before_pull = original_sessions
        .iter()
        .find(|session| &session.session_id == modified_id)
        .expect("the modified session was there before the pull");
    assert!(
        pulled.message_count() > before_pull.message_count(),
        "Modified session should have more messages"
    );

    std::env::remove_var("HOME");
}

#[test]
fn test_undo_pull_restores_files() {
    let test_dir = TempDir::new().unwrap();
    let history_path = test_dir.path().join("history.json");
    let snapshots_dir = test_dir.path().join("snapshots");
    let claude_dir = test_dir.path().join("claude_projects");

    fs::create_dir_all(&claude_dir).unwrap();
    fs::create_dir_all(&snapshots_dir).unwrap();

    let conv_file = claude_dir.join("test-conversation.jsonl");
    let original_content = r#"{"type":"user","uuid":"1","sessionId":"test-session","timestamp":"2025-01-01T00:00:00Z"}
{"type":"assistant","uuid":"2","sessionId":"test-session","timestamp":"2025-01-01T00:01:00Z"}
"#;
    fs::write(&conv_file, original_content).unwrap();

    let snapshot = Snapshot::create(OperationType::Pull, vec![&conv_file], None).unwrap();

    let snapshot_path = snapshot.save_to_disk(Some(&snapshots_dir)).unwrap();

    let mut history = OperationHistory::from_path(Some(history_path.clone())).unwrap();

    let conv_summary = ConversationSummary::new(
        "test-session".to_string(),
        "test-conversation.jsonl".to_string(),
        Some("2025-01-01T00:01:00Z".to_string()),
        2,
        SyncOperation::Modified,
    )
    .unwrap();

    let mut pull_record = claude_code_sync::history::OperationRecord::new(
        OperationType::Pull,
        Some("main".to_string()),
        vec![conv_summary],
    );
    pull_record.snapshot_path = Some(snapshot_path.clone());

    history.operations.insert(0, pull_record);
    history.save_to(Some(history_path.clone())).unwrap();

    let modified_content = r#"{"type":"user","uuid":"1","sessionId":"test-session","timestamp":"2025-01-01T00:00:00Z"}
{"type":"assistant","uuid":"2","sessionId":"test-session","timestamp":"2025-01-01T00:01:00Z"}
{"type":"user","uuid":"3","sessionId":"test-session","timestamp":"2025-01-01T00:02:00Z"}
"#;
    fs::write(&conv_file, modified_content).unwrap();

    let content_after_pull = fs::read_to_string(&conv_file).unwrap();
    assert_eq!(content_after_pull, modified_content);

    let result = undo_pull(Some(history_path.clone()), Some(test_dir.path())).unwrap();
    assert!(result.contains("Successfully undone"));

    let content_after_undo = fs::read_to_string(&conv_file).unwrap();
    assert_eq!(content_after_undo, original_content);

    let history_after_undo = OperationHistory::from_path(Some(history_path)).unwrap();
    assert_eq!(
        history_after_undo.len(),
        0,
        "Pull operation should be removed from history"
    );

    assert!(
        !snapshot_path.exists(),
        "Snapshot should be deleted after successful undo"
    );
}

#[test]
fn test_undo_push_resets_repo() {
    let test_dir = TempDir::new().unwrap();
    let history_path = test_dir.path().join("history.json");
    let snapshots_dir = test_dir.path().join("snapshots");

    let repo = GitRepo::init(test_dir.path()).unwrap();

    let file1 = test_dir.path().join("file1.txt");
    fs::write(&file1, "initial content").unwrap();
    repo.stage_all().unwrap();
    repo.commit("Initial commit").unwrap();

    let initial_commit_hash = repo.current_commit_hash().unwrap();

    let file2 = test_dir.path().join("file2.txt");
    fs::write(&file2, "new content").unwrap();
    repo.stage_all().unwrap();
    repo.commit("Second commit").unwrap();

    let second_commit_hash = repo.current_commit_hash().unwrap();
    assert_ne!(initial_commit_hash, second_commit_hash);

    let commit_hash = repo.current_commit_hash().ok();
    let mut snapshot =
        Snapshot::create(OperationType::Push, vec![&file2], commit_hash.as_deref()).unwrap();

    snapshot.git_commit_hash = Some(initial_commit_hash.clone());
    snapshot.branch = Some("master".to_string());

    let snapshot_path = snapshot.save_to_disk(Some(&snapshots_dir)).unwrap();

    let mut history = OperationHistory::from_path(Some(history_path.clone())).unwrap();

    let conv_summary = ConversationSummary::new(
        "test-session".to_string(),
        "file2.txt".to_string(),
        None,
        1,
        SyncOperation::Added,
    )
    .unwrap();

    let mut push_record = claude_code_sync::history::OperationRecord::new(
        OperationType::Push,
        Some("master".to_string()),
        vec![conv_summary],
    );
    push_record.snapshot_path = Some(snapshot_path.clone());

    history.operations.insert(0, push_record);
    history.save_to(Some(history_path.clone())).unwrap();

    let current_hash_before_undo = repo.current_commit_hash().unwrap();
    assert_eq!(current_hash_before_undo, second_commit_hash);

    let result = undo_push(test_dir.path(), Some(history_path.clone())).unwrap();
    assert!(result.contains("Successfully undone"));
    assert!(result.contains(&initial_commit_hash[..8]));

    let reopened_repo = GitRepo::open(test_dir.path()).unwrap();
    let current_hash_after = reopened_repo.current_commit_hash().unwrap();
    assert_eq!(current_hash_after, initial_commit_hash);

    let history_after_undo = OperationHistory::from_path(Some(history_path)).unwrap();
    assert_eq!(
        history_after_undo.len(),
        0,
        "Push operation should be removed from history"
    );

    assert!(
        !snapshot_path.exists(),
        "Snapshot should be deleted after successful undo"
    );
}

fn pair_by_location<'a>(
    local_sessions: &'a [ConversationSession],
    local_root: &Path,
    remote_sessions: &'a [ConversationSession],
    remote_root: &Path,
) -> Vec<(&'a ConversationSession, &'a ConversationSession)> {
    remote_sessions
        .iter()
        .filter_map(|remote| {
            let relative = remote.file_path.strip_prefix(remote_root).ok()?;
            let local = local_sessions
                .iter()
                .find(|local| local.file_path == local_root.join(relative))?;
            Some((local, remote))
        })
        .collect()
}

#[test]
fn a_session_id_shared_by_two_projects_is_not_a_conflict() {
    use claude_code_sync::conflict::ConflictDetector;

    let local_dir = TempDir::new().unwrap();
    let remote_dir = TempDir::new().unwrap();
    let session_file = "e9ab1c40-0000-4000-8000-000000000001.jsonl";

    for (root, project, marker) in [
        (local_dir.path(), "first-project", "one"),
        (local_dir.path(), "second-project", "two"),
        (remote_dir.path(), "first-project", "one"),
        (remote_dir.path(), "second-project", "two"),
    ] {
        let path = root.join(project).join(session_file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            format!(
                "{{\"type\":\"user\",\"uuid\":\"{marker}\",\"sessionId\":\"shared\",\"timestamp\":\"2025-01-01T00:00:00Z\"}}\n"
            ),
        )
        .unwrap();
    }

    let local_sessions = discover_test_sessions(local_dir.path()).unwrap();
    let remote_sessions = discover_test_sessions(remote_dir.path()).unwrap();
    assert_eq!(local_sessions.len(), 2);

    let mut detector = ConflictDetector::new();
    detector.detect(&pair_by_location(
        &local_sessions,
        local_dir.path(),
        &remote_sessions,
        remote_dir.path(),
    ));

    assert!(
        !detector.has_conflicts(),
        "each transcript matches its own copy"
    );
}

#[test]
fn test_conflict_handling() {
    use claude_code_sync::conflict::ConflictDetector;

    let machine1_dir = TempDir::new().unwrap();
    let machine2_dir = TempDir::new().unwrap();
    let sync_repo_dir = TempDir::new().unwrap();

    let m1_projects = machine1_dir.path().join("projects");
    let m2_projects = machine2_dir.path().join("projects");
    let sync_projects = sync_repo_dir.path().join("projects");

    fs::create_dir_all(&m1_projects).unwrap();
    fs::create_dir_all(&m2_projects).unwrap();
    fs::create_dir_all(&sync_projects).unwrap();

    let session_id = "shared-session-123";
    let base_content = r#"{"type":"user","uuid":"1","sessionId":"shared-session-123","timestamp":"2025-01-01T00:00:00Z"}
{"type":"assistant","uuid":"2","sessionId":"shared-session-123","timestamp":"2025-01-01T00:01:00Z"}
"#;

    let m1_file = m1_projects.join("test").join(format!("{session_id}.jsonl"));
    let m2_file = m2_projects.join("test").join(format!("{session_id}.jsonl"));

    fs::create_dir_all(m1_file.parent().unwrap()).unwrap();
    fs::create_dir_all(m2_file.parent().unwrap()).unwrap();

    fs::write(&m1_file, base_content).unwrap();
    fs::write(&m2_file, base_content).unwrap();

    let m1_modified = format!(
        "{}{}\n",
        base_content,
        r#"{"type":"user","uuid":"3","sessionId":"shared-session-123","timestamp":"2025-01-01T00:02:00Z"}"#
    );
    fs::write(&m1_file, &m1_modified).unwrap();

    let m1_session = ConversationSession::from_file(&m1_file).unwrap();
    let sync_file = sync_projects
        .join("test")
        .join(format!("{session_id}.jsonl"));
    fs::create_dir_all(sync_file.parent().unwrap()).unwrap();
    m1_session.copy_to(&sync_file).unwrap();

    let m2_modified = format!(
        "{}{}\n",
        base_content,
        r#"{"type":"user","uuid":"4","sessionId":"shared-session-123","timestamp":"2025-01-01T00:03:00Z"}"#
    );
    fs::write(&m2_file, &m2_modified).unwrap();

    let local_sessions = discover_test_sessions(&m2_projects).unwrap();
    let remote_sessions = discover_test_sessions(&sync_projects).unwrap();

    let mut detector = ConflictDetector::new();
    detector.detect(&pair_by_location(
        &local_sessions,
        &m2_projects,
        &remote_sessions,
        &sync_projects,
    ));

    assert!(detector.has_conflicts(), "Should detect conflict");
    assert_eq!(detector.conflict_count(), 1);

    let conflict = &detector.conflicts()[0];
    assert_eq!(conflict.session_id, session_id);
    assert_ne!(conflict.local_hash, conflict.remote_hash);

    let renames = detector.resolve_all_keep_both().unwrap();
    assert_eq!(renames.len(), 1);

    let (_original, renamed) = &renames[0];

    assert!(renamed.to_string_lossy().contains("conflict-"));
    assert!(renamed.to_string_lossy().contains(".jsonl"));

    let remote_session = remote_sessions.first().unwrap();
    remote_session.copy_to(renamed).unwrap();

    assert!(m2_file.exists(), "Local version should remain");
    assert!(renamed.exists(), "Renamed remote version should exist");

    let local_content = fs::read_to_string(&m2_file).unwrap();
    let remote_content = fs::read_to_string(renamed).unwrap();
    assert_ne!(local_content, remote_content);
}

#[test]
fn test_operation_history_tracking() {
    let test_dir = TempDir::new().unwrap();
    let history_path = test_dir.path().join("history.json");

    let mut history = OperationHistory::from_path(Some(history_path.clone())).unwrap();

    let push_conv = ConversationSummary::new(
        "session-1".to_string(),
        "path/conv1.jsonl".to_string(),
        Some("2025-01-01T10:00:00Z".to_string()),
        5,
        SyncOperation::Added,
    )
    .unwrap();

    let push_record = claude_code_sync::history::OperationRecord::new(
        OperationType::Push,
        Some("main".to_string()),
        vec![push_conv.clone()],
    );

    history.operations.insert(0, push_record);
    history.save_to(Some(history_path.clone())).unwrap();

    let pull_conv = ConversationSummary::new(
        "session-2".to_string(),
        "path/conv2.jsonl".to_string(),
        Some("2025-01-01T11:00:00Z".to_string()),
        3,
        SyncOperation::Modified,
    )
    .unwrap();

    let pull_record = claude_code_sync::history::OperationRecord::new(
        OperationType::Pull,
        Some("main".to_string()),
        vec![pull_conv],
    );

    let mut history = OperationHistory::from_path(Some(history_path.clone())).unwrap();
    history.operations.insert(0, pull_record);
    history.save_to(Some(history_path.clone())).unwrap();

    let push_conv2 = ConversationSummary::new(
        "session-3".to_string(),
        "path/conv3.jsonl".to_string(),
        Some("2025-01-01T12:00:00Z".to_string()),
        7,
        SyncOperation::Modified,
    )
    .unwrap();

    let push_record2 = claude_code_sync::history::OperationRecord::new(
        OperationType::Push,
        Some("main".to_string()),
        vec![push_conv2],
    );

    let mut history = OperationHistory::from_path(Some(history_path.clone())).unwrap();
    history.operations.insert(0, push_record2);
    history.save_to(Some(history_path.clone())).unwrap();

    let history = OperationHistory::from_path(Some(history_path.clone())).unwrap();
    assert_eq!(history.len(), 3);

    let operations = history.list_operations();
    assert_eq!(operations[0].operation_type, OperationType::Push);
    assert_eq!(operations[1].operation_type, OperationType::Pull);
    assert_eq!(operations[2].operation_type, OperationType::Push);

    let last_op = history.get_last_operation().unwrap();
    assert_eq!(last_op.operation_type, OperationType::Push);
    assert_eq!(last_op.affected_conversations[0].session_id, "session-3");

    let last_pull = history
        .get_last_operation_by_type(OperationType::Pull)
        .unwrap();
    assert_eq!(last_pull.affected_conversations[0].session_id, "session-2");

    let summary = last_op.summary();
    assert!(summary.contains("push"));
    assert!(summary.contains("main"));
    assert!(summary.contains("1 conversations affected"));

    let stats = last_op.operation_stats();
    assert_eq!(stats.get(&SyncOperation::Modified), Some(&1));

    let reloaded = OperationHistory::from_path(Some(history_path)).unwrap();
    assert_eq!(reloaded.len(), 3);
    assert_eq!(
        reloaded.list_operations()[0].operation_type,
        OperationType::Push
    );
}

#[test]
fn test_with_real_test_data() {
    let test_data = Path::new(TEST_DATA_DIR);

    if !test_data.exists() {
        eprintln!("Skipping test_with_real_test_data: test data directory not found");
        return;
    }

    let sessions = discover_test_sessions(test_data).unwrap();

    assert!(
        sessions.len() >= 5,
        "Expected at least 5 test sessions, found {}",
        sessions.len()
    );

    for session in &sessions {
        assert!(
            !session.session_id.is_empty(),
            "Session ID should not be empty"
        );
        assert!(
            !session.load_entries().unwrap().is_empty(),
            "Session should have entries"
        );
        assert!(
            session.file_path.is_file(),
            "File path should name the transcript"
        );

        let _message_count = session.message_count();

        let hash = session.content_hash();
        assert!(!hash.is_empty(), "Session should have content hash");
    }

    let temp_dir = TempDir::new().unwrap();

    for session in &sessions {
        let dest_path = temp_dir
            .path()
            .join(format!("{}.jsonl", session.session_id));
        session.copy_to(&dest_path).unwrap();

        let reloaded = ConversationSession::from_file(&dest_path).unwrap();
        assert_eq!(reloaded.session_id, session.session_id);
        assert_eq!(reloaded.message_count(), session.message_count());
        assert_eq!(reloaded.content_hash(), session.content_hash());
    }
}

#[test]
fn test_snapshot_with_multiple_files() {
    let test_dir = TempDir::new().unwrap();
    let snapshots_dir = test_dir.path().join("snapshots");

    let files = vec![
        (
            "conv1.jsonl",
            r#"{"type":"user","uuid":"1","sessionId":"s1","timestamp":"2025-01-01T00:00:00Z"}"#,
        ),
        (
            "conv2.jsonl",
            r#"{"type":"user","uuid":"2","sessionId":"s2","timestamp":"2025-01-01T00:00:00Z"}"#,
        ),
        (
            "conv3.jsonl",
            r#"{"type":"user","uuid":"3","sessionId":"s3","timestamp":"2025-01-01T00:00:00Z"}"#,
        ),
    ];

    let mut file_paths = Vec::new();
    for (name, content) in &files {
        let path = test_dir.path().join(name);
        fs::write(&path, content).unwrap();
        file_paths.push(path);
    }

    let snapshot = Snapshot::create(OperationType::Pull, &file_paths, None).unwrap();

    assert_eq!(snapshot.files.len(), 3);

    let snapshot_path = snapshot.save_to_disk(Some(&snapshots_dir)).unwrap();
    let loaded = Snapshot::load_from_disk(&snapshot_path).unwrap();

    assert_eq!(loaded.files.len(), 3);

    for (name, _) in &files {
        let path = test_dir.path().join(name);
        fs::write(&path, "modified").unwrap();
    }

    loaded.restore_with_base(Some(test_dir.path())).unwrap();

    for (name, original_content) in &files {
        let path = test_dir.path().join(name);
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, *original_content);
    }
}

#[test]
fn test_concurrent_push_pull_operations() {
    let test_dir = TempDir::new().unwrap();
    let history_path = test_dir.path().join("history.json");

    let mut history = OperationHistory::from_path(Some(history_path.clone())).unwrap();

    for i in 0..10 {
        let op_type = if i % 2 == 0 {
            OperationType::Push
        } else {
            OperationType::Pull
        };

        let conv = ConversationSummary::new(
            format!("session-{i}"),
            format!("path/conv{i}.jsonl"),
            Some(format!("2025-01-01T{i:02}:00:00Z")),
            i + 1,
            SyncOperation::Modified,
        )
        .unwrap();

        let record = claude_code_sync::history::OperationRecord::new(
            op_type,
            Some("main".to_string()),
            vec![conv],
        );

        history.add_operation(record).unwrap();
    }

    assert_eq!(history.len(), 5);

    let operations = history.list_operations();
    for (idx, op) in operations.iter().enumerate() {
        let expected_session_id = format!("session-{}", 9 - idx);
        assert_eq!(op.affected_conversations[0].session_id, expected_session_id);
    }
}

#[test]
fn test_push_on_new_repo_without_commits() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path();

    let repo = GitRepo::init(repo_path).unwrap();

    let commit_result = repo.current_commit_hash();
    assert!(commit_result.is_err(), "New repo should have no commits");

    fs::write(repo_path.join("test.txt"), "hello").unwrap();
    repo.stage_all().unwrap();

    repo.commit("Initial commit").unwrap();

    let hash = repo.current_commit_hash().unwrap();
    assert!(!hash.is_empty());
}

#[test]
fn test_operation_record_with_no_commit_hash() {
    use claude_code_sync::history::OperationRecord;

    let conv = ConversationSummary::new(
        "test-session".to_string(),
        "path/test.jsonl".to_string(),
        Some("2025-01-01T00:00:00Z".to_string()),
        10,
        SyncOperation::Added,
    )
    .unwrap();

    let mut record =
        OperationRecord::new(OperationType::Push, Some("main".to_string()), vec![conv]);

    assert!(record.commit_hash.is_none());

    record.commit_hash = None;
    assert!(record.commit_hash.is_none());

    record.commit_hash = Some("abc123".to_string());
    assert_eq!(record.commit_hash, Some("abc123".to_string()));
}
