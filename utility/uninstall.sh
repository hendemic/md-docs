#!/usr/bin/env bash
set -euo pipefail

# md-docs uninstaller
# Removes the md-docs installation. Optionally removes config and data.

APP_DIR="$HOME/.local/share/md-docs/app"
BIN_DIR="$HOME/.local/bin"
WRAPPER="$BIN_DIR/md-docs"
CONFIG_DIR="$HOME/.config/md-docs"
DATA_DIR="$HOME/.local/share/md-docs"
CACHE_DIR="$HOME/.cache/md-docs"

info()  { echo "  $*"; }

# --- Check for custom directories before removing anything ---

CUSTOM_TEMPLATES_DIR=""
CUSTOM_BRANDS_DIR=""
CUSTOM_USER_TEMPLATES_DIR=""
CUSTOM_USER_BRANDS_DIR=""
GLOBAL_CONFIG="$CONFIG_DIR/config.toml"

if [ -f "$GLOBAL_CONFIG" ]; then
    # Extract custom paths from config (simple grep — TOML values are quoted strings)
    _tmp=$(grep -oP 'templates_dir\s*=\s*"\K[^"]+' "$GLOBAL_CONFIG" 2>/dev/null || true)
    if [ -n "$_tmp" ] && [ "$_tmp" != "$DATA_DIR/templates" ]; then
        CUSTOM_TEMPLATES_DIR="$_tmp"
    fi
    _tmp=$(grep -oP 'brands_dir\s*=\s*"\K[^"]+' "$GLOBAL_CONFIG" 2>/dev/null || true)
    if [ -n "$_tmp" ] && [ "$_tmp" != "$DATA_DIR/brands" ]; then
        CUSTOM_BRANDS_DIR="$_tmp"
    fi
    _tmp=$(grep -oP 'custom_templates_dir\s*=\s*"\K[^"]+' "$GLOBAL_CONFIG" 2>/dev/null || true)
    if [ -n "$_tmp" ] && [ "$_tmp" != "$DATA_DIR/custom/templates" ]; then
        CUSTOM_USER_TEMPLATES_DIR="$_tmp"
    fi
    _tmp=$(grep -oP 'custom_brands_dir\s*=\s*"\K[^"]+' "$GLOBAL_CONFIG" 2>/dev/null || true)
    if [ -n "$_tmp" ] && [ "$_tmp" != "$DATA_DIR/custom/brands" ]; then
        CUSTOM_USER_BRANDS_DIR="$_tmp"
    fi
fi

echo "Uninstalling md-docs..."

# Remove wrapper script
if [ -f "$WRAPPER" ]; then
    rm "$WRAPPER"
    info "Removed $WRAPPER"
else
    info "Wrapper not found at $WRAPPER (skipping)"
fi

# Remove app directory (venv + source)
if [ -d "$APP_DIR" ]; then
    rm -rf "$APP_DIR"
    info "Removed $APP_DIR"
else
    info "App directory not found (skipping)"
fi

# Remove cache
if [ -d "$CACHE_DIR" ]; then
    rm -rf "$CACHE_DIR"
    info "Removed $CACHE_DIR"
fi

# Ask about config and data (templates/brands)
echo ""
TEMPLATES_COUNT=0
BRANDS_COUNT=0
if [ -d "$DATA_DIR/templates" ]; then
    TEMPLATES_COUNT=$(find "$DATA_DIR/templates" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
fi
if [ -d "$DATA_DIR/brands" ]; then
    BRANDS_COUNT=$(find "$DATA_DIR/brands" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
fi

HAS_DATA=false
if [ -d "$CONFIG_DIR" ] || { [ -d "$DATA_DIR" ] && { [ "$TEMPLATES_COUNT" -gt 0 ] || [ "$BRANDS_COUNT" -gt 0 ]; }; }; then
    HAS_DATA=true
fi

if [ "$HAS_DATA" = true ]; then
    echo "You have user data that was not removed:"
    [ -d "$CONFIG_DIR" ] && info "Config:    $CONFIG_DIR"
    [ "$TEMPLATES_COUNT" -gt 0 ] && info "Templates: $DATA_DIR/templates ($TEMPLATES_COUNT installed)"
    [ "$BRANDS_COUNT" -gt 0 ] && info "Brands:    $DATA_DIR/brands ($BRANDS_COUNT installed)"
    echo ""
    read -rp "Remove config and data too? [y/N] " answer
    if [[ "$answer" =~ ^[Yy]$ ]]; then
        [ -d "$CONFIG_DIR" ] && rm -rf "$CONFIG_DIR" && info "Removed $CONFIG_DIR"
        [ -d "$DATA_DIR" ] && rm -rf "$DATA_DIR" && info "Removed $DATA_DIR"
    else
        info "Kept user config and data."
    fi
else
    # No user data, clean up empty directories
    [ -d "$DATA_DIR" ] && rmdir --ignore-fail-on-non-empty "$DATA_DIR" 2>/dev/null || true
fi

# Notify about custom directories that were not touched
if [ -n "$CUSTOM_TEMPLATES_DIR" ] || [ -n "$CUSTOM_BRANDS_DIR" ] || \
   [ -n "$CUSTOM_USER_TEMPLATES_DIR" ] || [ -n "$CUSTOM_USER_BRANDS_DIR" ]; then
    echo ""
    echo "NOTE: You have custom directories configured that were not removed:"
    [ -n "$CUSTOM_TEMPLATES_DIR" ] && info "Templates dir:        $CUSTOM_TEMPLATES_DIR"
    [ -n "$CUSTOM_BRANDS_DIR" ] && info "Brands dir:           $CUSTOM_BRANDS_DIR"
    [ -n "$CUSTOM_USER_TEMPLATES_DIR" ] && info "Custom templates dir: $CUSTOM_USER_TEMPLATES_DIR"
    [ -n "$CUSTOM_USER_BRANDS_DIR" ] && info "Custom brands dir:    $CUSTOM_USER_BRANDS_DIR"
    echo "  You may want to remove these manually."
fi

echo ""
echo "md-docs uninstalled."
