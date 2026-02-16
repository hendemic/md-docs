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

use crate::domain::{
    Brand, Config, ContentSections, ConversionContext, Document, Metadata, MdDocsError, Template,
    resolve_modifiers,
};

use self::config::ConfigLoader;
use self::templates::TemplateManager;

/// Central application controller.
///
/// Owns the resolved configuration and provides methods for each CLI command.
/// Created once per invocation in `cli::run()`.
pub struct AppController {
    /// The resolved layered configuration.
    config: Config,

    /// Template and brand discovery manager.
    template_manager: TemplateManager,
}

impl AppController {
    /// Build a new AppController by loading layered configuration.
    ///
    /// Loads config from: defaults <- global <- project <- (CLI args applied later).
    pub fn new() -> anyhow::Result<Self> {
        let config = ConfigLoader::load()?;
        let template_manager = TemplateManager::new(&config);
        Ok(Self {
            config,
            template_manager,
        })
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
        input: PathBuf,
        template_name: Option<String>,
        brand_name: Option<String>,
        output: Option<PathBuf>,
    ) -> anyhow::Result<()> {
        // 1. Parse the input markdown file
        let mut document = self.parse_input(&input)?;

        // 2. Resolve template and brand
        let template = self.resolve_template(template_name)?;
        let brand = self.resolve_brand(brand_name, &template)?;

        // 3. Load and resolve modifiers
        let registry = templates::load_modifiers()?;
        let resolved = resolve_modifiers(&registry, &template.metadata.ignore);
        let context = ConversionContext::from_resolved(&resolved);

        // 4. Convert markdown to Typst
        let typst_content = converter::markdown_to_typst(&document.raw_body, &context)?;

        // 5. Split into content sections at the COLUMNS_START marker
        let sections = ContentSections::from_typst_content(&typst_content, "%%COLUMNS_START%%");
        document.sections = sections;

        // 6. Resolve output path
        let output_path = self.resolve_output(&input, output);

        // 7. Compile to PDF
        println!("Compiling {} with template '{}' and brand '{}'...", input.display(), template.id, brand.id);
        compiler::compile(&document, &template, &brand, &output_path)?;
        println!("Output: {}", output_path.display());

        Ok(())
    }

    /// Parse a markdown file into a Document.
    ///
    /// Reads the file, extracts YAML frontmatter, and stores the raw body.
    /// The config author is injected as a fallback if frontmatter has no author.
    fn parse_input(&self, input: &Path) -> anyhow::Result<Document> {
        if !input.is_file() {
            return Err(MdDocsError::InputNotFound(input.to_path_buf()).into());
        }

        let content = std::fs::read_to_string(input)?;
        let (mut metadata, raw_body) = Metadata::parse_from_content(&content)?;

        // Inject config author as fallback if frontmatter has no author
        if metadata.author.is_none() {
            if let Some(config_author) = self.config.author() {
                metadata.author = Some(config_author.to_string());
            }
        }

        Ok(Document::new(metadata, raw_body))
    }

    /// Resolve template by name, config default, or interactive selection.
    fn resolve_template(&self, name: Option<String>) -> anyhow::Result<Template> {
        // 1. Try explicit name
        if let Some(name) = name {
            return self.template_manager.resolve_template(&name);
        }

        // 2. Try config default
        if let Some(default) = self.config.default_template() {
            return self.template_manager.resolve_template(default);
        }

        // 3. Interactive selection via dialoguer
        let templates = self.template_manager.discover_templates()?;
        if templates.is_empty() {
            anyhow::bail!("No templates found. Install templates with 'md-docs templates install <repo_url>'.");
        }

        let items: Vec<String> = templates.iter().map(|t| t.to_string()).collect();
        let selection = dialoguer::Select::new()
            .with_prompt("Select a template")
            .items(&items)
            .default(0)
            .interact()?;

        Ok(templates.into_iter().nth(selection).unwrap())
    }

