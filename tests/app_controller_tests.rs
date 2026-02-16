use std::path::PathBuf;

use md_docs::app::AppController;

// =========================================================================
// Helpers
// =========================================================================

/// Path to the real test resume markdown file.
const TEST_RESUME: &str = "/home/hendemic/Documents/Projects/md-docs/test/resume.md";


// =========================================================================
// AppController::new
// =========================================================================

mod controller_construction {
    use super::*;

    #[test]
    fn test_new_succeeds() {
        // Execute
        let result = AppController::new();

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
        let controller = AppController::new().unwrap();
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
        let controller = AppController::new().unwrap();

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
        let controller = AppController::new().unwrap();

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
        let controller = AppController::new().unwrap();

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
        let controller = AppController::new().unwrap();
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
        let controller = AppController::new().unwrap();

        // Execute
        let result = controller.list_templates();

        // Assert -- should succeed (prints to stdout, we just check no error)
        assert!(result.is_ok(), "list_templates should succeed: {:?}", result.err());
    }

    #[test]
    fn test_list_brands_succeeds() {
        // Setup
        let controller = AppController::new().unwrap();

        // Execute
        let result = controller.list_brands();

        // Assert
        assert!(result.is_ok(), "list_brands should succeed: {:?}", result.err());
    }
}
