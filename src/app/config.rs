//! Layered configuration loader.
//!
//! Resolves configuration from multiple sources in priority order:
//! 1. Built-in defaults (hardcoded)
//! 2. Global config: `$XDG_CONFIG_HOME/md-docs/config.toml` (typically `~/.config/md-docs/config.toml`)
//! 3. Project config: `.md-docs.toml` in the current working directory
//! 4. CLI argument overrides (applied by the caller after loading)
//!
//! Each layer only overrides fields that are explicitly set (non-None).
//! This allows sparse config files -- users only specify what they want to change.

use std::path::{Path, PathBuf};

use crate::domain::Config;

/// Handles discovery and merging of configuration from multiple sources.
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load and merge configuration from all layers.
    ///
    /// Returns the resolved `Config` with defaults filled in for any
    /// fields not specified by any config source.
    pub fn load() -> anyhow::Result<Config> {
        todo!("Load defaults, merge global, merge project, return resolved Config")
    }

    /// Return the path to the global config file.
    ///
    /// Uses `$XDG_CONFIG_HOME/md-docs/config.toml`, falling back to
    /// `$HOME/.config/md-docs/config.toml`.
    fn global_config_path() -> PathBuf {
        todo!("Use dirs::config_dir() or env XDG_CONFIG_HOME, append md-docs/config.toml")
    }

    /// Return the path to the project-level config file.
    ///
    /// Looks for `.md-docs.toml` in the current working directory.
    fn project_config_path() -> PathBuf {
        todo!("Return current_dir() / .md-docs.toml")
    }

    /// Load a TOML config file into a partial Config.
    ///
    /// Returns `Ok(None)` if the file doesn't exist.
    /// Returns `Err` if the file exists but is malformed.
    fn load_toml(_path: &Path) -> anyhow::Result<Option<Config>> {
        todo!("Read file if exists, deserialize with toml, return Some(Config) or None")
    }

    /// Merge a partial config on top of a base config.
    ///
    /// Only non-None fields in `overlay` replace the corresponding fields in `base`.
    fn merge(_base: Config, _overlay: Config) -> Config {
        todo!("For each field: if overlay field is Some, use it; otherwise keep base")
    }

    /// Return the built-in default configuration.
    ///
    /// Uses XDG base directories for templates and brands locations.
    fn defaults() -> Config {
        todo!("Build Config with XDG_DATA_HOME/md-docs/templates and brands as defaults")
    }
}
