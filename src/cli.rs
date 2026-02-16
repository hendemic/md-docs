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

    /// Create a `.md-docs.toml` project config in the current directory.
    Init,
}

/// Subcommands for template management.
#[derive(Debug, Subcommand)]
pub enum TemplateCommands {
    /// List all available templates with metadata.
    List,

    /// Install templates from a git repository.
    Install {
        /// Git repository URL to clone.
        repo_url: String,
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
    let _controller = AppController::new()?;

    match cli.command {
        Commands::Convert {
            file: _file,
            template: _template,
            brand: _brand,
            output: _output,
        } => {
            todo!("Call controller.convert(file, template, brand, output)")
        }

        Commands::Templates { action } => match action {
            TemplateCommands::List => {
                todo!("Call controller.list_templates() and print results")
            }
            TemplateCommands::Install { repo_url: _repo_url } => {
                todo!("Call controller.install_templates(repo_url)")
            }
            TemplateCommands::Update { name: _name } => {
                todo!("Call controller.update_templates(name)")
            }
            TemplateCommands::Remove { name: _name } => {
                todo!("Call controller.remove_template(name)")
            }
        },

        Commands::Brands { action } => match action {
            BrandCommands::List => {
                todo!("Call controller.list_brands() and print results")
            }
        },

        Commands::Config => {
            todo!("Call controller.show_config() and print the resolved config")
        }

        Commands::Init => {
            todo!("Call controller.init_project() to create .md-docs.toml in cwd")
        }
    }
}
