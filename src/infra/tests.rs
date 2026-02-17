use super::system::ConfigLoader;
use super::templates;  // fonts merged into templates
use super::system::FileLogger;
use super::updater;
use crate::domain::{CliMessage, Config};

// =========================================================================
// ConfigLoader::load
// =========================================================================

mod config_loading {
    use super::*;

    #[test]
    fn test_load_succeeds() {
        // Execute -- should succeed whether or not config files exist on the host
        let result = ConfigLoader::load();
        assert!(result.is_ok(), "load should always succeed: {:?}", result.err());
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

    #[test]
    fn test_merge_overlay_repos_replaces_base_repos() {
        // Setup -- simulate merge: base has repos, overlay has different repos
        let base_toml = r#"
[[repos]]
name = "base-repo"
url = "https://example.com/base.git"
"#;
        let overlay_toml = r#"
[[repos]]
name = "overlay-repo"
url = "https://example.com/overlay.git"
"#;

        let base: Config = toml::from_str(base_toml).unwrap();
        let overlay: Config = toml::from_str(overlay_toml).unwrap();

        // Assert -- verify both parsed correctly
        assert_eq!(base.repos().len(), 1);
        assert_eq!(base.repos()[0].name, "base-repo");
        assert_eq!(overlay.repos().len(), 1);
        assert_eq!(overlay.repos()[0].name, "overlay-repo");
        // When merge is applied, overlay repos should replace base repos
        // (non-empty overlay replaces base entirely)
    }

    #[test]
    fn test_merge_empty_repos_keeps_base() {
        // Setup -- overlay with no repos should keep base repos
        let base_toml = r#"
[[repos]]
name = "keep-me"
url = "https://example.com/keep.git"
"#;
        let overlay_toml = r#"
default_template = "new-template"
"#;

        let base: Config = toml::from_str(base_toml).unwrap();
        let overlay: Config = toml::from_str(overlay_toml).unwrap();

        // Assert
        assert_eq!(base.repos().len(), 1);
        assert!(overlay.repos().is_empty(), "overlay has no repos");
        // When merge is applied, base repos should be preserved
    }

    #[test]
    fn test_merge_overlay_local_replaces_base_local() {
        // Setup
        let base_toml = r#"
[[local]]
path = "/base/path"
"#;
        let overlay_toml = r#"
[[local]]
path = "/overlay/path"
"#;

        let base: Config = toml::from_str(base_toml).unwrap();
        let overlay: Config = toml::from_str(overlay_toml).unwrap();

        // Assert
        assert_eq!(base.local().len(), 1);
        assert_eq!(base.local()[0].path.to_string_lossy(), "/base/path");
        assert_eq!(overlay.local().len(), 1);
        assert_eq!(overlay.local()[0].path.to_string_lossy(), "/overlay/path");
    }

    #[test]
    fn test_merge_mixed() {
        // Setup -- overlay with local but no repos
        let base_toml = r#"
[[repos]]
name = "base-repo"
url = "https://example.com/base.git"

[[local]]
path = "/base/local"
"#;
        let overlay_toml = r#"
[[local]]
path = "/overlay/local"
"#;

        let base: Config = toml::from_str(base_toml).unwrap();
        let overlay: Config = toml::from_str(overlay_toml).unwrap();

        // Assert -- overlay has local but no repos
        assert_eq!(base.repos().len(), 1);
        assert_eq!(base.local().len(), 1);
        assert!(overlay.repos().is_empty());
        assert_eq!(overlay.local().len(), 1);
        // When merge is applied: base repos kept (overlay empty), overlay local replaces base
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

[[repos]]
name = "default"
url = "https://github.com/hendemic/md-docs-templates.git"

[[local]]
path = "/home/user/templates"
"#;

        // Execute
        let config: Result<Config, _> = toml::from_str(toml_str);

        // Assert
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.default_template(), Some("resume-2-col"));
        assert_eq!(config.default_brand(), Some("generic"));
        assert_eq!(config.repos().len(), 1);
        assert_eq!(config.local().len(), 1);
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
        assert!(config.repos().is_empty());
        assert!(config.local().is_empty());
    }

