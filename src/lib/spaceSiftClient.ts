import type {
  CompletedScan,
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
  openPathInExplorer(path: string): Promise<void>;
  subscribeToScanProgress(
    listener: (snapshot: ScanStatusSnapshot) => void,
  ): Promise<Unsubscribe>;
};

export const idleScanStatus: ScanStatusSnapshot = {
  scanId: null,
  rootPath: null,
  state: "idle",
  filesDiscovered: 0,
  directoriesDiscovered: 0,
  bytesProcessed: 0,
  message: null,
  completedScanId: null,
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
  async openPathInExplorer() {
    throw new Error("The Space Sift desktop bridge is not connected yet.");
  },
  async subscribeToScanProgress() {
    return () => {};
  },
};
