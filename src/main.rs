//! md-docs: Convert Markdown files into professional PDFs using Typst templates.
//!
//! Entry point. Delegates immediately to `cli::run()`.
//! The `cli` module is binary-only; `app` and `domain` come from the library crate.

mod cli;

use md_docs::domain::CliMessage;

fn main() {
    if let Err(err) = cli::run() {
        CliMessage::Error(format!("{:#}", err)).print(false);
        std::process::exit(1);
    }
}
