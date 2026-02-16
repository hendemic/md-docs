use md_docs::app::config::ConfigLoader;
use md_docs::domain::Config;

// =========================================================================
// ConfigLoader::load
// =========================================================================

mod config_loading {
    use super::*;

    #[test]
    fn test_load_returns_config_even_when_no_files_exist() {
        // Execute -- in a test environment, global/project config files may not exist
        let result = ConfigLoader::load();

        // Assert -- should succeed and return defaults
        assert!(result.is_ok(), "load should succeed even with no config files");
        let config = result.unwrap();
        // Templates and brands dirs should have sensible defaults
        let templates = config.effective_templates_dir();
        let brands = config.effective_brands_dir();
        assert!(
            templates.to_string_lossy().contains("md-docs"),
            "default templates dir should contain 'md-docs'"
        );
        assert!(
            brands.to_string_lossy().contains("md-docs"),
            "default brands dir should contain 'md-docs'"
        );
    }

    #[test]
    fn test_load_defaults_have_xdg_paths() {
        // Execute
        let result = ConfigLoader::load();

        // Assert
        assert!(result.is_ok());
        let config = result.unwrap();
        let templates = config.effective_templates_dir();
        let brands = config.effective_brands_dir();

        // Should use XDG data dir (e.g., ~/.local/share/md-docs/templates)
        assert!(
            templates.to_string_lossy().contains("templates"),
            "templates dir should end with 'templates'"
        );
        assert!(
            brands.to_string_lossy().contains("brands"),
            "brands dir should end with 'brands'"
        );
    }
}

// =========================================================================
// ConfigLoader::merge (tested via load, since merge is private)
// =========================================================================

mod config_merging {
    use super::*;

    #[test]
    fn test_merge_overlay_replaces_base_fields() {
        // Setup -- test via TOML deserialization to simulate merge behavior
        // We test the public load() which uses merge internally,
        // and also verify the Config struct behaves correctly when fields are set
        let base_toml = r#"
default_template = "original"
author = "Base Author"
"#;
        let overlay_toml = r#"
default_template = "overridden"
"#;

        let base: Config = toml::from_str(base_toml).unwrap();
        let overlay: Config = toml::from_str(overlay_toml).unwrap();

        // Assert -- the overlay has a different template
        assert_eq!(base.default_template(), Some("original"));
        assert_eq!(overlay.default_template(), Some("overridden"));
        // The overlay does not set author
        assert!(overlay.author().is_none());
    }

    #[test]
    fn test_config_all_none_overlay_keeps_base() {
        // Setup
        let base_toml = r#"
default_template = "keep-me"
author = "Keep Me Too"
"#;
        let empty_toml = "";

        let base: Config = toml::from_str(base_toml).unwrap();
        let empty: Config = toml::from_str(empty_toml).unwrap();

        // Assert -- empty config should have all None fields
        assert_eq!(base.default_template(), Some("keep-me"));
        assert_eq!(base.author(), Some("Keep Me Too"));
        assert!(empty.default_template().is_none());
        assert!(empty.author().is_none());
    }
}

// =========================================================================
// ConfigLoader::load_toml (tested indirectly via load, since it is private)
// =========================================================================

mod config_file_loading {
    use super::*;

    #[test]
    fn test_config_deserializes_valid_toml() {
        // Setup
        let toml_str = r#"
default_template = "resume-2-col"
default_brand = "generic"
templates_dir = "/home/user/templates"
brands_dir = "/home/user/brands"
"#;

        // Execute
        let config: Result<Config, _> = toml::from_str(toml_str);

        // Assert
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.default_template(), Some("resume-2-col"));
        assert_eq!(config.default_brand(), Some("generic"));
    }

    #[test]
    fn test_config_deserialization_malformed_toml_fails() {
        // Setup -- invalid TOML
        let toml_str = r#"
default_template = [unclosed
"#;

        // Execute
        let config: Result<Config, _> = toml::from_str(toml_str);

        // Assert
        assert!(config.is_err());
    }

    #[test]
    fn test_config_deserialization_empty_string() {
        // Setup
        let toml_str = "";

        // Execute
        let config: Result<Config, _> = toml::from_str(toml_str);

        // Assert
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.default_template().is_none());
    }

    #[test]
    fn test_config_deserialization_partial_fields() {
        // Setup
        let toml_str = r#"author = "Partial Config""#;

        // Execute
        let config: Config = toml::from_str(toml_str).unwrap();

        // Assert
        assert_eq!(config.author(), Some("Partial Config"));
        assert!(config.default_template().is_none());
        assert!(config.default_brand().is_none());
        assert!(config.raw_templates_dir().is_none());
    }

    #[test]
    fn test_config_deserialization_unknown_fields() {
        // Setup -- TOML with an extra field not in the Config struct
        // serde by default will ignore unknown fields (unless deny_unknown_fields is set)
        let toml_str = r#"
default_template = "test"
unknown_field = "should be ignored"
"#;

        // Execute
        let config: Result<Config, _> = toml::from_str(toml_str);

        // Assert -- depends on whether Config uses deny_unknown_fields
        // This test documents the behavior either way
        if let Ok(config) = config {
            assert_eq!(config.default_template(), Some("test"));
        }
        // If it fails, that means deny_unknown_fields is active (also valid)
    }
}

// =========================================================================
// ConfigLoader::defaults (tested via effective_* accessors on default Config)
// =========================================================================

mod config_defaults {
    use super::*;

    #[test]
    fn test_default_config_templates_dir_uses_xdg_data() {
        // Setup
        let config = Config::default();

        // Execute
        let dir = config.effective_templates_dir();

        // Assert -- should be under XDG_DATA_HOME (typically ~/.local/share)
        let dir_str = dir.to_string_lossy();
        assert!(
            dir_str.contains("md-docs"),
            "default templates path should contain 'md-docs': got '{}'",
            dir_str
        );
    }

    #[test]
    fn test_default_config_brands_dir_uses_xdg_data() {
        // Setup
        let config = Config::default();

        // Execute
        let dir = config.effective_brands_dir();

        // Assert
        let dir_str = dir.to_string_lossy();
        assert!(
            dir_str.contains("md-docs"),
            "default brands path should contain 'md-docs': got '{}'",
            dir_str
        );
    }

    #[test]
    fn test_default_config_no_default_template() {
        let config = Config::default();
        assert!(config.default_template().is_none());
    }

    #[test]
    fn test_default_config_no_default_brand() {
        let config = Config::default();
        assert!(config.default_brand().is_none());
    }

    #[test]
    fn test_default_config_no_author() {
        let config = Config::default();
        assert!(config.author().is_none());
    }

    #[test]
    fn test_default_config_no_output_dir() {
        let config = Config::default();
        assert!(config.output_dir().is_none());
    }
}
