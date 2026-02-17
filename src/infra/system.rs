//! System-level infrastructure: XDG paths, layered config loading, and file logging.
//!
//! This module consolidates platform concerns that don't belong in any
//! domain-specific module:
//!
//! - **XDG base directories** — `xdg_config_home()` and `xdg_data_home()` resolve
//!   the standard locations for config files and application data.
//! - **ConfigLoader** — discovers and merges configuration from built-in defaults,
//!   the global config file, and the project-level config file.
//! - **FileLogger** — appends timestamped entries to a rotating log file in the
//!   XDG data directory.

use std::fs::{self, OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::{CliMessage, Config};

// ---------------------------------------------------------------------------
// XDG base directories
// ---------------------------------------------------------------------------

/// Return the XDG config home directory (`$XDG_CONFIG_HOME` or `~/.config`).
///
/// Uses `~/.config` on all platforms, including macOS, to match CLI developer
/// tool conventions rather than the macOS `~/Library/Application Support/` default.
pub fn xdg_config_home() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
        })
}

/// Return the XDG data home directory (`$XDG_DATA_HOME` or `~/.local/share`).
///
/// Uses `~/.local/share` on all platforms, including macOS.
pub fn xdg_data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share")
        })
}

// ---------------------------------------------------------------------------
// ConfigLoader
// ---------------------------------------------------------------------------

/// Handles discovery and merging of configuration from multiple sources.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load and merge configuration from all layers.
    ///
    /// Returns the resolved `Config` with defaults filled in for any
    /// fields not specified by any config source.
    pub fn load() -> anyhow::Result<Config> {
        let mut config = Self::defaults();

        // Merge global config if it exists
        if let Some(global) = Self::load_toml(&Self::global_config_path())? {
            config = Self::merge(config, global);
        }

        // Merge project config if it exists
        if let Some(project) = Self::load_toml(&Self::project_config_path())? {
            config = Self::merge(config, project);
        }

        Ok(config)
    }

    /// Return the path to the global config file.
    ///
    /// Uses `$XDG_CONFIG_HOME/md-docs/config.toml`, falling back to
    /// `$HOME/.config/md-docs/config.toml`.
    pub fn global_config_path() -> PathBuf {
        xdg_config_home().join("md-docs/config.toml")
    }

    /// Return the path to the project-level config file.
    ///
    /// Looks for `.md-docs.toml` in the current working directory.
    fn project_config_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".md-docs.toml")
    }

    /// Load a TOML config file into a partial Config.
    ///
    /// Returns `Ok(None)` if the file doesn't exist.
    /// Returns `Err` if the file exists but is malformed.
    fn load_toml(path: &Path) -> anyhow::Result<Option<Config>> {
        if !path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(Some(config))
    }

    /// Merge a partial config on top of a base config.
    ///
    /// For `Option` fields, non-None overlay replaces base.
    /// For `Vec` fields (repos, local), non-empty overlay replaces base entirely.
    fn merge(base: Config, overlay: Config) -> Config {
        Config {
            default_template: overlay.default_template.or(base.default_template),
            default_brand: overlay.default_brand.or(base.default_brand),
            output_dir: overlay.output_dir.or(base.output_dir),
            author: overlay.author.or(base.author),
            repos: if overlay.repos.is_empty() {
                base.repos
            } else {
                overlay.repos
            },
            local: if overlay.local.is_empty() {
                base.local
            } else {
                overlay.local
            },
        }
    }

    /// Return the built-in default configuration.
    ///
    /// All Option fields are None, Vec fields are empty.
    fn defaults() -> Config {
        Config::default()
    }
}

// ---------------------------------------------------------------------------
// FileLogger
// ---------------------------------------------------------------------------

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
        let data_dir = xdg_data_home().join("md-docs");

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
/// Shifts existing backups: .log.2 -> .log.3, .log.1 -> .log.2, .log -> .log.1.
/// Deletes the oldest backup if the limit is reached. Best-effort -- silently
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
