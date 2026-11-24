mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, Once};

use claude_code_sync::artifacts::registry::ArtifactToggles;
use claude_code_sync::filter::FilterConfig;
use claude_code_sync::history::OperationHistory;
use claude_code_sync::sync::{pull_history, push_history, sync_bidirectional, SyncState};
use claude_code_sync::VerbosityLevel;
use common::EnvRestore;
use serial_test::serial;
use tempfile::TempDir;

struct Machine {
    _root: TempDir,
    home: PathBuf,
    config: PathBuf,
}

impl Machine {
    fn new(sync_repo: &Path) -> Machine {
        let root = TempDir::new().unwrap();
        let home = root.path().join("home");
        let config = root.path().join("cfg");
        fs::create_dir_all(home.join(".claude")).unwrap();
        fs::create_dir_all(config.join("claude-code-sync")).unwrap();

        let state = SyncState {
            sync_repo_path: sync_repo.to_path_buf(),
            has_remote: false,
            is_cloned_repo: false,
        };
        fs::write(
            config.join("claude-code-sync/state.json"),
            serde_json::to_string_pretty(&state).unwrap(),
        )
        .unwrap();

        let filter = FilterConfig {
            sync_artifacts: ArtifactToggles::all_enabled(),
            ..common::every_project_filter()
        };
        fs::write(
            config.join("claude-code-sync/config.toml"),
            toml::to_string_pretty(&filter).unwrap(),
        )
        .unwrap();

        Machine {
            _root: root,
            home,
            config,
        }
    }

    fn activate(&self) {
        common::point_env_at(&self.home, &self.claude(), &self.config);
    }

    fn claude(&self) -> PathBuf {
        self.home.join(".claude")
    }

    fn use_remote(&self, sync_repo: &Path) {
        let state = SyncState {
            sync_repo_path: sync_repo.to_path_buf(),
            has_remote: true,
            is_cloned_repo: false,
        };
        fs::write(
            self.config.join("claude-code-sync/state.json"),
            serde_json::to_string_pretty(&state).unwrap(),
        )
        .unwrap();
    }

    fn write_filter(&self, filter: &FilterConfig) {
        fs::write(
            self.config.join("claude-code-sync/config.toml"),
            toml::to_string_pretty(filter).unwrap(),
        )
        .unwrap();
    }
}

fn seed_full_claude_home(claude: &Path) {
    fs::write(claude.join("settings.json"), b"{\"model\":\"opus\"}").unwrap();
    fs::write(claude.join("CLAUDE.md"), b"# global memory\n").unwrap();
    fs::write(claude.join(".credentials.json"), b"{\"token\":\"sk-e2e\"}").unwrap();
    fs::create_dir_all(claude.join("skills/deploy")).unwrap();
    fs::write(claude.join("skills/deploy/SKILL.md"), b"# deploy\n").unwrap();
    fs::write(claude.join("history.jsonl"), history_line(1000, "from A")).unwrap();

    let proj = claude.join("projects/-home-user-webapp");
    fs::create_dir_all(proj.join("memory")).unwrap();
    fs::write(
        proj.join("aaaa-1111.jsonl"),
        "{\"type\":\"user\",\"sessionId\":\"aaaa-1111\",\"uuid\":\"u1\",\"timestamp\":\"2025-01-01T00:00:00Z\",\"cwd\":\"/home/user/webapp\"}\n",
    )
    .unwrap();
    fs::write(proj.join("diagram.png"), b"PNGDATA").unwrap();
    fs::write(proj.join("memory/MEMORY.md"), b"# project memory\n").unwrap();
}

fn history_line(ts: u64, display: &str) -> String {
    format!(
        "{{\"display\":\"{display}\",\"timestamp\":{ts},\"project\":\"/w\",\"sessionId\":\"s{ts}\"}}\n"
    )
}

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn init_git_repo(path: &Path) {
    fs::create_dir_all(path).unwrap();
    claude_code_sync::git::GitRepo::init(path).unwrap();
}