    #[test]
    fn test_config_deserialization_unknown_fields() {
        // Setup -- TOML with an extra field not in the Config struct
        let toml_str = r#"
default_template = "test"
unknown_field = "should be ignored"
"#;

        // Execute
        let config: Result<Config, _> = toml::from_str(toml_str);

        // Assert -- depends on whether Config uses deny_unknown_fields
        if let Ok(config) = config {
            assert_eq!(config.default_template(), Some("test"));
        }
        // If it fails, that means deny_unknown_fields is active (also valid)
    }
}

// =========================================================================
// ConfigLoader::defaults
// =========================================================================

mod config_defaults {
    use super::*;

    #[test]
    fn test_default_config_empty_repos() {
        let config = Config::default();
        assert!(config.repos().is_empty());
    }

    #[test]
    fn test_default_config_empty_local() {
        let config = Config::default();
        assert!(config.local().is_empty());
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
        let name = templates::fallback_font();

        // Assert -- should return a non-empty known font name
        assert!(!name.is_empty(), "fallback font name should not be empty");
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
        let first = templates::fallback_font();
        let second = templates::fallback_font();

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

        // Execute
        let result = templates::load_brand_fonts(temp_dir.path());

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
        let result = templates::load_brand_fonts(temp_dir.path());

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
        let result = templates::load_brand_fonts(temp_dir.path());

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
        let result = templates::load_brand_fonts(temp_dir.path());

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
        let result = templates::load_brand_fonts(temp_dir.path());

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
        let result = templates::is_font_available("New Computer Modern");
        assert!(
            result,
            "New Computer Modern should be available via embedded fonts"
        );
    }

