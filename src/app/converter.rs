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

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::domain::{escape_typst, ConversionContext};

/// Convert markdown text to Typst markup, applying modifiers from the context.
///
/// This is the main entry point for conversion. It:
/// 1. Parses the markdown with pulldown-cmark
/// 2. Walks the event stream, emitting Typst markup
/// 3. Resolves block modifiers (HTML comments) during event processing
/// 4. Applies inline modifier substitutions as a postprocessing pass
///
/// Returns the complete Typst markup string.
pub fn markdown_to_typst(markdown: &str, context: &ConversionContext) -> anyhow::Result<String> {
    let raw_typst = events_to_typst(markdown, context);
    let final_typst = postprocess_inline_modifiers(&raw_typst, context);
    Ok(final_typst)
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
fn events_to_typst(markdown: &str, context: &ConversionContext) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown, options);

    let mut output = String::new();
    let mut list_stack: Vec<Option<u64>> = vec![]; // None = unordered, Some(start) = ordered
    let mut link_urls: Vec<String> = vec![]; // stack for nested links
    let mut in_code_block = false;
    // Buffer to accumulate Html events inside an HtmlBlock, so we can resolve
    // the complete block modifier marker after all lines are gathered.
    let mut html_block_buf: Option<String> = None;

    for event in parser {
        match event {
            // -----------------------------------------------------------------
            // Block-level start tags
            // -----------------------------------------------------------------
            Event::Start(Tag::Heading { level, .. }) => {
                let depth = heading_level_to_usize(level);
                for _ in 0..depth {
                    output.push('=');
                }
                output.push(' ');
            }
            Event::Start(Tag::Paragraph) => {
                // Nothing on paragraph open
            }
            Event::Start(Tag::BlockQuote(..)) => {
                output.push_str("#quote[");
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let lang_str = lang.as_ref();
                        output.push_str("```");
                        if !lang_str.is_empty() {
                            output.push_str(lang_str);
                        }
                        output.push('\n');
                    }
                    CodeBlockKind::Indented => {
                        output.push_str("```\n");
                    }
                }
            }
            Event::Start(Tag::List(start)) => {
                list_stack.push(start);
            }
            Event::Start(Tag::Item) => {
                // Emit indentation based on nesting depth (depth - 1 because
                // the current list is already on the stack)
                let indent_level = list_stack.len().saturating_sub(1);
                for _ in 0..indent_level {
                    output.push_str("  ");
                }
                // Check the top of the stack to determine ordered vs unordered
                match list_stack.last() {
                    Some(Some(_)) => output.push_str("+ "),
                    _ => output.push_str("- "),
                }
            }
            Event::Start(Tag::HtmlBlock) => {
                // Begin accumulating HTML block content
                html_block_buf = Some(String::new());
            }

            // -----------------------------------------------------------------
            // Inline start tags
            // -----------------------------------------------------------------
            Event::Start(Tag::Emphasis) => {
                output.push('_');
            }
            Event::Start(Tag::Strong) => {
                output.push('*');
            }
            Event::Start(Tag::Strikethrough) => {
                output.push_str("#strike[");
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                output.push_str(&format!("#link(\"{}\")[", dest_url));
                link_urls.push(dest_url.to_string());
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                output.push_str(&format!("#image(\"{}\")", dest_url));
            }

            // Tables: stub for now
            Event::Start(Tag::Table(_)) => {
                // TODO: table support
                output.push_str("// TODO: table support\n");
            }

            // Catch-all for other start tags we don't handle
            Event::Start(_) => {}

            // -----------------------------------------------------------------
            // End tags
            // -----------------------------------------------------------------
            Event::End(TagEnd::Heading(_)) => {
                output.push_str("\n\n");
            }
            Event::End(TagEnd::Paragraph) => {
                output.push_str("\n\n");
            }
            Event::End(TagEnd::BlockQuote(..)) => {
                output.push_str("]\n\n");
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                output.push_str("```\n\n");
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                output.push('\n');
            }
            Event::End(TagEnd::Item) => {
                output.push('\n');
            }
            Event::End(TagEnd::HtmlBlock) => {
                // Process the accumulated HTML block content
                if let Some(buf) = html_block_buf.take() {
                    let trimmed = buf.trim();
                    if let Some(effective) = context.block_modifiers.get(trimmed) {
                        match effective {
                            Some(replacement) if !replacement.is_empty() => {
                                output.push_str(replacement);
                                output.push_str("\n\n");
                            }
                            Some(_) => {
                                // empty string = on_ignore Remove, drop silently
                            }
                            None => {
                                // on_ignore Keep — leave raw marker
                                output.push_str(trimmed);
                                output.push('\n');
                            }
                        }
                    } else {
                        // Check for <br> tags in HTML blocks
                        let lower = trimmed.to_lowercase();
                        if lower == "<br>" || lower == "<br/>" || lower == "<br />" {
                            output.push_str("#v(1em)\n");
                        }
                        // Other unknown HTML: silently drop
                    }
                }
            }
            Event::End(TagEnd::Emphasis) => {
                output.push('_');
            }
            Event::End(TagEnd::Strong) => {
                output.push('*');
            }
            Event::End(TagEnd::Strikethrough) => {
                output.push(']');
            }
            Event::End(TagEnd::Link) => {
                output.push(']');
                link_urls.pop();
            }
            Event::End(TagEnd::Image) => {
                // Image is self-closing in our output, nothing extra needed
            }

            // Catch-all for other end tags
            Event::End(_) => {}

            // -----------------------------------------------------------------
            // Leaf events
            // -----------------------------------------------------------------
            Event::Text(text) => {
                if html_block_buf.is_some() {
                    // Accumulate text inside an HTML block (shouldn't normally happen,
                    // but handle gracefully)
                    if let Some(ref mut buf) = html_block_buf {
                        buf.push_str(&text);
                    }
                } else if in_code_block {
                    // Do NOT escape text inside code blocks
                    output.push_str(&text);
                } else {
                    output.push_str(&escape_typst(&text));
                }
            }
            Event::Code(text) => {
                // Inline code: no escaping inside code spans
                output.push('`');
                output.push_str(&text);
                output.push('`');
            }
            Event::Html(html) => {
                if let Some(ref mut buf) = html_block_buf {
                    // Inside an HtmlBlock: accumulate lines
                    buf.push_str(&html);
                } else {
                    // Standalone Html event outside HtmlBlock — resolve as block modifier
                    let trimmed = html.trim();
                    if let Some(effective) = context.block_modifiers.get(trimmed) {
                        match effective {
                            Some(replacement) if !replacement.is_empty() => {
                                output.push_str(replacement);
                                output.push_str("\n\n");
                            }
                            Some(_) => {}
                            None => {
                                output.push_str(trimmed);
                                output.push('\n');
                            }
                        }
                    } else {
                        // Check for <br> tags
                        let lower = trimmed.to_lowercase();
                        if lower == "<br>" || lower == "<br/>" || lower == "<br />" {
                            output.push_str("#v(1em)\n");
                        }
                        // Other unknown HTML: silently drop
                    }
                }
            }
            Event::InlineHtml(html) => {
                let trimmed = html.trim().to_lowercase();
                if trimmed == "<br>" || trimmed == "<br/>" || trimmed == "<br />" {
                    output.push_str("#v(1em)");
                }
                // Other inline HTML is silently dropped
            }
            Event::SoftBreak => {
                // Emit a Typst forced line break (\) so that source line breaks
                // are preserved in the output. Without this, Typst treats \n as
                // a space and adjacent lines merge (e.g., job title + description).
                output.push_str("\\\n");
            }
            Event::HardBreak => {
                output.push_str("#v(1em)\n");
            }
            Event::Rule => {
                output.push_str("#line(length: 100%)\n\n");
            }

            // Events we don't handle
            Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => {}
        }
    }

    output
}

