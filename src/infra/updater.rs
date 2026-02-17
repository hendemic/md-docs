//! Self-update infrastructure: version checking and binary replacement.
//!
//! Handles all I/O for the `md-docs update` command:
//! - Fetching the latest release from GitHub
//! - Comparing versions
//! - Downloading and replacing the current binary

use std::path::{Path, PathBuf};

use anyhow::{bail, Context};

/// Information about a GitHub release.
#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    /// The git tag name (e.g. "v0.2.0").
    pub tag_name: String,
    /// The parsed semantic version.
    pub version: semver::Version,
    /// Downloadable assets attached to the release.
    pub assets: Vec<ReleaseAsset>,
}

/// A single downloadable asset from a GitHub release.
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    /// Filename of the asset (e.g. "md-docs-v0.2.0-linux-x86_64.tar.gz").
    pub name: String,
    /// Direct download URL.
    pub download_url: String,
}

/// Result of comparing the current version against the latest release.
#[derive(Debug)]
pub enum UpdateCheck {
    /// Current version matches or exceeds the latest release.
    UpToDate(semver::Version),
    /// A newer version is available.
    UpdateAvailable {
        current: semver::Version,
        latest: semver::Version,
        release: ReleaseInfo,
    },
    /// The binary was installed via the system package manager (AUR).
    AurInstall,
}

/// Return the current binary's version parsed from `Cargo.toml` at compile time.
pub fn current_version() -> semver::Version {
    // CARGO_PKG_VERSION is guaranteed to be valid semver by Cargo.
    semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .expect("CARGO_PKG_VERSION is not valid semver")
}

/// Check whether the running binary was installed via pacman (AUR).
///
/// Only meaningful on Linux. Returns `false` on all other platforms.
pub fn is_aur_install() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };

    std::process::Command::new("pacman")
        .arg("-Qo")
        .arg(&exe)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Fetch the latest release metadata from the GitHub API.
pub fn fetch_latest_release() -> anyhow::Result<ReleaseInfo> {
    let url = "https://api.github.com/repos/hendemic/md-docs/releases/latest";

    let body = ureq::get(url)
        .header("User-Agent", "md-docs-updater")
        .call()
        .context("Failed to fetch latest release from GitHub")?
        .into_body()
        .read_to_string()
        .context("Failed to read GitHub API response body")?;

    let json: serde_json::Value =
        serde_json::from_str(&body).context("Failed to parse GitHub API response as JSON")?;

    let tag_name = json["tag_name"]
        .as_str()
        .context("GitHub release missing 'tag_name'")?
        .to_string();

    let version_str = tag_name.strip_prefix('v').unwrap_or(&tag_name);
    let version = semver::Version::parse(version_str)
        .with_context(|| format!("Invalid version in tag '{}': '{}'", tag_name, version_str))?;

    let assets = json["assets"]
        .as_array()
        .context("GitHub release missing 'assets' array")?
        .iter()
        .filter_map(|a| {
            let name = a["name"].as_str()?.to_string();
            let download_url = a["browser_download_url"].as_str()?.to_string();
            Some(ReleaseAsset { name, download_url })
        })
        .collect();

    Ok(ReleaseInfo {
        tag_name,
        version,
        assets,
    })
}

/// Check whether an update is available.
///
/// Returns `AurInstall` if the binary was installed via the system package manager,
/// `UpToDate` if already on the latest version, or `UpdateAvailable` with the
/// release info otherwise.
pub fn check_for_update() -> anyhow::Result<UpdateCheck> {
    if is_aur_install() {
        return Ok(UpdateCheck::AurInstall);
    }

    let release = fetch_latest_release()?;
    let current = current_version();

    if current >= release.version {
        Ok(UpdateCheck::UpToDate(current))
    } else {
        Ok(UpdateCheck::UpdateAvailable {
            current,
            latest: release.version.clone(),
            release,
        })
    }
}

