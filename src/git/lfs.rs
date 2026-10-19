use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub fn is_installed() -> bool {
    Command::new("git-lfs")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn init(repo_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["lfs", "install", "--local"])
        .current_dir(repo_path)
        .output()
        .context("Failed to run 'git lfs install'")?;

    if !output.status.success() {
        bail!(
            "git lfs install failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

pub fn configure_gitattributes(repo_path: &Path, patterns: &[String]) -> Result<()> {
    let lfs_lines: Vec<String> = patterns
        .iter()
        .map(|pattern| format!("{pattern} filter=lfs diff=lfs merge=lfs -text"))
        .collect();

    super::attributes::ensure_lines(&repo_path.join(".gitattributes"), &lfs_lines)?;

    Ok(())
}

pub fn setup(repo_path: &Path, patterns: &[String]) -> Result<()> {
    if !is_installed() {
        bail!(
            "git-lfs is not installed.\n\
            Install it with:\n  \
            - macOS: brew install git-lfs\n  \
            - Ubuntu/Debian: apt install git-lfs\n  \
            - Windows: https://git-lfs.github.com"
        );
    }

    init(repo_path)?;
    configure_gitattributes(repo_path, patterns)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_configure_gitattributes_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let patterns = vec!["*.jsonl".to_string()];

        configure_gitattributes(temp_dir.path(), &patterns).unwrap();

        let content = fs::read_to_string(temp_dir.path().join(".gitattributes")).unwrap();
        assert!(content.contains("*.jsonl filter=lfs diff=lfs merge=lfs -text"));
    }

    #[test]
    fn test_configure_gitattributes_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let gitattributes = temp_dir.path().join(".gitattributes");

        fs::write(&gitattributes, "*.txt text\n").unwrap();

        let patterns = vec!["*.jsonl".to_string()];
        configure_gitattributes(temp_dir.path(), &patterns).unwrap();

        let content = fs::read_to_string(&gitattributes).unwrap();
        assert!(content.contains("*.txt text"));
        assert!(content.contains("*.jsonl filter=lfs diff=lfs merge=lfs -text"));
    }

    #[test]
    fn test_configure_gitattributes_no_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        let patterns = vec!["*.jsonl".to_string()];

        configure_gitattributes(temp_dir.path(), &patterns).unwrap();
        configure_gitattributes(temp_dir.path(), &patterns).unwrap();

        let content = fs::read_to_string(temp_dir.path().join(".gitattributes")).unwrap();
        assert_eq!(content.matches("*.jsonl filter=lfs").count(), 1);
    }

    #[test]
    fn test_is_installed() {
        let _ = is_installed();
    }

    #[test]
    fn test_setup_configures_gitattributes() {
        let temp_dir = TempDir::new().unwrap();
        let patterns = vec!["*.jsonl".to_string(), "*.png".to_string()];

        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        let result = setup(temp_dir.path(), &patterns);

        if is_installed() {
            assert!(result.is_ok(), "setup failed: {:?}", result.err());

            let content = fs::read_to_string(temp_dir.path().join(".gitattributes")).unwrap();
            assert!(content.contains("*.jsonl filter=lfs"));
            assert!(content.contains("*.png filter=lfs"));
        } else {
            assert!(result.is_err());
            let err = result.unwrap_err().to_string();
            assert!(err.contains("git-lfs is not installed"));
        }
    }
}
