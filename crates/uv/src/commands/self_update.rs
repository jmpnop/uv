//! `uv self update` for the fork.
//!
//! Queries `https://api.github.com/repos/jmpnop/uv/releases` for the latest (or specified) tag,
//! compares it against the currently-running binary's `CARGO_PKG_VERSION`, and — if an update is
//! needed — downloads and executes the installer script published with that release. The installer
//! overwrites the binary at the same location the user originally installed into, which we
//! recover from [`std::env::current_exe`].
//!
//! The implementation intentionally talks *only* to `github.com/jmpnop/uv`: no astral mirror, no
//! PyPI fallback, no axoupdater receipt. Users who installed this fork via `install.sh` /
//! `install.ps1` (the scripts shipped in every release) are the only supported target.
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result, anyhow, bail};
use owo_colors::OwoColorize;
use serde::Deserialize;
use tempfile::TempDir;
use tokio::process::Command;

use uv_client::BaseClientBuilder;
use uv_pep440::Version as Pep440Version;
use uv_redacted::DisplaySafeUrl;

use crate::commands::ExitStatus;
use crate::printer::Printer;

/// The GitHub repository this fork publishes releases to.
const FORK_REPO: &str = "jmpnop/uv";

/// Name of the installer script attached to every release (matches `install.sh` in the repo).
#[cfg(not(windows))]
const INSTALLER_FILENAME: &str = "uv-installer.sh";
#[cfg(windows)]
const INSTALLER_FILENAME: &str = "uv-installer.ps1";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
}

/// Run `uv self update`.
pub(crate) async fn self_update(
    version: Option<String>,
    token: Option<String>,
    dry_run: bool,
    printer: Printer,
    client_builder: BaseClientBuilder<'_>,
) -> Result<ExitStatus> {
    if client_builder.is_offline() {
        writeln!(
            printer.stderr_important(),
            "{}{} Self-update is not possible because network connectivity is disabled (i.e., with `--offline`)",
            "error".red().bold(),
            ":".bold()
        )?;
        return Ok(ExitStatus::Failure);
    }

    // If `publish-release.sh` set `UV_FORK_VERSION` at build time (e.g.
    // `UV_FORK_VERSION=v0.11.7-fork.1`), use it as the identity of the running binary so
    // self-update compares like-with-like against the release tag. Falls back to
    // `CARGO_PKG_VERSION` for locally-built binaries that weren't produced by the release script.
    let current = match option_env!("UV_FORK_VERSION") {
        Some(tag) if !tag.is_empty() => parse_tag_as_version(tag).with_context(|| {
            format!("UV_FORK_VERSION (`{tag}`) set at build time does not parse as a version")
        })?,
        _ => Pep440Version::from_str(env!("CARGO_PKG_VERSION"))
            .context("failed to parse the current uv version")?,
    };

    let client = client_builder.build()?;

    writeln!(
        printer.stderr(),
        "{}{} Checking for updates...",
        "info".cyan().bold(),
        ":".bold()
    )?;

    let target_tag = match version.as_deref() {
        Some(explicit) => normalize_tag(explicit),
        None => fetch_latest_release_tag(&client, token.as_deref())
            .await
            .context("failed to look up the latest release on GitHub")?,
    };

    let target = parse_tag_as_version(&target_tag)
        .with_context(|| format!("the release tag `{target_tag}` does not parse as a version"))?;

    if !is_update_needed(&current, &target, version.is_some()) {
        writeln!(
            printer.stderr(),
            "{}{} You're already on version {} of uv{}.",
            "success".green().bold(),
            ":".bold(),
            format!("v{current}").bold().cyan(),
            if version.is_none() {
                " (the latest version)"
            } else {
                ""
            }
        )?;
        return Ok(ExitStatus::Success);
    }

    if dry_run {
        writeln!(
            printer.stderr_important(),
            "Would update uv from {} to {}",
            format!("v{current}").bold().white(),
            format!("v{target}").bold().white(),
        )?;
        return Ok(ExitStatus::Success);
    }

    // Resolve the installer URL and the install directory.
    let installer_url = format!(
        "https://github.com/{FORK_REPO}/releases/download/{target_tag}/{INSTALLER_FILENAME}"
    );
    let install_dir = installed_binary_parent().context(
        "failed to determine where the current uv binary lives; re-run the installer manually",
    )?;

    let temp_dir = TempDir::new()?;
    let installer_path = temp_dir.path().join(INSTALLER_FILENAME);

    writeln!(
        printer.stderr(),
        "{}{} Downloading {}",
        "info".cyan().bold(),
        ":".bold(),
        target_tag.bold().cyan(),
    )?;
    download_installer(&installer_url, &installer_path, &client, token.as_deref()).await?;

    writeln!(
        printer.stderr(),
        "{}{} Installing to {}",
        "info".cyan().bold(),
        ":".bold(),
        install_dir.display().to_string().bold().cyan(),
    )?;
    run_installer(&installer_path, &install_dir, &target_tag).await?;

    writeln!(
        printer.stderr(),
        "{}{} Updated uv to {}",
        "success".green().bold(),
        ":".bold(),
        format!("v{target}").bold().cyan(),
    )?;
    Ok(ExitStatus::Success)
}

