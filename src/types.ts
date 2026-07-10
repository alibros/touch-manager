export type ExecutionLayout = "internal" | "boot_sram" | "boot_qspi" | "unknown";
export type TargetProfile =
  | "touch2-stm-internal-v1"
  | "touch2-daisy-sram-v1"
  | "touch2-daisy-qspi-v1";

export interface FirmwareAnalysis {
  path: string;
  filename: string;
  size: number;
  sha256: string;
  initialStackPointer: string;
  resetVector: string;
  executionLayout: ExecutionLayout;
  inferredProfile?: TargetProfile;
  uploadAddress?: string;
  validStackPointer: boolean;
  validThumbVector: boolean;
  safeToPlan: boolean;
  warnings: string[];
}

export interface FirmwareRelease {
  id: string;
  name: string;
  version: string;
  author: string;
  channel: string;
  trust: string;
  category: string;
  summary: string;
  description: string;
  tags: string[];
  tone: string;
  featured: boolean;
  targetProfile: TargetProfile;
  sha256: string;
  artifactPath: string;
  downloadUrl?: string;
  sourceUrl: string;
  license?: string;
  runtimeUsb: boolean;
  managerCompatible: boolean;
}

export interface CatalogItem extends FirmwareRelease {
  localPath?: string;
  availableLocally: boolean;
  checksumMatches?: boolean;
  analysis?: FirmwareAnalysis;
}

export interface DownloadResult {
  path: string;
  analysis: FirmwareAnalysis;
}

export interface TouchDevice {
  state: "runtime" | "stm_rom_dfu" | "daisy_bootloader" | "dfu_unknown";
  vendorId: string;
  productId: string;
  product?: string;
  manufacturer?: string;
  serialNumber?: string;
  topologyPath: string;
  deviceAddress: number;
  serialPort?: string;
}

export interface HistoryEntry {
  id: string;
  createdAt: string;
  firmwareId: string;
  firmwareName: string;
  version: string;
  sha256: string;
  targetProfile: string;
  status: string;
  transcript: string;
}

export interface FlashEvent {
  phase: string;
  message: string;
}

export interface FlashResult {
  status: string;
  transferCompleted: boolean;
  runtimeReturned: boolean;
  transcript: string;
  message: string;
}

export interface FlashingEngineStatus {
  ready: boolean;
  source: "bundled" | "development" | "missing";
  version?: string;
  message: string;
}