    /// Resolve brand by name, template default, config default, or interactive selection.
    fn resolve_brand(
        &self,
        name: Option<String>,
        template: &Template,
    ) -> anyhow::Result<Brand> {
        // 1. Try explicit name
        if let Some(name) = name {
            return self.template_manager.resolve_brand(&name);
        }

        // 2. Try template default brand
        if let Some(ref default) = template.metadata.default_brand {
            return self.template_manager.resolve_brand(default);
        }

        // 3. Try config default
        if let Some(default) = self.config.default_brand() {
            return self.template_manager.resolve_brand(default);
        }

        // 4. Interactive selection via dialoguer
        let brands = self.template_manager.discover_brands()?;
        if brands.is_empty() {
            anyhow::bail!("No brands found. Install brands with 'md-docs templates install <repo_url>'.");
        }

        let items: Vec<String> = brands.iter().map(|b| b.to_string()).collect();
        let selection = dialoguer::Select::new()
            .with_prompt("Select a brand")
            .items(&items)
            .default(0)
            .interact()?;

        Ok(brands.into_iter().nth(selection).unwrap())
    }

    /// Determine the output PDF path.
    ///
    /// Priority: explicit output flag -> config output_dir -> input file with .pdf extension.
    fn resolve_output(&self, input: &Path, output: Option<PathBuf>) -> PathBuf {
        // 1. Explicit output path
        if let Some(output) = output {
            return output;
        }

        // 2. Config output_dir (use input filename with .pdf extension inside it)
        if let Some(output_dir) = self.config.output_dir() {
            let filename = input
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            return output_dir.join(format!("{}.pdf", filename));
        }

        // 3. Input file with .pdf extension
        input.with_extension("pdf")
    }

    // -----------------------------------------------------------------------
    // Template commands
    // -----------------------------------------------------------------------

    /// List all available templates and print their metadata.
    pub fn list_templates(&self) -> anyhow::Result<()> {
        let templates = self.template_manager.discover_templates()?;
        if templates.is_empty() {
            println!("No templates found.");
            return Ok(());
        }
        println!("Available templates:");
        for template in &templates {
            println!("  {}", template);
        }
        Ok(())
    }

    /// Install templates from a git repository URL.
    pub fn install_templates(&self, repo_url: &str) -> anyhow::Result<()> {
        println!("Installing templates from {}...", repo_url);
        self.template_manager.install_repo(repo_url)?;
        println!("Templates installed successfully.");
        Ok(())
    }

    /// Update installed templates (git pull).
    pub fn update_templates(&self, name: Option<String>) -> anyhow::Result<()> {
        println!("Updating templates...");
        self.template_manager.update_repo(name.as_deref())?;
        println!("Templates updated successfully.");
        Ok(())
    }

    /// Remove an installed template by name.
    pub fn remove_template(&self, name: &str) -> anyhow::Result<()> {
        self.template_manager.remove_template(name)?;
        println!("Template '{}' removed.", name);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Brand commands
    // -----------------------------------------------------------------------

    /// List all available brands and print their metadata.
    pub fn list_brands(&self) -> anyhow::Result<()> {
        let brands = self.template_manager.discover_brands()?;
        if brands.is_empty() {
            println!("No brands found.");
            return Ok(());
        }
        println!("Available brands:");
        for brand in &brands {
            println!("  {}", brand);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Config commands
    // -----------------------------------------------------------------------

    /// Display the current resolved configuration.
    pub fn show_config(&self) -> anyhow::Result<()> {
        println!("Current configuration:");
        println!("  Templates dir: {}", self.config.effective_templates_dir().display());
        println!("  Brands dir:    {}", self.config.effective_brands_dir().display());
        if let Some(t) = self.config.default_template() {
            println!("  Default template: {}", t);
        }
        if let Some(b) = self.config.default_brand() {
            println!("  Default brand:    {}", b);
        }
        if let Some(d) = self.config.output_dir() {
            println!("  Output dir:       {}", d.display());
        }
        if let Some(a) = self.config.author() {
            println!("  Author:           {}", a);
        }
        Ok(())
    }

    /// Create a `.md-docs.toml` project config file in the current directory.
    pub fn init_project(&self) -> anyhow::Result<()> {
        let config_path = std::env::current_dir()?.join(".md-docs.toml");

        if config_path.exists() {
            anyhow::bail!(".md-docs.toml already exists in the current directory.");
        }

        let default_content = r#"# md-docs project configuration
# Uncomment and edit the options you want to set.

# default_template = "resume-2-col"
# default_brand = "generic"
# output_dir = "./output"
# author = "Your Name"
# templates_dir = "/path/to/templates"
# brands_dir = "/path/to/brands"
"#;

        std::fs::write(&config_path, default_content)?;
        println!("Created {}", config_path.display());

        Ok(())
    }
}
