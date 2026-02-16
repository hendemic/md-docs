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
/// Useful for pre-flight validation before compilation.
pub fn is_font_available(_font_name: &str) -> bool {
    todo!("Query system fonts and embedded fonts for the given family name")
}

/// Collect font file bytes from a brand's fonts/ directory.
///
/// Returns a vector of font file contents that can be passed to
/// `TypstEngine::builder().fonts(...)` for brand-bundled fonts.
///
/// Returns an empty Vec if the brand has no bundled fonts.
pub fn load_brand_fonts(_brand_dir: &Path) -> anyhow::Result<Vec<Vec<u8>>> {
    todo!("Read all .ttf/.otf/.woff2 files from brand_dir/fonts/, return their bytes")
}

/// Return a known-good fallback font family name.
///
/// Used when a brand-specified font is not found on the system.
/// Prefers "New Computer Modern" (embedded by typst-kit-embed-fonts),
/// falling back to generic sans-serif.
pub fn fallback_font() -> &'static str {
    todo!("Return 'New Computer Modern' or another embedded font name")
}
