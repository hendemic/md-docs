use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::*;

// =========================================================================
// Metadata::parse_from_content
// =========================================================================

mod metadata_parsing {
    use super::*;

    #[test]
    fn test_parse_from_content_valid_frontmatter_returns_metadata_and_body() {
        // Setup
        let content = "---\ntitle: My Resume\nauthor: Jane Doe\ndate: January 2026\n---\n# Hello World\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_ok());
        let (metadata, body) = result.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("My Resume"));
        assert_eq!(metadata.author.as_deref(), Some("Jane Doe"));
        assert_eq!(metadata.date.as_deref(), Some("January 2026"));
        assert!(body.contains("# Hello World"));
    }

    #[test]
    fn test_parse_from_content_no_frontmatter_returns_empty_metadata() {
        // Setup
        let content = "# Just a heading\nSome text here.\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_ok());
        let (metadata, body) = result.unwrap();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.date.is_none());
        assert!(body.contains("# Just a heading"));
    }

    #[test]
    fn test_parse_from_content_empty_string_returns_empty_metadata() {
        // Setup
        let content = "";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_ok());
        let (metadata, body) = result.unwrap();
        assert!(metadata.title.is_none());
        assert!(metadata.author.is_none());
        assert!(metadata.date.is_none());
        assert!(body.is_empty());
    }

    #[test]
    fn test_parse_from_content_frontmatter_only_no_body() {
        // Setup
        let content = "---\ntitle: Test\n---\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_ok());
        let (metadata, body) = result.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("Test"));
        assert!(body.trim().is_empty());
    }

    #[test]
    fn test_parse_from_content_extra_fields_preserved() {
        // Setup
        let content = "---\ntitle: Resume\nemail: test@test.com\nphone: 555-1234\n---\nBody text\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_ok());
        let (metadata, _body) = result.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("Resume"));
        assert!(metadata.extra.contains_key("email"));
        assert!(metadata.extra.contains_key("phone"));
    }

    #[test]
    fn test_parse_from_content_extra_fields_list_value() {
        // Setup -- YAML list value should be stored as serde_yml::Value
        let content = "---\ntitle: Test\nskills:\n  - rust\n  - python\n---\nBody\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_ok());
        let (metadata, _body) = result.unwrap();
        assert!(metadata.extra.contains_key("skills"));
    }

    #[test]
    fn test_parse_from_content_extra_fields_numeric_value() {
        // Setup
        let content = "---\ntitle: Test\nyear: 2026\n---\nBody\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_ok());
        let (metadata, _body) = result.unwrap();
        assert!(metadata.extra.contains_key("year"));
    }

    #[test]
    fn test_parse_from_content_malformed_yaml_returns_error() {
        // Setup -- invalid YAML between delimiters
        let content = "---\ntitle: [unclosed bracket\n---\nBody\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, MdDocsError::InvalidFrontmatter(_)));
    }

    #[test]
    fn test_parse_from_content_partial_frontmatter_delimiter() {
        // Setup -- only one `---` delimiter, not a complete frontmatter block
        let content = "---\nThis is just text with a horizontal rule above.\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert -- should either treat as no frontmatter or return the text
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_from_content_title_only() {
        // Setup
        let content = "---\ntitle: Solo Title\n---\nContent here.\n";

        // Execute
        let result = Metadata::parse_from_content(content);

        // Assert
        assert!(result.is_ok());
        let (metadata, _body) = result.unwrap();
        assert_eq!(metadata.title.as_deref(), Some("Solo Title"));
        assert!(metadata.author.is_none());
        assert!(metadata.date.is_none());
    }
}

// =========================================================================
// Document::new
// =========================================================================

mod document_construction {
    use super::*;

    #[test]
    fn test_new_creates_document_with_empty_sections() {
        // Setup
        let metadata = Metadata {
            title: Some("Test".to_string()),
            author: None,
            date: None,
            extra: HashMap::new(),
        };

        // Execute
        let doc = Document::new(metadata.clone(), "# Hello".to_string());

        // Assert
        assert_eq!(doc.metadata.title.as_deref(), Some("Test"));
        assert_eq!(doc.raw_body, "# Hello");
        // Sections should be default/empty before conversion
        assert!(doc.sections.header.is_empty());
        assert!(doc.sections.body.is_empty());
        assert!(doc.sections.content.is_empty());
    }
}

