import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SpaceSiftClient } from "./spaceSiftClient";
import type {
  CleanupExecutionMode,
  CleanupExecutionResult,
  CleanupPreview,
  CleanupRuleDefinition,
  CompletedDuplicateAnalysis,
  CompletedScan,
  DuplicateStatusSnapshot,
  PrivilegedCleanupCapability,
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
  async startDuplicateAnalysis(scanId) {
    return invoke<{ analysisId: string }>("start_duplicate_analysis", { scanId });
  },
  async cancelDuplicateAnalysis() {
    await invoke("cancel_duplicate_analysis");
  },
  async getDuplicateAnalysisStatus() {
    return invoke<DuplicateStatusSnapshot>("get_duplicate_analysis_status");
  },
  async openDuplicateAnalysis(analysisId) {
    return invoke<CompletedDuplicateAnalysis>("open_duplicate_analysis", { analysisId });
  },
  async listCleanupRules() {
    return invoke<CleanupRuleDefinition[]>("list_cleanup_rules");
  },
  async previewCleanup({ scanId, duplicateDeletePaths, enabledRuleIds }) {
    return invoke<CleanupPreview>("preview_cleanup", {
      scanId,
      duplicateDeletePaths,
      enabledRuleIds,
    });
  },
  async executeCleanup({ previewId, actionIds, mode }) {
    return invoke<CleanupExecutionResult>("execute_cleanup", {
      previewId,
      actionIds,
      mode: mode as CleanupExecutionMode,
    });
  },
  async getPrivilegedCleanupCapability() {
    return invoke<PrivilegedCleanupCapability>("get_privileged_cleanup_capability");
  },
  async openPathInExplorer(path) {
    await invoke("open_path_in_explorer", { path });
  },
  async subscribeToScanProgress(listener) {
    return listen<ScanStatusSnapshot>("scan-progress", (event) => {
      listener(event.payload);
    });
  },
  async subscribeToDuplicateProgress(listener) {
    return listen<DuplicateStatusSnapshot>("duplicate-progress", (event) => {
      listener(event.payload);
    });
  },
};
