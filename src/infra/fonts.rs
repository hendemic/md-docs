//! Font discovery and loading.
//!
//! Brands define which fonts to use (e.g., "New Computer Modern" for headings).
//! This module discovers fonts from:
//! 1. System fonts (via `typst-kit-fonts` feature)
//! 2. Typst's embedded fonts (via `typst-kit-embed-fonts` feature)
//! 3. Brand-bundled font files (optional `fonts/` directory in the brand)
//!
//! If a brand-specified font is not available, this module provides a fallback
//! to a known-good system or embedded font.
//!
//! All functions are module-level (no struct needed -- font loading is stateless).
//! The `typst-as-lib` crate's `search_fonts_with()` method handles most of the
//! heavy lifting. This module provides configuration and fallback logic.

use std::path::Path;

/// Check whether a specific font family is available on the system.
///
/// Font availability cannot be reliably checked without running the full Typst
/// compiler. This is a best-effort check: it returns `false` as a conservative
/// default, since the compiler will emit warnings for missing fonts at compile time.
/// Actual font resolution is deferred to the Typst compilation step.
pub fn is_font_available(_font_name: &str) -> bool {
    false
}

/// Collect font file bytes from a brand's fonts/ directory.
///
/// Returns a vector of font file contents that can be passed to
/// `TypstEngine::builder().fonts(...)` for brand-bundled fonts.
///
/// Returns an empty Vec if the brand has no bundled fonts.
pub fn load_brand_fonts(brand_dir: &Path) -> anyhow::Result<Vec<Vec<u8>>> {
    let fonts_dir = brand_dir.join("fonts");
    if !fonts_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut font_bytes = Vec::new();
    for entry in std::fs::read_dir(&fonts_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "ttf" | "otf" | "woff2" => {
                        font_bytes.push(std::fs::read(&path)?);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(font_bytes)
}

/// Return a known-good fallback font family name.
///
/// Used when a brand-specified font is not found on the system.
/// Returns "New Computer Modern" which is embedded by typst-kit-embed-fonts.
pub fn fallback_font() -> &'static str {
    "New Computer Modern"
}
