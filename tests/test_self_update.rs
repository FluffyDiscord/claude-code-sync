//! Integration tests for `self-update` run as a real process.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use tempfile::TempDir;

#[test]
fn self_update_outside_a_terminal_changes_nothing_without_yes() {
    let built = Path::new(env!("CARGO_BIN_EXE_claude-code-sync"));
    let install_dir = TempDir::new().unwrap();
    let installed = install_dir.path().join(built.file_name().unwrap());
    fs::copy(built, &installed).unwrap();
    let original = fs::read(&installed).unwrap();

    let output = Command::new(&installed)
        .args(["self-update", "--to", "v0.0.1"])
        .stdin(Stdio::null())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{}\nstdout: {stdout}\nstderr: {stderr}",
        output.status
    );
    assert!(stdout.contains("--yes"), "{stdout}");
    assert!(!stdout.contains("Downloading"), "{stdout}");
    assert_eq!(fs::read(&installed).unwrap(), original);
}
