mod catalog;
mod device;
mod diagnostics;
mod download;
mod firmware;
mod flash;
mod history;

use diagnostics::ConsoleManager;
use firmware::{analyze_firmware, FirmwareAnalysis};
use history::{HistoryEntry, HistoryStore};
use std::{fs, path::PathBuf};
use tauri::{Manager, State};

#[tauri::command]
fn get_catalog(app: tauri::AppHandle) -> Result<Vec<catalog::CatalogItem>, String> {
    let cache_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("firmware-cache");
    catalog::load_catalog(Some(&cache_dir)).map_err(|error| error.to_string())
}

#[tauri::command]
async fn download_official_firmware(
    app: tauri::AppHandle,
    firmware_id: String,
) -> Result<download::DownloadResult, String> {
    download::download_official(&app, &firmware_id).await
}

#[tauri::command]
fn analyze_firmware_file(path: String) -> Result<FirmwareAnalysis, String> {
    analyze_firmware(PathBuf::from(path).as_path()).map_err(|error| error.to_string())
}

#[tauri::command]
fn scan_touch_devices() -> Result<Vec<device::TouchDevice>, String> {
    device::scan_devices()
}

#[tauri::command]
fn list_history(history: State<'_, HistoryStore>) -> Result<Vec<HistoryEntry>, String> {
    history.list()
}

#[tauri::command]
fn start_serial_console(
    app: tauri::AppHandle,
    manager: State<'_, ConsoleManager>,
    port_name: String,
    baud_rate: Option<u32>,
) -> Result<String, String> {
    manager.start(app, port_name, baud_rate.unwrap_or(115_200))
}

#[tauri::command]
fn stop_serial_console(manager: State<'_, ConsoleManager>, session_id: String) -> bool {
    manager.stop(&session_id)
}

#[tauri::command]
fn request_update_mode(port_name: String) -> Result<String, String> {
    diagnostics::request_update_mode(&port_name)
}

#[tauri::command]
fn save_transcript(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|error| error.to_string())
}

#[tauri::command]
async fn flash_firmware(
    app: tauri::AppHandle,
    history: State<'_, HistoryStore>,
    request: flash::FlashRequest,
) -> Result<flash::FlashResult, String> {
    flash::run_flash(app, history, request).await
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            fs::create_dir_all(&data_dir)?;
            let history = HistoryStore::new(data_dir.join("touch-manager.sqlite"))
                .map_err(std::io::Error::other)?;
            app.manage(history);
            app.manage(ConsoleManager::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_catalog,
            download_official_firmware,
            analyze_firmware_file,
            scan_touch_devices,
            list_history,
            start_serial_console,
            stop_serial_console,
            request_update_mode,
            save_transcript,
            flash_firmware,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Touch Manager");
}
