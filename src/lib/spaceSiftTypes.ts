export type ScanLifecycleState =
  | "idle"
  | "running"
  | "completed"
  | "cancelled"
  | "failed";

export type SkipReasonCode =
  | "excluded"
  | "permission_denied"
  | "reparse_point"
  | "missing_path"
  | "metadata_error"
  | "read_dir_error";

export type SizedPath = {
  path: string;
  sizeBytes: number;
};

export type ScanEntryKind = "file" | "directory";

export type ScanEntry = {
  path: string;
  parentPath: string | null;
  kind: ScanEntryKind;
  sizeBytes: number;
};

export type SkippedPath = {
  path: string;
  reasonCode: SkipReasonCode;
  summary: string;
};

export type CompletedScan = {
  scanId: string;
  rootPath: string;
  startedAt: string;
  completedAt: string;
  totalBytes: number;
  totalFiles: number;
  totalDirectories: number;
  largestFiles: SizedPath[];
  largestDirectories: SizedPath[];
  skippedPaths: SkippedPath[];
  entries?: ScanEntry[];
};

export type ScanHistoryEntry = {
  scanId: string;
  rootPath: string;
  completedAt: string;
  totalBytes: number;
};

export type ScanStatusSnapshot = {
  scanId: string | null;
  rootPath: string | null;
  state: ScanLifecycleState;
  filesDiscovered: number;
  directoriesDiscovered: number;
  bytesProcessed: number;
  message: string | null;
  completedScanId: string | null;
};