#[test]
#[serial]
fn test_full_pipeline_push_pull_undo_across_two_machines() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());

    let machine_a = Machine::new(repo.path());
    machine_a.activate();
    seed_full_claude_home(&machine_a.claude());

    let report = push_history(
        Some("e2e initial"),
        false,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();
    assert_eq!(report.added, 1, "one session pushed");
    assert!(report.artifacts.total_added() >= 6, "artifacts pushed");

    let log = git(repo.path(), &["log", "--oneline"]);
    assert_eq!(log.lines().count(), 1, "single commit: {log}");
    let tracked = git(repo.path(), &["ls-files"]);
    assert!(tracked.contains("artifacts/settings/settings.json"));
    assert!(tracked.contains("artifacts/skills/deploy/SKILL.md"));
    assert!(tracked.contains("projects/-home-user-webapp/diagram.png"));
    assert!(tracked.contains(".gitignore"));
    assert!(
        !tracked.contains(".credentials.json"),
        "secrets never reach the repo: {tracked}"
    );

    let history = OperationHistory::load().unwrap();
    let last = history.get_last_operation_by_type(claude_code_sync::history::OperationType::Push);
    assert!(!last.unwrap().artifact_counts.is_empty());

    let machine_b = Machine::new(repo.path());
    machine_b.activate();

    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    let b = machine_b.claude();
    assert_eq!(
        fs::read(b.join("settings.json")).unwrap(),
        b"{\"model\":\"opus\"}"
    );
    assert_eq!(fs::read(b.join("CLAUDE.md")).unwrap(), b"# global memory\n");
    assert!(b.join("skills/deploy/SKILL.md").is_file());
    assert!(b.join("projects/-home-user-webapp/diagram.png").is_file());
    assert!(b
        .join("projects/-home-user-webapp/memory/MEMORY.md")
        .is_file());
    assert!(b.join("history.jsonl").is_file());
    assert!(!b.join(".credentials.json").exists());

    let pulled_transcript = b.join(PULLED_TRANSCRIPT);
    assert!(pulled_transcript.is_file());
    let preview = claude_code_sync::undo::preview_undo_pull(None).unwrap();
    assert!(
        preview
            .files_to_delete
            .iter()
            .any(|file| file.ends_with("aaaa-1111.jsonl")),
        "the preview lists the transcript undo will remove: {:?}",
        preview.files_to_delete
    );

    let summary = claude_code_sync::undo::undo_pull(None, Some(&machine_b.home)).unwrap();
    assert!(summary.contains("undone"), "undo summary: {summary}");
    assert!(
        !b.join("settings.json").exists(),
        "created settings removed"
    );
    assert!(!b.join("skills/deploy/SKILL.md").exists());
    assert!(!b.join("CLAUDE.md").exists());
    assert!(
        !pulled_transcript.exists(),
        "the transcript the pull added is removed"
    );

    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    assert!(b.join("settings.json").is_file());
}

const PULLED_TRANSCRIPT: &str = "projects/-home-user-webapp/aaaa-1111.jsonl";
const REMOTE_TRANSCRIPT: &str = "{\"type\":\"user\",\"sessionId\":\"aaaa-1111\",\"uuid\":\"u1\",\"timestamp\":\"2025-01-01T00:00:00Z\",\"cwd\":\"/home/user/webapp\"}\n";
const LOCAL_TRANSCRIPT: &str = "{\"type\":\"user\",\"sessionId\":\"aaaa-1111\",\"uuid\":\"u2\",\"timestamp\":\"2025-01-02T00:00:00Z\",\"cwd\":\"/home/user/webapp\"}\n";

fn push_transcript_from_another_machine(repo: &Path) {
    let machine = Machine::new(repo);
    machine.activate();
    write_transcript(&machine, REMOTE_TRANSCRIPT);
    push_history(
        Some("from another machine"),
        false,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();
}

fn write_transcript(machine: &Machine, content: &str) -> PathBuf {
    let path = machine.claude().join(PULLED_TRANSCRIPT);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, content).unwrap();
    path
}