// =========================================================================
// ContentSections::from_typst_content
// =========================================================================

mod content_sections {
    use super::*;

    #[test]
    fn test_from_typst_content_with_split_marker() {
        // Setup
        let typst_content = "= Title\nSome header text\n%%COLUMNS_START%%\n== Section\nBody text\n";
        let marker = "%%COLUMNS_START%%";

        // Execute
        let sections = ContentSections::from_typst_content(typst_content, marker);

        // Assert
        assert!(!sections.header.is_empty(), "header should contain content above the marker");
        assert!(sections.header.contains("Title"));
        assert!(!sections.body.is_empty(), "body should contain content below the marker");
        assert!(sections.body.contains("Body text"));
        assert_eq!(sections.content, typst_content);
    }

    #[test]
    fn test_from_typst_content_without_split_marker() {
        // Setup
        let typst_content = "= Title\nSome text\n== Section\nMore text\n";
        let marker = "%%COLUMNS_START%%";

        // Execute
        let sections = ContentSections::from_typst_content(typst_content, marker);

        // Assert
        assert!(sections.header.is_empty(), "header should be empty when no marker");
        assert!(!sections.body.is_empty(), "body should contain all content");
        assert!(sections.body.contains("Title"));
        assert!(sections.body.contains("More text"));
        assert_eq!(sections.content, typst_content);
    }

    #[test]
    fn test_from_typst_content_marker_at_start() {
        // Setup
        let typst_content = "%%COLUMNS_START%%\n== Body only\n";
        let marker = "%%COLUMNS_START%%";

        // Execute
        let sections = ContentSections::from_typst_content(typst_content, marker);

        // Assert
        assert!(sections.header.trim().is_empty(), "header should be empty/whitespace when marker is at start");
        assert!(sections.body.contains("Body only"));
    }

    #[test]
    fn test_from_typst_content_marker_at_end() {
        // Setup
        let typst_content = "= All header\n%%COLUMNS_START%%";
        let marker = "%%COLUMNS_START%%";

        // Execute
        let sections = ContentSections::from_typst_content(typst_content, marker);

        // Assert
        assert!(sections.header.contains("All header"));
        assert!(sections.body.trim().is_empty(), "body should be empty when marker is at end");
    }

    #[test]
    fn test_from_typst_content_empty_input() {
        // Setup
        let typst_content = "";
        let marker = "%%COLUMNS_START%%";

        // Execute
        let sections = ContentSections::from_typst_content(typst_content, marker);

        // Assert
        assert!(sections.header.is_empty());
        assert!(sections.body.is_empty());
        assert!(sections.content.is_empty());
    }
}

// =========================================================================
// escape_typst
// =========================================================================

mod typst_escaping {
    use super::*;

