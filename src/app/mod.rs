//! Application layer: orchestrates business logic.
//!
//! The `AppController` receives parsed CLI commands and coordinates between
//! config resolution, template/brand discovery, markdown conversion,
//! and Typst compilation. All I/O services live in `infra/`.
//!
//! # Module dependency graph
//! ```text
//! app/mod.rs  (AppController)
//!   |-- app/compiler.rs      (compile, assemble, generate)
//!   |-- app/converter.rs     (markdown_to_typst)
//!   |-- infra/config.rs      (ConfigLoader)
//!   |-- infra/templates.rs   (TemplateManager, load_modifiers)
//!   |-- infra/fonts.rs       (is_font_available, load_brand_fonts, fallback_font)
//!   +-- domain.rs            (all types)
//! ```

pub mod compiler;
pub mod converter;

use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::domain::{
    Brand, CliMessage, Config, ContentSections, ConversionContext, Document, Metadata, MdDocsError,
    Template, resolve_modifiers,
};

use crate::infra::config::ConfigLoader;
use crate::infra::logger::FileLogger;
use crate::infra::templates::TemplateManager;

/// Central application controller.
///
/// Owns the resolved configuration and provides methods for each CLI command.
/// Created once per invocation in `cli::run()`.
pub struct AppController {
    /// The resolved layered configuration.
    config: Config,

    /// Template and brand discovery manager.
    template_manager: TemplateManager,

    /// Whether to show verbose/debug output.
    verbose: bool,

    /// File logger for persistent log entries.
    logger: FileLogger,
}

impl AppController {
    /// Build a new AppController by loading layered configuration.
    ///
    /// Loads config from: defaults <- global <- project <- (CLI args applied later).
    pub fn new(verbose: bool) -> anyhow::Result<Self> {
        let config = ConfigLoader::load()?;
        let template_manager = TemplateManager::new(&config);
        let logger = FileLogger::new();
        Ok(Self {
            config,
            template_manager,
            verbose,
            logger,
        })
    }

    /// Emit a CLI message to terminal and log file.
    fn emit(&self, msg: CliMessage) {
        self.logger.log_message(&msg);
        msg.print(self.verbose);
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
        self.emit(CliMessage::Log(format!("Reading input: {}", input.display())));
        let mut document = self.parse_input(&input)?;
        self.emit(CliMessage::Log(format!(
            "Parsed frontmatter: title={:?}, author={:?}",
            document.metadata.title, document.metadata.author
        )));

        // 2. Resolve template and brand
        let template = self.resolve_template(template_name)?;
        self.emit(CliMessage::Log(format!(
            "Template: {} ({})", template.id, template.path.display()
        )));
        let brand = self.resolve_brand(brand_name, &template)?;
        self.emit(CliMessage::Log(format!(
            "Brand: {} ({})", brand.id, brand.path.display()
        )));

        // 3. Load and resolve modifiers
        let registry = crate::infra::templates::load_modifiers()?;
        let resolved = resolve_modifiers(&registry, &template.metadata.ignore);
        let context = ConversionContext::from_resolved(&resolved);

        // 4. Convert markdown to Typst
        let typst_content = converter::markdown_to_typst(&document.raw_body, &context)?;
        self.emit(CliMessage::Log(format!(
            "Converted markdown to Typst ({} bytes)", typst_content.len()
        )));

        // 5. Split into content sections at the COLUMNS_START marker
        let sections = ContentSections::from_typst_content(&typst_content, "%%COLUMNS_START%%");
        document.sections = sections;

        // 6. Resolve output path
        let output_path = self.resolve_output(&input, output);

        // 7. Compile to PDF
        self.emit(CliMessage::Info(format!(
            "Compiling {} with template '{}' and brand '{}'...",
            input.display(), template.id, brand.id
        )));
        let result = compiler::compile(&document, &template, &brand, &output_path)?;
        for w in &result.warnings {
            self.emit(CliMessage::Warning(format!("typst: {}", w)));
        }
        self.emit(CliMessage::Success(format!("Output: {}", output_path.display())));

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

        let rows: Vec<TableRow> = templates.iter().map(|t| TableRow {
            id: &t.id,
            name: &t.metadata.name,
            description: t.metadata.description.as_deref(),
        }).collect();
        let items = format_selector_items(&rows);
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

        let rows: Vec<TableRow> = brands.iter().map(|b| TableRow {
            id: &b.id,
            name: &b.metadata.name,
            description: b.metadata.description.as_deref(),
        }).collect();
        let items = format_selector_items(&rows);
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
            self.emit(CliMessage::Warning("No templates found.".to_string()));
            return Ok(());
        }

