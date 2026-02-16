use md_docs::infra::templates::{load_modifiers, TemplateManager};
use md_docs::domain::{Config, ModifierType, OnIgnore};

// =========================================================================
// Helpers
// =========================================================================

/// Path to the real templates directory in the md-docs-templates repo.
const TEMPLATES_DIR: &str =
    "/home/hendemic/Documents/Projects/md-docs/md-docs-templates/templates";

/// Path to the real brands directory in the md-docs-templates repo.
const BRANDS_DIR: &str =
    "/home/hendemic/Documents/Projects/md-docs/md-docs-templates/brands";

/// Build a Config that points to the real templates repo.
fn config_with_real_dirs() -> Config {
    let toml_str = format!(
        r#"
templates_dir = "{}"
brands_dir = "{}"
"#,
        TEMPLATES_DIR, BRANDS_DIR
    );
    toml::from_str(&toml_str).unwrap()
}

/// Build a Config pointing to a temporary (possibly empty) directory.
fn config_with_temp_dirs(temp_dir: &std::path::Path) -> Config {
    let templates = temp_dir.join("templates");
    let brands = temp_dir.join("brands");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::create_dir_all(&brands).unwrap();
    let toml_str = format!(
        r#"
templates_dir = "{}"
brands_dir = "{}"
"#,
        templates.display(),
        brands.display()
    );
    toml::from_str(&toml_str).unwrap()
}

// =========================================================================
// load_modifiers
// =========================================================================

mod modifier_loading {
    use super::*;

    #[test]
    fn test_load_modifiers_returns_all_defined_modifiers() {
        // Execute
        let result = load_modifiers();

        // Assert
        assert!(result.is_ok(), "load_modifiers should succeed: {:?}", result.err());
        let registry = result.unwrap();

        // Should have all 7 modifiers defined in modifiers.toml
        assert!(
            registry.len() >= 7,
            "should have at least 7 modifiers, got {}",
            registry.len()
        );
        assert!(registry.contains_key("date_separator"));
        assert!(registry.contains_key("column_break"));
        assert!(registry.contains_key("bottom_spacer"));
        assert!(registry.contains_key("pagebreak"));
        assert!(registry.contains_key("clearpage"));
        assert!(registry.contains_key("columnbreak"));
        assert!(registry.contains_key("columns_start"));
    }

    #[test]
    fn test_load_modifiers_date_separator_fields() {
        let registry = load_modifiers().unwrap();
        let ds = &registry["date_separator"];

        assert_eq!(ds.marker, " /| ");
        assert_eq!(ds.typst, " #h(1fr) ");
        assert_eq!(ds.on_ignore, OnIgnore::Newline);
        assert_eq!(ds.modifier_type, ModifierType::Inline);
    }

    #[test]
    fn test_load_modifiers_column_break_fields() {
        let registry = load_modifiers().unwrap();
        let cb = &registry["column_break"];

        assert_eq!(cb.marker, "<!-- COLUMN_BREAK -->");
        assert_eq!(cb.typst, "%%COLUMN_BREAK%%");
        assert_eq!(cb.on_ignore, OnIgnore::Remove);
        assert_eq!(cb.modifier_type, ModifierType::Block);
    }

    #[test]
    fn test_load_modifiers_columns_start_sentinel() {
        let registry = load_modifiers().unwrap();
        let cs = &registry["columns_start"];

        assert_eq!(cs.marker, "<!-- COLUMNS_START -->");
        assert_eq!(cs.typst, "%%COLUMNS_START%%");
        assert_eq!(cs.on_ignore, OnIgnore::Remove);
        assert_eq!(cs.modifier_type, ModifierType::Block);
    }
}

// =========================================================================
// TemplateManager discovery
// =========================================================================

mod template_discovery {
    use super::*;

