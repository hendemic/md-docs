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
    Brand, BrandMetadata, Config, MdDocsError, ModifierRegistry, Template, TemplateMetadata,
};

/// The embedded modifiers.toml file, included at compile time.
const MODIFIERS_TOML: &str = include_str!("../../modifiers.toml");

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
    let registry: ModifierRegistry = toml::from_str(MODIFIERS_TOML)?;
    Ok(registry)
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
    pub fn new(config: &Config) -> Self {
        Self {
            templates_dir: config.effective_templates_dir(),
            brands_dir: config.effective_brands_dir(),
        }
    }

    // -----------------------------------------------------------------------
    // Discovery
    // -----------------------------------------------------------------------

    /// Discover all installed templates by scanning the templates directory.
    ///
    /// Each subdirectory with a `template.typ` file is considered a template.
    /// Returns templates sorted by name.
    pub fn discover_templates(&self) -> anyhow::Result<Vec<Template>> {
        let mut templates = Vec::new();

        if !self.templates_dir.is_dir() {
            return Ok(templates);
        }

        for entry in std::fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // Skip hidden directories
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(true, |n| n.starts_with('.'))
            {
                continue;
            }

            // Must contain template.typ
            if !path.join("template.typ").is_file() {
                continue;
            }

            let id = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let metadata = Self::read_template_metadata(&path)?;

            templates.push(Template {
                id,
                path,
                metadata,
            });
        }

        templates.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        Ok(templates)
    }

    /// Discover all installed brands by scanning the brands directory.
    ///
    /// Each subdirectory with a `brand.typ` file is considered a brand.
    /// Returns brands sorted by name.
    pub fn discover_brands(&self) -> anyhow::Result<Vec<Brand>> {
        let mut brands = Vec::new();

        if !self.brands_dir.is_dir() {
            return Ok(brands);
        }

        for entry in std::fs::read_dir(&self.brands_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // Skip hidden directories
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(true, |n| n.starts_with('.'))
            {
                continue;
            }

            // Must contain brand.typ
            if !path.join("brand.typ").is_file() {
                continue;
            }

            let id = path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            let metadata = Self::read_brand_metadata(&path)?;

            brands.push(Brand {
                id,
                path,
                metadata,
            });
        }

        brands.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        Ok(brands)
    }

    /// Resolve a template by name.
    ///
    /// Looks up the template in the templates directory.
    /// Returns an error if the template is not found.
    pub fn resolve_template(&self, name: &str) -> anyhow::Result<Template> {
        let template_dir = self.templates_dir.join(name);

        if !template_dir.is_dir() || !template_dir.join("template.typ").is_file() {
            return Err(MdDocsError::TemplateNotFound(name.to_string()).into());
        }

        let metadata = Self::read_template_metadata(&template_dir)?;

        Ok(Template {
            id: name.to_string(),
            path: template_dir,
            metadata,
        })
    }

    /// Resolve a brand by name.
    ///
    /// Looks up the brand in the brands directory.
    /// Returns an error if the brand is not found.
    pub fn resolve_brand(&self, name: &str) -> anyhow::Result<Brand> {
        let brand_dir = self.brands_dir.join(name);

        if !brand_dir.is_dir() || !brand_dir.join("brand.typ").is_file() {
            return Err(MdDocsError::BrandNotFound(name.to_string()).into());
        }

        let metadata = Self::read_brand_metadata(&brand_dir)?;

        Ok(Brand {
            id: name.to_string(),
            path: brand_dir,
            metadata,
        })
    }

    // -----------------------------------------------------------------------
    // Metadata loading
    // -----------------------------------------------------------------------

    /// Read a template's metadata.toml file.
    ///
    /// Falls back to sensible defaults if the file is missing or malformed.
    fn read_template_metadata(template_dir: &Path) -> anyhow::Result<TemplateMetadata> {
        let metadata_path = template_dir.join("metadata.toml");

        if !metadata_path.is_file() {
            // Return sensible defaults derived from the directory name
            let dirname = template_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let name = dirname.replace('-', " ").replace('_', " ");
            // Capitalize first letter of each word
            let name = name
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => {
                            c.to_uppercase().to_string() + &chars.as_str().to_lowercase()
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            return Ok(TemplateMetadata {
                name,
                description: None,
                default_brand: None,
                ignore: Vec::new(),
                starter_file: None,
            });
        }

        let content = std::fs::read_to_string(&metadata_path)?;
        let metadata: TemplateMetadata = toml::from_str(&content)?;
        Ok(metadata)
    }

    /// Read a brand's metadata.toml file.
    ///
    /// Falls back to sensible defaults if the file is missing or malformed.
    fn read_brand_metadata(brand_dir: &Path) -> anyhow::Result<BrandMetadata> {
        let metadata_path = brand_dir.join("metadata.toml");

        if !metadata_path.is_file() {
            // Return sensible defaults derived from the directory name
            let dirname = brand_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let name = dirname.replace('-', " ").replace('_', " ");
            let name = name
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(c) => {
                            c.to_uppercase().to_string() + &chars.as_str().to_lowercase()
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            return Ok(BrandMetadata {
                name,
                description: None,
            });
        }

        let content = std::fs::read_to_string(&metadata_path)?;
        let metadata: BrandMetadata = toml::from_str(&content)?;
        Ok(metadata)
    }

    // -----------------------------------------------------------------------
    // Repository management
    // -----------------------------------------------------------------------

    /// Install templates from a git repository.
    ///
    /// Clones the repository to a cache directory, then copies templates
    /// and brands into the configured data directories.
    pub fn install_repo(&self, repo_url: &str) -> anyhow::Result<()> {
        let cache_dir = Self::cache_dir();

        if cache_dir.is_dir() && cache_dir.join(".git").is_dir() {
            anyhow::bail!(
                "Templates already installed. Use 'md-docs templates update' to update."
            );
        }

        // Ensure parent directory exists
        if let Some(parent) = cache_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Clone the repository
        let status = std::process::Command::new("git")
            .args(["clone", repo_url, &cache_dir.to_string_lossy()])
            .status()
            .map_err(|e| {
                MdDocsError::RepoOperationFailed(format!("failed to run git: {}", e))
            })?;

        if !status.success() {
            return Err(
                MdDocsError::RepoOperationFailed("git clone failed".to_string()).into(),
            );
        }

        // Sync templates and brands from cache to data directories
        self.sync_from_cache(&cache_dir)?;

        Ok(())
    }

    /// Update installed templates by pulling the latest from the cached repository.
    ///
    /// If `name` is Some, only updates the specified template.
    /// Otherwise updates all templates from the cached repo.
    pub fn update_repo(&self, _name: Option<&str>) -> anyhow::Result<()> {
        let cache_dir = Self::cache_dir();

        if !cache_dir.join(".git").is_dir() {
            anyhow::bail!(
                "Templates not installed. Run 'md-docs templates install' first."
            );
        }

        // Pull latest changes
        let status = std::process::Command::new("git")
            .args(["pull"])
            .current_dir(&cache_dir)
            .status()
            .map_err(|e| {
                MdDocsError::RepoOperationFailed(format!("failed to run git: {}", e))
            })?;

        if !status.success() {
            return Err(
                MdDocsError::RepoOperationFailed("git pull failed".to_string()).into(),
            );
        }

        // Re-sync from cache
        self.sync_from_cache(&cache_dir)?;

        Ok(())
    }

    /// Remove a template by name.
    ///
    /// Only removes templates that were installed from a repository.
    /// Returns an error if the template was user-added (not from a repo).
    pub fn remove_template(&self, name: &str) -> anyhow::Result<()> {
        let template_dir = self.templates_dir.join(name);

        if !template_dir.is_dir() {
            return Err(MdDocsError::TemplateNotFound(name.to_string()).into());
        }

        // Check if template exists in the cached repo (i.e., is repo-managed)
        let cache_dir = Self::cache_dir();
        let cached_template = cache_dir.join("templates").join(name);
        if !cached_template.is_dir() {
            return Err(MdDocsError::UserManagedTemplate(name.to_string()).into());
        }

        std::fs::remove_dir_all(&template_dir)?;

        Ok(())
    }

    /// Return the cache directory path for a cloned template repository.
    fn cache_dir() -> PathBuf {
        dirs::cache_dir()
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache")
            })
            .join("md-docs/repos")
    }

    /// Sync templates and brands from the cached repository to data directories.
    fn sync_from_cache(&self, cache_dir: &Path) -> anyhow::Result<()> {
        let templates_src = cache_dir.join("templates");
        let brands_src = cache_dir.join("brands");

        // Ensure destination directories exist
        std::fs::create_dir_all(&self.templates_dir)?;
        std::fs::create_dir_all(&self.brands_dir)?;

        // Copy templates
        if templates_src.is_dir() {
            for entry in std::fs::read_dir(&templates_src)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir()
                    && !path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map_or(true, |n| n.starts_with('.'))
                {
                    let dst = self.templates_dir.join(path.file_name().unwrap());
                    if dst.exists() {
                        std::fs::remove_dir_all(&dst)?;
                    }
                    copy_dir_all(&path, &dst)?;
                }
            }
        }

        // Copy brands
        if brands_src.is_dir() {
            for entry in std::fs::read_dir(&brands_src)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir()
                    && !path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map_or(true, |n| n.starts_with('.'))
                {
                    let dst = self.brands_dir.join(path.file_name().unwrap());
                    if dst.exists() {
                        std::fs::remove_dir_all(&dst)?;
                    }
                    copy_dir_all(&path, &dst)?;
                }
            }
        }

        Ok(())
    }
}

/// Recursively copy a directory and all its contents to a new location.
///
/// Rust's standard library does not provide a recursive directory copy,
/// so we implement it manually.
fn copy_dir_all(src: &Path, dst: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
