use claude_code_sync::git::attributes::ensure_sync_attributes;
use claude_code_sync::git::GitRepo;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_get_remote_url() {
    let temp = TempDir::new().unwrap();
    let repo = GitRepo::init(temp.path()).unwrap();

    let url = "https://example.com/repo.git";
    repo.add_remote("origin", url).unwrap();

    let retrieved = repo.get_remote_url("origin").unwrap();
    assert_eq!(retrieved, url);
}

#[test]
fn test_set_remote_url() {
    let temp = TempDir::new().unwrap();
    let repo = GitRepo::init(temp.path()).unwrap();

    repo.add_remote("origin", "https://example.com/old.git")
        .unwrap();

    let new_url = "https://example.com/new.git";
    repo.set_remote_url("origin", new_url).unwrap();

    let retrieved = repo.get_remote_url("origin").unwrap();
    assert_eq!(retrieved, new_url);
}

#[test]
fn test_remove_remote() {
    let temp = TempDir::new().unwrap();
    let repo = GitRepo::init(temp.path()).unwrap();

    repo.add_remote("origin", "https://example.com/repo.git")
        .unwrap();
    assert!(repo.has_remote("origin"));

    repo.remove_remote("origin").unwrap();
    assert!(!repo.has_remote("origin"));
}

#[test]
fn test_list_remotes() {
    let temp = TempDir::new().unwrap();
    let repo = GitRepo::init(temp.path()).unwrap();

    let remotes = repo.list_remotes().unwrap();
    assert!(remotes.is_empty());

    repo.add_remote("origin", "https://example.com/origin.git")
        .unwrap();
    repo.add_remote("upstream", "https://example.com/upstream.git")
        .unwrap();

    let remotes = repo.list_remotes().unwrap();
    assert_eq!(remotes.len(), 2);
    assert!(remotes.contains(&"origin".to_string()));
    assert!(remotes.contains(&"upstream".to_string()));
}

#[test]
fn test_reset_soft() {
    let temp = TempDir::new().unwrap();
    let repo = GitRepo::init(temp.path()).unwrap();

    fs::write(temp.path().join("file1.txt"), "content1").unwrap();
    repo.stage_all().unwrap();
    repo.commit("First commit").unwrap();
    let first_hash = repo.current_commit_hash().unwrap();

    fs::write(temp.path().join("file2.txt"), "content2").unwrap();
    repo.stage_all().unwrap();
    repo.commit("Second commit").unwrap();
    let second_hash = repo.current_commit_hash().unwrap();

    assert_ne!(first_hash, second_hash);

    repo.reset_soft(&first_hash).unwrap();

    let current_hash = repo.current_commit_hash().unwrap();
    assert_eq!(current_hash, first_hash);
    assert!(temp.path().join("file2.txt").exists());
}

