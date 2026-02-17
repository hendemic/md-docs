#!/bin/sh
set -eu

# Install script for md-docs
# Usage: curl -fsSL https://raw.githubusercontent.com/hendemic/md-docs/main/dist/install.sh | sh

REPO="hendemic/md-docs"
BINARY_NAME="mdocs"
INSTALL_DIR="${HOME}/.local/bin"

# --- Helpers ---

info() {
    printf "  \033[1;34m>\033[0m %s\n" "$1"
}

success() {
    printf "  \033[1;32m>\033[0m %s\n" "$1"
}

error() {
    printf "  \033[1;31merror:\033[0m %s\n" "$1" >&2
}

# --- Cleanup trap ---

TEMP_DIR=""

cleanup() {
    if [ -n "$TEMP_DIR" ] && [ -d "$TEMP_DIR" ]; then
        rm -rf "$TEMP_DIR"
    fi
}

trap cleanup EXIT INT TERM

# --- Platform detection ---

detect_platform() {
    local os arch

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)  os="linux" ;;
        Darwin) os="macos" ;;
        *)
            error "Unsupported operating system: $os"
            printf "\n" >&2
            printf "  Supported platforms:\n" >&2
            printf "    - Linux (x86_64, aarch64)\n" >&2
            printf "    - macOS (x86_64, Apple Silicon)\n" >&2
            exit 1
            ;;
    esac

    case "$arch" in
        x86_64)         arch="x86_64" ;;
        aarch64|arm64)  arch="aarch64" ;;
        *)
            error "Unsupported architecture: $arch"
            printf "\n" >&2
            printf "  Supported architectures:\n" >&2
            printf "    - x86_64\n" >&2
            printf "    - aarch64 / arm64\n" >&2
            exit 1
            ;;
    esac

    # macOS uses a universal binary regardless of architecture
    if [ "$os" = "macos" ]; then
        PLATFORM_OS="macos"
        PLATFORM_ARCH="universal"
    else
        PLATFORM_OS="$os"
        PLATFORM_ARCH="$arch"
    fi
}

# --- Download tool detection ---

detect_downloader() {
    if command -v curl >/dev/null 2>&1; then
        DOWNLOADER="curl"
    elif command -v wget >/dev/null 2>&1; then
        DOWNLOADER="wget"
    else
        error "Neither curl nor wget found."
        printf "\n" >&2
        printf "  Install one of the following:\n" >&2
        printf "    - curl: https://curl.se/\n" >&2
        printf "    - wget: https://www.gnu.org/software/wget/\n" >&2
        exit 1
    fi
}

# Download a URL to stdout
download() {
    local url="$1"
    if [ "$DOWNLOADER" = "curl" ]; then
        curl -fsSL "$url"
    else
        wget -qO- "$url"
    fi
}

# Download a URL to a file
download_to_file() {
    local url="$1"
    local dest="$2"
    if [ "$DOWNLOADER" = "curl" ]; then
        curl -fsSL -o "$dest" "$url"
    else
        wget -q -O "$dest" "$url"
    fi
}

# --- Check required tools ---

check_dependencies() {
    if ! command -v tar >/dev/null 2>&1; then
        error "Required tool 'tar' is not installed."
        exit 1
    fi
}

# --- Fetch latest version ---

fetch_latest_version() {
    local api_url="https://api.github.com/repos/${REPO}/releases/latest"
    local response

    info "Fetching latest release information..."

    response="$(download "$api_url" 2>/dev/null)" || {
        error "Failed to fetch release information from GitHub."
        printf "\n" >&2
        printf "  URL: %s\n" "$api_url" >&2
        printf "  Check your network connection and try again.\n" >&2
        exit 1
    }

    # Extract tag_name from JSON response (handles "tag_name": "v1.2.3" format)
    # Try to use a simple approach that works with basic tools
    VERSION="$(printf '%s' "$response" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"

    if [ -z "$VERSION" ]; then
        error "Could not determine latest version from GitHub API response."
        printf "\n" >&2
        printf "  This might mean there are no releases yet.\n" >&2
        printf "  Check: https://github.com/%s/releases\n" "$REPO" >&2
        exit 1
    fi

    info "Latest version: ${VERSION}"
}

# --- Download and install ---

download_and_install() {
    local archive_name="md-docs-${VERSION}-${PLATFORM_OS}-${PLATFORM_ARCH}.tar.gz"
    local download_url="https://github.com/${REPO}/releases/download/${VERSION}/${archive_name}"
    local extract_dir="md-docs-${VERSION}-${PLATFORM_OS}-${PLATFORM_ARCH}"

    TEMP_DIR="$(mktemp -d)"

    info "Downloading ${archive_name}..."

    download_to_file "$download_url" "${TEMP_DIR}/${archive_name}" 2>/dev/null || {
        error "Failed to download release archive."
        printf "\n" >&2
        printf "  URL: %s\n" "$download_url" >&2
        printf "  The release archive might not exist for your platform.\n" >&2
        printf "  Check available assets: https://github.com/%s/releases/tag/%s\n" "$REPO" "$VERSION" >&2
        exit 1
    }

    info "Extracting..."

    tar -xzf "${TEMP_DIR}/${archive_name}" -C "${TEMP_DIR}" 2>/dev/null || {
        error "Failed to extract archive. The download may be corrupted."
        exit 1
    }

    # The binary is inside a directory within the archive
    local binary_path="${TEMP_DIR}/${extract_dir}/${BINARY_NAME}"

    if [ ! -f "$binary_path" ]; then
        # Try finding the binary directly in the temp dir (fallback)
        binary_path="${TEMP_DIR}/${BINARY_NAME}"
    fi

    if [ ! -f "$binary_path" ]; then
        error "Could not find the ${BINARY_NAME} binary in the extracted archive."
        exit 1
    fi

    # Create install directory if it doesn't exist
    if [ ! -d "$INSTALL_DIR" ]; then
        info "Creating directory ${INSTALL_DIR}..."
        mkdir -p "$INSTALL_DIR"
    fi

    # Install the binary
    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
    cp "$binary_path" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
}

# --- Check PATH ---

check_path() {
    case ":${PATH}:" in
        *":${INSTALL_DIR}:"*)
            # Already on PATH
            ;;
        *)
            printf "\n"
            printf "  \033[1;33mNote:\033[0m %s is not on your PATH.\n" "$INSTALL_DIR"
            printf "  Add it by appending this to your shell profile:\n"
            printf "\n"
            printf "    export PATH=\"\$HOME/.local/bin:\$PATH\"\n"
            printf "\n"
            ;;
    esac
}

# --- Main ---

main() {
    printf "\n"
    printf "  \033[1mmd-docs installer\033[0m\n"
    printf "\n"

    detect_platform
    detect_downloader
    check_dependencies
    fetch_latest_version
    download_and_install

    printf "\n"
    success "Successfully installed ${BINARY_NAME} ${VERSION} to ${INSTALL_DIR}/${BINARY_NAME}"
    printf "\n"

    check_path
}

main