    #[test]
    fn test_escape_typst_plain_text_unchanged() {
        // Setup
        let text = "Hello world";

        // Execute
        let result = escape_typst(text);

        // Assert
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_escape_typst_asterisk_escaped() {
        // Execute
        let result = escape_typst("**bold**");

        // Assert
        assert!(result.contains("\\*"));
        assert!(!result.contains("**"));
    }

    #[test]
    fn test_escape_typst_underscore_escaped() {
        // Execute
        let result = escape_typst("_italic_");

        // Assert
        assert!(result.contains("\\_"));
    }

    #[test]
    fn test_escape_typst_hash_escaped() {
        // Execute
        let result = escape_typst("#heading");

        // Assert
        assert!(result.contains("\\#"));
    }

    #[test]
    fn test_escape_typst_at_sign_escaped() {
        // Execute
        let result = escape_typst("email@test.com");

        // Assert
        assert!(result.contains("\\@"));
    }

    #[test]
    fn test_escape_typst_dollar_escaped() {
        // Execute
        let result = escape_typst("$100");

        // Assert
        assert!(result.contains("\\$"));
    }

    #[test]
    fn test_escape_typst_angle_bracket_escaped() {
        // Execute
        let result = escape_typst("<br>");

        // Assert
        assert!(result.contains("\\<"));
    }

    #[test]
    fn test_escape_typst_backtick_escaped() {
        // Execute
        let result = escape_typst("`code`");

        // Assert
        assert!(result.contains("\\`"));
    }

    #[test]
    fn test_escape_typst_tilde_escaped() {
        // Execute
        let result = escape_typst("~strike~");

        // Assert
        assert!(result.contains("\\~"));
    }

    #[test]
    fn test_escape_typst_backslash_escaped() {
        // Execute
        let result = escape_typst("path\\to\\file");

        // Assert
        assert!(result.contains("\\\\"));
    }

    #[test]
    fn test_escape_typst_empty_string() {
        // Execute
        let result = escape_typst("");

        // Assert
        assert_eq!(result, "");
    }

    #[test]
    fn test_escape_typst_multiple_special_chars() {
        // Execute
        let result = escape_typst("*bold* and _italic_ with #hash");

        // Assert
        assert!(result.contains("\\*"));
        assert!(result.contains("\\_"));
        assert!(result.contains("\\#"));
    }

    #[test]
    fn test_escape_typst_equals_sign_escaped() {
        // Execute
        let result = escape_typst("= heading");

        // Assert
        assert!(result.contains("\\="));
    }

    #[test]
    fn test_escape_typst_plus_and_minus_escaped() {
        // Execute
        let result_plus = escape_typst("+ item");
        let result_minus = escape_typst("- item");

        // Assert
        assert!(result_plus.contains("\\+"));
        assert!(result_minus.contains("\\-"));
    }

    #[test]
    fn test_escape_typst_slash_escaped() {
        // Execute
        let result = escape_typst("a/b");

        // Assert
        assert!(result.contains("\\/"));
    }
}

// =========================================================================
// resolve_modifiers
// =========================================================================

mod modifier_resolution {
    use super::*;

    fn sample_registry() -> ModifierRegistry {
        let mut registry = HashMap::new();
        registry.insert(
            "date_separator".to_string(),
            ModifierDef {
                marker: " /| ".to_string(),
                description: "Inline date separator".to_string(),
                typst: " #h(1fr) ".to_string(),
                on_ignore: OnIgnore::Newline,
                modifier_type: ModifierType::Inline,
            },
        );
        registry.insert(
            "column_break".to_string(),
            ModifierDef {
                marker: "<!-- COLUMN_BREAK -->".to_string(),
                description: "Column break".to_string(),
                typst: "#colbreak()".to_string(),
                on_ignore: OnIgnore::Remove,
                modifier_type: ModifierType::Block,
            },
        );
        registry.insert(
            "bottom_spacer".to_string(),
            ModifierDef {
                marker: "<!-- BOTTOM -->".to_string(),
                description: "Push to bottom".to_string(),
                typst: "#v(1fr)".to_string(),
                on_ignore: OnIgnore::Remove,
                modifier_type: ModifierType::Block,
            },
        );
        registry
    }

    #[test]
    fn test_resolve_modifiers_no_ignore_list_all_use_normal_typst() {
        // Setup
        let registry = sample_registry();
        let ignore_list: Vec<String> = vec![];

        // Execute
        let resolved = resolve_modifiers(&registry, &ignore_list);

        // Assert
        assert_eq!(resolved.len(), 3);
        for rm in &resolved {
            assert!(
                rm.effective_typst.is_some(),
                "all non-ignored modifiers should have Some effective_typst"
            );
        }
    }

    #[test]
    fn test_resolve_modifiers_ignore_remove_gives_empty_string() {
        // Setup
        let registry = sample_registry();
        let ignore_list = vec!["column_break".to_string()];

        // Execute
        let resolved = resolve_modifiers(&registry, &ignore_list);

        // Assert
        let column_break = resolved.iter().find(|r| r.id == "column_break").unwrap();
        assert_eq!(column_break.effective_typst.as_deref(), Some(""));
    }

