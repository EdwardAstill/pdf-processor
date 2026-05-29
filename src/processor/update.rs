use anyhow::{bail, Context, Result};
use std::cmp::Ordering;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cli::UpdateArgs;

const REPO: &str = "EdwardAstill/pdf-processor";
const BIN_NAME: &str = "pdfp";

#[derive(serde::Deserialize)]
struct GithubRelease {
    #[serde(rename = "tag_name")]
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(serde::Deserialize)]
struct GithubAsset {
    name: String,
    #[serde(rename = "browser_download_url")]
    browser_download_url: String,
}

pub fn run(args: &UpdateArgs) -> Result<()> {
    let current_ver = env!("CARGO_PKG_VERSION");
    let target = detect_target()?;

    // Fetch latest release info from GitHub API
    let release_json = run_curl(&[
        "-fsSL",
        &format!("https://api.github.com/repos/{}/releases/latest", REPO),
    ])
    .context("Failed to fetch latest release info from GitHub")?;

    let release: GithubRelease =
        serde_json::from_str(&release_json).context("Failed to parse GitHub release response")?;

    let latest_tag = &release.tag_name;
    let latest_ver = latest_tag.strip_prefix('v').unwrap_or(latest_tag);

    // Compare versions
    let cmp = compare_versions(current_ver, latest_ver);
    if cmp != Ordering::Less && !args.force {
        println!("pdfp is already up to date (v{})", current_ver);
        return Ok(());
    }

    if args.check {
        println!(
            "New version available: v{} (current: v{})",
            latest_ver, current_ver
        );
        return Ok(());
    }

    // Find the right asset in the release
    let asset_name = format!("pdf-processor-{}.tar.gz", target);
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .with_context(|| {
            format!(
                "Asset '{}' not found in latest release v{}",
                asset_name, latest_tag
            )
        })?;

    // Determine where to install
    let exe_path = std::env::current_exe()?;
    let real_path = exe_path
        .canonicalize()
        .context("Failed to resolve current binary path")?;
    let install_root = find_install_root(&real_path)?;

    // Create temp directory for download and extraction
    let tmp_dir = create_temp_dir()?;
    let archive_path = tmp_dir.join(&asset_name);

    println!("Downloading pdfp v{}...", latest_ver);

    run_curl(&[
        "-fSL",
        "-o",
        &archive_path.to_string_lossy(),
        &asset.browser_download_url,
    ])
    .context("Failed to download release archive")?;

    // Extract the archive
    let extract_dir = tmp_dir.join("extracted");
    std::fs::create_dir(&extract_dir).context("Failed to create extraction directory")?;
    extract_tar_gz(&archive_path, &extract_dir).context("Failed to extract release archive")?;

    // Replace the binary
    let extracted_bin = extract_dir.join("bin").join(BIN_NAME);
    let target_bin = install_root.join("bin").join(BIN_NAME);

    if !extracted_bin.exists() {
        bail!("Extracted archive does not contain bin/{}", BIN_NAME);
    }

    std::fs::create_dir_all(target_bin.parent().unwrap())
        .context("Failed to create bin directory")?;
    std::fs::copy(&extracted_bin, &target_bin)
        .with_context(|| format!("Failed to write {}", target_bin.display()))?;
    std::fs::set_permissions(&target_bin, std::fs::Permissions::from_mode(0o755))?;

    println!("✅ pdfp updated to v{}", latest_ver);

    // Clean up temp dir
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(())
}

/// Map the current platform to the release target triple used in asset names.
fn detect_target() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("linux", "x86_64") => Ok("x86_64-linux".to_string()),
        ("linux", "aarch64") => Ok("aarch64-linux".to_string()),
        ("macos", "x86_64") => Ok("x86_64-macos".to_string()),
        ("macos", "aarch64") => Ok("aarch64-macos".to_string()),
        _ => bail!(
            "Unsupported platform: {}-{}. pdfp only provides pre-built binaries for Linux x86_64, \
             Linux aarch64, macOS x86_64, and macOS aarch64.",
            os,
            arch
        ),
    }
}

