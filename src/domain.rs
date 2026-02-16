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
    pub fn new(_metadata: Metadata, _raw_body: String) -> Self {
        todo!("Construct Document with empty sections; sections are populated after conversion")
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
    pub fn parse_from_content(_content: &str) -> Result<(Self, String), MdDocsError> {
        todo!("Split content at --- delimiters, parse YAML into Metadata fields + extra map")
    }
}

/// The three content sections that templates consume.
///
/// Templates import `content.typ` which exports `header`, `body`, and `content`.
/// The split happens at the `COLUMNS_START` modifier marker.
#[derive(Debug, Clone, Default)]
pub struct ContentSections {
    /// Typst markup for everything above the column split marker.
    /// If no split marker exists, this is empty.
    pub header: String,

    /// Typst markup for everything below the column split marker.
    /// If no split marker exists, this contains all content.
    pub body: String,

    /// Full Typst markup (header + body combined). Always populated.
    pub content: String,
}

impl ContentSections {
    /// Split converted Typst content at the COLUMNS_START marker.
    ///
    /// If the marker is present, content above goes to `header` and below to `body`.
    /// If absent, everything goes to `body` and `header` is empty.
    /// `content` is always the full text.
    pub fn from_typst_content(_typst_content: &str, _split_marker: &str) -> Self {
        todo!("Split typst_content at split_marker into header/body, set content = full text")
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
pub fn resolve_modifiers(_registry: &ModifierRegistry, _ignore_list: &[String]) -> Vec<ResolvedModifier> {
    todo!("Iterate registry, check ignore_list membership, apply on_ignore logic, return resolved list")
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
    default_template: Option<String>,

    /// Default brand name to use when none is specified.
    default_brand: Option<String>,

    /// Directory containing installed templates.
    templates_dir: Option<PathBuf>,

    /// Directory containing installed brands.
    brands_dir: Option<PathBuf>,

    /// Default output directory for generated PDFs.
    output_dir: Option<PathBuf>,

    /// Default author name injected into metadata when frontmatter doesn't specify one.
    author: Option<String>,
}

impl Config {
    /// Return the effective templates directory, falling back to the XDG data default.
    pub fn effective_templates_dir(&self) -> PathBuf {
        todo!("Return self.templates_dir if set, otherwise XDG_DATA_HOME/md-docs/templates")
    }

    /// Return the effective brands directory, falling back to the XDG data default.
    pub fn effective_brands_dir(&self) -> PathBuf {
        todo!("Return self.brands_dir if set, otherwise XDG_DATA_HOME/md-docs/brands")
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
const _TYPST_SPECIAL_CHARS: &[char] = &[
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
pub fn escape_typst(_text: &str) -> String {
    todo!("Iterate chars, prefix _TYPST_SPECIAL_CHARS with backslash")
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
    pub fn from_resolved(_modifiers: &[ResolvedModifier]) -> Self {
        todo!("Partition modifiers into block_modifiers map and inline_modifiers vec")
    }
}
