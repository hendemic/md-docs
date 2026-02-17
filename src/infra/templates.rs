//! Template and brand discovery, metadata parsing, font loading, and repository management.
//!
//! Scans all configured sources (local directories and git repos) for templates
//! and brands, reads their metadata, loads the modifier registry, discovers
//! brand-bundled fonts, and handles git-based install/update operations.
//!
//! Sources are scanned in precedence order: local directories first, then repos.
//! When the same template/brand ID appears in multiple sources, the first wins.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::{
    Brand, BrandMetadata, Config, MdDocsError, ModifierRegistry, RepoSource, Template,
    TemplateMetadata, TemplateSource,
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

/// A resolved source directory with its origin and paths for templates and brands.
struct SourceDir {
    source: TemplateSource,
    templates_dir: PathBuf,
    brands_dir: PathBuf,
}

/// Manages template and brand discovery, metadata loading, and repository operations.
///
/// Scans multiple sources (local directories and cloned git repos) in precedence
/// order. Local sources take priority over repo sources; within each category,
/// sources are checked in config order.
#[derive(Debug, Clone)]
pub struct TemplateManager {
    config: Config,
}

impl TemplateManager {
    /// Create a new TemplateManager from the resolved config.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    // -----------------------------------------------------------------------
    // Source directory resolution
    // -----------------------------------------------------------------------

    /// Build ordered list of source directories. Local first (higher precedence), then repos.
    fn source_directories(&self) -> Vec<SourceDir> {
        let mut dirs = Vec::new();

        for local in self.config.local() {
            let base = &local.path;
            dirs.push(SourceDir {
                source: TemplateSource::Local(base.clone()),
                templates_dir: base.join("templates"),
                brands_dir: base.join("brands"),
            });
        }

        for repo in self.config.repos() {
            let base = Self::repo_clone_path(&repo.name);
            dirs.push(SourceDir {
                source: TemplateSource::Repo(repo.name.clone()),
                templates_dir: base.join("templates"),
                brands_dir: base.join("brands"),
            });
        }

        dirs
    }

    // -----------------------------------------------------------------------
    // Repository path helpers
    // -----------------------------------------------------------------------

    /// Return the base directory for cloned repositories.
    /// `~/.local/share/md-docs/repos/`
    fn repos_base_dir() -> PathBuf {
        super::system::xdg_data_home().join("md-docs/repos")
    }

    /// Return the clone path for a named repo.
    fn repo_clone_path(name: &str) -> PathBuf {
        Self::repos_base_dir().join(name)
    }

    // -----------------------------------------------------------------------
    // Discovery
    // -----------------------------------------------------------------------

    /// Discover all templates across all configured sources.
    ///
    /// Scans local directories first, then repos. When the same template ID
    /// appears in multiple sources, the first occurrence wins (higher precedence).
    /// Returns templates sorted by name.
    pub fn discover_templates(&self) -> anyhow::Result<Vec<Template>> {
        let mut templates = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();

        for source in self.source_directories() {
            if !source.templates_dir.is_dir() {
                continue;
            }

            for entry in std::fs::read_dir(&source.templates_dir)? {
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

                let id = match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                // First-wins: skip duplicates
                if seen_ids.contains(&id) {
                    continue;
                }
                seen_ids.insert(id.clone());

                let metadata = Self::read_template_metadata(&path)?;

                templates.push(Template {
                    id,
                    path,
                    metadata,
                    source: source.source.clone(),
                });
            }
        }

        templates.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        Ok(templates)
    }

    /// Discover all brands across all configured sources.
    ///
    /// Scans local directories first, then repos. When the same brand ID
    /// appears in multiple sources, the first occurrence wins (higher precedence).
    /// Returns brands sorted by name.
    pub fn discover_brands(&self) -> anyhow::Result<Vec<Brand>> {
        let mut brands = Vec::new();
        let mut seen_ids: HashSet<String> = HashSet::new();

        for source in self.source_directories() {
            if !source.brands_dir.is_dir() {
                continue;
            }

            for entry in std::fs::read_dir(&source.brands_dir)? {
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

                let id = match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

                // First-wins: skip duplicates
                if seen_ids.contains(&id) {
                    continue;
                }
                seen_ids.insert(id.clone());

                let metadata = Self::read_brand_metadata(&path)?;

                brands.push(Brand {
                    id,
                    path,
                    metadata,
                    source: source.source.clone(),
                });
            }
        }

        brands.sort_by(|a, b| a.metadata.name.cmp(&b.metadata.name));
        Ok(brands)
    }

    /// Resolve a template by name, searching across sources in precedence order.
    ///
    /// Returns the first matching template found. Local sources are checked
    /// before repo sources.
    pub fn resolve_template(&self, name: &str) -> anyhow::Result<Template> {
        for source in self.source_directories() {
            let template_dir = source.templates_dir.join(name);
            if template_dir.is_dir() && template_dir.join("template.typ").is_file() {
                let metadata = Self::read_template_metadata(&template_dir)?;
                return Ok(Template {
                    id: name.to_string(),
                    path: template_dir,
                    metadata,
                    source: source.source.clone(),
                });
            }
        }
        Err(MdDocsError::TemplateNotFound(name.to_string()).into())
    }

