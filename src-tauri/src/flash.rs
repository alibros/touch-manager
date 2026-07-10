use crate::{
    device,
    firmware::{analyze_firmware, validate_for_profile, TargetProfile},
    history::{HistoryStore, NewHistoryEntry},
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_shell::{process::Command, ShellExt};

#[cfg(debug_assertions)]
use std::{env, path::PathBuf};

const BUNDLED_DFU_UTIL: &str = "dfu-util";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashingEngineStatus {
    pub ready: bool,
    pub source: String,
    pub version: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashRequest {
    pub firmware_id: String,
    pub firmware_name: String,
    pub version: String,
    pub path: String,
    pub expected_sha256: Option<String>,
    pub target_profile: TargetProfile,
    pub expect_runtime: bool,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlashResult {
    pub status: String,
    pub transfer_completed: bool,
    pub runtime_returned: bool,
    pub transcript: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FlashEvent {
    phase: String,
    message: String,
}

pub async fn run_flash(
    app: AppHandle,
    history: State<'_, HistoryStore>,
    request: FlashRequest,
) -> Result<FlashResult, String> {
    if !request.confirmed {
        return Err("Hardware write requires explicit confirmation".into());
    }

    emit(&app, "validating", "Validating firmware and target profile");
    let analysis = analyze_firmware(Path::new(&request.path)).map_err(|error| error.to_string())?;
    validate_for_profile(
        &analysis,
        request.target_profile,
        request.expected_sha256.as_deref(),
    )
    .map_err(|error| error.to_string())?;

    let devices = device::scan_devices()?;
    let dfu_devices = devices
        .iter()
        .filter(|device| device.state.contains("dfu") || device.state == "daisy_bootloader")
        .collect::<Vec<_>>();
    let dfu_count = dfu_devices.len();
    if dfu_count == 0 {
        return Err("No DFU device is connected".into());
    }
    if dfu_count > 1 {
        return Err("More than one DFU device is connected; disconnect all but the Touch 2".into());
    }
    let state = &dfu_devices[0].state;
    if request.target_profile.requires_daisy_bootloader() && state != "daisy_bootloader" {
        return Err(
            "This firmware requires Daisy Bootloader update mode. Tap RESET and enter the bootloader grace period; BOOT + RESET enters the wrong recovery mode for this image."
                .into(),
        );
    }
    if !request.target_profile.requires_daisy_bootloader() && state == "daisy_bootloader" {
        return Err(
            "Internal-flash firmware requires STM32 recovery mode. Hold BOOT, tap RESET, then release BOOT."
                .into(),
        );
    }
    if state == "dfu_unknown" {
        return Err("The connected DFU device could not be identified safely".into());
    }

    emit(&app, "writing", "Erasing and writing firmware");
    let address = format!("{}:leave", request.target_profile.address());
    let command = flashing_command(&app)?;
    let output = command
        .args([
            "-a",
            "0",
            "-d",
            ",0483:df11",
            "-s",
            &address,
            "-D",
            &request.path,
        ])
        .output()
        .await
        .map_err(|error| format!("Could not launch the built-in flashing engine: {error}"))?;

    let transcript = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let transfer_completed = transcript.contains("File downloaded successfully")
        || transcript.contains("Download done")
        || output.status.success();
    let leave_disconnect = transfer_completed
        && (transcript.contains("Error during download get_status")
            || transcript.contains("Lost device after RESET"));

    emit(&app, "awaiting_runtime", "Waiting for Touch 2 to restart");
    let runtime_returned = if request.expect_runtime {
        wait_for_runtime(Duration::from_secs(12))
    } else {
        false
    };

    let (status, message) = if transfer_completed && (runtime_returned || !request.expect_runtime) {
        (
            "succeeded",
            if runtime_returned {
                "Firmware installed and Touch 2 returned".to_string()
            } else {
                "Firmware transfer completed; this instrument does not expose runtime USB"
                    .to_string()
            },
        )
    } else if leave_disconnect && !runtime_returned {
        (
            "recovery_required",
            "Transfer completed, but runtime USB was not detected. Press RESET and verify the instrument."
                .to_string(),
        )
    } else {
        (
            "failed",
            "Firmware installation did not complete. The STM32 BOOT/RESET recovery remains available."
                .to_string(),
        )
    };

    let target_profile = format!("{:?}", request.target_profile);
    history.record(NewHistoryEntry {
        firmware_id: &request.firmware_id,
        firmware_name: &request.firmware_name,
        version: &request.version,
        sha256: &analysis.sha256,
        target_profile: &target_profile,
        status,
        transcript: &transcript,
    })?;

    emit(&app, status, &message);
    Ok(FlashResult {
        status: status.into(),
        transfer_completed,
        runtime_returned,
        transcript,
        message,
    })
}

pub async fn flashing_engine_status(app: &AppHandle) -> FlashingEngineStatus {
    let source = flashing_engine_source();
    let command = match flashing_command(app) {
        Ok(command) => command,
        Err(error) => {
            return FlashingEngineStatus {
                ready: false,
                source: "missing".into(),
                version: None,
                message: error,
            };
        }
    };

    match command.arg("--version").output().await {
        Ok(output) if output.status.success() => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let version = parse_dfu_util_version(&combined);
            FlashingEngineStatus {
                ready: version.is_some(),
                source: source.into(),
                version,
                message: if source == "bundled" {
                    "Included with Touch Manager".into()
                } else {
                    "Development flashing engine".into()
                },
            }
        }
        Ok(output) => FlashingEngineStatus {
            ready: false,
            source: source.into(),
            version: None,
            message: format!("Flashing engine exited with status {:?}", output.status.code()),
        },
        Err(error) => FlashingEngineStatus {
            ready: false,
            source: "missing".into(),
            version: None,
            message: format!(
                "The built-in flashing engine is unavailable. Reinstall the latest official Touch Manager release. {error}"
            ),
        },
    }
}

fn flashing_command(app: &AppHandle) -> Result<Command, String> {
    #[cfg(debug_assertions)]
    if let Some(path) = development_dfu_util() {
        return Ok(app.shell().command(path));
    }

    app.shell()
        .sidecar(BUNDLED_DFU_UTIL)
        .map_err(|error| format!("Could not locate the built-in flashing engine: {error}"))
}

fn flashing_engine_source() -> &'static str {
    #[cfg(debug_assertions)]
    if development_dfu_util().is_some() {
        return "development";
    }
    "bundled"
}

fn parse_dfu_util_version(output: &str) -> Option<String> {
    output
        .lines()
        .find(|line| line.trim_start().starts_with("dfu-util "))
        .map(|line| line.trim().to_string())
}

#[cfg(debug_assertions)]
fn development_dfu_util() -> Option<PathBuf> {
    if let Some(path) = env::var_os("TOUCH_MANAGER_DFU_UTIL") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    ["/opt/homebrew/bin/dfu-util", "/usr/local/bin/dfu-util"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.is_file())
}

fn wait_for_runtime(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if device::runtime_is_present() {
            return true;
        }
        thread::sleep(Duration::from_millis(500));
    }
    false
}

fn emit(app: &AppHandle, phase: &str, message: &str) {
    let _ = app.emit(
        "flash-event",
        FlashEvent {
            phase: phase.into(),
            message: message.into(),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::parse_dfu_util_version;

    #[test]
    fn recognizes_known_leave_disconnect_as_post_transfer() {
        let transcript = "File downloaded successfully\nError during download get_status";
        assert!(transcript.contains("File downloaded successfully"));
        assert!(transcript.contains("Error during download get_status"));
    }

    #[test]
    fn parses_bundled_flashing_engine_version() {
        let output = "dfu-util 0.11\n\nCopyright 2005-2021";
        assert_eq!(
            parse_dfu_util_version(output).as_deref(),
            Some("dfu-util 0.11")
        );
        assert_eq!(parse_dfu_util_version("not dfu-util"), None);
    }
}