    #[test]
    fn test_resolve_modifiers_ignore_newline_gives_linebreak() {
        // Setup
        let registry = sample_registry();
        let ignore_list = vec!["date_separator".to_string()];

        // Execute
        let resolved = resolve_modifiers(&registry, &ignore_list);

        // Assert
        let date_sep = resolved.iter().find(|r| r.id == "date_separator").unwrap();
        let effective = date_sep.effective_typst.as_deref().unwrap();
        assert!(
            effective.contains('\n') || effective.contains("linebreak") || effective.contains("#v("),
            "newline on_ignore should produce some kind of line break: got '{}'",
            effective
        );
    }

    #[test]
    fn test_resolve_modifiers_ignore_keep_gives_none() {
        // Setup -- we need a modifier with on_ignore = Keep
        let mut registry = HashMap::new();
        registry.insert(
            "keep_mod".to_string(),
            ModifierDef {
                marker: "KEEP_ME".to_string(),
                description: "A kept modifier".to_string(),
                typst: "replaced".to_string(),
                on_ignore: OnIgnore::Keep,
                modifier_type: ModifierType::Inline,
            },
        );
        let ignore_list = vec!["keep_mod".to_string()];

        // Execute
        let resolved = resolve_modifiers(&registry, &ignore_list);

        // Assert
        let keep_mod = resolved.iter().find(|r| r.id == "keep_mod").unwrap();
        assert!(
            keep_mod.effective_typst.is_none(),
            "on_ignore = Keep should produce effective_typst = None (leave marker as-is)"
        );
    }

    #[test]
    fn test_resolve_modifiers_non_ignored_uses_normal_typst() {
        // Setup
        let registry = sample_registry();
        let ignore_list = vec!["column_break".to_string()]; // only ignore column_break

        // Execute
        let resolved = resolve_modifiers(&registry, &ignore_list);

        // Assert
        let date_sep = resolved.iter().find(|r| r.id == "date_separator").unwrap();
        assert_eq!(
            date_sep.effective_typst.as_deref(),
            Some(" #h(1fr) "),
            "non-ignored modifier should use its normal typst output"
        );
    }

    #[test]
    fn test_resolve_modifiers_preserves_modifier_type() {
        // Setup
        let registry = sample_registry();
        let ignore_list: Vec<String> = vec![];

        // Execute
        let resolved = resolve_modifiers(&registry, &ignore_list);

        // Assert
        let date_sep = resolved.iter().find(|r| r.id == "date_separator").unwrap();
        assert_eq!(date_sep.modifier_type, ModifierType::Inline);

        let col_break = resolved.iter().find(|r| r.id == "column_break").unwrap();
        assert_eq!(col_break.modifier_type, ModifierType::Block);
    }

    #[test]
    fn test_resolve_modifiers_empty_registry() {
        // Setup
        let registry = HashMap::new();
        let ignore_list: Vec<String> = vec![];

        // Execute
        let resolved = resolve_modifiers(&registry, &ignore_list);

        // Assert
        assert!(resolved.is_empty());
    }
}

// =========================================================================
// ConversionContext::from_resolved
// =========================================================================

mod conversion_context {
    use super::*;

    #[test]
    fn test_from_resolved_partitions_block_and_inline() {
        // Setup
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
                effective_typst: Some("#colbreak()".to_string()),
                modifier_type: ModifierType::Block,
            },
        ];

        // Execute
        let ctx = ConversionContext::from_resolved(&modifiers);