fn git(dir: &std::path::Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .args(["-c", "user.name=test", "-c", "user.email=test@local"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn write(path: std::path::PathBuf, contents: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

struct SharedRemote {
    _root: TempDir,
    url: String,
    other_machine: std::path::PathBuf,
    thin_machine: std::path::PathBuf,
}

fn remote_with_two_projects() -> SharedRemote {
    let root = TempDir::new().unwrap();
    let bare = root.path().join("remote.git");
    fs::create_dir_all(&bare).unwrap();
    git(&bare, &["init", "--quiet", "--bare", "-b", "main"]);
    git(&bare, &["config", "uploadpack.allowFilter", "true"]);
    git(&bare, &["config", "uploadpack.allowAnySHA1InWant", "true"]);
    let url = file_url(&bare);

    let other_machine = root.path().join("other");
    git(root.path(), &["clone", "--quiet", &url, "other"]);
    ensure_sync_attributes(&other_machine).unwrap();
    write(other_machine.join("projects/a/s.jsonl"), "a1\n");
    write(other_machine.join("projects/b/s.jsonl"), "b1\n");
    write(other_machine.join("artifacts/skills/x.md"), "x\n");
    git(&other_machine, &["add", "-A"]);
    git(&other_machine, &["commit", "--quiet", "-m", "seed"]);
    git(&other_machine, &["push", "--quiet", "origin", "main"]);

    let thin_machine = root.path().join("thin");
    SharedRemote {
        _root: root,
        url,
        other_machine,
        thin_machine,
    }
}

fn file_url(path: &std::path::Path) -> String {
    let slashed = path.display().to_string().replace('\\', "/");
    let is_unix_absolute = slashed.starts_with('/');
    if is_unix_absolute {
        return format!("file://{slashed}");
    }
    format!("file:///{slashed}")
}

fn push_a_new_file_to_project_b(remote: &SharedRemote) {
    let other = &remote.other_machine;
    write(other.join("projects/b/new.jsonl"), "b2\n");
    git(other, &["add", "-A"]);
    git(other, &["commit", "--quiet", "-m", "add to b"]);
    git(other, &["push", "--quiet", "origin", "main"]);
}

fn thin_clone_of_project_a(remote: &SharedRemote) -> GitRepo {
    let repo = GitRepo::clone(&remote.url, &remote.thin_machine).unwrap();
    git(&remote.thin_machine, &["config", "user.name", "thin"]);
    git(
        &remote.thin_machine,
        &["config", "user.email", "thin@local"],
    );
    let directories = ["artifacts".to_string(), "projects/a".to_string()];
    repo.limit_checkout(Some(&directories)).unwrap();
    repo
}

fn is_missing_locally(repo_dir: &std::path::Path, path_in_head: &str) -> bool {
    let blob = git(repo_dir, &["rev-parse", &format!("HEAD:{path_in_head}")]);
    let objects = git(
        repo_dir,
        &["rev-list", "--objects", "--all", "--missing=print"],
    );
    objects.lines().any(|line| line == format!("?{blob}"))
}

#[test]
fn a_limited_checkout_holds_only_the_mapped_project() {
    let remote = remote_with_two_projects();

    thin_clone_of_project_a(&remote);

    let thin = &remote.thin_machine;
    assert!(thin.join("projects/a/s.jsonl").is_file());
    assert!(thin.join("artifacts/skills/x.md").is_file());
    assert!(!thin.join("projects/b").exists());
    assert!(
        is_missing_locally(thin, "projects/b/s.jsonl"),
        "the other project's contents were never downloaded"
    );
}

#[test]
fn a_pull_merges_other_projects_without_downloading_them() {
    let remote = remote_with_two_projects();
    let repo = thin_clone_of_project_a(&remote);
    let other = &remote.other_machine;
    git(other, &["rm", "--quiet", "projects/a/s.jsonl"]);
    write(other.join("projects/b/new.jsonl"), "b2\n");
    git(other, &["add", "-A"]);
    git(other, &["commit", "--quiet", "-m", "purge a, add to b"]);
    git(other, &["push", "--quiet", "origin", "main"]);

    write(remote.thin_machine.join("projects/a/mine.jsonl"), "a2\n");
    repo.stage_all().unwrap();
    repo.commit("add to a").unwrap();
    repo.pull("origin", "main", &|_| unreachable!("no conflict"))
        .unwrap();
    repo.push("origin", "main").unwrap();

    assert!(
        is_missing_locally(&remote.thin_machine, "projects/b/new.jsonl"),
        "neither the merge summary nor rename detection fetched it"
    );
    git(other, &["pull", "--quiet", "--no-rebase", "origin", "main"]);
    assert_eq!(
        fs::read_to_string(other.join("projects/a/mine.jsonl")).unwrap(),
        "a2\n"
    );
    assert!(other.join("projects/b/new.jsonl").is_file());
}

#[test]
fn staging_ignores_what_lies_outside_the_limited_checkout() {
    let remote = remote_with_two_projects();
    let repo = thin_clone_of_project_a(&remote);

    write(remote.thin_machine.join("projects/c/left.jsonl"), "c\n");
    repo.stage_all().unwrap();
    repo.commit("stray project").unwrap();

    let tracked = git(&remote.thin_machine, &["ls-files", "--sparse"]);
    assert!(tracked.contains("projects/b"), "not staged as deleted");
    assert!(tracked.contains("projects/c/left.jsonl"));
}

#[test]
fn an_unusual_project_name_is_checked_out_literally() {
    let remote = remote_with_two_projects();
    let other = &remote.other_machine;
    write(other.join("projects/weird [x]/s.jsonl"), "literal\n");
    write(other.join("projects/weird x/s.jsonl"), "glob match\n");
    git(other, &["add", "-A"]);
    git(other, &["commit", "--quiet", "-m", "odd names"]);
    git(other, &["push", "--quiet", "origin", "main"]);
    let repo = thin_clone_of_project_a(&remote);

    let directories = ["artifacts".to_string(), "projects/weird [x]".to_string()];
    repo.limit_checkout(Some(&directories)).unwrap();

    let thin = &remote.thin_machine;
    assert!(thin.join("projects/weird [x]/s.jsonl").is_file());
    assert!(
        !thin.join("projects/weird x").exists(),
        "the name is not a pattern"
    );
    assert!(!thin.join("projects/a").exists());
}

#[test]
fn an_existing_full_clone_stops_downloading_other_projects() {
    let remote = remote_with_two_projects();
    let thin = &remote.thin_machine;
    git(
        thin.parent().unwrap(),
        &["clone", "--quiet", &remote.url, "thin"],
    );
    git(thin, &["config", "user.name", "thin"]);
    git(thin, &["config", "user.email", "thin@local"]);
    let repo = GitRepo::open(thin).unwrap();
    let directories = ["artifacts".to_string(), "projects/a".to_string()];
    repo.limit_checkout(Some(&directories)).unwrap();
    push_a_new_file_to_project_b(&remote);

    repo.pull("origin", "main", &|_| unreachable!("no conflict"))
        .unwrap();

    assert!(!thin.join("projects/b").exists());
    assert!(is_missing_locally(thin, "projects/b/new.jsonl"));
}

#[test]
fn removing_the_limit_restores_every_project() {
    let remote = remote_with_two_projects();
    let repo = thin_clone_of_project_a(&remote);

    repo.limit_checkout(None).unwrap();

    assert_eq!(
        fs::read_to_string(remote.thin_machine.join("projects/b/s.jsonl")).unwrap(),
        "b1\n"
    );
}
