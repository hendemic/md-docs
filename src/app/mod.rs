//! Application layer: orchestrates domain logic with infra services.

pub mod converter;

use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::domain::{
    Brand, Config, ContentSections, ConversionContext, Document, Metadata, MdDocsError,
    Template, resolve_modifiers,
};

use crate::infra::system::ConfigLoader;
use crate::infra::system::FileLogger;
use crate::infra::templates::TemplateManager;

// ---------------------------------------------------------------------------
// CLI output messages
// ---------------------------------------------------------------------------

/// A CLI output message with semantic level.
#[derive(Debug, Clone)]
pub enum CliMessage {
    Success(String),
    Info(String),
    Log(String),
    Warning(String),
    Error(String),
    Plain(String),
}

impl CliMessage {
    /// Format this message with colors and prefix for terminal display.
    pub fn formatted(&self) -> String {
        match self {
            CliMessage::Success(msg) => format!("{} {}", "✓".green().bold(), msg),
            CliMessage::Info(msg) => format!("{}", msg.cyan()),
            CliMessage::Log(msg) => format!("{}", msg.dimmed()),
            CliMessage::Warning(msg) => format!("{} {}", "warning:".yellow().bold(), msg),
            CliMessage::Error(msg) => format!("{} {}", "error:".red().bold(), msg),
            CliMessage::Plain(msg) => msg.clone(),
        }
    }

    /// Print this message to the appropriate stream (stdout or stderr).
    pub fn print(&self, verbose: bool) {
        match self {
            CliMessage::Log(_) => {
                if verbose {
                    println!("{}", self.formatted());
                }
            }
            CliMessage::Warning(_) | CliMessage::Error(_) => {
                eprintln!("{}", self.formatted());
            }
            _ => {
                println!("{}", self.formatted());
            }
        }
    }
}

impl std::fmt::Display for CliMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.formatted())
    }
}

/// Central application controller.
pub struct AppController {
    config: Config,
    template_manager: TemplateManager,
    verbose: bool,
    logger: FileLogger,
}

impl AppController {
    /// Build a new AppController by loading layered configuration.
    pub fn new(verbose: bool) -> anyhow::Result<Self> {
        let config = ConfigLoader::load()?;
        let template_manager = TemplateManager::new(config.clone());
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
        let (level, text) = match &msg {
            CliMessage::Success(s) => ("SUCCESS", s.as_str()),
            CliMessage::Info(s) => ("INFO", s.as_str()),
            CliMessage::Log(s) => ("DEBUG", s.as_str()),
            CliMessage::Warning(s) => ("WARN", s.as_str()),
            CliMessage::Error(s) => ("ERROR", s.as_str()),
            CliMessage::Plain(s) => ("INFO", s.as_str()),
        };
        self.logger.log(level, text);
        msg.print(self.verbose);
    }

    // -----------------------------------------------------------------------
    // Convert command
    // -----------------------------------------------------------------------

    /// Convert a markdown file to PDF.
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
        let result = crate::infra::compiler::compile(&document, &template, &brand, &output_path)?;

        // Deduplicate and surface font warnings with actionable advice
        let mut seen_font_warnings = std::collections::HashSet::new();
        for w in &result.warnings {
            if let Some(font_name) = w.strip_prefix("unknown font family: ") {
                if seen_font_warnings.insert(font_name.to_string()) {
                    self.emit(CliMessage::Warning(format!(
                        "font '{}' not found — falling back to embedded default. \
                         Install the font or update the brand to use an available font.",
                        font_name
                    )));
                }
            } else {
                self.emit(CliMessage::Warning(format!("typst: {}", w)));
            }
        }
        self.emit(CliMessage::Success(format!("Output: {}", output_path.display())));