        // Assert
        assert_eq!(ctx.block_modifiers.len(), 1, "should have one block modifier");
        assert!(
            ctx.block_modifiers.contains_key("<!-- COLUMN_BREAK -->"),
            "block_modifiers should be keyed by marker"
        );
        assert_eq!(ctx.inline_modifiers.len(), 1, "should have one inline modifier");
        assert_eq!(ctx.inline_modifiers[0].id, "date_separator");
    }

    #[test]
    fn test_from_resolved_empty_list() {
        // Setup
        let modifiers: Vec<ResolvedModifier> = vec![];

        // Execute
        let ctx = ConversionContext::from_resolved(&modifiers);

        // Assert
        assert!(ctx.block_modifiers.is_empty());
        assert!(ctx.inline_modifiers.is_empty());
    }

    #[test]
    fn test_from_resolved_all_inline() {
        // Setup
        let modifiers = vec![
            ResolvedModifier {
                id: "mod1".to_string(),
                marker: "M1".to_string(),
                effective_typst: Some("T1".to_string()),
                modifier_type: ModifierType::Inline,
            },
            ResolvedModifier {
                id: "mod2".to_string(),
                marker: "M2".to_string(),
                effective_typst: Some("T2".to_string()),
                modifier_type: ModifierType::Inline,
            },
        ];

        // Execute
        let ctx = ConversionContext::from_resolved(&modifiers);

        // Assert
        assert!(ctx.block_modifiers.is_empty());
        assert_eq!(ctx.inline_modifiers.len(), 2);
    }

    #[test]
    fn test_from_resolved_all_block() {
        // Setup
        let modifiers = vec![
            ResolvedModifier {
                id: "mod1".to_string(),
                marker: "<!-- M1 -->".to_string(),
                effective_typst: Some("T1".to_string()),
                modifier_type: ModifierType::Block,
            },
        ];

        // Execute
        let ctx = ConversionContext::from_resolved(&modifiers);

        // Assert
        assert_eq!(ctx.block_modifiers.len(), 1);
        assert!(ctx.inline_modifiers.is_empty());
    }
}

// =========================================================================
// RepoSource deserialization
// =========================================================================

mod repo_source_deserialization {
    use super::*;

    #[test]
    fn test_repo_source_deserializes_from_toml() {
        // Setup
        let toml_str = r#"
[[repos]]
name = "default"
url = "https://github.com/hendemic/md-docs-templates.git"
"#;

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(config.repos().len(), 1);
        assert_eq!(config.repos()[0].name, "default");
        assert_eq!(config.repos()[0].url, "https://github.com/hendemic/md-docs-templates.git");
    }

    #[test]
    fn test_multiple_repos_deserialize() {
        // Setup
        let toml_str = r#"
[[repos]]
name = "default"
url = "https://github.com/hendemic/md-docs-templates.git"

[[repos]]
name = "custom"
url = "https://github.com/example/custom-templates.git"
"#;

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(config.repos().len(), 2);
        assert_eq!(config.repos()[0].name, "default");
        assert_eq!(config.repos()[1].name, "custom");
        assert_eq!(config.repos()[1].url, "https://github.com/example/custom-templates.git");
    }
}

// =========================================================================
// LocalSource deserialization
// =========================================================================

mod local_source_deserialization {
    use super::*;

    #[test]
    fn test_local_source_deserializes_from_toml() {
        // Setup
        let toml_str = r#"
[[local]]
path = "/home/user/my-templates"
"#;

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(config.local().len(), 1);
        assert_eq!(config.local()[0].path, PathBuf::from("/home/user/my-templates"));
    }

    #[test]
    fn test_multiple_locals_deserialize() {
        // Setup
        let toml_str = r#"
[[local]]
path = "/home/user/templates-a"

[[local]]
path = "/home/user/templates-b"
"#;

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(config.local().len(), 2);
        assert_eq!(config.local()[0].path, PathBuf::from("/home/user/templates-a"));
        assert_eq!(config.local()[1].path, PathBuf::from("/home/user/templates-b"));
    }
}

// =========================================================================
// Config new format
// =========================================================================

mod config_new_format {
    use super::*;

    #[test]
    fn test_config_with_repos_and_local() {
        // Setup
        let toml_str = r#"
default_template = "resume-2-col"
default_brand = "generic"
author = "Test Author"
output_dir = "/home/user/output"

[[repos]]
name = "default"
url = "https://github.com/hendemic/md-docs-templates.git"

[[local]]
path = "/home/user/my-templates"
"#;

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(config.default_template(), Some("resume-2-col"));
        assert_eq!(config.default_brand(), Some("generic"));
        assert_eq!(config.author(), Some("Test Author"));
        assert_eq!(config.output_dir(), Some(Path::new("/home/user/output")));
        assert_eq!(config.repos().len(), 1);
        assert_eq!(config.repos()[0].name, "default");
        assert_eq!(config.local().len(), 1);
        assert_eq!(config.local()[0].path, PathBuf::from("/home/user/my-templates"));
    }

    #[test]
    fn test_empty_config_has_empty_vecs() {
        // Setup
        let toml_str = "";

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert!(config.repos().is_empty());
        assert!(config.local().is_empty());
    }

}

