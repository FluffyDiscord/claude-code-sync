use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::config::ConfigManager;
use crate::parser::ConversationSession;

/// A transcript's summary, valid while the file keeps this length and mtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CachedSummary {
    length: u64,
    modified: SystemTime,
    pub(crate) session: ConversationSession,
}

/// Transcript summaries from earlier runs, so a sync reads only the
/// transcripts that changed since — the way git's index skips unchanged files.
///
/// Purely a speed-up: a cache that cannot be read or written costs a full
/// read of the history, never a failed sync.
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct SummaryCache {
    version: String,
    entries: HashMap<PathBuf, CachedSummary>,
}

impl SummaryCache {
    /// The summaries the last run saved, or none when they are missing,
    /// unreadable, or written by another version of this tool.
    pub(crate) fn load() -> Self {
        let cache = match Self::read() {
            Ok(cache) => cache,
            Err(error) => {
                log::debug!("Reading every transcript, no usable summary cache: {error:#}");
                return Self::default();
            }
        };

        if cache.version != env!("CARGO_PKG_VERSION") {
            return Self::default();
        }

        cache
    }

    fn read() -> Result<Self> {
        let path = ConfigManager::session_summaries_path()?;
        let contents = fs::read(&path)?;
        Ok(serde_json::from_slice(&contents)?)
    }

    /// The transcript's summary: the cached one while the file is unchanged,
    /// a fresh read otherwise.
    pub(crate) fn summarize(&self, path: &Path) -> Result<CachedSummary> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to read metadata of {}", path.display()))?;
        let length = metadata.len();
        let modified = metadata
            .modified()
            .with_context(|| format!("Failed to read mtime of {}", path.display()))?;

        if let Some(cached) = self.entries.get(path) {
            let unchanged = cached.length == length && cached.modified == modified;
            if unchanged {
                return Ok(cached.clone());
            }
        }

        let session = ConversationSession::from_file(path)?;
        Ok(CachedSummary {
            length,
            modified,
            session,
        })
    }

    /// Replace everything cached under `base_path` with this walk's
    /// summaries, forget files that no longer exist, and write it out.
    pub(crate) fn save(mut self, base_path: &Path, summaries: &[CachedSummary]) {
        self.entries.retain(|path, _| {
            let rewalked = path.starts_with(base_path);
            !rewalked && path.exists()
        });

        for summary in summaries {
            let path = &summary.session.file_path;
            let is_utf8 = path.to_str().is_some();
            if is_utf8 {
                self.entries.insert(path.clone(), summary.clone());
            }
        }

        self.version = env!("CARGO_PKG_VERSION").to_string();
        if let Err(error) = self.write() {
            log::warn!("Failed to save the transcript summary cache: {error:#}");
        }
    }

    fn write(&self) -> Result<()> {
        let config_dir = ConfigManager::ensure_config_dir()?;
        let path = ConfigManager::session_summaries_path()?;
        let contents = serde_json::to_vec(self)?;

        let mut temporary = tempfile::NamedTempFile::new_in(config_dir)?;
        temporary.write_all(&contents)?;
        temporary.persist(&path)?;

        Ok(())
    }
}