        let rows: Vec<TableRow> = templates
            .iter()
            .map(|t| TableRow {
                id: &t.id,
                name: &t.metadata.name,
                description: t.metadata.description.as_deref(),
            })
            .collect();

        self.emit(CliMessage::Info("Available templates:".to_string()));
        for line in format_table(&rows) {
            self.emit(CliMessage::Plain(line));
        }
        Ok(())
    }

    /// Install templates from a git repository URL.
    ///
    /// Defaults to the official md-docs-templates repo if no URL is provided.
    pub fn install_templates(&self, repo_url: Option<String>) -> anyhow::Result<()> {
        const DEFAULT_REPO: &str = "https://github.com/hendemic/md-docs-templates.git";
        let url = repo_url.as_deref().unwrap_or(DEFAULT_REPO);
        self.emit(CliMessage::Info(format!("Installing templates from {}...", url)));
        self.template_manager.install_repo(url)?;
        self.emit(CliMessage::Success("Templates installed successfully.".to_string()));
        Ok(())
    }

    /// Update installed templates (git pull).
    pub fn update_templates(&self, name: Option<String>) -> anyhow::Result<()> {
        self.emit(CliMessage::Info("Updating templates...".to_string()));
        self.template_manager.update_repo(name.as_deref())?;
        self.emit(CliMessage::Success("Templates updated successfully.".to_string()));
        Ok(())
    }

    /// Remove an installed template by name.
    pub fn remove_template(&self, name: &str) -> anyhow::Result<()> {
        self.template_manager.remove_template(name)?;
        self.emit(CliMessage::Success(format!("Template '{}' removed.", name)));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Brand commands
    // -----------------------------------------------------------------------

    /// List all available brands and print their metadata.
    pub fn list_brands(&self) -> anyhow::Result<()> {
        let brands = self.template_manager.discover_brands()?;
        if brands.is_empty() {
            self.emit(CliMessage::Warning("No brands found.".to_string()));
            return Ok(());
        }

        let rows: Vec<TableRow> = brands
            .iter()
            .map(|b| TableRow {
                id: &b.id,
                name: &b.metadata.name,
                description: b.metadata.description.as_deref(),
            })
            .collect();

        self.emit(CliMessage::Info("Available brands:".to_string()));
        for line in format_table(&rows) {
            self.emit(CliMessage::Plain(line));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Config commands
    // -----------------------------------------------------------------------

    /// Display the current resolved configuration.
    pub fn show_config(&self) -> anyhow::Result<()> {
        self.emit(CliMessage::Info("Current configuration:".to_string()));
        self.emit(CliMessage::Plain(format!(
            "  {}: {}",
            "Templates dir".bold(),
            self.config.effective_templates_dir().display().to_string().dimmed()
        )));
        self.emit(CliMessage::Plain(format!(
            "  {}: {}",
            "Brands dir".bold(),
            self.config.effective_brands_dir().display().to_string().dimmed()
        )));
        if let Some(t) = self.config.default_template() {
            self.emit(CliMessage::Plain(format!(
                "  {}: {}", "Default template".bold(), t.dimmed()
            )));
        }
        if let Some(b) = self.config.default_brand() {
            self.emit(CliMessage::Plain(format!(
                "  {}: {}", "Default brand".bold(), b.dimmed()
            )));
        }
        if let Some(d) = self.config.output_dir() {
            self.emit(CliMessage::Plain(format!(
                "  {}: {}", "Output dir".bold(), d.display().to_string().dimmed()
            )));
        }
        if let Some(a) = self.config.author() {
            self.emit(CliMessage::Plain(format!(
                "  {}: {}", "Author".bold(), a.dimmed()
            )));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // New command
    // -----------------------------------------------------------------------

    /// Create a new document from a template's starter file.
    ///
    /// Copies the template's starter markdown file to the specified output
    /// directory (or current directory), optionally renaming it.
    pub fn new_from_template(
        &self,
        template_name: &str,
        output_dir: Option<PathBuf>,
        name: Option<&str>,
    ) -> anyhow::Result<()> {
        let template = self.template_manager.resolve_template(template_name)?;

        let starter_filename = template.metadata.starter_file.as_deref()
            .ok_or_else(|| anyhow::anyhow!(
                "Template '{}' does not include a starter file.", template_name
            ))?;

        let starter_path = template.path.join(starter_filename);
        if !starter_path.is_file() {
            anyhow::bail!(
                "Starter file '{}' not found in template '{}'.",
                starter_filename, template_name
            );
        }

        let base_dir = match output_dir {
            Some(dir) => {
                std::fs::create_dir_all(&dir)?;
                dir
            }
            None => std::env::current_dir()?,
        };

        let output_filename = name.unwrap_or(starter_filename);
        let output_path = base_dir.join(output_filename);

        if output_path.exists() {
            anyhow::bail!(
                "File '{}' already exists. Use -n to specify a different name.",
                output_path.display()
            );
        }

        std::fs::copy(&starter_path, &output_path)?;
        self.emit(CliMessage::Success(format!("Created {}", output_path.display())));

        Ok(())
    }

    /// Create the global config file at `~/.config/md-docs/config.toml`.
    pub fn init_project(&self) -> anyhow::Result<()> {
        let config_path = ConfigLoader::global_config_path();

        if config_path.exists() {
            anyhow::bail!("Global config already exists at {}", config_path.display());
        }

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default_content = r#"# md-docs global configuration

# default_template = "resume-2-col"
# default_brand = "generic"
# author = "Your Name"
"#;

        std::fs::write(&config_path, default_content)?;
        self.emit(CliMessage::Success(format!("Created {}", config_path.display())));

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Table formatting helpers
// ---------------------------------------------------------------------------

/// A row of data to display in a formatted table.
struct TableRow<'a> {
    id: &'a str,
    name: &'a str,
    description: Option<&'a str>,
}

/// Minimum padding (in spaces) between the widest value and the next column.
const TABLE_COL_PAD: usize = 3;

/// Format a list of table rows into aligned, colored output lines.
///
/// Computes column widths dynamically from the data, emits a bold header row
/// followed by data rows with bold id, normal name, and dimmed description.
/// Each line is prefixed with two spaces of indentation.
fn format_table(rows: &[TableRow<'_>]) -> Vec<String> {
    let header_id = "ID";
    let header_name = "Name";
    let header_desc = "Description";

    // Compute max width for each column (considering both header and data).
    let id_width = rows.iter().map(|r| r.id.len()).max().unwrap_or(0).max(header_id.len());
    let name_width = rows
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(0)
        .max(header_name.len());

    let id_col = id_width + TABLE_COL_PAD;
    let name_col = name_width + TABLE_COL_PAD;

    let mut lines = Vec::with_capacity(rows.len() + 1);

    // Header row (bold). Pad manually to avoid ANSI codes disrupting alignment.
    let hdr_id = format!(
        "{}{}",
        header_id.bold(),
        " ".repeat(id_col.saturating_sub(header_id.len()))
    );
    let hdr_name = format!(
        "{}{}",
        header_name.bold(),
        " ".repeat(name_col.saturating_sub(header_name.len()))
    );
    lines.push(format!("  {}{}{}", hdr_name, hdr_id, header_desc.bold()));

    // Data rows.
    for row in rows {
        let desc = row.description.unwrap_or("");
        // Pad id and name manually so ANSI escape codes from bold/dimmed
        // don't interfere with the width formatting.
        let name_padded = format!("{}{}", row.name.cyan(), " ".repeat(name_col.saturating_sub(row.name.len())));
        let id_padded = format!("{}{}", row.id.bold(), " ".repeat(id_col.saturating_sub(row.id.len())));
        lines.push(format!("  {}{}{}", name_padded, id_padded, desc.dimmed()));
    }

    lines
}

/// Format table rows as padded, colored lines for interactive selectors.
///
/// Same column alignment as `format_table` but without header row or indentation.
fn format_selector_items(rows: &[TableRow<'_>]) -> Vec<String> {
    let name_width = rows.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let id_width = rows.iter().map(|r| r.id.len()).max().unwrap_or(0);

    let name_col = name_width + TABLE_COL_PAD;
    let id_col = id_width + TABLE_COL_PAD;

    rows.iter()
        .map(|row| {
            let desc = row.description.unwrap_or("");
            let name_padded = format!("{}{}", row.name.cyan(), " ".repeat(name_col.saturating_sub(row.name.len())));
            let id_padded = format!("{}{}", row.id.bold(), " ".repeat(id_col.saturating_sub(row.id.len())));
            format!("{}{}{}", name_padded, id_padded, desc.dimmed())
        })
        .collect()
}

#[cfg(test)]
mod tests;
