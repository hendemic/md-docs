//! Template and brand discovery, metadata parsing, and repository management.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::domain::{
    Brand, BrandMetadata, Config, MdDocsError, ModifierRegistry, RepoSource, Template,
    TemplateMetadata, TemplateSource,
};

const MODIFIERS_TOML: &str = include_str!("../../modifiers.toml");

// ---

/// Load the modifier registry from the embedded modifiers.toml.
pub fn load_modifiers() -> anyhow::Result<ModifierRegistry> {
    let registry: ModifierRegistry = toml::from_str(MODIFIERS_TOML)?;
    Ok(registry)
}

// ---

/// A resolved source directory with its origin and paths.
struct SourceDir {
    source: TemplateSource,
    templates_dir: PathBuf,
    brands_dir: PathBuf,
}

/// Template and brand discovery, metadata loading, and repository operations.
#[derive(Debug, Clone)]
pub struct TemplateManager {
    config: Config,
}

impl TemplateManager {
    /// Create from the resolved config.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    // ---

    /// Build ordered list of source directories (local first, then repos).
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

    // ---

    /// Base directory for cloned repositories.
    fn repos_base_dir() -> PathBuf {
        super::system::xdg_data_home().join("md-docs/repos")
    }

    /// Clone path for a named repo.
    fn repo_clone_path(name: &str) -> PathBuf {
        Self::repos_base_dir().join(name)
    }

    // ---

    /// Discover all templates across all configured sources.
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

                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map_or(true, |n| n.starts_with('.'))
                {
                    continue;
                }

                if !path.join("template.typ").is_file() {
                    continue;
                }

                let id = match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

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

                if path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map_or(true, |n| n.starts_with('.'))
                {
                    continue;
                }

                if !path.join("brand.typ").is_file() {
                    continue;
                }

                let id = match path.file_name().and_then(|n| n.to_str()) {
                    Some(name) => name.to_string(),
                    None => continue,
                };

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

    /// Resolve a template by name (first match wins).
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

    /// Resolve a brand by name (first match wins).
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

    // ---

    /// Read a template's metadata.toml, falling back to dirname-derived defaults.
    fn read_template_metadata(template_dir: &Path) -> anyhow::Result<TemplateMetadata> {
        let metadata_path = template_dir.join("metadata.toml");

        if !metadata_path.is_file() {
            let dirname = template_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            return Ok(TemplateMetadata {
                name: title_case(&dirname),
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

    /// Read a brand's metadata.toml, falling back to dirname-derived defaults.
    fn read_brand_metadata(brand_dir: &Path) -> anyhow::Result<BrandMetadata> {
        let metadata_path = brand_dir.join("metadata.toml");

        if !metadata_path.is_file() {
            let dirname = brand_dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            return Ok(BrandMetadata {
                name: title_case(&dirname),
                description: None,
            });
        }

        let content = std::fs::read_to_string(&metadata_path)?;
        let metadata: BrandMetadata = toml::from_str(&content)?;
        Ok(metadata)
    }

    // ---

    /// Install template repositories by cloning from configured URLs.
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
                continue;
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

// ---

/// Convert a hyphen/underscore-separated string to Title Case.
fn title_case(s: &str) -> String {
    s.replace('-', " ")
        .replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