    #[test]
    fn test_discover_templates_finds_real_templates() {
        // Setup
        let config = config_with_real_dirs();
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.discover_templates();

        // Assert
        assert!(result.is_ok(), "discover_templates should succeed: {:?}", result.err());
        let templates = result.unwrap();
        assert!(
            templates.len() >= 2,
            "should find at least 2 templates (resume-2-col, resume-ats), got {}",
            templates.len()
        );

        let ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"resume-2-col"), "should find resume-2-col");
        assert!(ids.contains(&"resume-ats"), "should find resume-ats");
    }

    #[test]
    fn test_discover_brands_finds_real_brands() {
        // Setup
        let config = config_with_real_dirs();
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.discover_brands();

        // Assert
        assert!(result.is_ok(), "discover_brands should succeed: {:?}", result.err());
        let brands = result.unwrap();
        assert!(
            !brands.is_empty(),
            "should find at least one brand (generic)"
        );

        let ids: Vec<&str> = brands.iter().map(|b| b.id.as_str()).collect();
        assert!(ids.contains(&"generic"), "should find generic brand");
    }

    #[test]
    fn test_discover_templates_empty_dir() {
        // Setup
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_with_temp_dirs(temp_dir.path());
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.discover_templates();

        // Assert
        assert!(result.is_ok());
        let templates = result.unwrap();
        assert!(templates.is_empty(), "empty dir should yield no templates");
    }

    #[test]
    fn test_discover_brands_empty_dir() {
        // Setup
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_with_temp_dirs(temp_dir.path());
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.discover_brands();

        // Assert
        assert!(result.is_ok());
        let brands = result.unwrap();
        assert!(brands.is_empty(), "empty dir should yield no brands");
    }

    #[test]
    fn test_discover_templates_skips_dirs_without_template_typ() {
        // Setup -- create a directory without template.typ
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_with_temp_dirs(temp_dir.path());
        let fake_template_dir = temp_dir.path().join("templates").join("not-a-template");
        std::fs::create_dir_all(&fake_template_dir).unwrap();
        std::fs::write(fake_template_dir.join("random.txt"), "not a template").unwrap();

        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.discover_templates();

        // Assert
        assert!(result.is_ok());
        let templates = result.unwrap();
        assert!(
            templates.is_empty(),
            "dirs without template.typ should be skipped"
        );
    }
}

// =========================================================================
// TemplateManager resolve_template / resolve_brand
// =========================================================================

mod template_resolution {
    use super::*;

    #[test]
    fn test_resolve_template_found() {
        // Setup
        let config = config_with_real_dirs();
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.resolve_template("resume-2-col");

        // Assert
        assert!(result.is_ok(), "resolving existing template should succeed");
        let template = result.unwrap();
        assert_eq!(template.id, "resume-2-col");
        assert_eq!(template.metadata.name, "Resume (Two Column)");
    }

    #[test]
    fn test_resolve_template_not_found() {
        // Setup
        let config = config_with_real_dirs();
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.resolve_template("nonexistent-template");

        // Assert
        assert!(result.is_err(), "resolving nonexistent template should fail");
    }

    #[test]
    fn test_resolve_brand_found() {
        // Setup
        let config = config_with_real_dirs();
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.resolve_brand("generic");

        // Assert
        assert!(result.is_ok(), "resolving existing brand should succeed");
        let brand = result.unwrap();
        assert_eq!(brand.id, "generic");
        assert_eq!(brand.metadata.name, "Generic");
    }

    #[test]
    fn test_resolve_brand_not_found() {
        // Setup
        let config = config_with_real_dirs();
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.resolve_brand("nonexistent-brand");

        // Assert
        assert!(result.is_err(), "resolving nonexistent brand should fail");
    }
}

// =========================================================================
// Metadata reading (private methods, tested indirectly via discover/resolve)
// =========================================================================

mod metadata_reading {
    use super::*;

