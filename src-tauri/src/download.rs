use crate::{
    catalog,
    firmware::{analyze_bytes, validate_for_profile, FirmwareAnalysis},
};
use reqwest::{redirect::Policy, Client, Url};
use serde::Serialize;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

const MAX_DOWNLOAD_BYTES: u64 = 16 * 1024 * 1024;
const RELEASE_PATH_PREFIX: &str = "/alibros/touch-manager/releases/download/";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    pub path: String,
    pub analysis: FirmwareAnalysis,
}

pub async fn download_official(
    app: &AppHandle,
    firmware_id: &str,
) -> Result<DownloadResult, String> {
    let release = catalog::find_release(firmware_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Firmware is not in the bundled catalog".to_string())?;

    if release.trust != "official" || release.license.as_deref() != Some("MIT") {
        return Err("Only explicitly licensed official firmware can be downloaded".into());
    }
    let url = release.download_url.as_deref().ok_or_else(|| {
        "This official firmware is not available for managed download".to_string()
    })?;
    let url = approved_release_url(url)?;

    let client = Client::builder()
        .redirect(Policy::limited(5))
        .user_agent(concat!("Touch-Manager/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("Could not prepare secure download: {error}"))?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("Firmware download failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Firmware download failed: {error}"))?;

    if response.url().scheme() != "https" {
        return Err("Firmware download redirected to an insecure address".into());
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_DOWNLOAD_BYTES)
    {
        return Err("Firmware download exceeds the 16 MB safety limit".into());
    }

    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("Could not read firmware download: {error}"))?
    {
        if bytes.len() as u64 + chunk.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err("Firmware download exceeds the 16 MB safety limit".into());
        }
        bytes.extend_from_slice(&chunk);
    }

    let cache_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("firmware-cache");
    fs::create_dir_all(&cache_dir).map_err(|error| error.to_string())?;
    let cache_path = cache_dir.join(format!("{}.bin", release.sha256));
    let analysis = analyze_bytes(&cache_path, &bytes).map_err(|error| error.to_string())?;
    validate_for_profile(&analysis, release.target_profile, Some(&release.sha256))
        .map_err(|error| format!("Downloaded firmware was rejected: {error}"))?;

    let temporary = temporary_path(&cache_dir);
    fs::write(&temporary, &bytes).map_err(|error| error.to_string())?;
    if cache_path.exists() {
        fs::remove_file(&cache_path).map_err(|error| error.to_string())?;
    }
    fs::rename(&temporary, &cache_path).map_err(|error| error.to_string())?;

    Ok(DownloadResult {
        path: cache_path.to_string_lossy().to_string(),
        analysis,
    })
}

fn approved_release_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|_| "Firmware download URL is invalid".to_string())?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || !url.path().starts_with(RELEASE_PATH_PREFIX)
    {
        return Err("Firmware download URL is outside the approved release origin".into());
    }
    Ok(url)
}

fn temporary_path(cache_dir: &std::path::Path) -> PathBuf {
    cache_dir.join(format!(".{}.part", Uuid::new_v4()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_touch_manager_https_release_assets() {
        assert!(approved_release_url(
            "https://github.com/alibros/touch-manager/releases/download/v0.1.0/TouchString.bin"
        )
        .is_ok());
        assert!(approved_release_url(
            "http://github.com/alibros/touch-manager/releases/download/v0.1.0/file.bin"
        )
        .is_err());
        assert!(approved_release_url("https://example.com/firmware.bin").is_err());
        assert!(approved_release_url(
            "https://github.com/other/repo/releases/download/v1/file.bin"
        )
        .is_err());
    }
}
