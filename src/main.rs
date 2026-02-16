//! md-docs: Convert Markdown files into professional PDFs using Typst templates.
//!
//! Entry point. Delegates immediately to `cli::run()`.
//! The `cli` module is binary-only; `app` and `domain` come from the library crate.

mod cli;

fn main() -> anyhow::Result<()> {
    cli::run()
}