    #[test]
    fn test_resolve_template_reads_metadata_correctly() {
        // Setup
        let config = config_with_real_dirs();
        let manager = TemplateManager::new(&config);

        // Execute
        let template = manager.resolve_template("resume-ats").unwrap();

        // Assert -- verify metadata matches the actual metadata.toml
        assert_eq!(template.metadata.name, "Resume (ATS)");
        assert_eq!(
            template.metadata.description.as_deref(),
            Some("Single-column ATS-friendly resume layout")
        );
        assert_eq!(template.metadata.default_brand.as_deref(), Some("generic"));
        assert_eq!(template.metadata.ignore.len(), 2);
        assert!(template.metadata.ignore.contains(&"date_separator".to_string()));
        assert!(template.metadata.ignore.contains(&"column_break".to_string()));
    }

    #[test]
    fn test_resolve_brand_reads_metadata_correctly() {
        // Setup
        let config = config_with_real_dirs();
        let manager = TemplateManager::new(&config);

        // Execute
        let brand = manager.resolve_brand("generic").unwrap();

        // Assert
        assert_eq!(brand.metadata.name, "Generic");
        assert_eq!(
            brand.metadata.description.as_deref(),
            Some("Clean defaults using standard Typst fonts and neutral colors")
        );
    }

    #[test]
    fn test_resolve_template_with_minimal_metadata() {
        // Setup -- create a template with only the required 'name' field
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_with_temp_dirs(temp_dir.path());
        let template_dir = temp_dir.path().join("templates").join("minimal");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(template_dir.join("template.typ"), "// minimal template").unwrap();
        std::fs::write(template_dir.join("metadata.toml"), "name = \"Minimal\"").unwrap();

        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.resolve_template("minimal");

        // Assert
        assert!(result.is_ok(), "minimal metadata should be valid");
        let template = result.unwrap();
        assert_eq!(template.metadata.name, "Minimal");
        assert!(template.metadata.description.is_none());
        assert!(template.metadata.default_brand.is_none());
        assert!(template.metadata.ignore.is_empty());
    }

    #[test]
    fn test_resolve_template_with_malformed_metadata() {
        // Setup -- create a template with invalid TOML in metadata.toml
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_with_temp_dirs(temp_dir.path());
        let template_dir = temp_dir.path().join("templates").join("bad-meta");
        std::fs::create_dir_all(&template_dir).unwrap();
        std::fs::write(template_dir.join("template.typ"), "// template").unwrap();
        std::fs::write(template_dir.join("metadata.toml"), "name = [broken").unwrap();

        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.resolve_template("bad-meta");

        // Assert -- may return error or fallback defaults depending on implementation
        // Document whichever behavior the implementation chooses
        if result.is_err() {
            // Valid: malformed metadata causes an error
        } else {
            // Valid: implementation falls back to sensible defaults
            let template = result.unwrap();
            assert!(!template.metadata.name.is_empty());
        }
    }
}

// =========================================================================
// Repository management
// =========================================================================

mod repo_management {
    use super::*;

    #[test]
    #[ignore] // Requires network access and git -- run manually
    fn test_install_repo_clones_and_copies() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_with_temp_dirs(temp_dir.path());
        let manager = TemplateManager::new(&config);

        let result = manager.install_repo("https://github.com/hendemic/md-docs-templates.git");

        assert!(result.is_ok(), "install should succeed: {:?}", result.err());
        // Verify templates were copied
        let templates = manager.discover_templates().unwrap();
        assert!(!templates.is_empty(), "should have installed templates");
    }

    #[test]
    fn test_remove_template_nonexistent_returns_error() {
        // Setup
        let temp_dir = tempfile::tempdir().unwrap();
        let config = config_with_temp_dirs(temp_dir.path());
        let manager = TemplateManager::new(&config);

        // Execute
        let result = manager.remove_template("does-not-exist");

        // Assert
        assert!(
            result.is_err(),
            "removing nonexistent template should return error"
        );
    }
}