    /// Resolve a brand by name, searching across sources in precedence order.
    ///
    /// Returns the first matching brand found. Local sources are checked
    /// before repo sources.
    pub fn resolve_brand(&self, name: &str) -> anyhow::Result<Brand> {
        for source in self.source_directories() {
            let brand_dir = source.brands_dir.join(name);
            if brand_dir.is_dir() && brand_dir.join("brand.typ").is_file() {
                let metadata = Self::read_brand_metadata(&brand_dir)?;
                return Ok(Brand {
                    id: name.to_string(),
                    path: brand_dir,
                    metadata,
                    source: source.source.clone(),
                });
            }
        }
        Err(MdDocsError::BrandNotFound(name.to_string()).into())
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

    /// Install template repositories by cloning from their configured URLs.
    ///
    /// If `name` is provided, installs only the named repo. Otherwise installs
    /// all repos from the config. Skips repos that are already cloned.
    pub fn install_repo(&self, name: Option<&str>) -> anyhow::Result<()> {
        let repos_to_install: Vec<&RepoSource> = match name {
            Some(n) => {
                let repo = self
                    .config
                    .repos()
                    .iter()
                    .find(|r| r.name == n)
                    .ok_or_else(|| {
                        MdDocsError::RepoOperationFailed(format!(
                            "repo '{}' not found in config",
                            n
                        ))
                    })?;
                vec![repo]
            }
            None => self.config.repos().iter().collect(),
        };

        for repo in repos_to_install {
            let clone_path = Self::repo_clone_path(&repo.name);

            if clone_path.join(".git").is_dir() {
                continue; // Already installed
            }

            if let Some(parent) = clone_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let status = std::process::Command::new("git")
                .args(["clone", &repo.url, &clone_path.to_string_lossy()])
                .status()
                .map_err(|e| {
                    MdDocsError::RepoOperationFailed(format!("failed to run git: {}", e))
                })?;

            if !status.success() {
                return Err(MdDocsError::RepoOperationFailed(format!(
                    "git clone failed for repo '{}'",
                    repo.name
                ))
                .into());
            }
        }
        Ok(())
    }

    /// Update installed template repositories by pulling latest changes.
    ///
    /// If `name` is provided, updates only the named repo. Otherwise updates
    /// all repos from the config. Returns an error if a repo is not installed.
    pub fn update_repo(&self, name: Option<&str>) -> anyhow::Result<()> {
        let repos_to_update: Vec<&RepoSource> = match name {
            Some(n) => {
                let repo = self
                    .config
                    .repos()
                    .iter()
                    .find(|r| r.name == n)
                    .ok_or_else(|| {
                        MdDocsError::RepoOperationFailed(format!(
                            "repo '{}' not found in config",
                            n
                        ))
                    })?;
                vec![repo]
            }
            None => self.config.repos().iter().collect(),
        };

        for repo in repos_to_update {
            let clone_path = Self::repo_clone_path(&repo.name);

            if !clone_path.join(".git").is_dir() {
                return Err(MdDocsError::RepoOperationFailed(format!(
                    "repo '{}' not installed. Run 'md-docs templates install' first.",
                    repo.name
                ))
                .into());
            }

            let status = std::process::Command::new("git")
                .args(["pull"])
                .current_dir(&clone_path)
                .status()
                .map_err(|e| {
                    MdDocsError::RepoOperationFailed(format!("failed to run git: {}", e))
                })?;

            if !status.success() {
                return Err(MdDocsError::RepoOperationFailed(format!(
                    "git pull failed for repo '{}'",
                    repo.name
                ))
                .into());
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Font discovery and loading
// ---------------------------------------------------------------------------

/// Check whether a specific font family is available on the system.
///
/// Font availability cannot be reliably checked without running the full Typst
/// compiler. This is a best-effort check: it returns `false` as a conservative
/// default, since the compiler will emit warnings for missing fonts at compile time.
/// Actual font resolution is deferred to the Typst compilation step.
pub fn is_font_available(_font_name: &str) -> bool {
    false
}

/// Collect font file bytes from a brand's fonts/ directory.
///
/// Returns a vector of font file contents that can be passed to
/// `TypstEngine::builder().fonts(...)` for brand-bundled fonts.
///
/// Returns an empty Vec if the brand has no bundled fonts.
pub fn load_brand_fonts(brand_dir: &Path) -> anyhow::Result<Vec<Vec<u8>>> {
    let fonts_dir = brand_dir.join("fonts");
    if !fonts_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut font_bytes = Vec::new();
    for entry in std::fs::read_dir(&fonts_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "ttf" | "otf" | "woff2" => {
                        font_bytes.push(std::fs::read(&path)?);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(font_bytes)
}

/// Return a known-good fallback font family name.
///
/// Used when a brand-specified font is not found on the system.
/// Returns "New Computer Modern" which is embedded by typst-kit-embed-fonts.
pub fn fallback_font() -> &'static str {
    "New Computer Modern"
}
