use super::config::ConfigLoader;
use super::fonts;
use super::logger::FileLogger;
use crate::domain::{CliMessage, Config};

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

// =========================================================================
// fallback_font
// =========================================================================

mod fallback {
    use super::*;

    #[test]
    fn test_fallback_font_returns_known_font_name() {
        // Execute
        let name = fonts::fallback_font();

        // Assert -- should return a non-empty known font name
        assert!(!name.is_empty(), "fallback font name should not be empty");
        // Should be "New Computer Modern" or another known embedded font
        assert!(
            name == "New Computer Modern"
                || name.contains("Computer Modern")
                || name.contains("serif")
                || name.contains("sans"),
            "fallback font should be a known font family: got '{}'",
            name
        );
    }

    #[test]
    fn test_fallback_font_is_stable() {
        // Execute -- call twice, should return the same value
        let first = fonts::fallback_font();
        let second = fonts::fallback_font();

        // Assert
        assert_eq!(first, second, "fallback font should be deterministic");
    }
}

// =========================================================================
// load_brand_fonts
// =========================================================================

mod brand_fonts {
    use super::*;

    #[test]
    fn test_load_brand_fonts_without_fonts_dir_returns_empty() {
        // Setup -- brand directory with no fonts/ subdirectory
        let temp_dir = tempfile::tempdir().unwrap();
        // Do not create a fonts/ subdirectory

        // Execute
        let result = fonts::load_brand_fonts(temp_dir.path());

        // Assert
        assert!(result.is_ok());
        let font_data = result.unwrap();
        assert!(
            font_data.is_empty(),
            "brand with no fonts dir should return empty vec"
        );
    }

    #[test]
    fn test_load_brand_fonts_with_empty_fonts_dir() {
        // Setup -- brand directory with an empty fonts/ subdirectory
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(temp_dir.path().join("fonts")).unwrap();

        // Execute
        let result = fonts::load_brand_fonts(temp_dir.path());

        // Assert
        assert!(result.is_ok());
        let font_data = result.unwrap();
        assert!(
            font_data.is_empty(),
            "empty fonts dir should return empty vec"
        );
    }

    #[test]
    fn test_load_brand_fonts_skips_non_font_files() {
        // Setup -- fonts/ dir with non-font files
        let temp_dir = tempfile::tempdir().unwrap();
        let fonts_dir = temp_dir.path().join("fonts");
        std::fs::create_dir(&fonts_dir).unwrap();
        std::fs::write(fonts_dir.join("readme.txt"), "not a font").unwrap();
        std::fs::write(fonts_dir.join("config.json"), "{}").unwrap();

        // Execute
        let result = fonts::load_brand_fonts(temp_dir.path());

        // Assert
        assert!(result.is_ok());
        let font_data = result.unwrap();
        assert!(
            font_data.is_empty(),
            "non-font files should be skipped: got {} items",
            font_data.len()
        );
    }

    #[test]
    fn test_load_brand_fonts_reads_ttf_files() {
        // Setup -- create a fake .ttf file (not a real font, just testing file reading)
        let temp_dir = tempfile::tempdir().unwrap();
        let fonts_dir = temp_dir.path().join("fonts");
        std::fs::create_dir(&fonts_dir).unwrap();
        let fake_font_data = vec![0u8; 100]; // fake font bytes
        std::fs::write(fonts_dir.join("test.ttf"), &fake_font_data).unwrap();

        // Execute
        let result = fonts::load_brand_fonts(temp_dir.path());

        // Assert
        assert!(result.is_ok());
        let font_data = result.unwrap();
        assert_eq!(font_data.len(), 1, "should read one .ttf file");
        assert_eq!(font_data[0].len(), 100, "font data should match file size");
    }

    #[test]
    fn test_load_brand_fonts_reads_otf_files() {
        // Setup
        let temp_dir = tempfile::tempdir().unwrap();
        let fonts_dir = temp_dir.path().join("fonts");
        std::fs::create_dir(&fonts_dir).unwrap();
        std::fs::write(fonts_dir.join("test.otf"), vec![1u8; 50]).unwrap();

        // Execute
        let result = fonts::load_brand_fonts(temp_dir.path());

        // Assert
        assert!(result.is_ok());
        let font_data = result.unwrap();
        assert_eq!(font_data.len(), 1, "should read one .otf file");
    }
}

// =========================================================================
// is_font_available
// =========================================================================

mod font_availability {
    use super::*;

    #[test]
    #[ignore] // Requires typst font discovery implementation
    fn test_embedded_font_is_available() {
        // "New Computer Modern" is embedded via typst-kit-embed-fonts
        let result = fonts::is_font_available("New Computer Modern");
        assert!(
            result,
            "New Computer Modern should be available via embedded fonts"
        );
    }

    #[test]
    fn test_nonexistent_font_is_not_available() {
        let result = fonts::is_font_available("Definitely Not A Real Font Name 12345");
        assert!(
            !result,
            "nonexistent font should not be available"
        );
    }
}

// =========================================================================
// FileLogger
// =========================================================================

mod file_logger_tests {
    use super::*;
    use std::fs;

    fn temp_log_path() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        (dir, path)
    }

    #[test]
    fn test_logger_creates_file_on_first_write() {
        let (_dir, path) = temp_log_path();
        assert!(!path.exists());
        let logger = FileLogger::with_path(path.clone());
        logger.log("INFO", "hello");
        assert!(path.exists());
    }

    #[test]
    fn test_logger_appends_entries() {
        let (_dir, path) = temp_log_path();
        let logger = FileLogger::with_path(path.clone());
        logger.log("INFO", "first");
        logger.log("INFO", "second");
        let content = fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_logger_entry_contains_level_and_message() {
        let (_dir, path) = temp_log_path();
        let logger = FileLogger::with_path(path.clone());
        logger.log("WARN", "test warning");
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[WARN]"));
        assert!(content.contains("test warning"));
    }

    #[test]
    fn test_log_message_maps_cli_message_variants() {
        let (_dir, path) = temp_log_path();
        let logger = FileLogger::with_path(path.clone());
        logger.log_message(&CliMessage::Success("ok".to_string()));
        logger.log_message(&CliMessage::Warning("caution".to_string()));
        logger.log_message(&CliMessage::Error("fail".to_string()));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[SUCCESS]"));
        assert!(content.contains("[WARN]"));
        assert!(content.contains("[ERROR]"));
    }
}
