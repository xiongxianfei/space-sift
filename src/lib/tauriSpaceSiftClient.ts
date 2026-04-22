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
  ScanRunDetail,
  ScanRunSummary,
  ScanStatusSnapshot,
  StartScanOptions,
  WorkspaceRestoreContext,
  WorkspaceRestoreContextInput,
} from "./spaceSiftTypes";

export const tauriSpaceSiftClient: SpaceSiftClient = {
  async startScan(rootPath, options?: StartScanOptions) {
    return invoke<{ scanId: string }>("start_scan", { rootPath, options });
  },
  async cancelActiveScan() {
    await invoke("cancel_active_scan");
  },
  async cancelScanRun(runId) {
    await invoke("cancel_scan_run", { runId });
  },
  async getScanStatus() {
    return invoke<ScanStatusSnapshot>("get_scan_status");
  },
  async getWorkspaceRestoreContext() {
    return invoke<WorkspaceRestoreContext | null>("get_workspace_restore_context");
  },
  async saveWorkspaceRestoreContext(input: WorkspaceRestoreContextInput) {
    return invoke<WorkspaceRestoreContext>("save_workspace_restore_context", { input });
  },
  async listScanHistory() {
    return invoke<ScanHistoryEntry[]>("list_scan_history");
  },
  async openScanHistory(scanId) {
    return invoke<CompletedScan>("open_scan_history", { scanId });
  },
  async listScanRuns() {
    return invoke<ScanRunSummary[]>("list_scan_runs");
  },
  async openScanRun(runId, page, pageSize) {
    return invoke<ScanRunDetail>("open_scan_run", { runId, page, pageSize });
  },
  async resumeScanRun(runId) {
    return invoke<{ runId: string }>("resume_scan_run", { runId });
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
