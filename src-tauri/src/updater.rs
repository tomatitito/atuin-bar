use semver::Version;
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf, process::Command};
use tauri::AppHandle;

const GITHUB_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/tomatitito/atuin-bar/releases/latest";
const USER_AGENT: &str = concat!("atuin-bar/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Serialize)]
pub struct SelfUpdateResult {
    pub updated: bool,
    pub current_version: String,
    pub latest_version: String,
    pub message: String,
    pub release_url: String,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[tauri::command]
pub async fn self_update(app: AppHandle) -> Result<SelfUpdateResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let release = latest_release().await?;
    let latest_version = normalize_version(&release.tag_name);

    if !is_newer_version(&current_version, &latest_version)? {
        return Ok(SelfUpdateResult {
            updated: false,
            current_version,
            latest_version,
            message: "atuin-bar is already up to date.".to_string(),
            release_url: release.html_url,
        });
    }

    let asset = release
        .assets
        .iter()
        .find(|asset| {
            let name = asset.name.to_ascii_lowercase();
            name.ends_with(".zip") && name.contains("macos")
        })
        .or_else(|| {
            release
                .assets
                .iter()
                .find(|asset| asset.name.to_ascii_lowercase().ends_with(".zip"))
        })
        .ok_or_else(|| "Latest release does not contain a macOS zip asset.".to_string())?;

    let app_bundle = current_app_bundle()?;
    let download_path = download_asset(&asset.browser_download_url, &asset.name).await?;
    let script_path = write_install_script()?;

    Command::new("/bin/sh")
        .arg(&script_path)
        .arg(&app_bundle)
        .arg(&download_path)
        .spawn()
        .map_err(|error| format!("Failed to start updater: {error}"))?;

    app.exit(0);

    Ok(SelfUpdateResult {
        updated: true,
        current_version,
        latest_version,
        message: "Update downloaded. atuin-bar will restart after installing it.".to_string(),
        release_url: release.html_url,
    })
}

async fn latest_release() -> Result<GitHubRelease, String> {
    reqwest::Client::new()
        .get(GITHUB_LATEST_RELEASE_URL)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("Failed to check GitHub releases: {error}"))?
        .error_for_status()
        .map_err(|error| format!("GitHub release check failed: {error}"))?
        .json::<GitHubRelease>()
        .await
        .map_err(|error| format!("Failed to parse GitHub release response: {error}"))
}

async fn download_asset(url: &str, asset_name: &str) -> Result<PathBuf, String> {
    let bytes = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("Failed to download update: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Update download failed: {error}"))?
        .bytes()
        .await
        .map_err(|error| format!("Failed to read update download: {error}"))?;

    let path = env::temp_dir().join(format!("atuin-bar-update-{asset_name}"));
    fs::write(&path, bytes).map_err(|error| format!("Failed to save update: {error}"))?;
    Ok(path)
}

fn current_app_bundle() -> Result<PathBuf, String> {
    let exe =
        env::current_exe().map_err(|error| format!("Failed to locate current app: {error}"))?;
    for ancestor in exe.ancestors() {
        if ancestor
            .extension()
            .is_some_and(|extension| extension == "app")
        {
            return Ok(ancestor.to_path_buf());
        }
    }

    Err("Self-update is only available when running the macOS .app bundle.".to_string())
}

fn write_install_script() -> Result<PathBuf, String> {
    let path = env::temp_dir().join("atuin-bar-install-update.sh");
    fs::write(
        &path,
        r#"set -euo pipefail
APP_BUNDLE="$1"
ZIP_PATH="$2"
WORK_DIR="$(/usr/bin/mktemp -d)"
cleanup() {
  /bin/rm -rf "$WORK_DIR" "$ZIP_PATH" "$0"
}
trap cleanup EXIT

# Give the running app time to quit before replacing its bundle.
/bin/sleep 2
/usr/bin/ditto -x -k "$ZIP_PATH" "$WORK_DIR"
NEW_APP="$(/usr/bin/find "$WORK_DIR" -maxdepth 1 -name '*.app' -type d | /usr/bin/head -n 1)"
if [ -z "$NEW_APP" ]; then
  echo "No .app bundle found in update archive" >&2
  exit 1
fi
/usr/bin/ditto "$NEW_APP" "$APP_BUNDLE"
/usr/bin/open "$APP_BUNDLE"
"#,
    )
    .map_err(|error| format!("Failed to write updater script: {error}"))?;
    Ok(path)
}

fn normalize_version(version: &str) -> String {
    version.trim_start_matches('v').to_string()
}

fn is_newer_version(current: &str, latest: &str) -> Result<bool, String> {
    let current = Version::parse(current)
        .map_err(|error| format!("Current app version is invalid ({current}): {error}"))?;
    let latest = Version::parse(latest)
        .map_err(|error| format!("Latest release version is invalid ({latest}): {error}"))?;
    Ok(latest > current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_v_prefixed_versions() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
    }

    #[test]
    fn compares_semver_versions() {
        assert_eq!(is_newer_version("1.0.0", "1.0.1").unwrap(), true);
        assert_eq!(is_newer_version("1.0.1", "1.0.1").unwrap(), false);
        assert_eq!(is_newer_version("1.1.0", "1.0.9").unwrap(), false);
    }
}