#[test]
#[serial]
fn test_undo_pull_keeps_a_pulled_transcript_continued_after_the_pull() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());
    push_transcript_from_another_machine(repo.path());

    let machine = Machine::new(repo.path());
    machine.activate();
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    let continued = format!("{REMOTE_TRANSCRIPT}{LOCAL_TRANSCRIPT}");
    let pulled = write_transcript(&machine, &continued);

    let undo = claude_code_sync::undo::undo_pull(None, Some(&machine.home));

    assert!(
        pulled.is_file(),
        "a transcript continued after the pull survives undo; undo returned {undo:?}"
    );
    assert_eq!(fs::read_to_string(&pulled).unwrap(), continued);
    assert!(undo.is_ok(), "undo pull failed: {undo:?}");
}

#[test]
#[serial]
fn test_undo_pull_removes_the_remote_copy_saved_beside_a_conflict() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());
    push_transcript_from_another_machine(repo.path());

    let machine = Machine::new(repo.path());
    machine.activate();
    let local = write_transcript(&machine, LOCAL_TRANSCRIPT);
    common::make_unmergeable_behind_the_summary_cache(&machine.claude().join("projects"), &local);
    let unmergeable = fs::read(&local).unwrap();

    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    let copies: Vec<PathBuf> = fs::read_dir(local.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.to_string_lossy().contains("-conflict-"))
        .collect();
    assert_eq!(copies.len(), 1, "the pull saves the remote copy beside");

    let undo = claude_code_sync::undo::undo_pull(None, Some(&machine.home));

    assert!(
        !copies[0].exists(),
        "undo pull removes the remote copy; undo returned {undo:?}"
    );
    assert_eq!(fs::read(&local).unwrap(), unmergeable);
    assert!(undo.is_ok(), "undo pull failed: {undo:?}");
}

#[test]
#[serial]
fn test_undo_pull_keeps_a_merged_transcript_continued_after_the_pull() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());
    push_transcript_from_another_machine(repo.path());

    let machine = Machine::new(repo.path());
    machine.activate();
    let local = write_transcript(&machine, LOCAL_TRANSCRIPT);
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    let merged = fs::read_to_string(&local).unwrap();
    assert_ne!(
        merged, LOCAL_TRANSCRIPT,
        "the pull merges the remote version in"
    );
    let continued = format!("{merged}{}", LOCAL_TRANSCRIPT.replace("u2", "u3"));
    fs::write(&local, &continued).unwrap();
    let preview = claude_code_sync::undo::preview_undo_pull(None).unwrap();
    assert_eq!(
        preview.files_changed_since_pull,
        vec![local.to_string_lossy().to_string()],
        "the preview lists the transcript undo will keep"
    );

    let undo = claude_code_sync::undo::undo_pull(None, Some(&machine.home));

    assert_eq!(
        fs::read_to_string(&local).unwrap(),
        continued,
        "a merged transcript continued after the pull survives undo; undo returned {undo:?}"
    );
    assert!(undo.is_ok(), "undo pull failed: {undo:?}");
}

#[test]
#[serial]
fn test_undo_pull_after_a_failed_pull_removes_what_it_wrote() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());
    let other = Machine::new(repo.path());
    other.activate();
    write_transcript(&other, REMOTE_TRANSCRIPT);
    fs::create_dir_all(other.claude().join("skills/deploy")).unwrap();
    fs::write(other.claude().join("skills/deploy/SKILL.md"), b"# deploy\n").unwrap();
    push_history(
        Some("from another machine"),
        false,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();

    let machine = Machine::new(repo.path());
    machine.activate();
    let skills_blocker = machine.claude().join("skills");
    fs::write(&skills_blocker, b"a file where the skills directory goes").unwrap();

    let pulled = pull_history(false, None, false, VerbosityLevel::Quiet);
    assert!(pulled.is_err(), "the skill can't be written under a file");
    let transcript = machine.claude().join(PULLED_TRANSCRIPT);
    assert!(transcript.is_file(), "the pull wrote the transcript first");

    let undo = claude_code_sync::undo::undo_pull(None, Some(&machine.home));

    assert!(
        !transcript.exists(),
        "undo removes what the failed pull wrote; undo returned {undo:?}"
    );
    assert!(undo.is_ok(), "undo pull failed: {undo:?}");
    assert!(skills_blocker.is_file());
}

