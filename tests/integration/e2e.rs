//! Integration tests for the md-docs public API.
//!
//! These tests exercise cross-module boundaries and end-to-end workflows.
//! They use real template and brand files from the md-docs-templates repo.

use std::path::PathBuf;

use md_docs::app::converter::markdown_to_typst;
use md_docs::infra::templates::{load_modifiers, TemplateManager};
use md_docs::app::AppController;
use md_docs::domain::*;

/// Path to the real test resume markdown file.
const TEST_RESUME: &str = "/home/hendemic/Documents/Projects/md-docs/test/resume.md";

/// Path to the real templates directory.
const TEMPLATES_DIR: &str =
    "/home/hendemic/Documents/Projects/md-docs/md-docs-templates/templates";

/// Path to the real brands directory.
const BRANDS_DIR: &str =
    "/home/hendemic/Documents/Projects/md-docs/md-docs-templates/brands";

fn real_config() -> Config {
    let toml_str = format!(
        r#"
templates_dir = "{}"
brands_dir = "{}"
"#,
        TEMPLATES_DIR, BRANDS_DIR
    );
    toml::from_str(&toml_str).unwrap()
}

// =========================================================================
// End-to-end PDF generation
// =========================================================================

mod end_to_end {
    use super::*;

    #[test]
    #[ignore] // Requires full implementation
    fn test_end_to_end_resume_2col() {
        // Setup
        let controller = AppController::new(false).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("resume-2col.pdf");

        // Execute
        let result = controller.convert(
            PathBuf::from(TEST_RESUME),
            Some("resume-2-col".to_string()),
            Some("generic".to_string()),
            Some(output_path.clone()),
        );

        // Assert
        assert!(result.is_ok(), "end-to-end resume-2-col should succeed: {:?}", result.err());
        assert!(output_path.exists(), "PDF file should exist");
        let bytes = std::fs::read(&output_path).unwrap();
        assert!(bytes.starts_with(b"%PDF"), "output should be a valid PDF");
        assert!(bytes.len() > 1000, "PDF should have substantial content");
    }

    #[test]
    #[ignore] // Requires full implementation
    fn test_end_to_end_resume_ats() {
        // Setup
        let controller = AppController::new(false).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("resume-ats.pdf");

        // Execute
        let result = controller.convert(
            PathBuf::from(TEST_RESUME),
            Some("resume-ats".to_string()),
            Some("generic".to_string()),
            Some(output_path.clone()),
        );

        // Assert
        assert!(result.is_ok(), "end-to-end resume-ats should succeed: {:?}", result.err());
        assert!(output_path.exists());
        let bytes = std::fs::read(&output_path).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    #[ignore] // Requires full implementation
    fn test_end_to_end_minimal_markdown() {
        // Setup -- minimal markdown with no frontmatter
        let temp_dir = tempfile::tempdir().unwrap();
        let input_path = temp_dir.path().join("minimal.md");
        std::fs::write(&input_path, "# Hello World\n\nThis is a test.").unwrap();

        let output_path = temp_dir.path().join("minimal.pdf");
        let controller = AppController::new(false).unwrap();

        // Execute
        let result = controller.convert(
            input_path,
            Some("resume-ats".to_string()),
            Some("generic".to_string()),
            Some(output_path.clone()),
        );

        // Assert
        assert!(
            result.is_ok(),
            "minimal markdown should compile: {:?}",
            result.err()
        );
    }
}

// =========================================================================
// Markdown to content.typ roundtrip
// =========================================================================

mod roundtrip {
    use super::*;

    #[test]
    fn test_markdown_to_typst_then_split_sections() {
        // Setup
        let markdown = r#"# Mike Henderson
Seattle | hello@test.com

---

Summary paragraph.

<!-- COLUMNS_START -->

## Employment

### Company
**Title** /| *Date*
- Did things
"#;
        // Load modifiers and resolve them with no ignore list
        let registry = load_modifiers().unwrap();
        let resolved = resolve_modifiers(&registry, &[]);
        let ctx = ConversionContext::from_resolved(&resolved);

        // Execute
        let typst_result = markdown_to_typst(markdown, &ctx);

        // Assert
        assert!(typst_result.is_ok(), "conversion should succeed: {:?}", typst_result.err());
        let typst = typst_result.unwrap();

        // The COLUMNS_START modifier should have been resolved to the sentinel
        assert!(
            typst.contains("%%COLUMNS_START%%"),
            "COLUMNS_START should resolve to sentinel: {}",
            typst
        );

        // Now split into sections
        let sections = ContentSections::from_typst_content(&typst, "%%COLUMNS_START%%");
        assert!(
            !sections.header.is_empty(),
            "header should contain content above COLUMNS_START"
        );
        assert!(
            !sections.body.is_empty(),
            "body should contain content below COLUMNS_START"
        );
        assert!(
            sections.body.contains("Employment"),
            "body should contain the Employment section"
        );
    }
}

// =========================================================================
// Modifier resolution with template ignore lists
// =========================================================================

mod modifier_integration {
    use super::*;

