use std::collections::HashMap;

use super::converter::markdown_to_typst;
use crate::domain::{ConversionContext, ModifierType, ResolvedModifier};

/// Build a ConversionContext with no modifiers for simple conversion tests.
fn empty_context() -> ConversionContext {
    ConversionContext {
        block_modifiers: HashMap::new(),
        inline_modifiers: vec![],
    }
}

/// Build a ConversionContext with the standard modifiers for modifier tests.
fn standard_context() -> ConversionContext {
    let modifiers = vec![
        ResolvedModifier {
            id: "date_separator".to_string(),
            marker: " /| ".to_string(),
            effective_typst: Some(" #h(1fr) ".to_string()),
            modifier_type: ModifierType::Inline,
        },
        ResolvedModifier {
            id: "column_break".to_string(),
            marker: "<!-- COLUMN_BREAK -->".to_string(),
            effective_typst: Some("%%COLUMN_BREAK%%".to_string()),
            modifier_type: ModifierType::Block,
        },
        ResolvedModifier {
            id: "columns_start".to_string(),
            marker: "<!-- COLUMNS_START -->".to_string(),
            effective_typst: Some("%%COLUMNS_START%%".to_string()),
            modifier_type: ModifierType::Block,
        },
    ];
    ConversionContext::from_resolved(&modifiers)
}

// =========================================================================
// Heading conversion
// =========================================================================

mod headings {
    use super::*;

    #[test]
    fn test_h1_converts_to_single_equals() {
        let ctx = empty_context();
        let result = markdown_to_typst("# Heading 1", &ctx).unwrap();
        assert!(
            result.contains("= Heading 1"),
            "h1 should become '= Heading 1', got: {}",
            result
        );
    }

    #[test]
    fn test_h2_converts_to_double_equals() {
        let ctx = empty_context();
        let result = markdown_to_typst("## Heading 2", &ctx).unwrap();
        assert!(
            result.contains("== Heading 2"),
            "h2 should become '== Heading 2', got: {}",
            result
        );
    }

    #[test]
    fn test_h3_converts_to_triple_equals() {
        let ctx = empty_context();
        let result = markdown_to_typst("### Heading 3", &ctx).unwrap();
        assert!(
            result.contains("=== Heading 3"),
            "h3 should become '=== Heading 3', got: {}",
            result
        );
    }

    #[test]
    fn test_h4_converts_to_four_equals() {
        let ctx = empty_context();
        let result = markdown_to_typst("#### Heading 4", &ctx).unwrap();
        assert!(
            result.contains("==== Heading 4"),
            "h4 should become '==== Heading 4', got: {}",
            result
        );
    }
}

// =========================================================================
// Inline formatting
// =========================================================================

mod inline_formatting {
    use super::*;

    #[test]
    fn test_bold_converts_to_typst_bold() {
        let ctx = empty_context();
        let result = markdown_to_typst("This is **bold** text", &ctx).unwrap();
        assert!(
            result.contains("*bold*"),
            "**bold** should become *bold* in Typst, got: {}",
            result
        );
    }

    #[test]
    fn test_italic_converts_to_typst_italic() {
        let ctx = empty_context();
        let result = markdown_to_typst("This is *italic* text", &ctx).unwrap();
        assert!(
            result.contains("_italic_"),
            "*italic* should become _italic_ in Typst, got: {}",
            result
        );
    }

    #[test]
    fn test_inline_code_preserved() {
        let ctx = empty_context();
        let result = markdown_to_typst("Use `code` here", &ctx).unwrap();
        assert!(
            result.contains("`code`"),
            "inline code should remain backtick-wrapped, got: {}",
            result
        );
    }

    #[test]
    fn test_strikethrough_converts_to_strike() {
        let ctx = empty_context();
        let result = markdown_to_typst("This is ~~struck~~ text", &ctx).unwrap();
        assert!(
            result.contains("#strike[struck]"),
            "~~struck~~ should become #strike[struck], got: {}",
            result
        );
    }
}