#[test]
#[serial]
fn test_undo_pull_restores_a_local_file_discovery_skipped() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());
    push_transcript_from_another_machine(repo.path());

    let machine = Machine::new(repo.path());
    machine.activate();
    let unparseable = "not a transcript\n";
    let local = write_transcript(&machine, unparseable);

    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    assert_eq!(fs::read_to_string(&local).unwrap(), REMOTE_TRANSCRIPT);

    let undo = claude_code_sync::undo::undo_pull(None, Some(&machine.home));

    assert_eq!(
        fs::read_to_string(&local).unwrap(),
        unparseable,
        "undo pull restores the file the pull overwrote; undo returned {undo:?}"
    );
    assert!(undo.is_ok(), "undo pull failed: {undo:?}");
}

#[test]
#[serial]
fn test_full_pipeline_second_push_creates_no_commit() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());

    let machine = Machine::new(repo.path());
    machine.activate();
    seed_full_claude_home(&machine.claude());

    push_history(
        Some("first"),
        false,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();
    let report = push_history(
        Some("second"),
        false,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();

    assert_eq!(report.added, 0);
    assert_eq!(report.modified, 0);
    assert_eq!(report.artifacts.total_added(), 0);
    assert_eq!(report.artifacts.total_modified(), 0);
    let log = git(repo.path(), &["log", "--oneline"]);
    assert_eq!(log.lines().count(), 1, "no empty second commit: {log}");
}

#[test]
#[serial]
fn test_push_sends_every_transcript_while_pull_takes_only_mapped_projects() {
    let _restore = EnvRestore::capture();
    let root = TempDir::new().unwrap();
    let origin = root.path().join("origin.git");
    fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "--quiet", "--bare", "-b", "main"]);
    git(&origin, &["config", "uploadpack.allowFilter", "true"]);
    git(
        &origin,
        &["config", "uploadpack.allowAnySHA1InWant", "true"],
    );
    let origin_url = origin.to_string_lossy().into_owned();

    let repo_a = root.path().join("a");
    init_git_repo(&repo_a);
    git(&repo_a, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(&repo_a, &["remote", "add", "origin", &origin_url]);
    let mut mapped_only = FilterConfig {
        sync_artifacts: ArtifactToggles::all_enabled(),
        mapped_projects_only: true,
        ..Default::default()
    };
    mapped_only
        .project_map
        .insert("webapp".to_string(), PathBuf::from("/home/user/webapp"));

    let machine_a = Machine::new(&repo_a);
    machine_a.use_remote(&repo_a);
    machine_a.activate();
    machine_a.write_filter(&mapped_only);
    seed_full_claude_home(&machine_a.claude());
    let own_unmapped = format!(
        "projects/{}-other",
        claude_code_sync::project_map::encode_project_path(&machine_a.home)
    );
    let unmapped = machine_a.claude().join(&own_unmapped);
    fs::create_dir_all(unmapped.join("memory")).unwrap();
    let other_machines_copy = machine_a.claude().join("projects/-home-zz-other");
    fs::create_dir_all(&other_machines_copy).unwrap();
    fs::write(
        other_machines_copy.join("dddd-4444.jsonl"),
        "{\"type\":\"user\",\"sessionId\":\"dddd-4444\",\"uuid\":\"u4\",\"timestamp\":\"2025-01-02T00:00:00Z\",\"cwd\":\"/home/zz/other\"}\n",
    )
    .unwrap();
    fs::write(
        unmapped.join("bbbb-2222.jsonl"),
        "{\"type\":\"user\",\"sessionId\":\"bbbb-2222\",\"uuid\":\"u2\",\"timestamp\":\"2025-01-02T00:00:00Z\",\"cwd\":\"/home/user/other\"}\n",
    )
    .unwrap();
    fs::write(unmapped.join("memory/MEMORY.md"), b"# other memory\n").unwrap();

    let first = push_history(Some("A"), true, None, false, false, VerbosityLevel::Quiet).unwrap();
    assert_eq!(first.added, 2, "mapped and unmapped transcripts pushed");
    let tracked = git(&repo_a, &["ls-files"]);
    assert!(
        tracked.contains("projects/webapp/aaaa-1111.jsonl"),
        "{tracked}"
    );
    assert!(
        tracked.contains(&format!("{own_unmapped}/bbbb-2222.jsonl")),
        "{tracked}"
    );
    assert!(
        !tracked.contains(&format!("{own_unmapped}/memory")),
        "artifacts stay mapped-only: {tracked}"
    );
    assert!(
        !tracked.contains("projects/-home-zz-other"),
        "another machine's leftover copy is not pushed: {tracked}"
    );

    let repo_b = root.path().join("b");
    claude_code_sync::git::GitRepo::clone(&origin_url, &repo_b).unwrap();
    git(&repo_b, &["config", "user.name", "B"]);
    git(&repo_b, &["config", "user.email", "b@local"]);
    let machine_b = Machine::new(&repo_b);
    machine_b.use_remote(&repo_b);
    machine_b.activate();
    machine_b.write_filter(&mapped_only);
    let b_webapp = machine_b.claude().join("projects/-home-user-webapp");
    fs::create_dir_all(&b_webapp).unwrap();
    pull_history(true, None, false, VerbosityLevel::Quiet).unwrap();

    assert!(b_webapp.join("aaaa-1111.jsonl").is_file());
    assert!(!machine_b.claude().join(&own_unmapped).exists());

    fs::write(
        b_webapp.join("cccc-3333.jsonl"),
        "{\"type\":\"user\",\"sessionId\":\"cccc-3333\",\"uuid\":\"u3\",\"timestamp\":\"2025-01-03T00:00:00Z\",\"cwd\":\"/home/user/webapp\"}\n",
    )
    .unwrap();
    push_history(Some("B"), true, None, false, false, VerbosityLevel::Quiet).unwrap();

    machine_a.activate();
    pull_history(true, None, false, VerbosityLevel::Quiet).unwrap();
    assert!(
        repo_a
            .join(format!("{own_unmapped}/bbbb-2222.jsonl"))
            .is_file(),
        "a pull keeps this machine's unmapped transcripts checked out"
    );

    let second = push_history(Some("A2"), true, None, false, false, VerbosityLevel::Quiet).unwrap();
    assert_eq!(second.added, 0, "unmapped transcript is not re-added");
    assert_eq!(second.modified, 0);
}