/// Run curl with the given arguments and return stdout as a String.
fn run_curl(args: &[&str]) -> Result<String> {
    let output = Command::new("curl")
        .args(args)
        .output()
        .with_context(|| {
            "Failed to run curl. Is curl installed? Try running the install script directly:\n\
             curl -fsSL https://github.com/EdwardAstill/pdf-processor/releases/latest/download/install.sh | sh"
                .to_string()
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("curl failed: {}", stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Extract a .tar.gz archive into the destination directory using tar(1).
fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<()> {
    let output = Command::new("tar")
        .args([
            "-xzf",
            &archive.to_string_lossy(),
            "-C",
            &dest.to_string_lossy(),
        ])
        .output()
        .with_context(|| "Failed to run tar; is tar installed?".to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("tar extraction failed: {}", stderr.trim());
    }

    Ok(())
}

/// Create a uniquely-named temp directory under the system temp dir.
fn create_temp_dir() -> Result<PathBuf> {
    let base = std::env::temp_dir().join(format!("pdfp-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base)
        .with_context(|| format!("Failed to create temp directory at {}", base.display()))?;
    Ok(base)
}

/// Determine the install root from the binary's real path.
///
/// The standard install layout is:
///   ~/.local/share/pdfp/bin/pdfp
///   ~/.local/share/pdfp/tools/...
///
/// If the binary lives outside a known structure, we use its parent directory
/// as the target binary directory and return its grandparent as the root.
fn find_install_root(exe_path: &Path) -> Result<PathBuf> {
    // Resolve symlinks to get the real path
    let path = if exe_path.exists() {
        exe_path.canonicalize()?
    } else {
        // For tests with fictional paths, use the path as-is
        exe_path.to_path_buf()
    };
    // Walk up looking for a bin/ parent
    if let Some(parent) = path.parent() {
        if parent.file_name().and_then(|n| n.to_str()) == Some("bin") {
            // Standard install layout: .../pdfp/bin/pdfp
            if let Some(root) = parent.parent() {
                return Ok(root.to_path_buf());
            }
        }
    }
    // Fallback: use the binary's directory parent as the install root
    Ok(path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/local")))
}

fn compare_versions(a: &str, b: &str) -> Ordering {
    let a_parts: Vec<u64> = a.split('.').filter_map(|s| s.parse().ok()).collect();
    let b_parts: Vec<u64> = b.split('.').filter_map(|s| s.parse().ok()).collect();

    for i in 0..a_parts.len().max(b_parts.len()) {
        let a_val = a_parts.get(i).copied().unwrap_or(0);
        let b_val = b_parts.get(i).copied().unwrap_or(0);
        if a_val != b_val {
            return if a_val > b_val {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_target_on_ci() {
        // We don't check the actual value (depends on where tests run),
        // just that it doesn't panic on common platforms.
        let _ = detect_target();
    }

    #[test]
    fn compare_versions_equal() {
        assert_eq!(compare_versions("0.4.1", "0.4.1"), Ordering::Equal);
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Ordering::Equal);
    }

    #[test]
    fn compare_versions_newer() {
        assert_eq!(compare_versions("0.4.2", "0.4.1"), Ordering::Greater);
        assert_eq!(compare_versions("0.5.0", "0.4.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0", "0.9.9"), Ordering::Greater);
    }

    #[test]
    fn compare_versions_older() {
        assert_eq!(compare_versions("0.4.0", "0.4.1"), Ordering::Less);
        assert_eq!(compare_versions("0.3.9", "0.4.0"), Ordering::Less);
    }

    #[test]
    fn compare_versions_different_lengths() {
        assert_eq!(compare_versions("0.4", "0.4.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.4", "0.4.1"), Ordering::Less);
    }

    #[test]
    fn find_install_root_bin_parent() {
        let path = Path::new("/home/user/.local/share/pdfp/bin/pdfp");
        let root = find_install_root(path).expect("should resolve root");
        assert_eq!(root, Path::new("/home/user/.local/share/pdfp"));
    }

    #[test]
    fn find_install_root_fallback() {
        let path = Path::new("/usr/local/bin/pdfp");
        let root = find_install_root(path).expect("should resolve root");
        assert_eq!(root, Path::new("/usr/local"));
    }
}
