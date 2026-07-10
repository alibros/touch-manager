import { invoke } from "@tauri-apps/api/core";
import rawCatalog from "../src-tauri/catalog.json";
import type {
  CatalogItem,
  DownloadResult,
  FirmwareAnalysis,
  FlashingEngineStatus,
  FlashResult,
  HistoryEntry,
  TargetProfile,
  TouchDevice,
} from "./types";

export const isDesktop = "__TAURI_INTERNALS__" in window;

const previewCatalog: CatalogItem[] = rawCatalog.releases.map((release) => ({
  ...(release as CatalogItem),
  availableLocally: false,
}));

export async function getCatalog(): Promise<CatalogItem[]> {
  if (!isDesktop) return previewCatalog;
  return invoke<CatalogItem[]>("get_catalog");
}

export async function downloadOfficialFirmware(firmwareId: string): Promise<DownloadResult> {
  return invoke<DownloadResult>("download_official_firmware", { firmwareId });
}

export async function scanDevices(): Promise<TouchDevice[]> {
  if (!isDesktop) return [];
  return invoke<TouchDevice[]>("scan_touch_devices");
}

export async function analyzeFirmware(path: string): Promise<FirmwareAnalysis> {
  return invoke<FirmwareAnalysis>("analyze_firmware_file", { path });
}

export async function getHistory(): Promise<HistoryEntry[]> {
  if (!isDesktop) return [];
  return invoke<HistoryEntry[]>("list_history");
}

export async function getFlashingEngine(): Promise<FlashingEngineStatus> {
  if (!isDesktop) {
    return {
      ready: false,
      source: "missing",
      message: "Available in the packaged desktop application",
    };
  }
  return invoke<FlashingEngineStatus>("get_flashing_engine");
}

export async function startConsole(portName: string): Promise<string> {
  return invoke<string>("start_serial_console", { portName, baudRate: 115200 });
}

export async function stopConsole(sessionId: string): Promise<boolean> {
  return invoke<boolean>("stop_serial_console", { sessionId });
}

export async function requestUpdateMode(portName: string): Promise<string> {
  return invoke<string>("request_update_mode", { portName });
}

export async function saveTranscript(path: string, content: string): Promise<void> {
  return invoke("save_transcript", { path, content });
}

export interface FlashRequest {
  firmwareId: string;
  firmwareName: string;
  version: string;
  path: string;
  expectedSha256?: string;
  targetProfile: TargetProfile;
  expectRuntime: boolean;
  confirmed: boolean;
}

export async function flashFirmware(request: FlashRequest): Promise<FlashResult> {
  return invoke<FlashResult>("flash_firmware", { request });
}
