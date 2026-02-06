#!/usr/bin/env bash
set -euo pipefail

# md-docs updater
# Pulls the latest md-docs source from GitHub and reinstalls.
#
# Usage:
#   update.sh        — update to latest stable release (main branch)
#   update.sh dev    — update to latest development build (development branch)

REPO_URL="https://github.com/hendemic/md-docs.git"
APP_DIR="$HOME/.local/share/md-docs/app"
DATA_DIR="$HOME/.local/share/md-docs"
CACHE_DIR="$HOME/.cache/md-docs"
REPO_CACHE="$CACHE_DIR/md-docs"

# --- Parse arguments ---

BRANCH="main"
DEV_MODE=false

if [ "${1:-}" = "dev" ]; then
    BRANCH="development"
    DEV_MODE=true
fi

# --- Helpers ---

info()  { echo "  $*"; }
error() { echo "ERROR: $*" >&2; exit 1; }

# --- Prerequisite checks ---

if ! command -v git &>/dev/null; then
    error "git not found. Install git to update md-docs."
fi

if [ ! -d "$APP_DIR" ]; then
    error "md-docs is not installed. Run install.sh first."
fi

if [ ! -f "$APP_DIR/.venv/bin/python" ]; then
    error "venv not found at $APP_DIR/.venv. Run install.sh to reinstall."
fi

# --- Read current version ---

OLD_VERSION="unknown"
if [ -f "$DATA_DIR/.version" ]; then
    OLD_VERSION=$(cat "$DATA_DIR/.version")
fi

# --- Fetch latest source ---

if [ "$DEV_MODE" = true ]; then
    echo "Updating md-docs (development branch)..."
else
    echo "Updating md-docs..."
fi

if [ -d "$REPO_CACHE/.git" ]; then
    info "Fetching from $REPO_URL..."
    git -C "$REPO_CACHE" fetch --quiet origin
    git -C "$REPO_CACHE" checkout --quiet "$BRANCH"
    git -C "$REPO_CACHE" pull --quiet origin "$BRANCH"
else
    info "Cloning $REPO_URL..."
    mkdir -p "$CACHE_DIR"
    git clone --quiet "$REPO_URL" "$REPO_CACHE"
    git -C "$REPO_CACHE" checkout --quiet "$BRANCH"
fi

# --- Extract new version ---

NEW_VERSION=$(grep -oP 'version\s*=\s*"\K[^"]+' "$REPO_CACHE/pyproject.toml" || echo "unknown")

# In dev mode, always update regardless of version
# In stable mode, skip if version unchanged
if [ "$DEV_MODE" = false ] && [ "$OLD_VERSION" = "$NEW_VERSION" ]; then
    echo ""
    echo "Already up to date (v${NEW_VERSION})."
    exit 0
fi

# --- Copy source ---

rm -rf "$APP_DIR/src"
cp -r "$REPO_CACHE/src" "$APP_DIR/src"
cp "$REPO_CACHE/pyproject.toml" "$APP_DIR/pyproject.toml"
cp "$REPO_CACHE/modifiers.toml" "$APP_DIR/modifiers.toml"
info "Copied source to $APP_DIR"

# --- Update dependencies ---

"$APP_DIR/.venv/bin/pip" install --quiet --upgrade pip
"$APP_DIR/.venv/bin/pip" install --quiet mistune pyyaml jinja2 simple-term-menu
info "Updated dependencies"

# --- Save version ---

if [ "$DEV_MODE" = true ]; then
    # For dev builds, include branch and short commit hash
    COMMIT_HASH=$(git -C "$REPO_CACHE" rev-parse --short HEAD)
    echo "${NEW_VERSION}-dev+${COMMIT_HASH}" > "$DATA_DIR/.version"
    echo ""
    echo "md-docs updated to development build: ${NEW_VERSION}-dev+${COMMIT_HASH}"
else
    echo "$NEW_VERSION" > "$DATA_DIR/.version"
    echo ""
    echo "md-docs updated: v${OLD_VERSION} -> v${NEW_VERSION}"
fi
