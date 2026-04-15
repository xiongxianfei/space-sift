import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SpaceSiftClient } from "./spaceSiftClient";
import type {
  CompletedScan,
  ScanHistoryEntry,
  ScanStatusSnapshot,
} from "./spaceSiftTypes";

export const tauriSpaceSiftClient: SpaceSiftClient = {
  async startScan(rootPath) {
    return invoke<{ scanId: string }>("start_scan", { rootPath });
  },
  async cancelActiveScan() {
    await invoke("cancel_active_scan");
  },
  async getScanStatus() {
    return invoke<ScanStatusSnapshot>("get_scan_status");
  },
  async listScanHistory() {
    return invoke<ScanHistoryEntry[]>("list_scan_history");
  },
  async openScanHistory(scanId) {
    return invoke<CompletedScan>("open_scan_history", { scanId });
  },
  async openPathInExplorer(path) {
    await invoke("open_path_in_explorer", { path });
  },
  async subscribeToScanProgress(listener) {
    return listen<ScanStatusSnapshot>("scan-progress", (event) => {
      listener(event.payload);
    });
  },
};
