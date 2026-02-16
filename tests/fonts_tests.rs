use md_docs::infra::fonts;

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
