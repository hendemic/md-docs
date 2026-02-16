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
    fn global_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default())
                    .join(".config")
            })
            .join("md-docs/config.toml")
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
    /// Only non-None fields in `overlay` replace the corresponding fields in `base`.
    fn merge(base: Config, overlay: Config) -> Config {
        Config {
            default_template: overlay.default_template.or(base.default_template),
            default_brand: overlay.default_brand.or(base.default_brand),
            templates_dir: overlay.templates_dir.or(base.templates_dir),
            brands_dir: overlay.brands_dir.or(base.brands_dir),
            output_dir: overlay.output_dir.or(base.output_dir),
            author: overlay.author.or(base.author),
        }
    }

    /// Return the built-in default configuration.
    ///
    /// All fields are None -- the effective_* methods on Config provide
    /// XDG-based fallbacks when no value is configured.
    fn defaults() -> Config {
        Config::default()
    }
}
