//! A pull takes only what the other machine changed since this one last
//! synced. Two `~/.claude` trees share one repository directory; the engine
//! functions take explicit paths, and a plan built here never prompts.

use std::fs;
use std::path::Path;

use claude_code_sync::artifacts::engine::{
    apply_pull, plan_pull, push_artifacts, record_synced_hashes,
};
use claude_code_sync::artifacts::registry::ArtifactToggles;
use claude_code_sync::filter::FilterConfig;
use tempfile::TempDir;

fn all_on_filter() -> FilterConfig {
    FilterConfig {
        sync_artifacts: ArtifactToggles::all_enabled(),
        ..Default::default()
    }
}

fn push(claude: &Path, repo: &Path) {
    push_artifacts(claude, repo, &all_on_filter()).unwrap();
    record_synced_hashes(claude, repo, &all_on_filter()).unwrap();
}

fn pull(claude: &Path, repo: &Path) {
    let plan = plan_pull(claude, repo, &all_on_filter()).unwrap();
    apply_pull(&plan, false).unwrap();
    record_synced_hashes(claude, repo, &all_on_filter()).unwrap();
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

/// Machine A pushes, machine B pulls: both now agree on everything.
fn two_machines_in_sync(seed: &[(&str, &str)]) -> (TempDir, TempDir, TempDir) {
    let repo = TempDir::new().unwrap();
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    for (relative, text) in seed {
        write(&a.path().join(relative), text);
    }
    push(a.path(), repo.path());
    pull(b.path(), repo.path());
    (repo, a, b)
}

#[test]
fn a_file_both_machines_changed_is_settled_not_overwritten() {
    let (repo, a, b) = two_machines_in_sync(&[("skills/s/SKILL.md", "v1\n")]);
    write(&b.path().join("skills/s/SKILL.md"), "edited on B\n");
    write(&a.path().join("skills/s/SKILL.md"), "edited on A\n");
    push(a.path(), repo.path());

    let plan = plan_pull(b.path(), repo.path(), &all_on_filter()).unwrap();
    assert_eq!(plan.changed_on_both_sides.len(), 1);
    assert!(plan.overwrites.is_empty());
    apply_pull(&plan, false).unwrap();

    assert_eq!(
        read(&b.path().join("skills/s/SKILL.md")),
        "edited on B\n",
        "with nobody to ask, this machine's edit is kept for the push"
    );
}

#[test]
fn a_remote_change_to_a_path_bearing_setting_arrives_rendered_for_this_machine() {
    let repo = TempDir::new().unwrap();
    let a = TempDir::new().unwrap();
    let b = TempDir::new().unwrap();
    let setting = |claude: &Path, model: &str| {
        let hook = format!("{}/hooks/run.sh", claude.display());
        serde_json::json!({ "model": model, "hook": hook }).to_string()
    };
    write(&a.path().join("settings.json"), &setting(a.path(), "opus"));
    push(a.path(), repo.path());
    pull(b.path(), repo.path());
    write(
        &a.path().join("settings.json"),
        &setting(a.path(), "sonnet"),
    );
    push(a.path(), repo.path());

    let plan = plan_pull(b.path(), repo.path(), &all_on_filter()).unwrap();
    assert!(
        plan.changed_on_both_sides.is_empty(),
        "B never touched it, so only the remote changed it"
    );
    apply_pull(&plan, false).unwrap();

    assert_eq!(
        read(&b.path().join("settings.json")),
        setting(b.path(), "sonnet")
    );
}

#[test]
fn a_machine_upgraded_from_a_version_without_hashes_keeps_its_skill_edit() {
    let (repo, a, b) = two_machines_in_sync(&[("skills/s/SKILL.md", "v1\n")]);
    let record_path = b.path().join(".claude-code-sync-tracked.json");
    let mut record: serde_json::Value = serde_json::from_str(&read(&record_path)).unwrap();
    record.as_object_mut().unwrap().remove("synced_hashes");
    fs::write(&record_path, record.to_string()).unwrap();
    write(&b.path().join("skills/s/SKILL.md"), "edited on B\n");
    write(&a.path().join("skills/s/SKILL.md"), "edited on A\n");
    push(a.path(), repo.path());

    pull(b.path(), repo.path());

    assert_eq!(read(&b.path().join("skills/s/SKILL.md")), "edited on B\n");
}

#[test]
fn a_category_removed_here_entirely_is_restored_by_the_next_pull() {
    let (repo, _a, b) = two_machines_in_sync(&[("skills/s/SKILL.md", "v1\n")]);
    fs::remove_dir_all(b.path().join("skills")).unwrap();

    pull(b.path(), repo.path());

    assert_eq!(read(&b.path().join("skills/s/SKILL.md")), "v1\n");
}

#[test]
fn a_file_the_remote_deleted_survives_here_when_edited_here_since() {
    let (repo, a, b) = two_machines_in_sync(&[
        ("skills/s/SKILL.md", "v1\n"),
        ("skills/keep/SKILL.md", "keep\n"),
    ]);
    fs::remove_dir_all(a.path().join("skills/s")).unwrap();
    push(a.path(), repo.path());
    write(&b.path().join("skills/s/SKILL.md"), "edited on B\n");

    pull(b.path(), repo.path());

    assert_eq!(read(&b.path().join("skills/s/SKILL.md")), "edited on B\n");
}