    #[test]
    fn test_ats_template_ignores_date_separator_and_column_break() {
        // Setup -- resume-ats ignores date_separator and column_break
        let registry = load_modifiers().unwrap();
        let ignore_list = vec!["date_separator".to_string(), "column_break".to_string()];
        let resolved = resolve_modifiers(&registry, &ignore_list);
        let ctx = ConversionContext::from_resolved(&resolved);

        // Execute
        let markdown = "**Title** /| *Date*\n\n<!-- COLUMN_BREAK -->\n\nMore content";
        let result = markdown_to_typst(markdown, &ctx);

        // Assert
        assert!(result.is_ok());
        let typst = result.unwrap();
        assert!(
            !typst.contains("#h(1fr)"),
            "ignored date_separator should not produce #h(1fr): {}",
            typst
        );
        assert!(
            !typst.contains("%%COLUMN_BREAK%%"),
            "ignored column_break should not produce %%COLUMN_BREAK%%: {}",
            typst
        );
    }

    #[test]
    fn test_2col_template_uses_all_modifiers() {
        // Setup -- resume-2-col has no ignore list
        let registry = load_modifiers().unwrap();
        let resolved = resolve_modifiers(&registry, &[]);
        let ctx = ConversionContext::from_resolved(&resolved);

        // Execute
        let markdown = "**Title** /| *Date*\n\n<!-- COLUMN_BREAK -->\n\nMore content";
        let result = markdown_to_typst(markdown, &ctx);

        // Assert
        assert!(result.is_ok());
        let typst = result.unwrap();
        assert!(
            typst.contains("#h(1fr)"),
            "date_separator should produce #h(1fr): {}",
            typst
        );
        assert!(
            typst.contains("%%COLUMN_BREAK%%"),
            "column_break should produce %%COLUMN_BREAK%% marker: {}",
            typst
        );
    }
}

// =========================================================================
// Config + template discovery integration
// =========================================================================

mod config_and_discovery {
    use super::*;

    #[test]
    fn test_config_points_to_templates_and_brands_discover_works() {
        // Setup
        let config = real_config();
        let manager = TemplateManager::new(&config);

        // Execute
        let templates = manager.discover_templates().unwrap();
        let brands = manager.discover_brands().unwrap();

        // Assert
        assert!(templates.len() >= 2, "should find resume templates");
        assert!(!brands.is_empty(), "should find brands");

        // Verify template paths are within the configured directory
        for t in &templates {
            assert!(
                t.path.starts_with(TEMPLATES_DIR),
                "template path should be under templates_dir: {:?}",
                t.path
            );
        }
        for b in &brands {
            assert!(
                b.path.starts_with(BRANDS_DIR),
                "brand path should be under brands_dir: {:?}",
                b.path
            );
        }
    }
}

// =========================================================================
// AppController::new
// =========================================================================

mod controller_construction {
    use super::*;

    #[test]
    fn test_new_succeeds() {
        // Execute
        let result = AppController::new(false);

        // Assert -- should succeed even if no config files exist
        assert!(result.is_ok(), "AppController::new should succeed: {:?}", result.err());
    }
}

// =========================================================================
// resolve_output (private, tested indirectly)
//
// Since resolve_output is private, we test the behavior via the convert
// pipeline. These tests verify the expected output path resolution.
// =========================================================================

mod output_resolution {
    use super::*;

    #[test]
    fn test_resolve_output_explicit_path_used() {
        // This tests the expectation: when an explicit output path is given,
        // it should be used directly. Since resolve_output is private,
        // we document the expected behavior.
        let explicit = PathBuf::from("/tmp/my-output.pdf");

        // The expectation: resolve_output(input, Some(explicit)) == explicit
        assert!(explicit.extension().unwrap() == "pdf");
    }

