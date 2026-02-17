//! Simple file-based logger for debugging.
//!
//! Appends timestamped entries to a log file in the XDG data directory.
//! Rotates at startup when the log exceeds 1MB, keeping up to 3 backups.
//!
//! # Log location
//! `$XDG_DATA_HOME/md-docs/md-docs.log` (typically `~/.local/share/md-docs/md-docs.log`)

use std::fs::{self, OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;

use crate::domain::CliMessage;

/// Maximum log file size before rotation (1MB).
const MAX_LOG_SIZE: u64 = 1_048_576;

/// Number of rotated backup files to keep.
const MAX_BACKUPS: u32 = 3;

/// File logger that appends entries to the md-docs log file.
pub struct FileLogger {
    path: PathBuf,
}

impl FileLogger {
    /// Create a new FileLogger pointing to the XDG data directory.
    ///
    /// Eagerly creates the parent directory and rotates the log file if it
    /// exceeds `MAX_LOG_SIZE`. Rotation only happens at startup.
    pub fn new() -> Self {
        let data_dir = super::config::xdg_data_home().join("md-docs");

        // Best-effort: create the log directory at construction time
        let _ = create_dir_all(&data_dir);

        let path = data_dir.join("md-docs.log");
        rotate_if_needed(&path);

        Self { path }
    }

    /// Create a FileLogger with a custom path (for testing).
    pub fn with_path(path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            let _ = create_dir_all(parent);
        }
        rotate_if_needed(&path);
        Self { path }
    }

    /// Append a log entry with timestamp. Silently ignores errors.
    pub fn log(&self, level: &str, message: &str) {
        let _ = self.write_entry(level, message);
    }

    /// Log a CliMessage, mapping variant to level string.
    pub fn log_message(&self, msg: &CliMessage) {
        let (level, text) = match msg {
            CliMessage::Success(s) => ("SUCCESS", s.as_str()),
            CliMessage::Info(s) => ("INFO", s.as_str()),
            CliMessage::Log(s) => ("DEBUG", s.as_str()),
            CliMessage::Warning(s) => ("WARN", s.as_str()),
            CliMessage::Error(s) => ("ERROR", s.as_str()),
            CliMessage::Plain(s) => ("INFO", s.as_str()),
        };
        self.log(level, text);
    }

    fn write_entry(&self, level: &str, message: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        writeln!(file, "[{}] [{}] {}", timestamp, level, message)?;

        Ok(())
    }
}

/// Rotate the log file if it exceeds `MAX_LOG_SIZE`.
///
/// Shifts existing backups: .log.2 → .log.3, .log.1 → .log.2, .log → .log.1.
/// Deletes the oldest backup if the limit is reached. Best-effort — silently
/// ignores errors since logging should never break the application.
fn rotate_if_needed(path: &std::path::Path) {
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size < MAX_LOG_SIZE {
        return;
    }

    // Shift existing backups (highest number first to avoid overwriting)
    for i in (1..MAX_BACKUPS).rev() {
        let from = format!("{}.{}", path.display(), i);
        let to = format!("{}.{}", path.display(), i + 1);
        let _ = fs::rename(&from, &to);
    }

    // Rotate current log to .1
    let backup = format!("{}.1", path.display());
    let _ = fs::rename(path, &backup);
}
