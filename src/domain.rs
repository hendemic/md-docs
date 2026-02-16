//! Pure data model for md-docs.
//!
//! This module defines all types used across the application. It has NO dependencies
//! on the `app` or `cli` layers and performs NO I/O. All types here are pure data
//! with basic construction, access, and manipulation methods.
//!
//! # Module dependency graph
//! ```text
//! domain.rs  <--  app/  <--  cli.rs
//! (this file)     |           |
//!                 |           +-- uses domain types + app controller
//!                 +-- uses domain types only
//! ```

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// All domain-level errors that can occur in md-docs.
///
/// These represent logical failures in the application, not I/O errors.
/// I/O errors are wrapped by `anyhow` at the app layer.
#[derive(Debug, thiserror::Error)]
pub enum MdDocsError {
    /// The requested template was not found in any configured directory.
    #[error("template not found: {0}")]
    TemplateNotFound(String),

    /// The requested brand was not found in any configured directory.
    #[error("brand not found: {0}")]
    BrandNotFound(String),

    /// The input markdown file does not exist or is not readable.
    #[error("input file not found: {0}")]
    InputNotFound(PathBuf),

    /// YAML frontmatter could not be parsed.
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),

    /// A modifier definition in modifiers.toml is malformed.
    #[error("invalid modifier definition: {0}")]
    InvalidModifier(String),

    /// Typst compilation produced errors.
    #[error("typst compilation failed: {0}")]
    CompilationFailed(String),

    /// Configuration file is malformed.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    /// Template repository operation failed (install/update/remove).
    #[error("repository operation failed: {0}")]
    RepoOperationFailed(String),

    /// A template is user-managed and cannot be removed via the repo commands.
    #[error("cannot remove user-managed template: {0}")]
    UserManagedTemplate(String),
}

// ---------------------------------------------------------------------------
// Document model
// ---------------------------------------------------------------------------

/// A parsed markdown document with extracted frontmatter and content sections.
///
/// The document is split into:
/// - `metadata`: YAML frontmatter key-value pairs (title, author, date, etc.)
/// - `sections`: the content split into header/body by the COLUMNS_START marker
/// - `raw_body`: the raw markdown text after frontmatter extraction
#[derive(Debug, Clone)]
pub struct Document {
    /// Frontmatter metadata extracted from the YAML block.
    pub metadata: Metadata,

    /// The content sections (header, body, full content) after markdown-to-Typst conversion.
    pub sections: ContentSections,

    /// The raw markdown body text (after frontmatter has been stripped).
    pub raw_body: String,
}

impl Document {
    /// Create a new document from parsed frontmatter and raw markdown body.
    pub fn new(metadata: Metadata, raw_body: String) -> Self {
        Self {
            metadata,
            raw_body,
            sections: ContentSections::default(),
        }
    }
}

/// Frontmatter metadata extracted from the YAML block at the top of a markdown file.
///
/// Common fields (title, author, date) are first-class. Additional arbitrary
/// key-value pairs are captured in `extra` for template pass-through.
/// The `extra` map uses `serde_yml::Value` to support non-string YAML values
/// (lists, numbers, nested objects).
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    /// Document title (e.g., a person's name for a resume).
    pub title: Option<String>,

    /// Document author.
    pub author: Option<String>,

    /// Date string (free-form, e.g. "February 2026").
    pub date: Option<String>,

    /// Any additional frontmatter keys not captured above.
    /// Uses `serde_yml::Value` to preserve non-string types (lists, numbers, etc.).
    pub extra: HashMap<String, serde_yml::Value>,
}

impl Metadata {
    /// Parse a YAML frontmatter string into a Metadata struct.
    ///
    /// Returns the metadata and the remaining markdown content after the
    /// frontmatter block (`---` delimiters). The returned String is owned
    /// because the caller needs to store it independently of the input.
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

/// The three content sections that templates consume.
///
/// Templates import `content.typ` which exports `header`, `body`, `content`,
/// and `body-columns`. The split happens at the `COLUMNS_START` modifier marker,
/// and body is further split at `COLUMN_BREAK` markers into `body_columns`.
#[derive(Debug, Clone, Default)]
pub struct ContentSections {
    /// Typst markup for everything above the column split marker.
    /// If no split marker exists, this is empty.
    pub header: String,

    /// Typst markup for everything below the column split marker,
    /// with `%%COLUMN_BREAK%%` markers stripped.
    /// If no split marker exists, this contains all content.
    pub body: String,

    /// Full Typst markup (header + body combined), with `%%COLUMN_BREAK%%` markers stripped.
    /// Always populated.
    pub content: String,

