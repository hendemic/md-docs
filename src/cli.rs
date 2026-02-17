//! CLI argument parsing and user interface.
//!
//! This module defines the command-line interface using `clap` derive macros.
//! It parses arguments, then delegates to `AppController` for all business logic.
//!
//! # Module dependency graph
//! ```text
//! cli.rs  -->  app/mod.rs  -->  domain.rs
//!   |              |
//!   +-- Clap       +-- config, compiler, converter, templates, fonts
//!       structs        (app-layer modules)
//! ```

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use md_docs::app::AppController;

/// md-docs: Generate professional documents from Markdown using Typst templates.
#[derive(Debug, Parser)]
#[command(name = "mdocs", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output (show debug/log messages).
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Convert a Markdown file to PDF.
    Convert {
        /// Path to the input Markdown file.
        file: PathBuf,

        /// Template name to use for layout.
        #[arg(short, long)]
        template: Option<String>,

        /// Brand name to use for colors and fonts.
        #[arg(short, long)]
        brand: Option<String>,

        /// Output PDF file path.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Manage templates.
    Templates {
        #[command(subcommand)]
        action: TemplateCommands,
    },

    /// Manage brands.
    Brands {
        #[command(subcommand)]
        action: BrandCommands,
    },

    /// Check for updates and update md-docs to the latest version.
    Update {
        /// Only check for updates, don't install.
        #[arg(long)]
        check: bool,
    },

    /// Show current configuration (layered: defaults <- global <- project <- CLI).
    Config,

    /// Initialize global config at ~/.config/md-docs/config.toml.
    Init,

    /// Create a new document from a template's starter file.
    New {
        /// Template name to create from.
        template: String,

        /// Output directory (defaults to current directory).
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Custom filename for the output file.
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Remove md-docs, its configuration, and data from this system.
    Uninstall,
}

/// Subcommands for template management.
#[derive(Debug, Subcommand)]
pub enum TemplateCommands {
    /// List all available templates with metadata and source.
    List,

    /// Install template repos from config. Installs all repos if no name given.
    Install {
        /// Specific repo name to install. Installs all if omitted.
        name: Option<String>,
    },

    /// Update installed template repos (git pull). Updates all if no name given.
    Update {
        /// Specific repo name to update. Updates all if omitted.
        name: Option<String>,
    },

    /// Add a template source (git repo URL or local directory path) to config.
    Add {
        /// Git repo URL or local directory path.
        source: String,
    },
}

/// Subcommands for brand management.
#[derive(Debug, Subcommand)]
pub enum BrandCommands {
    /// List all available brands with metadata.
    List,
}

/// Parse CLI arguments and run the appropriate command.
///
/// This is the main entry point called from `main.rs`.
/// It builds an `AppController` with the resolved config, then dispatches
/// the parsed command to the appropriate controller method.
pub fn run() -> anyhow::Result<()> {
    // If the first arg looks like a markdown file, default to `convert`
    let mut args: Vec<String> = std::env::args().collect();
    if args.len() >= 2 && args[1].ends_with(".md") {
        args.insert(1, "convert".to_string());
    }
    let cli = Cli::parse_from(args);

    // Build the app controller with layered config
    let controller = AppController::new(cli.verbose)?;

    match cli.command {
        Commands::Convert {
            file,
            template,
            brand,
            output,
        } => controller.convert(file, template, brand, output),

        Commands::Templates { action } => match action {
            TemplateCommands::List => controller.list_templates(),
            TemplateCommands::Install { name } => controller.install_templates(name),
            TemplateCommands::Update { name } => controller.update_templates(name),
            TemplateCommands::Add { source } => controller.add_source(&source),
        },

        Commands::Brands { action } => match action {
            BrandCommands::List => controller.list_brands(),
        },

        Commands::Update { check } => controller.self_update(check),

        Commands::Config => controller.show_config(),

        Commands::Init => controller.init_project(),

        Commands::New { template, output_dir, name } => {
            controller.new_from_template(&template, output_dir, name.as_deref())
        }

        Commands::Uninstall => controller.self_uninstall(),
    }
}
