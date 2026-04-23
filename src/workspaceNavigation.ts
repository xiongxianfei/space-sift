import type {
  CleanupExecutionResult,
  CleanupPreview,
  CompletedScan,
  DuplicateStatusSnapshot,
  ScanRunSummary,
  ScanStatusSnapshot,
  WorkspaceRestoreContext,
  WorkspaceRestoreWorkspace,
} from "./lib/spaceSiftTypes";

export type WorkspaceTab = WorkspaceRestoreWorkspace;
export type WorkspaceNavigationReason =
  | "manual"
  | "startup"
  | "N1_START_SCAN"
  | "N2_SCAN_COMPLETED_AND_OPENED"
  | "N3_OPEN_HISTORY_SCAN"
  | "N4_START_DUPLICATE_ANALYSIS"
  | "N5_REQUEST_CLEANUP_PREVIEW"
  | "N6_REVIEW_INTERRUPTED_RUNS";

export type WorkspaceDefinition = {
  value: WorkspaceTab;
  label: string;
  description: string;
};

export type NextSafeAction = {
  label: string;
  target: WorkspaceTab;
};

export type GlobalStatusModel = {
  primaryStateLabel: string;
  contextLabel: string;
  summaryLabel: string | null;
  nextSafeAction: NextSafeAction | null;
  noActionLabel: string | null;
};

type DeriveGlobalStatusInput = {
  scanStatus: ScanStatusSnapshot;
  duplicateStatus: DuplicateStatusSnapshot;
  currentScan: CompletedScan | null;
  interruptedRuns: ScanRunSummary[];
  duplicateEligible: boolean;
  browseableScan: boolean;
  cleanupPreview: CleanupPreview | null;
  cleanupExecutionResult: CleanupExecutionResult | null;
  cleanupExecutionScanId: string | null;
  cleanupPreviewAvailable: boolean;
};

type ResolveInitialWorkspaceInput = {
  scanStatus: ScanStatusSnapshot;
  duplicateStatus: DuplicateStatusSnapshot;
  interruptedRuns: ScanRunSummary[];
  restoreContext: WorkspaceRestoreContext | null;
  loadedScan: CompletedScan | null;
};

export const workspaceDefinitions: WorkspaceDefinition[] = [
  {
    value: "overview",
    label: "Overview",
    description: "Current shell summary, active work, and the next safe action.",
  },
  {
    value: "scan",
    label: "Scan",
    description: "Scan start, active progress, cancellation, and scan errors.",
  },
  {
    value: "history",
    label: "History",
    description: "Completed scan history and interrupted-run review.",
  },
  {
    value: "explorer",
    label: "Explorer",
    description: "Loaded result browsing and Explorer handoff.",
  },
  {
    value: "duplicates",
    label: "Duplicates",
    description: "Duplicate-analysis lifecycle and verified duplicate review.",
  },
  {
    value: "cleanup",
    label: "Cleanup",
    description: "Preview-first cleanup sources, review, and execution.",
  },
  {
    value: "safety",
    label: "Safety",
    description: "Local-only, privilege, and destructive-action safeguards.",
  },
];

function formatBytes(bytes: number) {
  return `${bytes} bytes`;
}

function getDuplicateStageLabel(stage: DuplicateStatusSnapshot["stage"]) {
  switch (stage) {
    case "grouping":
      return "Grouping";
    case "partial_hash":
      return "Partial hash";
    case "full_hash":
      return "Full hash";
    case "completed":
      return "Completed";
    default:
      return "Waiting";
  }
}

function buildScanContext(scan: CompletedScan | null) {
  if (!scan) {
    return "No completed scan is loaded.";
  }

  return `${scan.scanId} | ${scan.rootPath}`;
}

function isInterruptedRun(run: ScanRunSummary) {
  return run.header.status === "stale" || run.header.status === "abandoned";
}

export function resolveInitialWorkspace({
  scanStatus,
  duplicateStatus,
  interruptedRuns,
  restoreContext,
  loadedScan,
}: ResolveInitialWorkspaceInput): WorkspaceTab {
  if (scanStatus.state === "running") {
    return "scan";
  }

  if (
    duplicateStatus.state === "running" &&
    loadedScan &&
    duplicateStatus.scanId === loadedScan.scanId
  ) {
    return "duplicates";
  }

  if (interruptedRuns.some(isInterruptedRun)) {
    return "history";
  }

  if (
    restoreContext?.lastWorkspace === "explorer" &&
    restoreContext.lastOpenedScanId &&
    loadedScan &&
    loadedScan.scanId === restoreContext.lastOpenedScanId
  ) {
    return "explorer";
  }

  return "overview";
}

