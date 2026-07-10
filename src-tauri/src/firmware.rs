use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};
use thiserror::Error;

const INTERNAL_MAX: u64 = 128 * 1024;
const SRAM_MAX: u64 = 480 * 1024;
const QSPI_MAX: u64 = 7_936 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionLayout {
    Internal,
    BootSram,
    BootQspi,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TargetProfile {
    Touch2StmInternalV1,
    Touch2DaisySramV1,
    Touch2DaisyQspiV1,
}

impl TargetProfile {
    pub fn address(self) -> &'static str {
        match self {
            Self::Touch2StmInternalV1 => "0x08000000",
            Self::Touch2DaisySramV1 | Self::Touch2DaisyQspiV1 => "0x90040000",
        }
    }

    pub fn expected_layout(self) -> ExecutionLayout {
        match self {
            Self::Touch2StmInternalV1 => ExecutionLayout::Internal,
            Self::Touch2DaisySramV1 => ExecutionLayout::BootSram,
            Self::Touch2DaisyQspiV1 => ExecutionLayout::BootQspi,
        }
    }

    pub fn requires_daisy_bootloader(self) -> bool {
        !matches!(self, Self::Touch2StmInternalV1)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareAnalysis {
    pub path: String,
    pub filename: String,
    pub size: u64,
    pub sha256: String,
    pub initial_stack_pointer: String,
    pub reset_vector: String,
    pub execution_layout: ExecutionLayout,
    pub inferred_profile: Option<TargetProfile>,
    pub upload_address: Option<String>,
    pub valid_stack_pointer: bool,
    pub valid_thumb_vector: bool,
    pub safe_to_plan: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error)]
pub enum FirmwareError {
    #[error("Could not read firmware: {0}")]
    Io(#[from] std::io::Error),
    #[error("Firmware is too small to contain a Cortex-M vector table")]
    TooSmall,
    #[error("Firmware target does not match the selected Touch 2 profile")]
    ProfileMismatch,
    #[error("Firmware SHA-256 does not match its catalog entry")]
    HashMismatch,
    #[error("Firmware has an unknown or unsupported execution layout")]
    UnknownLayout,
    #[error("Firmware exceeds the supported size for its execution layout")]
    TooLarge,
}

pub fn analyze_firmware(path: &Path) -> Result<FirmwareAnalysis, FirmwareError> {
    let bytes = fs::read(path)?;
    analyze_bytes(path, &bytes)
}

pub fn analyze_bytes(path: &Path, bytes: &[u8]) -> Result<FirmwareAnalysis, FirmwareError> {
    if bytes.len() < 8 {
        return Err(FirmwareError::TooSmall);
    }

    let initial_stack_pointer = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let reset_vector = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let reset_address = reset_vector & !1;
    let layout = classify_reset_address(reset_address);
    let inferred_profile = profile_for_layout(layout);
    let valid_stack_pointer = plausible_stack_pointer(initial_stack_pointer);
    let valid_thumb_vector = reset_vector & 1 == 1;
    let size = bytes.len() as u64;
    let within_size_limit = size_within_limit(layout, size);

    let mut warnings = Vec::new();
    if !valid_stack_pointer {
        warnings.push(format!(
            "Initial stack pointer 0x{initial_stack_pointer:08X} is outside expected Daisy RAM"
        ));
    }
    if !valid_thumb_vector {
        warnings.push("Reset vector does not have the Cortex-M Thumb bit set".into());
    }
    if layout == ExecutionLayout::Unknown {
        warnings.push(format!(
            "Reset vector 0x{reset_vector:08X} does not identify an approved Touch 2 layout"
        ));
    }
    if !within_size_limit {
        warnings.push("Binary exceeds the supported size for this layout".into());
    }
    if layout == ExecutionLayout::Internal {
        warnings
            .push("Installing this image overwrites any Daisy Bootloader in internal flash".into());
    }

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let sha256 = hex::encode(hasher.finalize());

    Ok(FirmwareAnalysis {
        path: path.to_string_lossy().to_string(),
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        size,
        sha256,
        initial_stack_pointer: format!("0x{initial_stack_pointer:08X}"),
        reset_vector: format!("0x{reset_vector:08X}"),
        execution_layout: layout,
        inferred_profile,
        upload_address: inferred_profile.map(|profile| profile.address().to_string()),
        valid_stack_pointer,
        valid_thumb_vector,
        safe_to_plan: valid_stack_pointer
            && valid_thumb_vector
            && layout != ExecutionLayout::Unknown
            && within_size_limit,
        warnings,
    })
}

pub fn validate_for_profile(
    analysis: &FirmwareAnalysis,
    profile: TargetProfile,
    expected_sha256: Option<&str>,
) -> Result<(), FirmwareError> {
    if analysis.execution_layout == ExecutionLayout::Unknown {
        return Err(FirmwareError::UnknownLayout);
    }
    if analysis.execution_layout != profile.expected_layout() {
        return Err(FirmwareError::ProfileMismatch);
    }
    if !size_within_limit(analysis.execution_layout, analysis.size) {
        return Err(FirmwareError::TooLarge);
    }
    if let Some(expected) = expected_sha256 {
        if !expected.eq_ignore_ascii_case(&analysis.sha256) {
            return Err(FirmwareError::HashMismatch);
        }
    }
    if !analysis.safe_to_plan {
        return Err(FirmwareError::UnknownLayout);
    }
    Ok(())
}

fn classify_reset_address(address: u32) -> ExecutionLayout {
    match address {
        0x0800_0000..=0x081F_FFFF => ExecutionLayout::Internal,
        0x2400_0000..=0x2407_FFFF => ExecutionLayout::BootSram,
        0x9004_0000..=0x907F_FFFF => ExecutionLayout::BootQspi,
        _ => ExecutionLayout::Unknown,
    }
}

fn profile_for_layout(layout: ExecutionLayout) -> Option<TargetProfile> {
    match layout {
        ExecutionLayout::Internal => Some(TargetProfile::Touch2StmInternalV1),
        ExecutionLayout::BootSram => Some(TargetProfile::Touch2DaisySramV1),
        ExecutionLayout::BootQspi => Some(TargetProfile::Touch2DaisyQspiV1),
        ExecutionLayout::Unknown => None,
    }
}

fn size_within_limit(layout: ExecutionLayout, size: u64) -> bool {
    match layout {
        ExecutionLayout::Internal => size <= INTERNAL_MAX,
        ExecutionLayout::BootSram => size <= SRAM_MAX,
        ExecutionLayout::BootQspi => size <= QSPI_MAX,
        ExecutionLayout::Unknown => false,
    }
}

fn plausible_stack_pointer(pointer: u32) -> bool {
    matches!(
        pointer,
        0x2000_0000..=0x2002_0000
            | 0x2400_0000..=0x2408_0000
            | 0x3000_0000..=0x3004_8000
            | 0x3800_0000..=0x3801_0000
            | 0xC000_0000..=0xC400_0000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(stack: u32, reset: u32, size: usize) -> Vec<u8> {
        let mut bytes = vec![0; size.max(8)];
        bytes[0..4].copy_from_slice(&stack.to_le_bytes());
        bytes[4..8].copy_from_slice(&reset.to_le_bytes());
        bytes
    }

    #[test]
    fn classifies_internal_image() {
        let analysis = analyze_bytes(
            Path::new("TouchBass.bin"),
            &image(0x2408_0000, 0x0800_0795, 64 * 1024),
        )
        .unwrap();
        assert_eq!(analysis.execution_layout, ExecutionLayout::Internal);
        assert_eq!(
            analysis.inferred_profile,
            Some(TargetProfile::Touch2StmInternalV1)
        );
        assert!(analysis.safe_to_plan);
    }

    #[test]
    fn classifies_boot_sram_image() {
        let analysis = analyze_bytes(
            Path::new("SimplePlaits.bin"),
            &image(0x2002_0000, 0x2400_0699, 256 * 1024),
        )
        .unwrap();
        assert_eq!(analysis.execution_layout, ExecutionLayout::BootSram);
        assert_eq!(analysis.upload_address.as_deref(), Some("0x90040000"));
    }

    #[test]
    fn classifies_boot_qspi_image() {
        let analysis = analyze_bytes(
            Path::new("TouchPlaited.bin"),
            &image(0x2002_0000, 0x9004_0691, 320 * 1024),
        )
        .unwrap();
        assert_eq!(analysis.execution_layout, ExecutionLayout::BootQspi);
    }

    #[test]
    fn rejects_profile_mismatch() {
        let analysis = analyze_bytes(
            Path::new("wrong.bin"),
            &image(0x2002_0000, 0x2400_0699, 128 * 1024),
        )
        .unwrap();
        assert!(matches!(
            validate_for_profile(&analysis, TargetProfile::Touch2StmInternalV1, None),
            Err(FirmwareError::ProfileMismatch)
        ));
    }
}