// =========================================================================
// Config accessors (kept: default_template, default_brand, author, output_dir)
// =========================================================================

mod config_accessors {
    use super::*;

    #[test]
    fn test_default_template_accessor() {
        // Setup
        let toml_str = r#"default_template = "resume-2-col""#;
        let config: Config = toml::from_str(toml_str).unwrap();

        // Execute & Assert
        assert_eq!(config.default_template(), Some("resume-2-col"));
    }

    #[test]
    fn test_default_template_accessor_none_when_unset() {
        // Setup
        let config = Config::default();

        // Execute & Assert
        assert!(config.default_template().is_none());
    }

    #[test]
    fn test_author_accessor() {
        // Setup
        let toml_str = r#"author = "Jane Doe""#;
        let config: Config = toml::from_str(toml_str).unwrap();

        // Execute & Assert
        assert_eq!(config.author(), Some("Jane Doe"));
    }

    #[test]
    fn test_output_dir_accessor() {
        // Setup
        let toml_str = r#"output_dir = "/home/user/output""#;
        let config: Config = toml::from_str(toml_str).unwrap();

        // Execute & Assert
        assert_eq!(config.output_dir(), Some(Path::new("/home/user/output")));
    }

    #[test]
    fn test_repos_accessor() {
        // Setup
        let toml_str = r#"
[[repos]]
name = "test"
url = "https://example.com/test.git"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();

        // Execute & Assert
        assert_eq!(config.repos().len(), 1);
        assert_eq!(config.repos()[0].name, "test");
    }

    #[test]
    fn test_local_accessor() {
        // Setup
        let toml_str = r#"
[[local]]
path = "/tmp/templates"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();

        // Execute & Assert
        assert_eq!(config.local().len(), 1);
        assert_eq!(config.local()[0].path, PathBuf::from("/tmp/templates"));
    }
}

// =========================================================================
// Display impls (updated for source field)
// =========================================================================

mod display_impls {
    use super::*;

    #[test]
    fn test_template_display_with_description() {
        // Setup
        let template = Template {
            id: "resume-2-col".to_string(),
            path: PathBuf::from("/templates/resume-2-col"),
            metadata: TemplateMetadata {
                name: "Resume (Two Column)".to_string(),
                description: Some("Two-column resume layout".to_string()),
                default_brand: Some("generic".to_string()),
                ignore: vec![],
                starter_file: None,
            },
            source: TemplateSource::Repo("default".to_string()),
        };

        // Execute
        let display = format!("{}", template);

        // Assert
        assert!(display.contains("resume-2-col"));
        assert!(display.contains("Resume (Two Column)"));
        assert!(display.contains("Two-column resume layout"));
    }

    #[test]
    fn test_template_display_without_description() {
        // Setup
        let template = Template {
            id: "minimal".to_string(),
            path: PathBuf::from("/templates/minimal"),
            metadata: TemplateMetadata {
                name: "Minimal Template".to_string(),
                description: None,
                default_brand: None,
                ignore: vec![],
                starter_file: None,
            },
            source: TemplateSource::Local(PathBuf::from("/dev/templates")),
        };

        // Execute
        let display = format!("{}", template);

        // Assert
        assert!(display.contains("minimal"));
        assert!(display.contains("Minimal Template"));
        assert!(!display.contains("--"));
    }

    #[test]
    fn test_brand_display_with_description() {
        // Setup
        let brand = Brand {
            id: "generic".to_string(),
            path: PathBuf::from("/brands/generic"),
            metadata: BrandMetadata {
                name: "Generic".to_string(),
                description: Some("Clean defaults".to_string()),
            },
            source: TemplateSource::Repo("default".to_string()),
        };

        // Execute
        let display = format!("{}", brand);

        // Assert
        assert!(display.contains("generic"));
        assert!(display.contains("Generic"));
        assert!(display.contains("Clean defaults"));
    }