export function getNextSafeActionReason(
  action: NextSafeAction | null,
): WorkspaceNavigationReason | null {
  if (!action) {
    return null;
  }

  if (action.label === "Review interrupted runs") {
    return "N6_REVIEW_INTERRUPTED_RUNS";
  }

  if (action.label === "Review cleanup preview") {
    return "N5_REQUEST_CLEANUP_PREVIEW";
  }

  return "manual";
}

export function deriveGlobalStatus({
  scanStatus,
  duplicateStatus,
  currentScan,
  interruptedRuns,
  duplicateEligible,
  browseableScan,
  cleanupPreview,
  cleanupExecutionResult,
  cleanupExecutionScanId,
  cleanupPreviewAvailable,
}: DeriveGlobalStatusInput): GlobalStatusModel {
  if (scanStatus.state === "running") {
    return {
      primaryStateLabel: "Live scan running",
      contextLabel: scanStatus.rootPath ?? "Scan root pending.",
      summaryLabel: `${formatBytes(scanStatus.bytesProcessed)} processed`,
      nextSafeAction: {
        label: "View scan progress",
        target: "scan",
      },
      noActionLabel: null,
    };
  }

  if (
    duplicateStatus.state === "running" &&
    currentScan &&
    duplicateStatus.scanId === currentScan.scanId
  ) {
    return {
      primaryStateLabel: "Live duplicate analysis running",
      contextLabel: buildScanContext(currentScan),
      summaryLabel: `${getDuplicateStageLabel(duplicateStatus.stage)} | ${duplicateStatus.itemsProcessed} items processed`,
      nextSafeAction: {
        label: "View duplicate analysis",
        target: "duplicates",
      },
      noActionLabel: null,
    };
  }

  if (
    cleanupPreview &&
    !cleanupExecutionResult &&
    currentScan &&
    cleanupPreview.scanId === currentScan.scanId
  ) {
    return {
      primaryStateLabel: "Cleanup preview ready",
      contextLabel: buildScanContext(currentScan),
      summaryLabel: `${cleanupPreview.candidates.length} candidates | ${formatBytes(
        cleanupPreview.totalBytes,
      )}`,
      nextSafeAction: {
        label: "Review cleanup preview",
        target: "cleanup",
      },
      noActionLabel: null,
    };
  }

  if (
    cleanupExecutionResult &&
    currentScan &&
    cleanupExecutionScanId === currentScan.scanId
  ) {
    return {
      primaryStateLabel: "Cleanup execution completed with rescan recommended",
      contextLabel: buildScanContext(currentScan),
      summaryLabel: `${cleanupExecutionResult.completedCount} completed | ${cleanupExecutionResult.failedCount} failed`,
      nextSafeAction: {
        label: "Start a fresh scan",
        target: "scan",
      },
      noActionLabel: null,
    };
  }

  if (interruptedRuns.length > 0) {
    return {
      primaryStateLabel: "Interrupted runs need review",
      contextLabel: interruptedRuns[0]?.header.rootPath ?? "Recovery records available.",
      summaryLabel: `${interruptedRuns.length} interrupted runs`,
      nextSafeAction: {
        label: "Review interrupted runs",
        target: "history",
      },
      noActionLabel: null,
    };
  }

  if (currentScan) {
    if (duplicateEligible) {
      return {
        primaryStateLabel: "Completed scan loaded",
        contextLabel: buildScanContext(currentScan),
        summaryLabel: `${formatBytes(currentScan.totalBytes)} | duplicate analysis available`,
        nextSafeAction: {
          label: "Find duplicates",
          target: "duplicates",
        },
        noActionLabel: null,
      };
    }

    if (cleanupPreviewAvailable) {
      return {
        primaryStateLabel: "Completed scan loaded",
        contextLabel: buildScanContext(currentScan),
        summaryLabel: `${formatBytes(currentScan.totalBytes)} | cleanup preview can be prepared`,
        nextSafeAction: {
          label: "Preview cleanup",
          target: "cleanup",
        },
        noActionLabel: null,
      };
    }

    if (browseableScan) {
      return {
        primaryStateLabel: "Completed scan loaded",
        contextLabel: buildScanContext(currentScan),
        summaryLabel: `${formatBytes(currentScan.totalBytes)} | browseable result ready`,
        nextSafeAction: {
          label: "Browse results",
          target: "explorer",
        },
        noActionLabel: null,
      };
    }

    return {
      primaryStateLabel: "Completed scan loaded",
      contextLabel: buildScanContext(currentScan),
      summaryLabel: `${formatBytes(currentScan.totalBytes)} | summary-only result`,
      nextSafeAction: null,
      noActionLabel: "No safe next action right now.",
    };
  }

  return {
    primaryStateLabel: "Ready / no scan loaded",
    contextLabel: "Manual workspace navigation is available.",
    summaryLabel: "Start a scan or reopen local history when ready.",
    nextSafeAction: {
      label: "Start a scan",
      target: "scan",
    },
    noActionLabel: null,
  };
}
