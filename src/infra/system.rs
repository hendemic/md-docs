//! XDG paths, layered config loading, and file logging.

use std::fs::{self, OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::domain::Config;

// ---

/// XDG config home (`$XDG_CONFIG_HOME` or `~/.config`).
pub fn xdg_config_home() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config")
        })
}

/// XDG data home (`$XDG_DATA_HOME` or `~/.local/share`).
pub fn xdg_data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share")
        })
}

// ---

/// Discovers and merges configuration from multiple sources.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load and merge configuration from all layers.
    pub fn load() -> anyhow::Result<Config> {
        let mut config = Config::default();

        if let Some(global) = Self::load_toml(&Self::global_config_path())? {
            config = Self::merge(config, global);
        }

        if let Some(project) = Self::load_toml(&Self::project_config_path())? {
            config = Self::merge(config, project);
        }

        Ok(config)
    }

    /// Path to the global config file.
    pub fn global_config_path() -> PathBuf {
        xdg_config_home().join("mdocs/config.toml")
    }

    /// Path to the project-level config file.
    fn project_config_path() -> PathBuf {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".mdocs.toml")
    }

    /// Load a TOML config file, returning `None` if it doesn't exist.
    fn load_toml(path: &Path) -> anyhow::Result<Option<Config>> {
        if !path.exists() {
            return Ok(None);
        }

        let contents = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(Some(config))
    }

    /// Merge overlay on top of base (non-None/non-empty overlay wins).
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
}

// ---

const MAX_LOG_SIZE: u64 = 1_048_576;
const MAX_BACKUPS: u32 = 3;

/// Appends timestamped entries to a rotating log file.
pub struct FileLogger {
    path: PathBuf,
}

impl FileLogger {
    /// Create a logger pointing to the XDG data directory.
    pub fn new() -> Self {
        let data_dir = xdg_data_home().join("mdocs");
        let _ = create_dir_all(&data_dir);

        let path = data_dir.join("mdocs.log");
        rotate_if_needed(&path);

        Self { path }
    }

    /// Create a logger with a custom path (for testing).
    #[cfg(test)]
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

/// Rotate the log file if it exceeds `MAX_LOG_SIZE`. Best-effort.
fn rotate_if_needed(path: &std::path::Path) {
    let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if size < MAX_LOG_SIZE {
        return;
    }

    for i in (1..MAX_BACKUPS).rev() {
        let from = format!("{}.{}", path.display(), i);
        let to = format!("{}.{}", path.display(), i + 1);
        let _ = fs::rename(&from, &to);
    }

    let backup = format!("{}.1", path.display());
    let _ = fs::rename(path, &backup);
}
