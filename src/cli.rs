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
#[command(name = "md-docs", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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
}

/// Subcommands for template management.
#[derive(Debug, Subcommand)]
pub enum TemplateCommands {
    /// List all available templates with metadata.
    List,

    /// Install templates (defaults to official repo, or specify a custom URL).
    Install {
        /// Custom git repository URL. Uses the official repo if omitted.
        #[arg(short, long)]
        uri: Option<String>,
    },

    /// Update installed templates (git pull).
    Update {
        /// Specific template name to update. Updates all if omitted.
        name: Option<String>,
    },

    /// Remove an installed template.
    Remove {
        /// Template name to remove.
        name: String,
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
    let cli = Cli::parse();

    // Build the app controller with layered config
    let controller = AppController::new()?;

    match cli.command {
        Commands::Convert {
            file,
            template,
            brand,
            output,
        } => controller.convert(file, template, brand, output),

        Commands::Templates { action } => match action {
            TemplateCommands::List => controller.list_templates(),
            TemplateCommands::Install { uri } => controller.install_templates(uri),
            TemplateCommands::Update { name } => controller.update_templates(name),
            TemplateCommands::Remove { name } => controller.remove_template(&name),
        },

        Commands::Brands { action } => match action {
            BrandCommands::List => controller.list_brands(),
        },

        Commands::Config => controller.show_config(),

        Commands::Init => controller.init_project(),

        Commands::New { template, output_dir, name } => {
            controller.new_from_template(&template, output_dir, name.as_deref())
        }
    }
}
