#!/usr/bin/env bash
set -euo pipefail

# Build Linux release binaries for md-docs using cross-compilation.
# Produces tar.gz archives in dist/release/ for each target architecture.

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
for tool in cross tar; do
    if ! command -v "$tool" &>/dev/null; then
        echo "ERROR: Required tool '$tool' is not installed."
        echo "  Install cross: cargo install cross --git https://github.com/cross-rs/cross"
        exit 1
    fi
done

# Define targets
TARGETS=(
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-musl"
)

# Create/clean output directory
RELEASE_DIR="$PROJECT_ROOT/dist/release"
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

# Build each target
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

    # Cross-compile
    cross build --release --target "$target"

    # Stage the binary
    STAGING_DIR="$RELEASE_DIR/md-docs-v${VERSION}-linux-${arch}"
    mkdir -p "$STAGING_DIR"
    cp "target/${target}/release/md-docs" "$STAGING_DIR/"

    # Create archive
    ARCHIVE_NAME="md-docs-v${VERSION}-linux-${arch}.tar.gz"
    tar -czf "$RELEASE_DIR/$ARCHIVE_NAME" -C "$RELEASE_DIR" "md-docs-v${VERSION}-linux-${arch}"

    # Clean up staging directory
    rm -rf "$STAGING_DIR"

    echo "Created: dist/release/$ARCHIVE_NAME"
done

# Summary
echo ""
echo "=== Build Summary ==="
echo "Version: ${VERSION}"
echo "Artifacts:"
for f in "$RELEASE_DIR"/*.tar.gz; do
    echo "  $(basename "$f")  ($(du -h "$f" | cut -f1))"
done