    /// Body content split into per-column segments at `%%COLUMN_BREAK%%` markers.
    /// If no column break markers exist, contains the entire body as a single element.
    pub body_columns: Vec<String>,
}

/// The marker used to split body content into separate columns.
const COLUMN_BREAK_MARKER: &str = "%%COLUMN_BREAK%%";

impl ContentSections {
    /// Split converted Typst content at the COLUMNS_START marker, then further
    /// split the body at `%%COLUMN_BREAK%%` markers into `body_columns`.
    ///
    /// If the COLUMNS_START marker is present, content above goes to `header`
    /// and below to `body`. If absent, everything goes to `body` and `header`
    /// is empty.
    ///
    /// The `body` and `content` fields have all `%%COLUMN_BREAK%%` markers stripped.
    /// The `body_columns` vector contains each column segment (trimmed). If no
    /// column break markers exist, `body_columns` contains the entire body as
    /// a single element.
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

/// The type of a content modifier: where it appears in the markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModifierType {
    /// Appears inline within text (e.g., ` /| ` for date separators).
    Inline,

    /// Appears as a standalone block (HTML comment, e.g., `<!-- COLUMN_BREAK -->`).
    Block,
}

/// What to do with a modifier when a template ignores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnIgnore {
    /// Silently drop the modifier from the output.
    Remove,

    /// Replace the modifier with a line break.
    Newline,

    /// Leave the raw marker text in the output unchanged.
    Keep,
}

/// A modifier definition as loaded from `modifiers.toml`.
///
/// Modifiers transform special markers in markdown into Typst commands.
/// Each has a marker string, its Typst output, and fallback behavior
/// when a template chooses to ignore it.
#[derive(Debug, Clone, Deserialize)]
pub struct ModifierDef {
    /// The marker string the user writes in markdown (e.g., ` /| ` or `<!-- COLUMN_BREAK -->`).
    pub marker: String,

    /// Human-readable description of what this modifier does.
    pub description: String,

    /// The Typst markup this modifier produces (e.g., ` #h(1fr) `).
    pub typst: String,

    /// What happens when a template lists this modifier in its `ignore` list.
    pub on_ignore: OnIgnore,

    /// Whether this is an inline or block modifier.
    #[serde(rename = "type")]
    pub modifier_type: ModifierType,
}

/// A resolved modifier ready for use during conversion.
///
/// After cross-referencing a modifier definition with the template's ignore list,
/// this struct holds the effective output. If the template ignores this modifier,
/// `effective_typst` reflects the `on_ignore` behavior instead of the normal output.
#[derive(Debug, Clone)]
pub struct ResolvedModifier {
    /// The modifier's identifier (its key in modifiers.toml).
    pub id: String,

    /// The marker string to search for in the markdown/Typst output.
    pub marker: String,

    /// The effective Typst output after considering the template's ignore list.
    /// `None` means "keep the marker as-is" (on_ignore = keep).
    pub effective_typst: Option<String>,

    /// Whether this is an inline or block modifier.
    pub modifier_type: ModifierType,
}

/// The full set of modifier definitions loaded from modifiers.toml.
///
/// Keyed by modifier id (e.g., "date_separator", "column_break").
pub type ModifierRegistry = HashMap<String, ModifierDef>;

/// Resolve a modifier registry against a template's ignore list.
///
/// For each modifier, if the template ignores it, the effective output is
/// determined by the modifier's `on_ignore` field. Otherwise the normal
/// `typst` output is used.
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

/// Metadata for a template, loaded from `metadata.toml` in the template directory.
#[derive(Debug, Clone, Deserialize)]
pub struct TemplateMetadata {
    /// Human-readable template name (e.g., "Resume (Two Column)").
    pub name: String,

    /// Short description of the template's purpose.
    pub description: Option<String>,

    /// The brand to use if the user doesn't specify one.
    pub default_brand: Option<String>,

    /// List of modifier IDs that this template does not support.
    /// These modifiers will use their `on_ignore` behavior during conversion.
    #[serde(default)]
    pub ignore: Vec<String>,
}

/// A discovered template with its filesystem location and parsed metadata.
#[derive(Debug, Clone)]
pub struct Template {
    /// The template's short identifier (directory name, e.g., "resume-2-col").
    pub id: String,

    /// Absolute path to the template directory.
    pub path: PathBuf,

    /// Parsed metadata from the template's `metadata.toml`.
    pub metadata: TemplateMetadata,
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.metadata.description {
            Some(desc) => write!(f, "{}: {} -- {}", self.id, self.metadata.name, desc),
            None => write!(f, "{}: {}", self.id, self.metadata.name),
        }
    }
}