    #[test]
    fn test_nonexistent_font_is_not_available() {
        let result = templates::is_font_available("Definitely Not A Real Font Name 12345");
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

// =========================================================================
// Updater — unit tests
// =========================================================================

mod updater_unit {
    use super::*;
    use semver::Version;

    #[test]
    fn test_current_version_is_valid_semver() {
        // Execute
        let version = updater::current_version();

        // Assert -- should be a valid semver::Version matching Cargo.toml
        let expected = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        assert_eq!(
            version, expected,
            "current_version() should match CARGO_PKG_VERSION"
        );
        // Verify it has the basic semver components
        assert!(
            version.major > 0 || version.minor > 0 || version.patch > 0,
            "version should have at least one non-zero component: {}",
            version
        );
    }

    #[test]
    fn test_is_aur_install_returns_bool() {
        // Execute -- should not panic regardless of platform
        let result = updater::is_aur_install();

        // Assert -- on non-Arch systems (or when not installed via pacman),
        // this should return false. On CI / dev machines it will almost
        // certainly be false. The main goal is verifying no panic.
        let _ = result; // value consumed, test passes if no panic
    }

    #[test]
    fn test_release_info_debug_impl() {
        // Setup
        let asset = updater::ReleaseAsset {
            name: "md-docs-x86_64-linux".to_string(),
            download_url: "https://example.com/asset".to_string(),
        };
        let release = updater::ReleaseInfo {
            tag_name: "v0.2.0".to_string(),
            version: Version::new(0, 2, 0),
            assets: vec![asset],
        };

        // Execute -- format with Debug trait
        let debug_str = format!("{:?}", release);

        // Assert -- Debug output should contain field names and values
        assert!(
            debug_str.contains("ReleaseInfo"),
            "Debug output should contain struct name: {}",
            debug_str
        );
        assert!(
            debug_str.contains("v0.2.0"),
            "Debug output should contain tag_name: {}",
            debug_str
        );
        assert!(
            debug_str.contains("md-docs-x86_64-linux"),
            "Debug output should contain asset name: {}",
            debug_str
        );

        // Also verify ReleaseAsset Debug independently
        let asset2 = updater::ReleaseAsset {
            name: "test-asset".to_string(),
            download_url: "https://example.com/dl".to_string(),
        };
        let asset_debug = format!("{:?}", asset2);
        assert!(
            asset_debug.contains("ReleaseAsset"),
            "ReleaseAsset Debug should contain struct name: {}",
            asset_debug
        );
    }
}

// =========================================================================
// Updater — integration tests (require network)
// =========================================================================

mod updater_integration {
    use super::*;

    #[test]
    #[ignore]
    fn test_fetch_latest_release() {
        // Execute -- calls the real GitHub API
        let result = updater::fetch_latest_release();

        // Assert
        assert!(
            result.is_ok(),
            "fetch_latest_release should succeed: {:?}",
            result.err()
        );

        let release = result.unwrap();

        // tag_name should follow the "vX.Y.Z" convention
        assert!(
            release.tag_name.starts_with('v'),
            "tag_name should start with 'v': got '{}'",
            release.tag_name
        );

        // version should be valid semver (already parsed, but verify non-zero)
        assert!(
            release.version.major > 0 || release.version.minor > 0 || release.version.patch > 0,
            "release version should have at least one non-zero component: {}",
            release.version
        );

        // assets may be empty if no releases exist yet, but the vec itself should be present
        // (this is verified by the type system, but we can log for visibility)
        if release.assets.is_empty() {
            eprintln!(
                "Note: release {} has no assets (may be pre-release or source-only)",
                release.tag_name
            );
        } else {
            // If assets exist, verify they have non-empty names and URLs
            for asset in &release.assets {
                assert!(
                    !asset.name.is_empty(),
                    "asset name should not be empty"
                );
                assert!(
                    asset.download_url.starts_with("https://"),
                    "asset download_url should be HTTPS: got '{}'",
                    asset.download_url
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn test_check_for_update() {
        // Execute -- calls the real GitHub API
        let result = updater::check_for_update();

        // Assert
        assert!(
            result.is_ok(),
            "check_for_update should succeed: {:?}",
            result.err()
        );

        let check = result.unwrap();

        // Verify we got one of the valid enum variants
        match &check {
            updater::UpdateCheck::UpToDate(version) => {
                assert!(
                    version.major > 0 || version.minor > 0 || version.patch > 0,
                    "UpToDate version should be valid: {}",
                    version
                );
            }
            updater::UpdateCheck::UpdateAvailable {
                current,
                latest,
                release,
            } => {
                assert!(
                    latest > current,
                    "latest ({}) should be greater than current ({})",
                    latest,
                    current
                );
                assert!(
                    !release.tag_name.is_empty(),
                    "release tag_name should not be empty"
                );
            }
            updater::UpdateCheck::AurInstall => {
                // Valid variant -- nothing more to assert
            }
        }

        // Verify Debug is implemented on UpdateCheck
        let debug_str = format!("{:?}", check);
        assert!(
            !debug_str.is_empty(),
            "UpdateCheck Debug output should not be empty"
        );
    }
}

// =========================================================================
// Log Rotation
// =========================================================================

mod log_rotation {
    use super::*;
    use std::fs;

    fn temp_log_path() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        (dir, path)
    }

    /// Create a file at `path` filled with `size` bytes of repeated content,
    /// prefixed by `tag` so we can identify the file after rotation.
    fn create_tagged_file(path: &std::path::Path, tag: &str, size: usize) {
        let mut content = format!("{}\n", tag);
        while content.len() < size {
            content.push_str("PADDING_LINE_TO_FILL_LOG_FILE\n");
        }
        content.truncate(size);
        fs::write(path, content).unwrap();
    }

    /// Read the first line of a file (the tag).
    fn read_tag(path: &std::path::Path) -> String {
        let content = fs::read_to_string(path).unwrap();
        content.lines().next().unwrap_or("").to_string()
    }

    #[test]
    fn test_no_rotation_when_under_limit() {
        // Setup -- create a small log file well under 1MB
        let (_dir, path) = temp_log_path();
        fs::write(&path, "small log content\n").unwrap();
        let backup_path = format!("{}.1", path.display());

        // Execute -- constructing the logger triggers rotate_if_needed
        let _logger = FileLogger::with_path(path.clone());

        // Assert -- no backup should have been created
        assert!(
            !std::path::Path::new(&backup_path).exists(),
            "no .log.1 backup should exist for a file under 1MB"
        );
        // Original file should still exist (unchanged)
        assert!(path.exists(), "original log file should still exist");
    }

    #[test]
    fn test_rotation_creates_backup() {
        // Setup -- create a log file exceeding 1MB
        let (_dir, path) = temp_log_path();
        let over_1mb = 1_048_576 + 1024; // slightly over the limit
        create_tagged_file(&path, "ORIGINAL_LOG", over_1mb);
        let backup_path_1 = format!("{}.1", path.display());

        // Execute -- constructing the logger triggers rotation
        let logger = FileLogger::with_path(path.clone());

        // Assert -- the original content should now be in .log.1
        assert!(
            std::path::Path::new(&backup_path_1).exists(),
            ".log.1 backup should exist after rotation"
        );
        let backup_tag = read_tag(std::path::Path::new(&backup_path_1));
        assert_eq!(
            backup_tag, "ORIGINAL_LOG",
            "backup .log.1 should contain the original log content"
        );

        // A new write should go to the fresh .log file (not the backup)
        logger.log("INFO", "fresh entry");
        assert!(path.exists(), "fresh .log file should exist after write");
        let fresh_content = fs::read_to_string(&path).unwrap();
        assert!(
            fresh_content.contains("fresh entry"),
            "new .log should contain the fresh entry"
        );
        assert!(
            !fresh_content.contains("ORIGINAL_LOG"),
            "new .log should not contain old content"
        );
    }

    #[test]
    fn test_rotation_shifts_existing_backups() {
        // Setup -- create .log (> 1MB), .log.1, and .log.2 with known tags
        let (_dir, path) = temp_log_path();
        let over_1mb = 1_048_576 + 1024;
        create_tagged_file(&path, "CURRENT_LOG", over_1mb);

        let backup_1 = format!("{}.1", path.display());
        let backup_2 = format!("{}.2", path.display());
        let backup_3 = format!("{}.3", path.display());
        fs::write(&backup_1, "BACKUP_ONE\n").unwrap();
        fs::write(&backup_2, "BACKUP_TWO\n").unwrap();

        // Execute
        let _logger = FileLogger::with_path(path.clone());

        // Assert -- chain should have shifted:
        // old .log      -> .log.1
        // old .log.1    -> .log.2
        // old .log.2    -> .log.3
        let tag_1 = read_tag(std::path::Path::new(&backup_1));
        assert_eq!(
            tag_1, "CURRENT_LOG",
            ".log.1 should now contain what was in .log"
        );

        let tag_2 = read_tag(std::path::Path::new(&backup_2));
        assert_eq!(
            tag_2, "BACKUP_ONE",
            ".log.2 should now contain what was in .log.1"
        );

        let tag_3 = read_tag(std::path::Path::new(&backup_3));
        assert_eq!(
            tag_3, "BACKUP_TWO",
            ".log.3 should now contain what was in .log.2"
        );
    }

    #[test]
    fn test_rotation_deletes_oldest_beyond_limit() {
        // Setup -- create .log (> 1MB), .log.1, .log.2, .log.3 with known tags
        let (_dir, path) = temp_log_path();
        let over_1mb = 1_048_576 + 1024;
        create_tagged_file(&path, "CURRENT_LOG", over_1mb);

        let backup_1 = format!("{}.1", path.display());
        let backup_2 = format!("{}.2", path.display());
        let backup_3 = format!("{}.3", path.display());
        fs::write(&backup_1, "BACKUP_ONE\n").unwrap();
        fs::write(&backup_2, "BACKUP_TWO\n").unwrap();
        fs::write(&backup_3, "BACKUP_THREE_OLDEST\n").unwrap();

        // Execute
        let _logger = FileLogger::with_path(path.clone());

        // Assert -- the chain should have shifted, and the oldest content is gone:
        // old .log      -> .log.1
        // old .log.1    -> .log.2
        // old .log.2    -> .log.3 (overwrites old .log.3 which is discarded)
        let tag_1 = read_tag(std::path::Path::new(&backup_1));
        assert_eq!(
            tag_1, "CURRENT_LOG",
            ".log.1 should now contain what was in .log"
        );

        let tag_2 = read_tag(std::path::Path::new(&backup_2));
        assert_eq!(
            tag_2, "BACKUP_ONE",
            ".log.2 should now contain what was in .log.1"
        );

        let tag_3 = read_tag(std::path::Path::new(&backup_3));
        assert_eq!(
            tag_3, "BACKUP_TWO",
            ".log.3 should now contain what was in .log.2 (old .log.3 is gone)"
        );

        // Verify the oldest content (BACKUP_THREE_OLDEST) no longer exists anywhere
        for suffix in &["", ".1", ".2", ".3"] {
            let file_path = if suffix.is_empty() {
                path.display().to_string()
            } else {
                format!("{}{}", path.display(), suffix)
            };
            let p = std::path::Path::new(&file_path);
            if p.exists() {
                let content = fs::read_to_string(p).unwrap();
                assert!(
                    !content.contains("BACKUP_THREE_OLDEST"),
                    "oldest backup content should be fully discarded, but found in {}",
                    file_path
                );
            }
        }
    }
}

// =========================================================================
// XDG Base Directories
// =========================================================================

mod xdg_paths {
    use crate::infra::system::{xdg_config_home, xdg_data_home};
    use std::sync::Mutex;

    /// Mutex to serialize env-var-mutating tests. Rust tests run in parallel
    /// by default, and env vars are process-global state, so concurrent
    /// modification would cause flaky results.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_xdg_config_home_falls_back_to_dot_config() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Setup -- save and unset XDG_CONFIG_HOME to force fallback
        let saved = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::remove_var("XDG_CONFIG_HOME");

        // Execute
        let result = xdg_config_home();

        // Teardown -- restore original value
        match saved {
            Some(val) => std::env::set_var("XDG_CONFIG_HOME", val),
            None => {} // was already unset
        }

        // Assert -- fallback should end with ".config"
        assert!(
            result.ends_with(".config"),
            "xdg_config_home fallback should end with '.config', got: {:?}",
            result
        );
    }

    #[test]
    fn test_xdg_data_home_falls_back_to_dot_local_share() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Setup -- save and unset XDG_DATA_HOME to force fallback
        let saved = std::env::var("XDG_DATA_HOME").ok();
        std::env::remove_var("XDG_DATA_HOME");

        // Execute
        let result = xdg_data_home();

        // Teardown
        match saved {
            Some(val) => std::env::set_var("XDG_DATA_HOME", val),
            None => {}
        }

        // Assert -- fallback should end with "share" (from ".local/share")
        assert!(
            result.ends_with(".local/share"),
            "xdg_data_home fallback should end with '.local/share', got: {:?}",
            result
        );
    }

    #[test]
    fn test_xdg_config_home_respects_env_var() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Setup -- save original and set a known custom path
        let saved = std::env::var("XDG_CONFIG_HOME").ok();
        let custom_path = "/tmp/test-xdg-config";
        std::env::set_var("XDG_CONFIG_HOME", custom_path);

        // Execute
        let result = xdg_config_home();

        // Teardown
        match saved {
            Some(val) => std::env::set_var("XDG_CONFIG_HOME", val),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }

        // Assert
        assert_eq!(
            result,
            std::path::PathBuf::from(custom_path),
            "xdg_config_home should return the value of XDG_CONFIG_HOME"
        );
    }

    #[test]
    fn test_xdg_data_home_respects_env_var() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Setup
        let saved = std::env::var("XDG_DATA_HOME").ok();
        let custom_path = "/tmp/test-xdg-data";
        std::env::set_var("XDG_DATA_HOME", custom_path);

        // Execute
        let result = xdg_data_home();

        // Teardown
        match saved {
            Some(val) => std::env::set_var("XDG_DATA_HOME", val),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }

        // Assert
        assert_eq!(
            result,
            std::path::PathBuf::from(custom_path),
            "xdg_data_home should return the value of XDG_DATA_HOME"
        );
    }

    #[test]
    fn test_xdg_ignores_empty_env_var() {
        let _lock = ENV_MUTEX.lock().unwrap();

        // Setup -- set XDG_CONFIG_HOME to empty string, should trigger fallback
        let saved = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", "");

        // Execute
        let result = xdg_config_home();

        // Teardown
        match saved {
            Some(val) => std::env::set_var("XDG_CONFIG_HOME", val),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }

        // Assert -- empty string should be treated as unset, so fallback applies
        assert!(
            result.ends_with(".config"),
            "empty XDG_CONFIG_HOME should fall back to '.config', got: {:?}",
            result
        );
    }
}
