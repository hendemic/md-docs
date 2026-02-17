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
        let template_manager = TemplateManager::new(config.clone());
        let logger = FileLogger::new();

        // Warn if md-docs hasn't been initialized yet
        if !ConfigLoader::global_config_path().exists() {
            CliMessage::Warning(
                "md-docs is not initialized. Run 'md-docs init' to set up config and install templates.".to_string()
            ).print(verbose);
        }

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
            anyhow::bail!("No templates found. Install base templates with 'md-docs templates install'.");
        }

        let rows: Vec<TableRow> = templates.iter().map(|t| TableRow {
            id: &t.id,
            name: &t.metadata.name,
            description: t.metadata.description.as_deref(),
            source: String::new(),
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

        // 2. Try config default (user preference overrides template suggestion)
        if let Some(default) = self.config.default_brand() {
            return self.template_manager.resolve_brand(default);
        }

        // 3. Try template default brand
        if let Some(ref default) = template.metadata.default_brand {
            return self.template_manager.resolve_brand(default);
        }

        // 4. Interactive selection via dialoguer
        let brands = self.template_manager.discover_brands()?;
        if brands.is_empty() {
            anyhow::bail!("No brands found. Install base templates with 'md-docs templates install'.");
        }

        let rows: Vec<TableRow> = brands.iter().map(|b| TableRow {
            id: &b.id,
            name: &b.metadata.name,
            description: b.metadata.description.as_deref(),
            source: String::new(),
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
            self.emit(CliMessage::Warning("No templates found. Install base templates with 'md-docs templates install'.".to_string()));
            return Ok(());
        }

        let rows: Vec<TableRow> = templates
            .iter()
            .map(|t| TableRow {
                id: &t.id,
                name: &t.metadata.name,
                description: t.metadata.description.as_deref(),
                source: t.source.to_string(),
            })
            .collect();

        self.emit(CliMessage::Info("Available templates:".to_string()));
        for line in format_table(&rows) {
            self.emit(CliMessage::Plain(line));
        }
        Ok(())
    }

    /// Install template repos from config.
    ///
    /// If a name is given, installs only that repo. Otherwise installs all configured repos.
    pub fn install_templates(&self, name: Option<String>) -> anyhow::Result<()> {
        if self.config.repos().is_empty() {
            anyhow::bail!("No repos configured. Add [[repos]] to your config.");
        }
        match &name {
            Some(n) => self.emit(CliMessage::Info(format!("Installing repo '{}'...", n))),
            None => self.emit(CliMessage::Info("Installing all configured repos...".to_string())),
        }
        self.template_manager.install_repo(name.as_deref())?;
        self.emit(CliMessage::Success("Repos installed successfully.".to_string()));
        Ok(())
    }

    /// Update installed template repos (git pull).
    ///
    /// If a name is given, updates only that repo. Otherwise updates all.
    pub fn update_templates(&self, name: Option<String>) -> anyhow::Result<()> {
        match &name {
            Some(n) => self.emit(CliMessage::Info(format!("Updating repo '{}'...", n))),
            None => self.emit(CliMessage::Info("Updating all configured repos...".to_string())),
        }
        self.template_manager.update_repo(name.as_deref())?;
        self.emit(CliMessage::Success("Repos updated successfully.".to_string()));
        Ok(())
    }

    /// Add a template source (git repo or local directory) to the global config.
    ///
    /// Detects whether the argument is a git URI or local path, validates it,
    /// checks for duplicates, and appends the appropriate TOML entry.
    pub fn add_source(&self, source: &str) -> anyhow::Result<()> {
        let config_path = ConfigLoader::global_config_path();
        if !config_path.exists() {
            anyhow::bail!(
                "No config file found. Run 'md-docs init' first to create {}",
                config_path.display()
            );
        }

        if is_git_uri(source) {
            let name = repo_name_from_uri(source)?;

            // Check for duplicate repo name
            if self.config.repos().iter().any(|r| r.name == name) {
                anyhow::bail!("Repo '{}' already exists in config.", name);
            }

            let entry = format!("\n[[repos]]\nname = \"{}\"\nurl = \"{}\"\n", name, source);
            append_to_config(&config_path, &entry)?;
            self.emit(CliMessage::Success(format!(
                "Added repo '{}' ({}). Run 'md-docs templates install {}' to clone it.",
                name, source, name
            )));
        } else {
            let path = PathBuf::from(source);
            if !path.is_dir() {
                anyhow::bail!("Directory '{}' does not exist.", source);
            }

            let canonical = path.canonicalize()?;

            // Check for duplicate local path
            if self.config.local().iter().any(|l| {
                l.path.canonicalize().ok().as_ref() == Some(&canonical)
            }) {
                anyhow::bail!("Local source '{}' already exists in config.", canonical.display());
            }

            let entry = format!("\n[[local]]\npath = \"{}\"\n", canonical.display());
            append_to_config(&config_path, &entry)?;
            self.emit(CliMessage::Success(format!(
                "Added local source '{}'.", canonical.display()
            )));
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Brand commands
    // -----------------------------------------------------------------------

    /// List all available brands and print their metadata.
    pub fn list_brands(&self) -> anyhow::Result<()> {
        let brands = self.template_manager.discover_brands()?;
        if brands.is_empty() {
            self.emit(CliMessage::Warning("No brands found. Install base templates with 'md-docs templates install'.".to_string()));
            return Ok(());
        }

        let rows: Vec<TableRow> = brands
            .iter()
            .map(|b| TableRow {
                id: &b.id,
                name: &b.metadata.name,
                description: b.metadata.description.as_deref(),
                source: b.source.to_string(),
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

        if !self.config.repos().is_empty() {
            self.emit(CliMessage::Plain(format!("\n  {}:", "Repos".bold())));
            let repos_base = dirs::data_dir()
                .unwrap_or_else(|| {
                    PathBuf::from(std::env::var("HOME").unwrap_or_default())
                        .join(".local/share")
                })
                .join("md-docs/repos");
            for repo in self.config.repos() {
                let installed = repos_base.join(&repo.name).join(".git").is_dir();
                let status = if installed { "installed" } else { "not installed" };
                self.emit(CliMessage::Plain(format!(
                    "    {} -- {} ({})",
                    repo.name.bold(), repo.url.dimmed(), status.dimmed()
                )));
            }
        }

        if !self.config.local().is_empty() {
            self.emit(CliMessage::Plain(format!("\n  {}:", "Local sources".bold())));
            for local in self.config.local() {
                self.emit(CliMessage::Plain(format!(
                    "    {}", local.path.display().to_string().dimmed()
                )));
            }
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

    // -----------------------------------------------------------------------
    // Update command
    // -----------------------------------------------------------------------

    /// Check for updates and optionally perform a self-update.
    ///
    /// When `check_only` is true, reports whether an update is available
    /// but does not download or install it.
    pub fn self_update(&self, check_only: bool) -> anyhow::Result<()> {
        use crate::infra::updater;

        let check = updater::check_for_update()?;

        match check {
            updater::UpdateCheck::AurInstall => {
                self.emit(CliMessage::Info(
                    "md-docs was installed via your system package manager.".to_string(),
                ));
                self.emit(CliMessage::Info(
                    "Update using your AUR helper (e.g., yay -Syu md-docs).".to_string(),
                ));
            }
            updater::UpdateCheck::UpToDate(version) => {
                self.emit(CliMessage::Success(format!(
                    "md-docs v{} is already up to date.",
                    version
                )));
            }
            updater::UpdateCheck::UpdateAvailable {
                current,
                latest,
                release,
            } => {
                self.emit(CliMessage::Info(format!(
                    "Update available: v{} -> v{}",
                    current, latest
                )));
                if check_only {
                    return Ok(());
                }
                let confirm = dialoguer::Confirm::new()
                    .with_prompt(format!("Update md-docs from v{} to v{}?", current, latest))
                    .default(true)
                    .interact()?;
                if !confirm {
                    return Ok(());
                }
                self.emit(CliMessage::Info("Downloading update...".to_string()));
                updater::perform_update(&release)?;
                self.emit(CliMessage::Success(format!(
                    "Successfully updated md-docs to v{}.",
                    latest
                )));
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Init / New commands
    // -----------------------------------------------------------------------

    /// Create the global config file at `~/.config/md-docs/config.toml`.
    ///
    /// Writes a default config with the official `[[repos]]` entry, then
    /// reloads the config and installs all configured repos.
    pub fn init_project(&self) -> anyhow::Result<()> {
        let config_path = ConfigLoader::global_config_path();

        if config_path.exists() {
            anyhow::bail!("Global config already exists at {}", config_path.display());
        }

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let default_content = format!(
            r#"# md-docs global configuration

# default_template = "resume-2-col"
# default_brand = "generic"
# author = "Your Name"

[[repos]]
name = "{}"
url = "{}"
"#,
            crate::domain::DEFAULT_REPO_NAME, crate::domain::DEFAULT_REPO_URL
        );

        std::fs::write(&config_path, &default_content)?;
        self.emit(CliMessage::Success(format!("Created {}", config_path.display())));

        // Reload config to pick up new [[repos]], then install
        let fresh_config = ConfigLoader::load()?;
        let fresh_manager = TemplateManager::new(fresh_config);
        fresh_manager.install_repo(None)?;
        self.emit(CliMessage::Success("Default templates installed.".to_string()));

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
    source: String,
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
    let header_source = "Source";

    // Compute max width for each column (considering both header and data).
    let id_width = rows.iter().map(|r| r.id.len()).max().unwrap_or(0).max(header_id.len());
    let name_width = rows
        .iter()
        .map(|r| r.name.len())
        .max()
        .unwrap_or(0)
        .max(header_name.len());
    let desc_width = rows
        .iter()
        .map(|r| r.description.unwrap_or("").len())
        .max()
        .unwrap_or(0)
        .max(header_desc.len());

    let id_col = id_width + TABLE_COL_PAD;
    let name_col = name_width + TABLE_COL_PAD;
    let desc_col = desc_width + TABLE_COL_PAD;

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
    let hdr_desc = format!(
        "{}{}",
        header_desc.bold(),
        " ".repeat(desc_col.saturating_sub(header_desc.len()))
    );
    lines.push(format!("  {}{}{}{}", hdr_name, hdr_id, hdr_desc, header_source.bold()));

    // Data rows.
    for row in rows {
        let desc = row.description.unwrap_or("");
        // Pad id, name, and desc manually so ANSI escape codes from bold/dimmed
        // don't interfere with the width formatting.
        let name_padded = format!("{}{}", row.name.cyan(), " ".repeat(name_col.saturating_sub(row.name.len())));
        let id_padded = format!("{}{}", row.id.bold(), " ".repeat(id_col.saturating_sub(row.id.len())));
        let desc_padded = format!("{}{}", desc.dimmed(), " ".repeat(desc_col.saturating_sub(desc.len())));
        lines.push(format!("  {}{}{}{}", name_padded, id_padded, desc_padded, (&row.source).dimmed()));
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

// ---------------------------------------------------------------------------
// Source detection helpers
// ---------------------------------------------------------------------------

/// Returns true if the source string looks like a git URI.
fn is_git_uri(source: &str) -> bool {
    source.contains("://") || source.starts_with("git@")
}

/// Derive a repo name from a git URI.
///
/// Extracts the last path component and strips `.git` suffix.
/// e.g., `https://github.com/user/my-templates.git` → `my-templates`
fn repo_name_from_uri(uri: &str) -> anyhow::Result<String> {
    let trimmed = uri.trim_end_matches('/').trim_end_matches(".git");
    let name = trimmed
        .rsplit('/')
        .next()
        .or_else(|| trimmed.rsplit(':').next())
        .ok_or_else(|| anyhow::anyhow!("Could not derive repo name from URI: {}", uri))?;

    if name.is_empty() {
        anyhow::bail!("Could not derive repo name from URI: {}", uri);
    }

    Ok(name.to_string())
}

/// Append raw TOML text to the global config file.
fn append_to_config(path: &Path, entry: &str) -> anyhow::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    file.write_all(entry.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests;
