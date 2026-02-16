//! Simple file-based logger for debugging.
//!
//! Appends timestamped entries to a log file in the XDG data directory.
//! No log rotation — volume is low enough that this is not needed yet.
//!
//! # Log location
//! `$XDG_DATA_HOME/md-docs/md-docs.log` (typically `~/.local/share/md-docs/md-docs.log`)

use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::PathBuf;

use crate::domain::CliMessage;

/// File logger that appends entries to the md-docs log file.
pub struct FileLogger {
    path: PathBuf,
}

impl FileLogger {
    /// Create a new FileLogger pointing to the XDG data directory.
    ///
    /// Eagerly creates the parent directory so that `write_entry` does not
    /// need to call `create_dir_all` on every write.
    pub fn new() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join(".local/share")
            })
            .join("md-docs");

        // Best-effort: create the log directory at construction time
        let _ = create_dir_all(&data_dir);

        Self {
            path: data_dir.join("md-docs.log"),
        }
    }

    /// Create a FileLogger with a custom path (for testing).
    pub fn with_path(path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            let _ = create_dir_all(parent);
        }
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