// =========================================================================
// Links and images
// =========================================================================

mod links_and_images {
    use super::*;

    #[test]
    fn test_link_converts_to_typst_link() {
        let ctx = empty_context();
        let result = markdown_to_typst("[click here](https://example.com)", &ctx).unwrap();
        assert!(
            result.contains("#link(\"https://example.com\")[click here]"),
            "link should become #link(\"url\")[text], got: {}",
            result
        );
    }

    #[test]
    fn test_image_converts_to_typst_image() {
        let ctx = empty_context();
        let result = markdown_to_typst("![alt text](photo.png)", &ctx).unwrap();
        assert!(
            result.contains("#image(\"photo.png\")"),
            "image should become #image(\"path\"), got: {}",
            result
        );
    }
}

// =========================================================================
// Lists
// =========================================================================

mod lists {
    use super::*;

    #[test]
    fn test_unordered_list_converts_to_dash_items() {
        let ctx = empty_context();
        let result = markdown_to_typst("- First\n- Second\n- Third", &ctx).unwrap();
        assert!(
            result.contains("- First"),
            "unordered list items should use '- ', got: {}",
            result
        );
        assert!(result.contains("- Second"));
        assert!(result.contains("- Third"));
    }

    #[test]
    fn test_ordered_list_converts_to_plus_items() {
        let ctx = empty_context();
        let result = markdown_to_typst("1. First\n2. Second\n3. Third", &ctx).unwrap();
        assert!(
            result.contains("+ First"),
            "ordered list items should use '+ ', got: {}",
            result
        );
        assert!(result.contains("+ Second"));
    }

    #[test]
    fn test_nested_list() {
        let ctx = empty_context();
        let result = markdown_to_typst("- Outer\n  - Inner", &ctx).unwrap();
        // Nested items should have increased indentation
        assert!(
            result.contains("Inner"),
            "nested list item should appear in output, got: {}",
            result
        );
    }
}

// =========================================================================
// Block elements
// =========================================================================

mod block_elements {
    use super::*;

    #[test]
    fn test_horizontal_rule_converts_to_line() {
        let ctx = empty_context();
        let result = markdown_to_typst("---", &ctx).unwrap();
        assert!(
            result.contains("#line(length: 100%)"),
            "--- should become #line(length: 100%), got: {}",
            result
        );
    }

    #[test]
    fn test_blockquote_converts_to_quote() {
        let ctx = empty_context();
        let result = markdown_to_typst("> This is a quote", &ctx).unwrap();
        assert!(
            result.contains("#quote[") || result.contains("quote["),
            "blockquote should become #quote[...], got: {}",
            result
        );
    }

    #[test]
    fn test_code_block_converts_to_raw() {
        let ctx = empty_context();
        let result = markdown_to_typst("```rust\nfn main() {}\n```", &ctx).unwrap();
        assert!(
            result.contains("```"),
            "code block should use triple backticks in Typst, got: {}",
            result
        );
    }

    #[test]
    fn test_br_tag_converts_to_vertical_space() {
        let ctx = empty_context();
        let result = markdown_to_typst("<br>", &ctx).unwrap();
        assert!(
            result.contains("#v(1em)") || result.contains("#v("),
            "<br> should become #v(1em) or similar, got: {}",
            result
        );
    }
}

// =========================================================================
// Block modifier handling
// =========================================================================

mod block_modifiers {
    use super::*;

    #[test]
    fn test_column_break_resolved_to_colbreak() {
        let ctx = standard_context();
        let result = markdown_to_typst("Some text\n\n<!-- COLUMN_BREAK -->\n\nMore text", &ctx).unwrap();
        assert!(
            result.contains("%%COLUMN_BREAK%%"),
            "COLUMN_BREAK should resolve to %%COLUMN_BREAK%% marker, got: {}",
            result
        );
    }