        Ok(())
    }

    /// Parse a markdown file into a Document.
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
    pub fn update_templates(&self, name: Option<String>) -> anyhow::Result<()> {
        match &name {
            Some(n) => self.emit(CliMessage::Info(format!("Updating repo '{}'...", n))),
            None => self.emit(CliMessage::Info("Updating all configured repos...".to_string())),
        }
        self.template_manager.update_repo(name.as_deref())?;
        self.emit(CliMessage::Success("Repos updated successfully.".to_string()));
        Ok(())
    }

    /// Add a template source (git repo or local directory) to global config.
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
            let repos_base = crate::infra::system::xdg_data_home().join("md-docs/repos");
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
    // Uninstall command
    // -----------------------------------------------------------------------

    /// Remove md-docs, its configuration, and data from this system.
    pub fn self_uninstall(&self) -> anyhow::Result<()> {
        use crate::infra::updater;

        // 1. Check for AUR install
        if updater::is_aur_install() {
            self.emit(CliMessage::Info(
                "md-docs was installed via your system package manager.".to_string(),
            ));
            self.emit(CliMessage::Info(
                "Uninstall using: pacman -Rns md-docs".to_string(),
            ));
            return Ok(());
        }

        // 2. Gather items to remove
        let config_dir = crate::infra::system::xdg_config_home().join("md-docs");
        let data_dir = crate::infra::system::xdg_data_home().join("md-docs");
        let binary = std::env::current_exe()?.canonicalize()?;

        let mut items: Vec<PathBuf> = Vec::new();
        if config_dir.exists() {
            items.push(config_dir.clone());
        }
        if data_dir.exists() {
            items.push(data_dir.clone());
        }
        items.push(binary.clone());

        // 3. Print what will be removed
        self.emit(CliMessage::Plain("The following will be removed:".to_string()));
        for item in &items {
            self.emit(CliMessage::Plain(format!("  - {}", item.display())));
        }

        // 4. Prompt for confirmation
        {
            use std::io::Write;
            print!("\nType 'confirm' to proceed: ");
            std::io::stdout().flush()?;
        }

        let input = {
            use std::io::BufRead;
            let stdin = std::io::stdin();
            let mut line = String::new();
            stdin.lock().read_line(&mut line)?;
            line.trim().to_string()
        };

        if input != "confirm" {
            self.emit(CliMessage::Info("Uninstall cancelled.".to_string()));
            return Ok(());
        }

        // 5. Delete config dir
        if config_dir.exists() {
            std::fs::remove_dir_all(&config_dir)?;
            self.emit(CliMessage::Success(format!("Removed {}", config_dir.display())));
        }

        // 6. Delete data dir
        if data_dir.exists() {
            std::fs::remove_dir_all(&data_dir)?;
            self.emit(CliMessage::Success(format!("Removed {}", data_dir.display())));
        }

        // 7. Delete the binary last
        match std::fs::remove_file(&binary) {
            Ok(()) => {
                self.emit(CliMessage::Success(format!("Removed {}", binary.display())));
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                self.emit(CliMessage::Warning(format!(
                    "Could not remove {} (permission denied). Remove it manually with: sudo rm {}",
                    binary.display(),
                    binary.display()
                )));
            }
            Err(e) => return Err(e.into()),
        }

        // 8. Print success
        self.emit(CliMessage::Success("md-docs has been uninstalled.".to_string()));

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Init / New commands
    // -----------------------------------------------------------------------

    /// Create global config and install default templates.
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
        match fresh_manager.install_repo(None) {
            Ok(()) => {
                self.emit(CliMessage::Success("Default templates installed.".to_string()));
            }
            Err(e) => {
                self.emit(CliMessage::Warning(format!(
                    "Config created but template installation failed: {}. \
                     Run 'mdocs templates install' to retry.",
                    e
                )));
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Table formatting helpers
// ---------------------------------------------------------------------------

struct TableRow<'a> {
    id: &'a str,
    name: &'a str,
    description: Option<&'a str>,
    source: String,
}

const TABLE_COL_PAD: usize = 3;

/// Format table rows into aligned, colored output lines.
fn format_table(rows: &[TableRow<'_>]) -> Vec<String> {
    let header_id = "ID";
    let header_name = "Name";
    let header_desc = "Description";
    let header_source = "Source";

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

    // Pad manually so ANSI codes don't disrupt alignment
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

    for row in rows {
        let desc = row.description.unwrap_or("");
        let name_padded = format!("{}{}", row.name.cyan(), " ".repeat(name_col.saturating_sub(row.name.len())));
        let id_padded = format!("{}{}", row.id.bold(), " ".repeat(id_col.saturating_sub(row.id.len())));
        let desc_padded = format!("{}{}", desc.dimmed(), " ".repeat(desc_col.saturating_sub(desc.len())));
        lines.push(format!("  {}{}{}{}", name_padded, id_padded, desc_padded, (&row.source).dimmed()));
    }

    lines
}

/// Format table rows for interactive selectors (no header row).
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

fn is_git_uri(source: &str) -> bool {
    source.contains("://") || source.starts_with("git@")
}

/// Derive a repo name from a git URI (last path component, minus `.git`).
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

fn append_to_config(path: &Path, entry: &str) -> anyhow::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    file.write_all(entry.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests;