/// Metadata for a brand, loaded from `metadata.toml` in the brand directory.
#[derive(Debug, Clone, Deserialize)]
pub struct BrandMetadata {
    /// Human-readable brand name (e.g., "Generic").
    pub name: String,

    /// Short description of the brand.
    pub description: Option<String>,
}

/// A discovered brand with its filesystem location and parsed metadata.
#[derive(Debug, Clone)]
pub struct Brand {
    /// The brand's short identifier (directory name, e.g., "generic").
    pub id: String,

    /// Absolute path to the brand directory.
    pub path: PathBuf,

    /// Parsed metadata from the brand's `metadata.toml`.
    pub metadata: BrandMetadata,
}

impl fmt::Display for Brand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.metadata.description {
            Some(desc) => write!(f, "{}: {} -- {}", self.id, self.metadata.name, desc),
            None => write!(f, "{}: {}", self.id, self.metadata.name),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Application configuration assembled from layered sources.
///
/// Resolution order (later overrides earlier):
/// 1. Built-in defaults
/// 2. Global config: `~/.config/md-docs/config.toml`
/// 3. Project config: `.md-docs.toml` in the current directory
/// 4. CLI arguments
///
/// All fields are private. Use accessor methods to get effective values
/// with XDG defaults as fallbacks.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    /// Default template name to use when none is specified.
    pub(crate) default_template: Option<String>,

    /// Default brand name to use when none is specified.
    pub(crate) default_brand: Option<String>,

    /// Directory containing installed templates.
    pub(crate) templates_dir: Option<PathBuf>,

    /// Directory containing installed brands.
    pub(crate) brands_dir: Option<PathBuf>,

    /// Default output directory for generated PDFs.
    pub(crate) output_dir: Option<PathBuf>,

    /// Default author name injected into metadata when frontmatter doesn't specify one.
    pub(crate) author: Option<String>,
}

impl Config {
    /// Return the effective templates directory, falling back to the XDG data default.
    pub fn effective_templates_dir(&self) -> PathBuf {
        self.templates_dir.clone().unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var("HOME").unwrap_or_default())
                        .join(".local/share")
                })
                .join("md-docs/templates")
        })
    }

    /// Return the effective brands directory, falling back to the XDG data default.
    pub fn effective_brands_dir(&self) -> PathBuf {
        self.brands_dir.clone().unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var("HOME").unwrap_or_default())
                        .join(".local/share")
                })
                .join("md-docs/brands")
        })
    }

    /// Return the configured default template name, if any.
    pub fn default_template(&self) -> Option<&str> {
        self.default_template.as_deref()
    }

    /// Return the configured default brand name, if any.
    pub fn default_brand(&self) -> Option<&str> {
        self.default_brand.as_deref()
    }

    /// Return the configured output directory, if any.
    pub fn output_dir(&self) -> Option<&PathBuf> {
        self.output_dir.as_ref()
    }

    /// Return the configured default author name, if any.
    pub fn author(&self) -> Option<&str> {
        self.author.as_deref()
    }

    /// Return a reference to the raw templates_dir option (for config merging).
    pub fn raw_templates_dir(&self) -> Option<&PathBuf> {
        self.templates_dir.as_ref()
    }

    /// Return a reference to the raw brands_dir option (for config merging).
    pub fn raw_brands_dir(&self) -> Option<&PathBuf> {
        self.brands_dir.as_ref()
    }
}

// ---------------------------------------------------------------------------
// Typst content escaping
// ---------------------------------------------------------------------------

/// Characters that have special meaning in Typst markup mode and may need escaping.
const TYPST_SPECIAL_CHARS: &[char] = &[
    '*', '_', '`', '<', '@', '=', '#', '$', '~', '\\', '/', '+', '-',
];

/// Escape special Typst characters in plain text.
///
/// In Typst markup mode (inside `[...]`), certain characters trigger special
/// behavior. This function prefixes them with `\` to produce literal output.
///
/// Note: Context matters -- not all occurrences need escaping. This function
/// is conservative and escapes all potential special characters. The builder
/// should refine this based on integration testing.
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

/// Configuration for the markdown-to-Typst converter.
///
/// Bundles the resolved modifiers and any conversion options needed
/// by the pulldown-cmark event processor.
#[derive(Debug, Clone)]
pub struct ConversionContext {
    /// Block modifiers: maps HTML comment marker -> Typst output.
    /// Used during markdown event processing.
    pub block_modifiers: HashMap<String, Option<String>>,

    /// Inline modifiers: applied as text substitutions after conversion.
    pub inline_modifiers: Vec<ResolvedModifier>,
}

impl ConversionContext {
    /// Build a conversion context from a list of resolved modifiers.
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
