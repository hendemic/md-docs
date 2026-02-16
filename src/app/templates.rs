//! Template and brand discovery, metadata parsing, and repository management.
//!
//! Scans configured directories for templates and brands, reads their metadata,
//! loads the modifier registry, and handles git-based install/update/remove operations.
//!
//! # Directory structure expected
//! ```text
//! templates_dir/
//!   resume-2-col/
//!     template.typ
//!     metadata.toml
//!   resume-ats/
//!     template.typ
//!     metadata.toml
//!
//! brands_dir/
//!   generic/
//!     brand.typ
//!     metadata.toml
//! ```

use std::path::{Path, PathBuf};

use crate::domain::{
    Brand, BrandMetadata, Config, ModifierRegistry, Template, TemplateMetadata,
};

// ---------------------------------------------------------------------------
// Modifier loading (standalone, not tied to TemplateManager)
// ---------------------------------------------------------------------------

/// Load the modifier registry from the embedded modifiers.toml.
///
/// The modifiers.toml file is embedded at compile time via `include_str!()`.
/// Returns a HashMap of modifier id -> ModifierDef.
///
/// This is a standalone function because it deserializes a compile-time-embedded
/// string and does not depend on any runtime state or filesystem paths.
pub fn load_modifiers() -> anyhow::Result<ModifierRegistry> {
    todo!("Deserialize include_str!('../../modifiers.toml') into ModifierRegistry")
}

// ---------------------------------------------------------------------------
// TemplateManager
// ---------------------------------------------------------------------------

/// Manages template and brand discovery, metadata loading, and repository operations.
pub struct TemplateManager {
    /// Directory containing installed templates.
    templates_dir: PathBuf,

    /// Directory containing installed brands.
    brands_dir: PathBuf,
}

impl TemplateManager {
    /// Create a new TemplateManager from the resolved config.
    pub fn new(_config: &Config) -> Self {
        todo!("Initialize with templates_dir and brands_dir from config")
    }

    // -----------------------------------------------------------------------
    // Discovery
    // -----------------------------------------------------------------------

    /// Discover all installed templates by scanning the templates directory.
    ///
    /// Each subdirectory with a `template.typ` file is considered a template.
    /// Returns templates sorted by name.
    pub fn discover_templates(&self) -> anyhow::Result<Vec<Template>> {
        todo!("Scan templates_dir, read metadata.toml from each subdir, build Template structs")
    }

    /// Discover all installed brands by scanning the brands directory.
    ///
    /// Each subdirectory with a `brand.typ` file is considered a brand.
    /// Returns brands sorted by name.
    pub fn discover_brands(&self) -> anyhow::Result<Vec<Brand>> {
        todo!("Scan brands_dir, read metadata.toml from each subdir, build Brand structs")
    }

    /// Resolve a template by name.
    ///
    /// Looks up the template in the templates directory.
    /// Returns an error if the template is not found.
    pub fn resolve_template(&self, _name: &str) -> anyhow::Result<Template> {
        todo!("Check templates_dir/name exists, read metadata, return Template")
    }

    /// Resolve a brand by name.
    ///
    /// Looks up the brand in the brands directory.
    /// Returns an error if the brand is not found.
    pub fn resolve_brand(&self, _name: &str) -> anyhow::Result<Brand> {
        todo!("Check brands_dir/name exists, read metadata, return Brand")
    }

    // -----------------------------------------------------------------------
    // Metadata loading
    // -----------------------------------------------------------------------

    /// Read a template's metadata.toml file.
    ///
    /// Falls back to sensible defaults if the file is missing or malformed.
    fn read_template_metadata(_template_dir: &Path) -> anyhow::Result<TemplateMetadata> {
        todo!("Read template_dir/metadata.toml, deserialize into TemplateMetadata")
    }

    /// Read a brand's metadata.toml file.
    ///
    /// Falls back to sensible defaults if the file is missing or malformed.
    fn read_brand_metadata(_brand_dir: &Path) -> anyhow::Result<BrandMetadata> {
        todo!("Read brand_dir/metadata.toml, deserialize into BrandMetadata")
    }

    // -----------------------------------------------------------------------
    // Repository management
    // -----------------------------------------------------------------------

    /// Install templates from a git repository.
    ///
    /// Clones the repository to a cache directory, then copies templates
    /// and brands into the configured data directories.
    pub fn install_repo(&self, _repo_url: &str) -> anyhow::Result<()> {
        todo!("Git clone repo_url to cache, copy templates/ and brands/ to data dirs")
    }

    /// Update installed templates by pulling the latest from the cached repository.
    ///
    /// If `name` is Some, only updates the specified template.
    /// Otherwise updates all templates from the cached repo.
    pub fn update_repo(&self, _name: Option<&str>) -> anyhow::Result<()> {
        todo!("Git pull in cached repo, re-copy templates and brands to data dirs")
    }

    /// Remove a template by name.
    ///
    /// Only removes templates that were installed from a repository.
    /// Returns an error if the template was user-added (not from a repo).
    pub fn remove_template(&self, _name: &str) -> anyhow::Result<()> {
        todo!("Verify template is repo-managed, remove its directory")
    }

    /// Return the cache directory path for a cloned template repository.
    fn cache_dir() -> PathBuf {
        todo!("Return XDG_CACHE_HOME/md-docs/repos or similar")
    }
}