/// Fetch the `tag_name` of the most recent non-draft, non-prerelease release on the fork.
async fn fetch_latest_release_tag(
    client: &uv_client::BaseClient,
    token: Option<&str>,
) -> Result<String> {
    // Use `/releases` (not `/releases/latest`) so that a very young release still visible in the
    // UI but not yet marked "latest" by GitHub also works.
    let url = DisplaySafeUrl::parse(&format!(
        "https://api.github.com/repos/{FORK_REPO}/releases?per_page=10"
    ))?;

    let mut req = client
        .raw_client()
        .get(url.as_ref())
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "uv-self-update");
    if let Some(token) = token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }

    let response = req.send().await.context("failed to reach github.com")?;
    if !response.status().is_success() {
        bail!(
            "github.com returned HTTP {} when listing releases for `{FORK_REPO}`",
            response.status()
        );
    }
    let releases: Vec<GitHubRelease> = response
        .json()
        .await
        .context("invalid JSON from github.com")?;

    releases
        .into_iter()
        .find(|r| !r.draft && !r.prerelease)
        .map(|r| r.tag_name)
        .ok_or_else(|| anyhow!("no stable releases published yet for `{FORK_REPO}`"))
}

/// Download `url` to `dest`, authenticated with `token` if provided.
async fn download_installer(
    url: &str,
    dest: &Path,
    client: &uv_client::BaseClient,
    token: Option<&str>,
) -> Result<()> {
    let mut req = client
        .raw_client()
        .get(url)
        .header("User-Agent", "uv-self-update");
    if let Some(token) = token {
        req = req.header("Authorization", format!("Bearer {token}"));
    }
    let response = req
        .send()
        .await
        .with_context(|| format!("failed to GET {url}"))?;
    if !response.status().is_success() {
        bail!("download failed: HTTP {} from {url}", response.status());
    }
    let bytes = response.bytes().await?;
    let mut file = fs_err::File::create(dest)?;
    file.write_all(&bytes)?;
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        fs_err::set_permissions(dest, perms)?;
    }
    Ok(())
}

/// Invoke the downloaded installer with `UV_INSTALL_DIR` pointed at the current install location
/// and `UV_VERSION` pinned to the target tag.
async fn run_installer(installer: &Path, install_dir: &Path, tag: &str) -> Result<()> {
    #[cfg(not(windows))]
    let mut command = {
        let mut c = Command::new("sh");
        c.arg(installer);
        c
    };
    #[cfg(windows)]
    let mut command = {
        let mut c = Command::new("powershell.exe");
        c.args(["-ExecutionPolicy", "ByPass", "-File"])
            .arg(installer);
        c
    };

    command
        .env("UV_INSTALL_DIR", install_dir)
        .env("UV_VERSION", tag);

    let status = command
        .status()
        .await
        .context("failed to launch the installer script")?;
    if !status.success() {
        bail!("installer exited with status {status}");
    }
    Ok(())
}

/// Resolve the directory containing the currently-running `uv` binary.
fn installed_binary_parent() -> Result<PathBuf> {
    let current = std::env::current_exe().context("std::env::current_exe failed")?;
    let canonical = dunce::canonicalize(&current).unwrap_or(current);
    canonical
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("current executable has no parent directory"))
}

/// Convert an explicit user-supplied version to a tag form (`0.11.7` → `v0.11.7`, but `v0.11.7` /
/// `v0.11.7-fork.1` pass through).
fn normalize_tag(input: &str) -> String {
    if input.starts_with('v') {
        input.to_owned()
    } else {
        format!("v{input}")
    }
}

/// Parse a release tag like `v0.11.7` or `v0.11.7-fork.1` into a PEP 440 version (for comparison).
fn parse_tag_as_version(tag: &str) -> Result<Pep440Version> {
    let trimmed = tag.strip_prefix('v').unwrap_or(tag);
    // PEP 440 uses `.post` / `.dev` / `rc`, not `-fork.N`. Map `-fork.N` to `.postN` so ordering
    // works: v0.11.7-fork.2 > v0.11.7-fork.1 > v0.11.7.
    let normalized = if let Some((base, suffix)) = trimmed.split_once("-fork.") {
        format!("{base}.post{suffix}")
    } else {
        trimmed.to_owned()
    };
    Pep440Version::from_str(&normalized).map_err(|e| anyhow!("{e}"))
}

fn is_update_needed(
    current: &Pep440Version,
    target: &Pep440Version,
    has_explicit_target: bool,
) -> bool {
    if has_explicit_target {
        current != target
    } else {
        current < target
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_tag_adds_v_prefix() {
        assert_eq!(normalize_tag("0.11.7"), "v0.11.7");
        assert_eq!(normalize_tag("v0.11.7"), "v0.11.7");
        assert_eq!(normalize_tag("v0.11.7-fork.1"), "v0.11.7-fork.1");
    }

    #[test]
    fn parse_tag_understands_fork_suffix() {
        let plain = parse_tag_as_version("v0.11.7").unwrap();
        let fork1 = parse_tag_as_version("v0.11.7-fork.1").unwrap();
        let fork2 = parse_tag_as_version("v0.11.7-fork.2").unwrap();
        // `-fork.N` must sort above the plain release and monotonically by N.
        assert!(fork1 > plain);
        assert!(fork2 > fork1);
    }

    #[test]
    fn update_needed_compares_correctly() {
        let v1 = parse_tag_as_version("v0.11.6").unwrap();
        let v2 = parse_tag_as_version("v0.11.7").unwrap();
        assert!(is_update_needed(&v1, &v2, false));
        assert!(!is_update_needed(&v2, &v2, false));
        // Explicit target: any mismatch triggers update (downgrade support).
        assert!(is_update_needed(&v2, &v1, true));
    }
}
