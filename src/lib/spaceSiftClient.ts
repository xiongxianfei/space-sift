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

export type Unsubscribe = () => void;

export type SpaceSiftClient = {
  startScan(rootPath: string): Promise<{ scanId: string }>;
  cancelActiveScan(): Promise<void>;
  getScanStatus(): Promise<ScanStatusSnapshot>;
  listScanHistory(): Promise<ScanHistoryEntry[]>;
  openScanHistory(scanId: string): Promise<CompletedScan>;
  startDuplicateAnalysis(scanId: string): Promise<{ analysisId: string }>;
  getDuplicateAnalysisStatus(): Promise<DuplicateStatusSnapshot>;
  openDuplicateAnalysis(analysisId: string): Promise<CompletedDuplicateAnalysis>;
  listCleanupRules(): Promise<CleanupRuleDefinition[]>;
  previewCleanup(request: {
    scanId: string;
    duplicateDeletePaths: string[];
    enabledRuleIds: string[];
  }): Promise<CleanupPreview>;
  executeCleanup(request: {
    previewId: string;
    actionIds: string[];
    mode: CleanupExecutionMode;
  }): Promise<CleanupExecutionResult>;
  getPrivilegedCleanupCapability(): Promise<PrivilegedCleanupCapability>;
  openPathInExplorer(path: string): Promise<void>;
  subscribeToScanProgress(
    listener: (snapshot: ScanStatusSnapshot) => void,
  ): Promise<Unsubscribe>;
  subscribeToDuplicateProgress(
    listener: (snapshot: DuplicateStatusSnapshot) => void,
  ): Promise<Unsubscribe>;
};

export const idleScanStatus: ScanStatusSnapshot = {
  scanId: null,
  rootPath: null,
  state: "idle",
  filesDiscovered: 0,
  directoriesDiscovered: 0,
  bytesProcessed: 0,
  startedAt: null,
  updatedAt: null,
  currentPath: null,
  message: null,
  completedScanId: null,
};

export const idleDuplicateStatus: DuplicateStatusSnapshot = {
  analysisId: null,
  scanId: null,
  state: "idle",
  stage: null,
  itemsProcessed: 0,
  groupsEmitted: 0,
  message: null,
  completedAnalysisId: null,
};

export const unsupportedClient: SpaceSiftClient = {
  async startScan() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async cancelActiveScan() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async getScanStatus() {
    return idleScanStatus;
  },
  async listScanHistory() {
    return [];
  },
  async openScanHistory() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async startDuplicateAnalysis() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async getDuplicateAnalysisStatus() {
    return idleDuplicateStatus;
  },
  async openDuplicateAnalysis() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async listCleanupRules() {
    return [];
  },
  async previewCleanup() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async executeCleanup() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async getPrivilegedCleanupCapability() {
    return {
      available: false,
      message:
        "Protected-path cleanup stays outside the unprivileged desktop flow in this milestone.",
    };
  },
  async openPathInExplorer() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async subscribeToScanProgress() {
    return () => {};
  },
  async subscribeToDuplicateProgress() {
    return () => {};
  },
};