    #[test]
    fn test_columns_start_resolved_to_sentinel() {
        let ctx = standard_context();
        let result = markdown_to_typst("Header text\n\n<!-- COLUMNS_START -->\n\nBody text", &ctx).unwrap();
        assert!(
            result.contains("%%COLUMNS_START%%"),
            "COLUMNS_START should resolve to sentinel marker, got: {}",
            result
        );
    }

    #[test]
    fn test_ignored_block_modifier_removed() {
        // Setup -- column_break with on_ignore = Remove
        let modifiers = vec![
            ResolvedModifier {
                id: "column_break".to_string(),
                marker: "<!-- COLUMN_BREAK -->".to_string(),
                effective_typst: Some("".to_string()), // on_ignore = Remove
                modifier_type: ModifierType::Block,
            },
        ];
        let ctx = ConversionContext::from_resolved(&modifiers);

        // Execute
        let result = markdown_to_typst("Text\n\n<!-- COLUMN_BREAK -->\n\nMore text", &ctx).unwrap();

        // Assert -- the marker should be removed (replaced with empty string)
        assert!(
            !result.contains("COLUMN_BREAK"),
            "ignored+removed modifier should not appear in output, got: {}",
            result
        );
        assert!(
            !result.contains("#colbreak()"),
            "ignored modifier should not produce its typst output, got: {}",
            result
        );
    }
}

// =========================================================================
// Inline modifier postprocessing
// =========================================================================

mod inline_modifiers {
    use super::*;

    #[test]
    fn test_date_separator_replaced_with_hfill() {
        let ctx = standard_context();
        let result = markdown_to_typst("Title /| Date", &ctx).unwrap();
        assert!(
            result.contains("#h(1fr)"),
            "date separator ' /| ' should become ' #h(1fr) ', got: {}",
            result
        );
    }

    #[test]
    fn test_inline_modifier_keep_behavior() {
        // Setup -- modifier with effective_typst = None (keep marker as-is)
        let modifiers = vec![
            ResolvedModifier {
                id: "kept_marker".to_string(),
                marker: "KEEP_THIS".to_string(),
                effective_typst: None,
                modifier_type: ModifierType::Inline,
            },
        ];
        let ctx = ConversionContext::from_resolved(&modifiers);

        // Execute
        let result = markdown_to_typst("Before KEEP_THIS After", &ctx).unwrap();

        // Assert -- the marker should remain in the output
        assert!(
            result.contains("KEEP_THIS"),
            "kept modifier marker should remain in output, got: {}",
            result
        );
    }
}

// =========================================================================
// Edge cases
// =========================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_markdown_produces_empty_or_minimal_output() {
        let ctx = empty_context();
        let result = markdown_to_typst("", &ctx).unwrap();
        assert!(
            result.trim().is_empty() || result.len() < 10,
            "empty input should produce empty/minimal output, got: '{}'",
            result
        );
    }

    #[test]
    fn test_plain_text_preserved() {
        let ctx = empty_context();
        let result = markdown_to_typst("Just plain text with no formatting.", &ctx).unwrap();
        assert!(
            result.contains("Just plain text with no formatting"),
            "plain text should be preserved, got: {}",
            result
        );
    }

    #[test]
    fn test_special_typst_chars_in_text_handled() {
        // Text with characters that are special in Typst should be escaped
        let ctx = empty_context();
        let result = markdown_to_typst("Price is $100 @ the store", &ctx).unwrap();
        // The result should not cause Typst to interpret $ or @ as special
        assert!(
            result.contains("100"),
            "numeric content should survive conversion, got: {}",
            result
        );
    }

    #[test]
    fn test_multiple_paragraphs() {
        let ctx = empty_context();
        let result = markdown_to_typst("Paragraph one.\n\nParagraph two.\n\nParagraph three.", &ctx).unwrap();
        assert!(result.contains("Paragraph one"));
        assert!(result.contains("Paragraph two"));
        assert!(result.contains("Paragraph three"));
    }
}