#[test]
#[serial]
fn test_mapping_a_project_moves_its_repo_copy_under_the_id() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());

    let machine_a = Machine::new(repo.path());
    machine_a.activate();
    seed_full_claude_home(&machine_a.claude());
    push_history(Some("A"), false, None, false, false, VerbosityLevel::Quiet).unwrap();
    let tracked = git(repo.path(), &["ls-files"]);
    assert!(tracked.contains("projects/-home-user-webapp/memory/MEMORY.md"));

    let machine_b = Machine::new(repo.path());
    machine_b.activate();
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    let b_webapp = machine_b.claude().join("projects/-home-user-webapp");
    assert!(b_webapp.join("memory/MEMORY.md").is_file());

    let mut mapped = FilterConfig {
        sync_artifacts: ArtifactToggles::all_enabled(),
        ..Default::default()
    };
    mapped
        .project_map
        .insert("webapp".to_string(), PathBuf::from("/home/user/webapp"));
    machine_a.activate();
    machine_a.write_filter(&mapped);
    push_history(Some("A2"), false, None, false, false, VerbosityLevel::Quiet).unwrap();

    let tracked = git(repo.path(), &["ls-files"]);
    assert!(
        !tracked.contains("projects/-home-user-webapp/"),
        "{tracked}"
    );
    for file in ["aaaa-1111.jsonl", "diagram.png", "memory/MEMORY.md"] {
        assert!(
            tracked.contains(&format!("projects/webapp/{file}")),
            "{tracked}"
        );
    }

    let local_webapp = machine_a.claude().join("projects/-home-user-webapp");
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    assert!(local_webapp.join("aaaa-1111.jsonl").is_file());
    assert_eq!(
        fs::read(local_webapp.join("memory/MEMORY.md")).unwrap(),
        b"# project memory\n"
    );

    machine_b.activate();
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    assert!(
        b_webapp.join("memory/MEMORY.md").is_file(),
        "a machine without the mapping keeps the memory it synced before the move"
    );

    machine_b.write_filter(&mapped);
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    assert!(b_webapp.join("aaaa-1111.jsonl").is_file());
    assert!(
        b_webapp.join("memory/MEMORY.md").is_file(),
        "a machine that maps the project keeps its memory too"
    );
}

