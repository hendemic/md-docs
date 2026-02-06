#!/usr/bin/env bash
set -euo pipefail

# md-docs installer
# Installs md-docs into ~/.local/share/md-docs/app/ with a venv,
# and places a wrapper script at ~/.local/bin/mdocs.

APP_DIR="$HOME/.local/share/md-docs/app"
BIN_DIR="$HOME/.local/bin"
WRAPPER="$BIN_DIR/mdocs"
CONFIG_DIR="$HOME/.config/md-docs"
DATA_DIR="$HOME/.local/share/md-docs"
CACHE_DIR="$HOME/.cache/md-docs"

# --- Helpers ---

info()  { echo "  $*"; }
error() { echo "ERROR: $*" >&2; exit 1; }
warn()  { echo "WARN:  $*"; }

# --- Prerequisite checks ---

echo "Checking prerequisites..."

# Python >= 3.11
if ! command -v python3 &>/dev/null; then
    error "python3 not found. Install Python 3.11 or later."
fi

PYTHON_VERSION=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
PYTHON_MAJOR=$(echo "$PYTHON_VERSION" | cut -d. -f1)
PYTHON_MINOR=$(echo "$PYTHON_VERSION" | cut -d. -f2)

if [ "$PYTHON_MAJOR" -lt 3 ] || { [ "$PYTHON_MAJOR" -eq 3 ] && [ "$PYTHON_MINOR" -lt 11 ]; }; then
    error "Python 3.11+ required. Found: python3 $PYTHON_VERSION"
fi
info "python3 $PYTHON_VERSION"

# pdflatex (warn only)
if command -v pdflatex &>/dev/null; then
    PDFLATEX_VERSION=$(pdflatex --version | head -1)
    info "$PDFLATEX_VERSION"
else
    warn "pdflatex not found. Install texlive to generate PDFs."
    warn "  Arch: sudo pacman -S texlive-basic texlive-latexextra"
    warn "  Ubuntu/Debian: sudo apt install texlive-latex-base texlive-latex-extra"
    warn "  macOS: brew install --cask mactex-no-gui"
fi

# --- Determine source directory ---

# If running from the repo, use the repo as the source.
# Otherwise, the user needs to clone it first.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

if [ ! -f "$REPO_DIR/pyproject.toml" ]; then
    error "Cannot find pyproject.toml. Run this script from the md-docs repo."
fi

# Extract version from pyproject.toml
VERSION=$(grep -oP 'version\s*=\s*"\K[^"]+' "$REPO_DIR/pyproject.toml" || echo "unknown")

# --- Install ---

echo ""
echo "Installing md-docs v${VERSION}..."

# Clean previous install if present
if [ -d "$APP_DIR" ]; then
    info "Removing previous installation..."
    rm -rf "$APP_DIR"
fi

# Copy source to install location
mkdir -p "$APP_DIR"
cp -r "$REPO_DIR/src" "$APP_DIR/src"
cp -r "$REPO_DIR/utility" "$APP_DIR/utility"
cp "$REPO_DIR/pyproject.toml" "$APP_DIR/pyproject.toml"
cp "$REPO_DIR/modifiers.toml" "$APP_DIR/modifiers.toml"
info "Copied source to $APP_DIR"

# Create venv and install dependencies
python3 -m venv "$APP_DIR/.venv"
"$APP_DIR/.venv/bin/pip" install --quiet --upgrade pip
"$APP_DIR/.venv/bin/pip" install --quiet mistune pyyaml jinja2 simple-term-menu
info "Created venv and installed dependencies"

# --- Create wrapper script ---

mkdir -p "$BIN_DIR"
cat > "$WRAPPER" << 'EOF'
#!/usr/bin/env bash
APP_DIR="$HOME/.local/share/md-docs/app"
exec "$APP_DIR/.venv/bin/python" "$APP_DIR/src/cli.py" "$@"
EOF
chmod +x "$WRAPPER"
info "Created wrapper at $WRAPPER"

# --- Create XDG directories ---

mkdir -p "$CONFIG_DIR"
mkdir -p "$DATA_DIR/templates"
mkdir -p "$DATA_DIR/brands"
mkdir -p "$DATA_DIR/custom/templates"
mkdir -p "$DATA_DIR/custom/brands"
mkdir -p "$CACHE_DIR"
info "Created config/data/cache directories"

# Save installed version
echo "$VERSION" > "$DATA_DIR/.version"

# --- Done ---

echo ""
echo "md-docs installed successfully."
echo ""
echo "  Command:    $WRAPPER"
echo "  App dir:    $APP_DIR"
echo "  Config:     $CONFIG_DIR"
echo "  Templates:  $DATA_DIR/templates"
echo "  Brands:     $DATA_DIR/brands"
echo ""

# Check if ~/.local/bin is on PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo "NOTE: $BIN_DIR is not on your PATH."
    echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
    echo ""
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo ""
fi

echo "Next steps:"
echo "  mdocs doctor            # verify setup"
echo "  mdocs templates install # install templates from repo"
