export type ScanLifecycleState =
  | "idle"
  | "running"
  | "completed"
  | "cancelled"
  | "failed";

export type DuplicateAnalysisState =
  | "idle"
  | "running"
  | "completed"
  | "cancelled"
  | "failed";

export type DuplicateAnalysisStage =
  | "grouping"
  | "partial_hash"
  | "full_hash"
  | "completed";

export type SkipReasonCode =
  | "excluded"
  | "permission_denied"
  | "reparse_point"
  | "missing_path"
  | "metadata_error"
  | "read_dir_error";

export type DuplicateIssueCode =
  | "missing_path"
  | "metadata_changed"
  | "read_error";

export type CleanupIssueCode =
  | "missing_path"
  | "not_a_file"
  | "outside_root"
  | "metadata_changed"
  | "not_in_scan"
  | "read_error"
  | "requires_elevation";

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

export type DuplicateIssue = {
  path: string;
  code: DuplicateIssueCode;
  summary: string;
};

export type CleanupIssue = {
  path: string;
  code: CleanupIssueCode;
  summary: string;
};

export type CleanupRuleDefinition = {
  ruleId: string;
  label: string;
  description: string;
};

export type CleanupPreviewCandidate = {
  actionId: string;
  path: string;
  sizeBytes: number;
  sourceLabels: string[];
};

export type CleanupPreview = {
  previewId: string;
  scanId: string;
  rootPath: string;
  generatedAt: string;
  totalBytes: number;
  duplicateCandidateCount: number;
  ruleCandidateCount: number;
  candidates: CleanupPreviewCandidate[];
  issues: CleanupIssue[];
};

export type CleanupExecutionMode = "recycle" | "permanent";

export type CleanupExecutionItemStatus = "completed" | "failed";

export type CleanupExecutionEntry = {
  actionId: string;
  path: string;
  status: CleanupExecutionItemStatus;
  summary: string;
};

export type CleanupExecutionResult = {
  executionId: string;
  previewId: string;
  mode: CleanupExecutionMode;
  completedAt: string;
  completedCount: number;
  failedCount: number;
  entries: CleanupExecutionEntry[];
};

export type WorkspaceRestoreWorkspace =
  | "overview"
  | "scan"
  | "history"
  | "explorer"
  | "duplicates"
  | "cleanup"
  | "safety";

export type WorkspaceRestoreContext = {
  schemaVersion: number;
  lastWorkspace: WorkspaceRestoreWorkspace;
  lastOpenedScanId: string | null;
  updatedAt: string;
};

export type WorkspaceRestoreContextInput = {
  lastWorkspace: WorkspaceRestoreWorkspace;
  lastOpenedScanId: string | null;
};

export type PrivilegedCleanupCapability = {
  available: boolean;
  message: string;
};

export type DuplicateGroupMember = {
  path: string;
  sizeBytes: number;
  modifiedAt: string;
};

export type DuplicateGroup = {
  groupId: string;
  sizeBytes: number;
  reclaimableBytes: number;
  members: DuplicateGroupMember[];
};

export type CompletedDuplicateAnalysis = {
  analysisId: string;
  scanId: string;
  rootPath: string;
  startedAt: string;
  completedAt: string;
  groups: DuplicateGroup[];
  issues: DuplicateIssue[];
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

export type ScanRunStatus =
  | "running"
  | "stale"
  | "abandoned"
  | "completed"
  | "cancelled"
  | "failed";

export type ScanRunHeader = {
  runId: string;
  targetId: string;
  rootPath: string;
  status: ScanRunStatus;
  startedAt: string;
  lastSnapshotAt: string;
  lastProgressAt: string;
  staleSince: string | null;
  terminalAt: string | null;
  completedScanId: string | null;
  resumedFromRunId: string | null;
  createdAt: string;
  updatedAt: string;
  latestSeq: number;
  errorCode: string | null;
  errorMessage: string | null;
};

export type ScanRunSnapshot = {
  runId: string;
  seq: number;
  snapshotAt: string;
  createdAt: string;
  status: ScanRunStatus;
  filesDiscovered: number;
  directoriesDiscovered: number;
  itemsDiscovered: number;
  itemsScanned: number;
  errorsCount: number;
  bytesProcessed: number;
  scanRateItemsPerSec: number;
  progressPercent: number | null;
  currentPath: string | null;
  message: string | null;
};

export type ScanRunSummary = {
  header: ScanRunHeader;
  latestSnapshot: ScanRunSnapshot;
  snapshotPreview: ScanRunSnapshot[];
  seq: number;
  createdAt: string;
  itemsScanned: number;
  errorsCount: number;
  progressPercent: number | null;
  scanRateItemsPerSec: number;
  hasResume: boolean;
  canResume: boolean;
};

export type ScanRunDetail = ScanRunSummary & {
  snapshotPreviewPage: number;
  snapshotPreviewPageSize: number;
  snapshotPreviewTotal: number;
};

export type StartScanOptions = {
  resumeEnabled?: boolean;
};

export type ScanStatusSnapshot = {
  scanId: string | null;
  rootPath: string | null;
  state: ScanLifecycleState;
  filesDiscovered: number;
  directoriesDiscovered: number;
  bytesProcessed: number;
  startedAt: string | null;
  updatedAt: string | null;
  currentPath: string | null;
  message: string | null;
  completedScanId: string | null;
};

export type DuplicateStatusSnapshot = {
  analysisId: string | null;
  scanId: string | null;
  state: DuplicateAnalysisState;
  stage: DuplicateAnalysisStage | null;
  itemsProcessed: number;
  groupsEmitted: number;
  message: string | null;
  completedAnalysisId: string | null;
};