#[test]
#[serial]
fn test_full_pipeline_sync_converges_prompt_history() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());

    let machine_a = Machine::new(repo.path());
    machine_a.activate();
    seed_full_claude_home(&machine_a.claude());
    push_history(Some("A"), false, None, false, false, VerbosityLevel::Quiet).unwrap();

    let machine_b = Machine::new(repo.path());
    machine_b.activate();
    fs::write(
        machine_b.claude().join("history.jsonl"),
        history_line(2000, "from B"),
    )
    .unwrap();
    sync_bidirectional(Some("B sync"), None, false, false, VerbosityLevel::Quiet).unwrap();

    machine_a.activate();
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();

    let a_history = fs::read_to_string(machine_a.claude().join("history.jsonl")).unwrap();
    let b_history = fs::read_to_string(machine_b.claude().join("history.jsonl")).unwrap();
    assert_eq!(a_history, b_history, "machines converge");
    assert!(a_history.contains("from A"));
    assert!(a_history.contains("from B"));
    let ts: Vec<u64> = a_history
        .lines()
        .map(|l| {
            serde_json::from_str::<serde_json::Value>(l).unwrap()["timestamp"]
                .as_u64()
                .unwrap()
        })
        .collect();
    assert_eq!(ts, vec![1000, 2000], "chronological order");
}

#[test]
#[serial]
fn test_sync_keeps_local_edits_and_deletions_made_since_the_last_sync() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());
    let machine = Machine::new(repo.path());
    machine.activate();
    let claude = machine.claude();
    seed_full_claude_home(&claude);
    fs::create_dir_all(claude.join("skills/review")).unwrap();
    fs::write(claude.join("skills/review/SKILL.md"), b"# review\n").unwrap();
    push_history(
        Some("first"),
        false,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();

    fs::write(
        claude.join("skills/deploy/SKILL.md"),
        b"# deploy, edited here\n",
    )
    .unwrap();
    fs::remove_file(claude.join("skills/review/SKILL.md")).unwrap();
    sync_bidirectional(Some("sync"), None, false, false, VerbosityLevel::Quiet).unwrap();

    assert_eq!(
        fs::read(claude.join("skills/deploy/SKILL.md")).unwrap(),
        b"# deploy, edited here\n",
        "the local edit survives the pull half of the sync"
    );
    assert_eq!(
        fs::read(repo.path().join("artifacts/skills/deploy/SKILL.md")).unwrap(),
        b"# deploy, edited here\n",
        "and the push half sends it"
    );
    assert!(
        !claude.join("skills/review/SKILL.md").exists(),
        "the local deletion stands"
    );
    let tracked = git(repo.path(), &["ls-files"]);
    assert!(
        !tracked.contains("skills/review/SKILL.md"),
        "and reaches the repo: {tracked}"
    );
}

