//! Pure data model for md-docs. No I/O, no dependencies on `app` or `cli`.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Domain-level errors.
#[derive(Debug, thiserror::Error)]
pub enum MdDocsError {
    #[error("template not found: {0}")]
    TemplateNotFound(String),

    #[error("brand not found: {0}")]
    BrandNotFound(String),

    #[error("input file not found: {0}")]
    InputNotFound(PathBuf),

    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),

    #[error("repository operation failed: {0}")]
    RepoOperationFailed(String),
}

// ---------------------------------------------------------------------------
// Multi-source types
// ---------------------------------------------------------------------------

/// Default repository URL for templates and brands.
pub const DEFAULT_REPO_URL: &str = "https://github.com/hendemic/md-docs-templates.git";

/// Default repository name used when no name is specified.
pub const DEFAULT_REPO_NAME: &str = "default";

/// A named git repository source.
#[derive(Debug, Clone, Deserialize)]
pub struct RepoSource {
    pub name: String,
    pub url: String,
}

/// A local directory source.
#[derive(Debug, Clone, Deserialize)]
pub struct LocalSource {
    pub path: PathBuf,
}

/// Where a template or brand was discovered from.
#[derive(Debug, Clone)]
pub enum TemplateSource {
    Repo(String),
    Local(PathBuf),
}

impl fmt::Display for TemplateSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Repo(name) => write!(f, "repo:{}", name),
            Self::Local(path) => write!(f, "local:{}", path.display()),
        }
    }
}

// ---------------------------------------------------------------------------
// Document model
// ---------------------------------------------------------------------------

/// A parsed markdown document with frontmatter and content sections.
#[derive(Debug, Clone)]
pub struct Document {
    pub metadata: Metadata,
    pub sections: ContentSections,
    pub raw_body: String,
}

impl Document {
    pub fn new(metadata: Metadata, raw_body: String) -> Self {
        Self {
            metadata,
            raw_body,
            sections: ContentSections::default(),
        }
    }
}

/// YAML frontmatter metadata. Extra keys are passed through to templates.
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub extra: HashMap<String, serde_yml::Value>,
}

