#!/usr/bin/env bash
set -euo pipefail

# Build macOS release binaries for md-docs.
# Produces per-architecture tar.gz archives, a universal binary tar.gz,
# and an unsigned .pkg installer in dist/release/.

# Navigate to project root (parent of this script's directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Extract version from Cargo.toml
VERSION="$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')"
if [[ -z "$VERSION" ]]; then
    echo "ERROR: Could not extract version from Cargo.toml"
    exit 1
fi
echo "Building md-docs v${VERSION}"

# Check required tools
for tool in rustup lipo pkgbuild tar; do
    if ! command -v "$tool" &>/dev/null; then
        echo "ERROR: Required tool '$tool' is not installed."
        exit 1
    fi
done

# Add rustup targets if not already installed
for rust_target in x86_64-apple-darwin aarch64-apple-darwin; do
    if ! rustup target list --installed | grep -q "$rust_target"; then
        echo "Adding rustup target: $rust_target"
        rustup target add "$rust_target"
    fi
done

# Define targets
TARGETS=(
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

# Create/clean output directory
RELEASE_DIR="$PROJECT_ROOT/dist/release"
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

# Build each target and create per-architecture archives
for target in "${TARGETS[@]}"; do
    echo ""
    echo "=== Building for $target ==="

    # Determine archive architecture name
    case "$target" in
        x86_64-*)  arch="x86_64" ;;
        aarch64-*) arch="aarch64" ;;
        *)
            echo "ERROR: Unknown target architecture: $target"
            exit 1
            ;;
    esac

    # Build natively
    cargo build --release --target "$target"

    # Stage the binary
    STAGING_DIR="$RELEASE_DIR/md-docs-v${VERSION}-macos-${arch}"
    mkdir -p "$STAGING_DIR"
    cp "target/${target}/release/mdocs" "$STAGING_DIR/"

    # Create archive
    ARCHIVE_NAME="md-docs-v${VERSION}-macos-${arch}.tar.gz"
    tar -czf "$RELEASE_DIR/$ARCHIVE_NAME" -C "$RELEASE_DIR" "md-docs-v${VERSION}-macos-${arch}"

    # Clean up staging directory
    rm -rf "$STAGING_DIR"

    echo "Created: dist/release/$ARCHIVE_NAME"
done

# Create universal binary
echo ""
echo "=== Creating universal binary ==="

UNIVERSAL_STAGING="$RELEASE_DIR/md-docs-v${VERSION}-macos-universal"
mkdir -p "$UNIVERSAL_STAGING"

lipo -create -output "$UNIVERSAL_STAGING/mdocs" \
    "target/x86_64-apple-darwin/release/mdocs" \
    "target/aarch64-apple-darwin/release/mdocs"

UNIVERSAL_ARCHIVE="md-docs-v${VERSION}-macos-universal.tar.gz"
tar -czf "$RELEASE_DIR/$UNIVERSAL_ARCHIVE" -C "$RELEASE_DIR" "md-docs-v${VERSION}-macos-universal"

echo "Created: dist/release/$UNIVERSAL_ARCHIVE"

# Create unsigned .pkg installer from the universal binary
echo ""
echo "=== Creating .pkg installer ==="

PKG_STAGING="$RELEASE_DIR/pkg-root"
mkdir -p "$PKG_STAGING"
cp "$UNIVERSAL_STAGING/mdocs" "$PKG_STAGING/"

PKG_NAME="md-docs-v${VERSION}-macos.pkg"
pkgbuild \
    --root "$PKG_STAGING" \
    --install-location /usr/local/bin \
    --identifier com.hendemic.md-docs \
    --version "$VERSION" \
    "$RELEASE_DIR/$PKG_NAME"

echo "Created: dist/release/$PKG_NAME"

# Clean up staging directories
rm -rf "$UNIVERSAL_STAGING"
rm -rf "$PKG_STAGING"

# Summary
echo ""
echo "=== Build Summary ==="
echo "Version: ${VERSION}"
echo "Artifacts:"
for f in "$RELEASE_DIR"/*.tar.gz "$RELEASE_DIR"/*.pkg; do
    echo "  $(basename "$f")  ($(du -h "$f" | cut -f1))"
done
