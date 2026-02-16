//! Markdown-to-Typst conversion.
//!
//! Converts markdown content into Typst markup using `pulldown-cmark` for parsing.
//! Block modifiers (HTML comments) are resolved during event processing.
//! Inline modifiers are applied as text substitutions in postprocessing.
//!
//! # Conversion pipeline
//! ```text
//! markdown input
//!   -> pulldown-cmark event stream
//!   -> Typst markup builder (handles block modifiers inline)
//!   -> raw Typst string
//!   -> postprocess inline modifiers (text substitution)
//!   -> final Typst string
//! ```

use crate::domain::ConversionContext;

/// Convert markdown text to Typst markup, applying modifiers from the context.
///
/// This is the main entry point for conversion. It:
/// 1. Parses the markdown with pulldown-cmark
/// 2. Walks the event stream, emitting Typst markup
/// 3. Resolves block modifiers (HTML comments) during event processing
/// 4. Applies inline modifier substitutions as a postprocessing pass
///
/// Returns the complete Typst markup string.
pub fn markdown_to_typst(_markdown: &str, _context: &ConversionContext) -> anyhow::Result<String> {
    todo!("Parse markdown with pulldown-cmark, emit Typst, apply modifiers")
}

/// Process pulldown-cmark events and emit Typst markup.
///
/// Maps each markdown element to its Typst equivalent per the mapping table
/// in CLAUDE.md. Handles:
/// - Headings (= level prefix)
/// - Bold/italic/strikethrough
/// - Links, images
/// - Lists (unordered and ordered)
/// - Code spans and code blocks
/// - Block quotes
/// - Horizontal rules
/// - HTML blocks (block modifier resolution)
/// - Inline HTML (e.g., <br> -> #v(1em))
fn events_to_typst(_markdown: &str, _context: &ConversionContext) -> String {
    todo!("Walk pulldown-cmark events, emit Typst markup, resolve block modifiers")
}

/// Apply inline modifier substitutions to converted Typst text.
///
/// Searches for inline modifier markers (e.g., ` /| `) in the text
/// and replaces them with their effective Typst output.
/// Markers whose effective output is `None` (on_ignore = keep) are left as-is.
fn postprocess_inline_modifiers(_typst: &str, _context: &ConversionContext) -> String {
    todo!("Find and replace inline modifier markers with their effective Typst output")
}