impl Metadata {
    /// Parse YAML frontmatter, returning metadata and the remaining body.
    pub fn parse_from_content(content: &str) -> Result<(Self, String), MdDocsError> {
        // Check if content starts with a frontmatter delimiter
        if !content.starts_with("---\n") {
            return Ok((Metadata::default(), content.to_string()));
        }

        // Find the closing delimiter (skip the opening "---\n")
        let after_opening = &content[4..];
        let closing_pos = match after_opening.find("\n---\n") {
            Some(pos) => pos,
            None => {
                // Also check if the closing --- is at the very end of the file
                if after_opening.ends_with("\n---") {
                    after_opening.len() - 3
                } else {
                    // No closing delimiter found — treat as no frontmatter
                    // (the opening "---" was likely a horizontal rule or similar)
                    return Ok((Metadata::default(), content.to_string()));
                }
            }
        };

        let yaml_str = &after_opening[..closing_pos];
        let remaining = if closing_pos + 4 < after_opening.len() {
            // Skip past "\n---\n"
            &after_opening[closing_pos + 5..]
        } else if after_opening.ends_with("\n---") {
            ""
        } else {
            &after_opening[closing_pos + 4..]
        };

        // Parse the YAML content as a Mapping
        let mapping: serde_yml::Mapping = serde_yml::from_str(yaml_str)
            .map_err(|e| MdDocsError::InvalidFrontmatter(e.to_string()))?;

        // Extract well-known fields
        let title = mapping
            .get(serde_yml::Value::String("title".to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let author = mapping
            .get(serde_yml::Value::String("author".to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let date = mapping
            .get(serde_yml::Value::String("date".to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Collect remaining keys into extra
        let mut extra = HashMap::new();
        for (key, value) in &mapping {
            if let Some(key_str) = key.as_str() {
                match key_str {
                    "title" | "author" | "date" => continue,
                    _ => {
                        extra.insert(key_str.to_string(), value.clone());
                    }
                }
            }
        }

        let metadata = Metadata {
            title,
            author,
            date,
            extra,
        };

        Ok((metadata, remaining.to_string()))
    }
}

/// Content sections split at COLUMNS_START and COLUMN_BREAK markers.
#[derive(Debug, Clone, Default)]
pub struct ContentSections {
    pub header: String,
    pub body: String,
    pub content: String,
    pub body_columns: Vec<String>,
}

/// The marker used to split body content into separate columns.
const COLUMN_BREAK_MARKER: &str = "%%COLUMN_BREAK%%";

impl ContentSections {
    /// Split Typst content at the given split marker and column break markers.
    pub fn from_typst_content(typst_content: &str, split_marker: &str) -> Self {
        if let Some(pos) = typst_content.find(split_marker) {
            let header = typst_content[..pos].trim().to_string();
            let raw_body = typst_content[pos + split_marker.len()..].trim().to_string();

            // Split body into columns at %%COLUMN_BREAK%% markers
            let body_columns: Vec<String> = raw_body
                .split(COLUMN_BREAK_MARKER)
                .map(|s| s.trim().to_string())
                .collect();

            // Strip column break markers from body and content
            let body = raw_body.replace(COLUMN_BREAK_MARKER, "");
            let content = typst_content
                .replace(COLUMN_BREAK_MARKER, "");

            Self {
                header,
                body,
                content,
                body_columns,
            }
        } else {
            // No COLUMNS_START marker — all content is body.
            // Still split at COLUMN_BREAK markers for column support.
            let body_columns: Vec<String> = typst_content
                .split(COLUMN_BREAK_MARKER)
                .map(|s| s.trim().to_string())
                .collect();

            let body = typst_content.replace(COLUMN_BREAK_MARKER, "");
            let content = body.clone();

            Self {
                header: String::new(),
                body,
                content,
                body_columns,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Modifier system
// ---------------------------------------------------------------------------

/// Whether a modifier is inline or block-level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModifierType {
    Inline,
    Block,
}

/// Fallback behavior when a template ignores a modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnIgnore {
    Remove,
    Newline,
    Keep,
}

/// A modifier definition from `modifiers.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct ModifierDef {
    pub marker: String,
    pub description: String,
    pub typst: String,
    pub on_ignore: OnIgnore,
    #[serde(rename = "type")]
    pub modifier_type: ModifierType,
}

/// A modifier resolved against a template's ignore list.
#[derive(Debug, Clone)]
pub struct ResolvedModifier {
    pub id: String,
    pub marker: String,
    /// `None` means keep the marker as-is (on_ignore = keep).
    pub effective_typst: Option<String>,
    pub modifier_type: ModifierType,
}

/// All modifier definitions, keyed by modifier id.
pub type ModifierRegistry = HashMap<String, ModifierDef>;

/// Resolve modifiers against a template's ignore list.
pub fn resolve_modifiers(registry: &ModifierRegistry, ignore_list: &[String]) -> Vec<ResolvedModifier> {
    use std::collections::HashSet;

    let ignored: HashSet<&str> = ignore_list.iter().map(|s| s.as_str()).collect();

    registry
        .iter()
        .map(|(id, def)| {
            let effective_typst = if ignored.contains(id.as_str()) {
                match def.on_ignore {
                    OnIgnore::Remove => Some(String::new()),
                    OnIgnore::Newline => Some("\n".to_string()),
                    OnIgnore::Keep => None,
                }
            } else {
                Some(def.typst.clone())
            };

            ResolvedModifier {
                id: id.clone(),
                marker: def.marker.clone(),
                effective_typst,
                modifier_type: def.modifier_type,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Template and Brand
// ---------------------------------------------------------------------------

/// Template metadata from `metadata.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct TemplateMetadata {
    pub name: String,
    pub description: Option<String>,
    pub default_brand: Option<String>,
    #[serde(default)]
    pub ignore: Vec<String>,
    #[serde(default)]
    pub starter_file: Option<String>,
}

/// A discovered template with location and metadata.
#[derive(Debug, Clone)]
pub struct Template {
    pub id: String,
    pub path: PathBuf,
    pub metadata: TemplateMetadata,
    pub source: TemplateSource,
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.metadata.description {
            Some(desc) => write!(f, "{} ({}) -- {}", self.metadata.name, self.id, desc),
            None => write!(f, "{} ({})", self.metadata.name, self.id),
        }
    }
}

/// Brand metadata from `metadata.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct BrandMetadata {
    pub name: String,
    pub description: Option<String>,
}

/// A discovered brand with location and metadata.
#[derive(Debug, Clone)]
pub struct Brand {
    pub id: String,
    pub path: PathBuf,
    pub metadata: BrandMetadata,
    pub source: TemplateSource,
}

impl fmt::Display for Brand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.metadata.description {
            Some(desc) => write!(f, "{} ({}) -- {}", self.metadata.name, self.id, desc),
            None => write!(f, "{} ({})", self.metadata.name, self.id),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Layered application configuration (defaults < global < project < CLI).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    pub(crate) default_template: Option<String>,
    pub(crate) default_brand: Option<String>,
    pub(crate) output_dir: Option<PathBuf>,
    pub(crate) author: Option<String>,
    #[serde(default)]
    pub(crate) repos: Vec<RepoSource>,
    #[serde(default)]
    pub(crate) local: Vec<LocalSource>,
}

impl Config {
    pub fn default_template(&self) -> Option<&str> { self.default_template.as_deref() }
    pub fn default_brand(&self) -> Option<&str> { self.default_brand.as_deref() }
    pub fn output_dir(&self) -> Option<&Path> { self.output_dir.as_deref() }
    pub fn author(&self) -> Option<&str> { self.author.as_deref() }
    pub fn repos(&self) -> &[RepoSource] { &self.repos }
    pub fn local(&self) -> &[LocalSource] { &self.local }
}

// ---------------------------------------------------------------------------
// Typst content escaping
// ---------------------------------------------------------------------------

const TYPST_SPECIAL_CHARS: &[char] = &[
    '*', '_', '`', '<', '@', '=', '#', '$', '~', '\\', '/', '+', '-',
];

/// Escape special Typst markup characters with backslash prefix.
pub fn escape_typst(text: &str) -> String {
    let mut result = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        if TYPST_SPECIAL_CHARS.contains(&ch) {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}

// ---------------------------------------------------------------------------
// Markdown to Typst conversion types
// ---------------------------------------------------------------------------

/// Resolved modifiers partitioned for the converter.
#[derive(Debug, Clone)]
pub struct ConversionContext {
    pub block_modifiers: HashMap<String, Option<String>>,
    pub inline_modifiers: Vec<ResolvedModifier>,
}

impl ConversionContext {
    pub fn from_resolved(modifiers: &[ResolvedModifier]) -> Self {
        let mut block_modifiers = HashMap::new();
        let mut inline_modifiers = Vec::new();

        for m in modifiers {
            match m.modifier_type {
                ModifierType::Block => {
                    block_modifiers.insert(m.marker.clone(), m.effective_typst.clone());
                }
                ModifierType::Inline => {
                    inline_modifiers.push(m.clone());
                }
            }
        }

        Self {
            block_modifiers,
            inline_modifiers,
        }
    }
}

#[cfg(test)]
mod tests;