/// Apply inline modifier substitutions to converted Typst text.
///
/// Searches for inline modifier markers (e.g., ` /| `) in the text
/// and replaces them with their effective Typst output.
/// Markers whose effective output is `None` (on_ignore = keep) are left as-is.
///
/// Because `escape_typst()` runs on all text before this postprocessing step,
/// marker characters may have been escaped (e.g., ` /| ` becomes ` \/| `).
/// This function tries both the original marker and its escaped form.
fn postprocess_inline_modifiers(typst: &str, context: &ConversionContext) -> String {
    let mut result = typst.to_string();
    for modifier in &context.inline_modifiers {
        let escaped_marker = escape_typst(&modifier.marker);
        match &modifier.effective_typst {
            None => {
                // on_ignore = keep: if the marker was escaped, restore the original marker
                if escaped_marker != modifier.marker {
                    result = result.replace(&escaped_marker, &modifier.marker);
                }
            }
            Some(replacement) => {
                // Try the original marker first, then the escaped version
                result = result.replace(&modifier.marker, replacement);
                if escaped_marker != modifier.marker {
                    result = result.replace(&escaped_marker, replacement);
                }
            }
        }
    }
    result
}

/// Convert a `HeadingLevel` enum to its numeric depth.
fn heading_level_to_usize(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
