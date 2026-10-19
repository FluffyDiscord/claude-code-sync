use anyhow::{Context, Result};
use log::LevelFilter;
use std::fs::OpenOptions;
use std::io::Write;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

use crate::config::ConfigManager;

fn now_local_formatted(format: &[BorrowedFormatItem<'_>]) -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    now.format(format).unwrap_or_default()
}

pub fn init_logger() -> Result<()> {
    ConfigManager::ensure_config_dir()?;

    let default_level = std::env::var("RUST_LOG")
        .ok()
        .and_then(|s| s.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::Info);

    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{:5}] {}",
                now_local_formatted(format_description!("[hour]:[minute]:[second]")),
                record.level(),
                record.args()
            )
        })
        .filter_level(default_level)
        .target(env_logger::Target::Stdout)
        .try_init()
        .ok();

    log_to_file(&format!("Logger initialized with level: {default_level:?}"))?;

    Ok(())
}

pub fn log_to_file(message: &str) -> Result<()> {
    let log_path = ConfigManager::log_file_path()?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("Failed to open log file: {}", log_path.display()))?;

    writeln!(
        file,
        "[{}] {}",
        now_local_formatted(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        )),
        message
    )?;

    Ok(())
}

pub fn rotate_log_if_needed() -> Result<()> {
    const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;

    let log_path = ConfigManager::log_file_path()?;

    if log_path.exists() {
        let metadata = std::fs::metadata(&log_path)?;

        if metadata.len() > MAX_LOG_SIZE {
            let old_log_path = log_path.with_extension("log.old");

            if old_log_path.exists() {
                std::fs::remove_file(&old_log_path)?;
            }

            std::fs::rename(&log_path, &old_log_path)?;

            log::info!("Log file rotated to {}", old_log_path.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs::File;

    #[test]
    #[serial]
    fn test_init_logger_succeeds() {
        let result = init_logger();
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_log_to_file() -> Result<()> {
        let temp_dir = tempfile::TempDir::new()?;
        std::env::set_var("CLAUDE_CODE_SYNC_CONFIG_DIR", temp_dir.path());

        ConfigManager::ensure_config_dir()?;

        log_to_file("Test log message")?;

        let log_path = ConfigManager::log_file_path()?;
        assert!(log_path.exists());

        let contents = std::fs::read_to_string(&log_path)?;
        assert!(contents.contains("Test log message"));

        std::env::remove_var("CLAUDE_CODE_SYNC_CONFIG_DIR");

        Ok(())
    }

    #[test]
    #[serial]
    fn test_rotate_log_creates_backup() -> Result<()> {
        let temp_dir = tempfile::TempDir::new()?;
        std::env::set_var("CLAUDE_CODE_SYNC_CONFIG_DIR", temp_dir.path());

        let log_path = ConfigManager::log_file_path()?;
        ConfigManager::ensure_config_dir()?;
        let mut file = File::create(&log_path)?;

        let data = vec![b'a'; 11 * 1024 * 1024];
        file.write_all(&data)?;
        drop(file);

        rotate_log_if_needed()?;

        let old_log_path = log_path.with_extension("log.old");
        assert!(old_log_path.exists());

        if log_path.exists() {
            let metadata = std::fs::metadata(&log_path)?;
            assert!(metadata.len() < 11 * 1024 * 1024);
        }

        std::env::remove_var("CLAUDE_CODE_SYNC_CONFIG_DIR");

        Ok(())
    }
}
