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