/// Download the appropriate binary for this platform and replace the current executable.
pub fn perform_update(release: &ReleaseInfo) -> anyhow::Result<()> {
    let asset_name = platform_asset_name(&release.tag_name)?;

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| {
            let available: Vec<&str> = release.assets.iter().map(|a| a.name.as_str()).collect();
            format!(
                "No matching asset '{}' in release. Available: {:?}",
                asset_name, available
            )
        })?;

    // Download the archive into a temp directory
    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let archive_path = tmp_dir.path().join(&asset.name);

    download_file(&asset.download_url, &archive_path)?;

    // Extract the tar.gz archive
    let status = std::process::Command::new("tar")
        .arg("-xzf")
        .arg(&archive_path)
        .current_dir(tmp_dir.path())
        .status()
        .context("Failed to run 'tar' to extract archive")?;

    if !status.success() {
        bail!("tar extraction failed with exit code: {:?}", status.code());
    }

    // Find the md-docs binary in the extracted contents
    let new_binary = find_binary(tmp_dir.path())?;

    // Replace the current binary
    let current_exe = std::env::current_exe().context("Failed to determine current executable")?;
    let current_exe = current_exe
        .canonicalize()
        .context("Failed to canonicalize current executable path")?;

    // Atomic replacement: copy to temp location next to target, set permissions,
    // then rename. rename() on the same filesystem is atomic on Linux/macOS,
    // so a crash mid-update won't leave a corrupted binary.
    let tmp_dest = current_exe.with_extension("update-tmp");

    match std::fs::copy(&new_binary, &tmp_dest) {
        Ok(_) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755);
                std::fs::set_permissions(&tmp_dest, perms)
                    .context("Failed to set executable permissions")?;
            }
            std::fs::rename(&tmp_dest, &current_exe).with_context(|| {
                // Clean up the temp file if rename fails
                let _ = std::fs::remove_file(&tmp_dest);
                format!("Failed to replace binary at '{}'", current_exe.display())
            })?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            bail!(
                "Permission denied writing to '{}'. \
                 Try running with elevated permissions, or manually copy:\n  \
                 cp {} {}",
                current_exe.display(),
                new_binary.display(),
                current_exe.display()
            );
        }
        Err(e) => {
            return Err(e).context(format!(
                "Failed to copy new binary to '{}'",
                current_exe.display()
            ));
        }
    }

    Ok(())
}

/// Determine the expected asset filename for this platform.
fn platform_asset_name(tag: &str) -> anyhow::Result<String> {
    let version = tag.strip_prefix('v').unwrap_or(tag);

    if cfg!(target_os = "macos") {
        Ok(format!("md-docs-v{}-macos-universal.tar.gz", version))
    } else if cfg!(target_os = "linux") {
        let arch = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else {
            bail!("Unsupported architecture for self-update");
        };
        Ok(format!("md-docs-v{}-linux-{}.tar.gz", version, arch))
    } else {
        bail!("Self-update is not supported on this platform");
    }
}

/// Download a file from a URL to a local path.
fn download_file(url: &str, dest: &Path) -> anyhow::Result<()> {
    let buf = ureq::get(url)
        .header("User-Agent", "md-docs-updater")
        .call()
        .with_context(|| format!("Failed to download '{}'", url))?
        .into_body()
        .read_to_vec()
        .context("Failed to read download response body")?;

    std::fs::write(dest, &buf)
        .with_context(|| format!("Failed to write downloaded file to '{}'", dest.display()))?;

    Ok(())
}

/// Find the `md-docs` binary in an extracted archive directory.
///
/// Searches the directory (non-recursively first, then one level deep) for
/// a file named `md-docs`.
fn find_binary(dir: &std::path::Path) -> anyhow::Result<PathBuf> {
    // Check directly in the directory
    let direct = dir.join("md-docs");
    if direct.is_file() {
        return Ok(direct);
    }

    // Check one level deep (archives often have a top-level directory)
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let nested = entry.path().join("md-docs");
                if nested.is_file() {
                    return Ok(nested);
                }
            }
        }
    }

    bail!(
        "Could not find 'md-docs' binary in extracted archive at '{}'",
        dir.display()
    )
}
