//! Markdown-to-Typst conversion via pulldown-cmark events.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::domain::{escape_typst, ConversionContext};

/// Convert markdown text to Typst markup, applying modifiers from the context.
pub fn markdown_to_typst(markdown: &str, context: &ConversionContext) -> anyhow::Result<String> {
    let raw_typst = events_to_typst(markdown, context);
    let final_typst = postprocess_inline_modifiers(&raw_typst, context);
    Ok(final_typst)
}

/// Process pulldown-cmark events and emit Typst markup.
fn events_to_typst(markdown: &str, context: &ConversionContext) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(markdown, options);

    let mut output = String::new();
    let mut list_stack: Vec<Option<u64>> = vec![];
    let mut in_code_block = false;
    let mut html_block_buf: Option<String> = None;

    for event in parser {
        match event {
            // Block-level start tags
            Event::Start(Tag::Heading { level, .. }) => {
                let depth = heading_level_to_usize(level);
                for _ in 0..depth {
                    output.push('=');
                }
                output.push(' ');
            }
            Event::Start(Tag::Paragraph) => {}
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
                let indent_level = list_stack.len().saturating_sub(1);
                for _ in 0..indent_level {
                    output.push_str("  ");
                }
                match list_stack.last() {
                    Some(Some(_)) => output.push_str("+ "),
                    _ => output.push_str("- "),
                }
            }
            Event::Start(Tag::HtmlBlock) => {
                html_block_buf = Some(String::new());
            }

            // Inline start tags
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
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                output.push_str(&format!("#image(\"{}\")", dest_url));
            }

            Event::Start(Tag::Table(_)) => {}

            Event::Start(_) => {}

            // End tags
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
                if let Some(buf) = html_block_buf.take() {
                    let trimmed = buf.trim();
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
                        let lower = trimmed.to_lowercase();
                        if lower == "<br>" || lower == "<br/>" || lower == "<br />" {
                            output.push_str("#v(1em)\n");
                        }
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
            }
            Event::End(TagEnd::Image) => {}
            Event::End(_) => {}

            // Leaf events
            Event::Text(text) => {
                if let Some(ref mut buf) = html_block_buf {
                    buf.push_str(&text);
                } else if in_code_block {
                    output.push_str(&text);
                } else {
                    output.push_str(&escape_typst(&text));
                }
            }
            Event::Code(text) => {
                output.push('`');
                output.push_str(&text);
                output.push('`');
            }
            Event::Html(html) => {
                if let Some(ref mut buf) = html_block_buf {
                    buf.push_str(&html);
                } else {
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
                        let lower = trimmed.to_lowercase();
                        if lower == "<br>" || lower == "<br/>" || lower == "<br />" {
                            output.push_str("#v(1em)\n");
                        }
                    }
                }
            }
            Event::InlineHtml(html) => {
                let trimmed = html.trim().to_lowercase();
                if trimmed == "<br>" || trimmed == "<br/>" || trimmed == "<br />" {
                    output.push_str("#v(1em)");
                }
            }
            Event::SoftBreak => {
                // Forced line break — without this, Typst merges adjacent lines
                output.push_str("\\\n");
            }
            Event::HardBreak => {
                output.push_str("#v(1em)\n");
            }
            Event::Rule => {
                output.push_str("#line(length: 100%)\n\n");
            }

            Event::FootnoteReference(_)
            | Event::TaskListMarker(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => {}
        }
    }

    output
}

/// Apply inline modifier substitutions, trying both original and escaped markers.
fn postprocess_inline_modifiers(typst: &str, context: &ConversionContext) -> String {
    let mut result = typst.to_string();
    for modifier in &context.inline_modifiers {
        let escaped_marker = escape_typst(&modifier.marker);
        match &modifier.effective_typst {
            None => {
                if escaped_marker != modifier.marker {
                    result = result.replace(&escaped_marker, &modifier.marker);
                }
            }
            Some(replacement) => {
                result = result.replace(&modifier.marker, replacement);
                if escaped_marker != modifier.marker {
                    result = result.replace(&escaped_marker, replacement);
                }
            }
        }
    }
    result
}

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