#[test]
#[serial]
fn test_pull_takes_what_only_the_remote_changed_and_the_later_dates() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());
    let skill = |claude: &Path, name: &str| claude.join("skills").join(name).join("SKILL.md");
    let write_skill = |claude: &Path, name: &str, text: &str| {
        fs::create_dir_all(claude.join("skills").join(name)).unwrap();
        fs::write(skill(claude, name), text).unwrap();
    };

    let machine_a = Machine::new(repo.path());
    machine_a.activate();
    let a = machine_a.claude();
    write_skill(&a, "remote-only", "v1\n");
    write_skill(&a, "dates", "updated: 2026-01-01T00:00:00Z\n");
    push_history(Some("A1"), false, None, false, false, VerbosityLevel::Quiet).unwrap();

    let machine_b = Machine::new(repo.path());
    machine_b.activate();
    let b = machine_b.claude();
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();
    write_skill(&b, "dates", "updated: 2026-01-03T00:00:00Z\n");

    machine_a.activate();
    write_skill(&a, "remote-only", "v2 from A\n");
    write_skill(&a, "dates", "updated: 2026-01-02T00:00:00Z\n");
    push_history(Some("A2"), false, None, false, false, VerbosityLevel::Quiet).unwrap();

    machine_b.activate();
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();

    assert_eq!(
        fs::read_to_string(skill(&b, "remote-only")).unwrap(),
        "v2 from A\n",
        "a change only the remote made arrives"
    );
    assert_eq!(
        fs::read_to_string(skill(&b, "dates")).unwrap(),
        "updated: 2026-01-03T00:00:00Z\n",
        "a file that differs only in dates keeps the later one"
    );
}

struct WarningCollector;

static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static WARNING_LOGGER: Once = Once::new();

impl log::Log for WarningCollector {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            WARNINGS.lock().unwrap().push(record.args().to_string());
        }
    }

    fn flush(&self) {}
}

fn collected_warnings_since_reset() -> Vec<String> {
    WARNINGS.lock().unwrap().clone()
}

fn reset_collected_warnings() {
    WARNING_LOGGER.call_once(|| {
        log::set_boxed_logger(Box::new(WarningCollector)).unwrap();
        log::set_max_level(log::LevelFilter::Warn);
    });
    WARNINGS.lock().unwrap().clear();
}

fn lines_about_unplaceable_files(warnings: &[String]) -> Vec<&String> {
    warnings
        .iter()
        .filter(|line| line.contains("no local project"))
        .collect()
}

#[test]
#[serial]
fn test_pull_warns_once_for_a_project_this_machine_never_mapped() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());

    let machine_a = Machine::new(repo.path());
    machine_a.activate();
    seed_full_claude_home(&machine_a.claude());
    let mut mapped = FilterConfig {
        sync_artifacts: ArtifactToggles::all_enabled(),
        ..common::every_project_filter()
    };
    mapped
        .project_map
        .insert("webapp".to_string(), PathBuf::from("/home/user/webapp"));
    machine_a.write_filter(&mapped);
    push_history(Some("A"), false, None, false, false, VerbosityLevel::Quiet).unwrap();
    assert!(repo.path().join("projects/webapp").is_dir());

    let machine_b = Machine::new(repo.path());
    machine_b.activate();
    reset_collected_warnings();
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();

    let warnings = collected_warnings_since_reset();
    let unplaceable = lines_about_unplaceable_files(&warnings);
    assert_eq!(
        unplaceable.len(),
        1,
        "three unplaceable files, one warning: {warnings:?}"
    );
    assert!(
        unplaceable[0]
            .lines()
            .any(|row| row.split_whitespace().eq(["webapp", "3"])),
        "the warning names the project and counts its files: {}",
        unplaceable[0]
    );
    assert!(
        !machine_b.claude().join("projects").exists(),
        "nothing is written into a project this machine has no directory for"
    );

    let mut warn_each = mapped.clone();
    warn_each.project_map.clear();
    warn_each.warn_each_skipped_file = true;
    machine_b.write_filter(&warn_each);
    reset_collected_warnings();
    pull_history(false, None, false, VerbosityLevel::Quiet).unwrap();

    let warnings = collected_warnings_since_reset();
    assert_eq!(
        lines_about_unplaceable_files(&warnings).len(),
        3,
        "warn_each_skipped_file restores the line per file: {warnings:?}"
    );
}