    #[test]
    fn test_brand_display_without_description() {
        // Setup
        let brand = Brand {
            id: "custom".to_string(),
            path: PathBuf::from("/brands/custom"),
            metadata: BrandMetadata {
                name: "Custom Brand".to_string(),
                description: None,
            },
            source: TemplateSource::Local(PathBuf::from("/dev/brands")),
        };

        // Execute
        let display = format!("{}", brand);

        // Assert
        assert!(display.contains("custom"));
        assert!(display.contains("Custom Brand"));
        assert!(!display.contains("--"));
    }
}

// =========================================================================
// MdDocsError Display messages
// =========================================================================

mod error_display {
    use super::*;

    #[test]
    fn test_template_not_found_error_message() {
        let err = MdDocsError::TemplateNotFound("nonexistent".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("template not found"));
        assert!(msg.contains("nonexistent"));
    }

    #[test]
    fn test_brand_not_found_error_message() {
        let err = MdDocsError::BrandNotFound("missing-brand".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("brand not found"));
        assert!(msg.contains("missing-brand"));
    }

    #[test]
    fn test_input_not_found_error_message() {
        let err = MdDocsError::InputNotFound(PathBuf::from("/no/such/file.md"));
        let msg = format!("{}", err);
        assert!(msg.contains("input file not found"));
    }

    #[test]
    fn test_invalid_frontmatter_error_message() {
        let err = MdDocsError::InvalidFrontmatter("bad yaml".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("invalid frontmatter"));
    }

    #[test]
    fn test_compilation_failed_error_message() {
        let err = MdDocsError::CompilationFailed("missing font".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("typst compilation failed"));
    }

    #[test]
    fn test_invalid_config_error_message() {
        let err = MdDocsError::InvalidConfig("bad toml".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("invalid configuration"));
    }

    #[test]
    fn test_repo_operation_failed_error_message() {
        let err = MdDocsError::RepoOperationFailed("clone failed".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("repository operation failed"));
    }

    #[test]
    fn test_user_managed_template_error_message() {
        let err = MdDocsError::UserManagedTemplate("my-template".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("cannot remove user-managed template"));
    }
}

// =========================================================================
// Deserialization of types from TOML strings
// =========================================================================

mod deserialization {
    use super::*;

    #[test]
    fn test_modifier_def_deserialization_from_toml() {
        // Setup
        let toml_str = r#"
marker = " /| "
description = "Inline left/right alignment"
typst = " #h(1fr) "
on_ignore = "newline"
type = "inline"
"#;

        // Execute
        let def: ModifierDef = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(def.marker, " /| ");
        assert_eq!(def.typst, " #h(1fr) ");
        assert_eq!(def.on_ignore, OnIgnore::Newline);
        assert_eq!(def.modifier_type, ModifierType::Inline);
    }

    #[test]
    fn test_modifier_def_block_type_deserialization() {
        // Setup
        let toml_str = r##"
marker = "<!-- COLUMN_BREAK -->"
description = "Column break"
typst = "#colbreak()"
on_ignore = "remove"
type = "block"
"##;

        // Execute
        let def: ModifierDef = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(def.modifier_type, ModifierType::Block);
        assert_eq!(def.on_ignore, OnIgnore::Remove);
    }

    #[test]
    fn test_template_metadata_deserialization() {
        // Setup
        let toml_str = r#"
name = "Resume (Two Column)"
description = "Two-column resume layout with inline dates"
default_brand = "generic"
"#;

        // Execute
        let meta: TemplateMetadata = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(meta.name, "Resume (Two Column)");
        assert_eq!(meta.description.as_deref(), Some("Two-column resume layout with inline dates"));
        assert_eq!(meta.default_brand.as_deref(), Some("generic"));
        assert!(meta.ignore.is_empty(), "ignore should default to empty");
    }

    #[test]
    fn test_template_metadata_with_ignore_list() {
        // Setup
        let toml_str = r#"
name = "Resume (ATS)"
description = "ATS-friendly layout"
default_brand = "generic"
ignore = ["date_separator", "column_break"]
"#;

        // Execute
        let meta: TemplateMetadata = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(meta.ignore.len(), 2);
        assert!(meta.ignore.contains(&"date_separator".to_string()));
        assert!(meta.ignore.contains(&"column_break".to_string()));
    }

