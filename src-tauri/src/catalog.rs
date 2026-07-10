use crate::firmware::{analyze_firmware, FirmwareAnalysis, TargetProfile};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDocument {
    pub schema_version: u32,
    pub generated_at: String,
    pub releases: Vec<FirmwareRelease>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareRelease {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub channel: String,
    pub trust: String,
    pub category: String,
    pub summary: String,
    pub description: String,
    pub tags: Vec<String>,
    pub tone: String,
    pub featured: bool,
    pub target_profile: TargetProfile,
    pub sha256: String,
    pub artifact_path: String,
    pub source_url: String,
    pub license: Option<String>,
    pub runtime_usb: bool,
    pub manager_compatible: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogItem {
    #[serde(flatten)]
    pub release: FirmwareRelease,
    pub local_path: Option<String>,
    pub available_locally: bool,
    pub checksum_matches: Option<bool>,
    pub analysis: Option<FirmwareAnalysis>,
}

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("Bundled catalog is invalid: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn load_catalog() -> Result<Vec<CatalogItem>, CatalogError> {
    let document: CatalogDocument = serde_json::from_str(include_str!("../catalog.json"))?;
    let workspace = discover_workspace_root();

    Ok(document
        .releases
        .into_iter()
        .map(|release| {
            let local_path = workspace
                .as_ref()
                .map(|root| root.join(&release.artifact_path));
            let analysis = local_path
                .as_ref()
                .filter(|path| path.is_file())
                .and_then(|path| analyze_firmware(path).ok());
            let checksum_matches = analysis
                .as_ref()
                .map(|item| item.sha256.eq_ignore_ascii_case(&release.sha256));
            CatalogItem {
                available_locally: local_path.as_ref().is_some_and(|path| path.is_file()),
                local_path: local_path
                    .filter(|path| path.is_file())
                    .map(|path| path.to_string_lossy().to_string()),
                checksum_matches,
                analysis,
                release,
            }
        })
        .collect())
}

fn discover_workspace_root() -> Option<PathBuf> {
    if let Ok(root) = env::var("SYNTHUX_WORKSPACE") {
        let path = PathBuf::from(root);
        if path.join("Firmware/SHA256SUMS.txt").is_file() {
            return Some(path);
        }
    }

    env::current_dir().ok().and_then(|cwd| {
        cwd.ancestors()
            .find(|candidate| candidate.join("Firmware/SHA256SUMS.txt").is_file())
            .map(PathBuf::from)
    })
}

#[allow(dead_code)]
pub fn load_catalog_from_path(path: PathBuf) -> Result<CatalogDocument, String> {
    let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&contents).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn bundled_catalog_has_complete_unique_metadata() {
        let document: CatalogDocument =
            serde_json::from_str(include_str!("../catalog.json")).expect("catalog should parse");
        assert_eq!(document.schema_version, 1);
        assert_eq!(document.releases.len(), 23);

        let mut ids = HashSet::new();
        for release in document.releases {
            assert!(
                ids.insert(release.id.clone()),
                "duplicate id: {}",
                release.id
            );
            assert_eq!(release.sha256.len(), 64, "invalid hash: {}", release.id);
            assert!(!release.name.trim().is_empty());
            assert!(!release.artifact_path.trim().is_empty());
        }
    }

    #[test]
    fn local_staged_releases_are_verified_when_workspace_is_available() {
        if discover_workspace_root().is_none() {
            return;
        }
        let catalog = load_catalog().expect("bundled catalog should parse");
        assert_eq!(catalog.len(), 23, "catalog must cover every staged binary");

        for item in catalog {
            assert!(item.available_locally, "{} is missing", item.release.id);
            assert_eq!(
                item.checksum_matches,
                Some(true),
                "{} hash mismatch",
                item.release.id
            );
            let analysis = item.analysis.expect("catalog binary should be analyzable");
            assert!(
                analysis.safe_to_plan,
                "{} is unsafe: {:?}",
                item.release.id, analysis.warnings
            );
            assert_eq!(
                analysis.inferred_profile,
                Some(item.release.target_profile),
                "{} target mismatch",
                item.release.id
            );
        }
    }
}