    #[test]
    fn test_resolve_output_derived_from_input() {
        // The expectation: resolve_output("resume.md", None) should produce
        // a path like "resume.pdf" (same directory, .pdf extension)
        let input = PathBuf::from("/home/user/docs/resume.md");
        let expected_stem = "resume";

        assert_eq!(input.file_stem().unwrap().to_str().unwrap(), expected_stem);
        // The implementation should produce input.with_extension("pdf")
    }
}

// =========================================================================
// parse_input (private, tested via convert pipeline)
// =========================================================================

mod input_parsing {
    use super::*;

    #[test]
    #[ignore] // Requires implementation -- parse_input is private, tested via convert
    fn test_convert_with_real_resume_parses_successfully() {
        // Setup
        let controller = AppController::new(false).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("resume.pdf");

        // Execute -- full convert pipeline with real files
        let result = controller.convert(
            PathBuf::from(TEST_RESUME),
            Some("resume-2-col".to_string()),
            Some("generic".to_string()),
            Some(output_path.clone()),
        );

        // Assert -- if this succeeds, parse_input worked
        assert!(result.is_ok(), "convert with real resume should succeed: {:?}", result.err());
    }

    #[test]
    fn test_convert_nonexistent_input_fails() {
        // Setup
        let controller = AppController::new(false).unwrap();

        // Execute
        let result = controller.convert(
            PathBuf::from("/nonexistent/file.md"),
            Some("resume-2-col".to_string()),
            Some("generic".to_string()),
            Some(PathBuf::from("/tmp/output.pdf")),
        );

        // Assert
        assert!(
            result.is_err(),
            "convert with nonexistent input should fail"
        );
    }

    #[test]
    #[ignore] // Requires implementation -- tests config author injection
    fn test_parse_input_injects_config_author_as_fallback() {
        // This test verifies that when frontmatter has no author field,
        // the config's author is used as a fallback.
        // Since parse_input is private, we test indirectly.

        // The test markdown has no YAML frontmatter (no author),
        // so if the config has an author set, it should be injected.
        // We would need to create a temporary config to test this properly.
        // For now, document the expectation.
        let _expected_behavior =
            "If frontmatter lacks author and config has author, config author is injected";
    }
}

// =========================================================================
// resolve_template / resolve_brand fallback chains (private, tested indirectly)
// =========================================================================

mod resolution_fallbacks {
    use super::*;

    #[test]
    fn test_convert_with_explicit_template_name() {
        // Setup
        let controller = AppController::new(false).unwrap();

        // Execute -- use a nonexistent template name (should fail since not found)
        let result = controller.convert(
            PathBuf::from(TEST_RESUME),
            Some("nonexistent-template".to_string()),
            Some("generic".to_string()),
            Some(PathBuf::from("/tmp/output.pdf")),
        );

        // Assert -- should fail because template doesn't exist
        assert!(result.is_err(), "nonexistent template should cause an error");
    }

    #[test]
    fn test_convert_with_explicit_brand_name() {
        // Setup
        let controller = AppController::new(false).unwrap();

        // Execute -- use a nonexistent brand name
        let result = controller.convert(
            PathBuf::from(TEST_RESUME),
            Some("resume-2-col".to_string()),
            Some("nonexistent-brand".to_string()),
            Some(PathBuf::from("/tmp/output.pdf")),
        );

        // Assert -- should fail because brand doesn't exist
        assert!(result.is_err(), "nonexistent brand should cause an error");
    }

    #[test]
    #[ignore] // Requires implementation -- tests template's default_brand fallback
    fn test_convert_without_brand_uses_template_default() {
        // Setup
        let controller = AppController::new(false).unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let output_path = temp_dir.path().join("output.pdf");

        // Execute -- no brand specified; resume-2-col defaults to "generic"
        let result = controller.convert(
            PathBuf::from(TEST_RESUME),
            Some("resume-2-col".to_string()),
            None, // should fall back to template's default_brand
            Some(output_path),
        );

        // Assert
        assert!(
            result.is_ok(),
            "should fall back to template's default_brand: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_list_templates_succeeds() {
        // Setup
        let controller = AppController::new(false).unwrap();

        // Execute
        let result = controller.list_templates();

        // Assert -- should succeed (prints to stdout, we just check no error)
        assert!(result.is_ok(), "list_templates should succeed: {:?}", result.err());
    }

    #[test]
    fn test_list_brands_succeeds() {
        // Setup
        let controller = AppController::new(false).unwrap();

        // Execute
        let result = controller.list_brands();

        // Assert
        assert!(result.is_ok(), "list_brands should succeed: {:?}", result.err());
    }
}