    #[test]
    fn test_template_metadata_minimal() {
        // Setup -- only the required 'name' field
        let toml_str = r#"name = "Bare Minimum""#;

        // Execute
        let meta: TemplateMetadata = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(meta.name, "Bare Minimum");
        assert!(meta.description.is_none());
        assert!(meta.default_brand.is_none());
        assert!(meta.ignore.is_empty());
    }

    #[test]
    fn test_brand_metadata_deserialization() {
        // Setup
        let toml_str = r#"
name = "Generic"
description = "Clean defaults using standard Typst fonts and neutral colors"
"#;

        // Execute
        let meta: BrandMetadata = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(meta.name, "Generic");
        assert_eq!(
            meta.description.as_deref(),
            Some("Clean defaults using standard Typst fonts and neutral colors")
        );
    }

    #[test]
    fn test_brand_metadata_minimal() {
        // Setup
        let toml_str = r#"name = "Simple""#;

        // Execute
        let meta: BrandMetadata = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(meta.name, "Simple");
        assert!(meta.description.is_none());
    }

    #[test]
    fn test_config_deserialization_new_format() {
        // Setup
        let toml_str = r#"
default_template = "resume-2-col"
default_brand = "generic"
output_dir = "/home/user/output"
author = "Test Author"

[[repos]]
name = "default"
url = "https://github.com/hendemic/md-docs-templates.git"

[[local]]
path = "/home/user/templates"
"#;

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(config.default_template(), Some("resume-2-col"));
        assert_eq!(config.default_brand(), Some("generic"));
        assert_eq!(config.output_dir(), Some(Path::new("/home/user/output")));
        assert_eq!(config.author(), Some("Test Author"));
        assert_eq!(config.repos().len(), 1);
        assert_eq!(config.local().len(), 1);
    }

    #[test]
    fn test_config_deserialization_partial() {
        // Setup -- only some fields set
        let toml_str = r#"
default_template = "resume-ats"
"#;

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(config.default_template(), Some("resume-ats"));
        assert!(config.default_brand().is_none());
        assert!(config.repos().is_empty());
        assert!(config.local().is_empty());
        assert!(config.author().is_none());
    }
}

// =========================================================================
// Constants
// =========================================================================

mod constants {
    use super::*;

    #[test]
    fn test_default_repo_url_is_valid_git_url() {
        assert!(DEFAULT_REPO_URL.starts_with("https://"));
        assert!(DEFAULT_REPO_URL.ends_with(".git"));
    }

    #[test]
    fn test_default_repo_name_is_default() {
        assert_eq!(DEFAULT_REPO_NAME, "default");
    }
}

// =========================================================================
// CliMessage formatting
// =========================================================================

mod cli_message_tests {
    use super::*;

    #[test]
    fn test_success_formatted_contains_checkmark_and_message() {
        let msg = CliMessage::Success("done".to_string());
        let output = msg.formatted();
        assert!(output.contains("done"));
        assert!(output.contains("\u{2713}") || output.len() > "done".len());
    }

    #[test]
    fn test_info_formatted_contains_message() {
        let msg = CliMessage::Info("compiling...".to_string());
        let output = msg.formatted();
        assert!(output.contains("compiling..."));
    }

    #[test]
    fn test_warning_formatted_contains_prefix_and_message() {
        let msg = CliMessage::Warning("missing font".to_string());
        let output = msg.formatted();
        assert!(output.contains("warning:"));
        assert!(output.contains("missing font"));
    }

    #[test]
    fn test_error_formatted_contains_prefix_and_message() {
        let msg = CliMessage::Error("file not found".to_string());
        let output = msg.formatted();
        assert!(output.contains("error:"));
        assert!(output.contains("file not found"));
    }

    #[test]
    fn test_log_formatted_contains_message() {
        let msg = CliMessage::Log("debug info".to_string());
        let output = msg.formatted();
        assert!(output.contains("debug info"));
    }

    #[test]
    fn test_plain_formatted_is_passthrough() {
        let msg = CliMessage::Plain("raw text".to_string());
        let output = msg.formatted();
        assert_eq!(output, "raw text");
    }

    #[test]
    fn test_display_impl_matches_formatted() {
        let msg = CliMessage::Success("test".to_string());
        assert_eq!(format!("{}", msg), msg.formatted());
    }
}