fn count_push_operations() -> usize {
    OperationHistory::load()
        .unwrap()
        .list_operations()
        .iter()
        .filter(|op| op.operation_type == claude_code_sync::history::OperationType::Push)
        .count()
}

#[test]
#[serial]
fn test_answering_no_to_push_leaves_the_repo_and_tracked_record_untouched() {
    let _restore = EnvRestore::capture();
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());
    let machine = Machine::new(repo.path());
    machine.activate();
    let claude = machine.claude();
    seed_full_claude_home(&claude);
    push_history(
        Some("first"),
        false,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();
    let record = claude_code_sync::artifacts::tracked::record_path(&claude);
    let record_before = fs::read(&record).unwrap();

    fs::write(claude.join("settings.json"), b"{\"model\":\"sonnet\"}").unwrap();
    fs::remove_file(claude.join("skills/deploy/SKILL.md")).unwrap();
    fs::write(
        claude.join("projects/-home-user-webapp/eeee-5555.jsonl"),
        "{\"type\":\"user\",\"sessionId\":\"eeee-5555\",\"uuid\":\"u5\",\"timestamp\":\"2025-01-05T00:00:00Z\",\"cwd\":\"/home/user/webapp\"}\n",
    )
    .unwrap();
    let report = claude_code_sync::sync::push::push_history_with_confirmation(
        Some("declined"),
        false,
        None,
        false,
        Some(|| Ok(false)),
        VerbosityLevel::Quiet,
    )
    .unwrap();

    assert_eq!(report.added, 0, "a declined push reports nothing pushed");
    let status = git(
        repo.path(),
        &["status", "--porcelain", "--untracked-files=all"],
    );
    assert_eq!(status, "", "a declined push leaves the repo as it was");
    assert_eq!(
        fs::read(&record).unwrap(),
        record_before,
        "a declined push leaves the tracked record as it was"
    );
    assert_eq!(
        count_push_operations(),
        1,
        "a declined push records no operation"
    );
}

#[test]
#[serial]
fn test_rejected_remote_push_fails_the_sync_after_committing_locally() {
    let _restore = EnvRestore::capture();
    let root = TempDir::new().unwrap();
    let origin = root.path().join("origin.git");
    fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "--quiet", "--bare", "-b", "main"]);
    let repo = root.path().join("a");
    init_git_repo(&repo);
    git(&repo, &["symbolic-ref", "HEAD", "refs/heads/main"]);
    git(
        &repo,
        &["remote", "add", "origin", &origin.to_string_lossy()],
    );
    let machine = Machine::new(&repo);
    machine.use_remote(&repo);
    machine.activate();
    let claude = machine.claude();
    seed_full_claude_home(&claude);
    push_history(
        Some("first"),
        true,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();

    let hook = origin.join("hooks/pre-receive");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(claude.join("settings.json"), b"{\"model\":\"sonnet\"}").unwrap();
    let synced = sync_bidirectional(Some("rejected"), None, false, false, VerbosityLevel::Quiet);

    let error = format!("{:#}", synced.expect_err("a rejected push fails the sync"));
    assert!(error.contains("pushing to origin/"), "{error}");
    let local_head = git(&repo, &["rev-parse", "HEAD"]);
    assert_ne!(git(&origin, &["rev-parse", "main"]), local_head);
    let log = git(&repo, &["log", "--oneline"]);
    assert_eq!(log.lines().count(), 2, "the local commit stays: {log}");
    assert_eq!(count_push_operations(), 2, "the push is recorded for undo");

    fs::remove_file(&hook).unwrap();
    push_history(
        Some("retry"),
        true,
        None,
        false,
        false,
        VerbosityLevel::Quiet,
    )
    .unwrap();

    assert_eq!(
        git(&origin, &["rev-parse", "main"]),
        local_head,
        "a push with nothing new still sends the commit the remote rejected"
    );
}
