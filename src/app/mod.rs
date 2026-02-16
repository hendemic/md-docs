//! Application layer: orchestrates business logic.
//!
//! The `AppController` receives parsed CLI commands and coordinates between
//! config resolution, template/brand discovery, markdown conversion,
//! and Typst compilation.
//!
//! # Module dependency graph
//! ```text
//! app/mod.rs  (AppController)
//!   |-- app/config.rs      (ConfigLoader)
//!   |-- app/compiler.rs    (compile, assemble, generate)
//!   |-- app/converter.rs   (markdown_to_typst)
//!   |-- app/templates.rs   (TemplateManager, load_modifiers)
//!   |-- app/fonts.rs       (is_font_available, load_brand_fonts, fallback_font)
//!   +-- domain.rs          (all types)
//! ```

pub mod compiler;
pub mod config;
pub mod converter;
pub mod fonts;
pub mod templates;

use std::path::{Path, PathBuf};

use crate::domain::{Brand, Config, Document, Template};

use self::config::ConfigLoader;
use self::templates::TemplateManager;

/// Central application controller.
///
/// Owns the resolved configuration and provides methods for each CLI command.
/// Created once per invocation in `cli::run()`.
pub struct AppController {
    /// The resolved layered configuration.
    _config: Config,

    /// Template and brand discovery manager.
    _template_manager: TemplateManager,
}

impl AppController {
    /// Build a new AppController by loading layered configuration.
    ///
    /// Loads config from: defaults <- global <- project <- (CLI args applied later).
    pub fn new() -> anyhow::Result<Self> {
        todo!("Load config via ConfigLoader, build TemplateManager with resolved dirs")
    }

    // -----------------------------------------------------------------------
    // Convert command
    // -----------------------------------------------------------------------

    /// Convert a markdown file to PDF.
    ///
    /// Pipeline:
    /// 1. Read and parse the markdown file (frontmatter + body)
    /// 2. Resolve template and brand (by name or interactive selection)
    /// 3. Load and resolve modifiers for the chosen template
    /// 4. Convert markdown to Typst markup via `converter::markdown_to_typst`
    /// 5. Generate content.typ with metadata and content sections
    /// 6. Assemble temp dir with template.typ, brand.typ, content.typ
    /// 7. Compile via `compiler::compile` and write PDF to output path
    pub fn convert(
        &self,
        _input: PathBuf,
        _template_name: Option<String>,
        _brand_name: Option<String>,
        _output: Option<PathBuf>,
    ) -> anyhow::Result<()> {
        todo!("Orchestrate the full conversion pipeline")
    }

    /// Parse a markdown file into a Document.
    ///
    /// Reads the file, extracts YAML frontmatter, and stores the raw body.
    /// The config author is injected as a fallback if frontmatter has no author.
    fn parse_input(&self, _input: &Path) -> anyhow::Result<Document> {
        todo!("Read file, call Metadata::parse_from_content, inject config author fallback")
    }

    /// Resolve template by name, config default, or interactive selection.
    fn resolve_template(&self, _name: Option<String>) -> anyhow::Result<Template> {
        todo!("Try explicit name -> config default -> interactive selection via dialoguer")
    }

    /// Resolve brand by name, template default, config default, or interactive selection.
    fn resolve_brand(
        &self,
        _name: Option<String>,
        _template: &Template,
    ) -> anyhow::Result<Brand> {
        todo!("Try explicit name -> template default_brand -> config default -> interactive")
    }

    /// Determine the output PDF path.
    ///
    /// Priority: explicit output flag -> config output_dir -> input file with .pdf extension.
    fn resolve_output(&self, _input: &Path, _output: Option<PathBuf>) -> PathBuf {
        todo!("Resolve output path from explicit, config, or input-derived default")
    }

    // -----------------------------------------------------------------------
    // Template commands
    // -----------------------------------------------------------------------

    /// List all available templates and print their metadata.
    pub fn list_templates(&self) -> anyhow::Result<()> {
        todo!("Use template_manager.discover_templates(), format and print")
    }

    /// Install templates from a git repository URL.
    pub fn install_templates(&self, _repo_url: &str) -> anyhow::Result<()> {
        todo!("Use template_manager.install_repo(repo_url)")
    }

    /// Update installed templates (git pull).
    pub fn update_templates(&self, _name: Option<String>) -> anyhow::Result<()> {
        todo!("Use template_manager.update_repo(name)")
    }

    /// Remove an installed template by name.
    pub fn remove_template(&self, _name: &str) -> anyhow::Result<()> {
        todo!("Use template_manager.remove_template(name)")
    }

    // -----------------------------------------------------------------------
    // Brand commands
    // -----------------------------------------------------------------------

    /// List all available brands and print their metadata.
    pub fn list_brands(&self) -> anyhow::Result<()> {
        todo!("Use template_manager.discover_brands(), format and print")
    }

    // -----------------------------------------------------------------------
    // Config commands
    // -----------------------------------------------------------------------

    /// Display the current resolved configuration.
    pub fn show_config(&self) -> anyhow::Result<()> {
        todo!("Print the resolved config in a readable format")
    }

    /// Create a `.md-docs.toml` project config file in the current directory.
    pub fn init_project(&self) -> anyhow::Result<()> {
        todo!("Write a default .md-docs.toml to cwd if one doesn't already exist")
    }
}
