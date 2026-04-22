import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import type { SpaceSiftClient } from "./lib/spaceSiftClient";
import type {
  CleanupExecutionResult,
  CleanupPreview,
  CleanupRuleDefinition,
  CompletedDuplicateAnalysis,
  CompletedScan,
  DuplicateStatusSnapshot,
  PrivilegedCleanupCapability,
  ScanHistoryEntry,
  ScanStatusSnapshot,
} from "./lib/spaceSiftTypes";

const uiReadyTimeout = 5000;
const uiTestTimeout = 15000;

function makeCompletedScanStatus(scanId: string): ScanStatusSnapshot {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    state: "completed",
    filesDiscovered: 5,
    directoriesDiscovered: 3,
    bytesProcessed: 160,
    startedAt: "2026-04-15T10:59:00Z",
    updatedAt: "2026-04-15T11:00:00Z",
    currentPath: "C:\\Users\\xiongxianfei\\Downloads",
    message: "Scan complete.",
    completedScanId: scanId,
  };
}

function makeCompletedDuplicateStatus(
  scanId: string,
  analysisId: string,
): DuplicateStatusSnapshot {
  return {
    analysisId,
    scanId,
    state: "completed",
    stage: "completed",
    itemsProcessed: 5,
    groupsEmitted: 1,
    message: "Duplicate analysis complete.",
    completedAnalysisId: analysisId,
  };
}

function makeHistoryEntry(scanId: string): ScanHistoryEntry {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 160,
  };
}

function makeBrowseableScan(scanId: string): CompletedScan {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T10:59:00Z",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 160,
    totalFiles: 5,
    totalDirectories: 3,
    largestFiles: [],
    largestDirectories: [],
    skippedPaths: [],
    entries: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads",
        parentPath: null,
        kind: "directory",
        sizeBytes: 160,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\left.bin",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 32,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\right.bin",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 32,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Temp\\cache.tmp",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Temp",
        kind: "file",
        sizeBytes: 48,
      },
    ],
  };
}

function makeSummaryOnlyScan(scanId: string): CompletedScan {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T10:59:00Z",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 160,
    totalFiles: 5,
    totalDirectories: 3,
    largestFiles: [],
    largestDirectories: [],
    skippedPaths: [],
  };
}

function makeDuplicateAnalysis(scanId: string): CompletedDuplicateAnalysis {
  return {
    analysisId: "analysis-cleanup",
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T11:01:00Z",
    completedAt: "2026-04-15T11:01:10Z",
    groups: [
      {
        groupId: "analysis-cleanup-group-1",
        sizeBytes: 32,
        reclaimableBytes: 32,
        members: [
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\left.bin",
            sizeBytes: 32,
            modifiedAt: "2026-04-14T10:00:00Z",
          },
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\right.bin",
            sizeBytes: 32,
            modifiedAt: "2026-04-15T10:00:00Z",
          },
        ],
      },
    ],
    issues: [],
  };
}

function makeCleanupRules(): CleanupRuleDefinition[] {
  return [
    {
      ruleId: "temp-folder-files",
      label: "Files in Temp folders",
      description: "Files under directories named Temp or TMP within the current scan root.",
    },
    {
      ruleId: "download-partials",
      label: "Partial downloads",
      description: "Incomplete download files such as .crdownload and .part.",
    },
  ];
}

function makeCleanupPreview(scanId: string): CleanupPreview {
  return {
    previewId: "preview-1",
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    generatedAt: "2026-04-15T11:02:00Z",
    totalBytes: 80,
    duplicateCandidateCount: 1,
    ruleCandidateCount: 1,
    candidates: [
      {
        actionId: "action-duplicate",
        path: "C:\\Users\\xiongxianfei\\Downloads\\left.bin",
        sizeBytes: 32,
        sourceLabels: ["Duplicate selection"],
      },
      {
        actionId: "action-temp",
        path: "C:\\Users\\xiongxianfei\\Downloads\\Temp\\cache.tmp",
        sizeBytes: 48,
        sourceLabels: ["Files in Temp folders"],
      },
    ],
    issues: [],
  };
}

function makeExecutionResult(mode: "recycle" | "permanent"): CleanupExecutionResult {
  return {
    executionId: `execution-${mode}`,
    previewId: "preview-1",
    mode,
    completedAt: "2026-04-15T11:03:00Z",
    completedCount: 2,
    failedCount: 0,
    entries: [
      {
        actionId: "action-duplicate",
        path: "C:\\Users\\xiongxianfei\\Downloads\\left.bin",
        status: "completed",
        summary:
          mode === "recycle"
            ? "Moved to the Recycle Bin."
            : "Permanently deleted.",
      },
    ],
  };
}

function makeCapability(): PrivilegedCleanupCapability {
  return {
    available: false,
    message:
      "Protected-path cleanup stays outside the unprivileged desktop flow in this milestone.",
  };
}

function createCleanupClient(options?: {
  scan?: CompletedScan;
}) {
  const scan = options?.scan ?? makeBrowseableScan("scan-cleanup");
  const duplicateAnalysis = makeDuplicateAnalysis(scan.scanId);
  const client: SpaceSiftClient = {
    startScan: vi.fn(async () => ({ scanId: "scan-running" })),
    cancelActiveScan: vi.fn(async () => {}),
    cancelScanRun: vi.fn(async () => {}),
    getScanStatus: vi.fn(async () => makeCompletedScanStatus(scan.scanId)),
    getWorkspaceRestoreContext: vi.fn(async () => null),
    saveWorkspaceRestoreContext: vi.fn(async ({ lastWorkspace, lastOpenedScanId }) => ({
      schemaVersion: 1,
      lastWorkspace,
      lastOpenedScanId,
      updatedAt: "2026-04-22T10:00:00Z",
    })),
    listScanHistory: vi.fn(async () => [makeHistoryEntry(scan.scanId)]),
    openScanHistory: vi.fn(async () => scan),
    listScanRuns: vi.fn(async () => []),
    openScanRun: vi.fn(async () => {
      throw new Error("no scan run");
    }),
    resumeScanRun: vi.fn(async () => ({ runId: "run-resumed" })),
    startDuplicateAnalysis: vi.fn(async () => ({ analysisId: duplicateAnalysis.analysisId })),
    cancelDuplicateAnalysis: vi.fn(async () => {}),
    getDuplicateAnalysisStatus: vi.fn(async () =>
      makeCompletedDuplicateStatus(scan.scanId, duplicateAnalysis.analysisId),
    ),
    openDuplicateAnalysis: vi.fn(async () => duplicateAnalysis),
    listCleanupRules: vi.fn(async () => makeCleanupRules()),
    previewCleanup: vi.fn(async () => makeCleanupPreview(scan.scanId)),
    executeCleanup: vi.fn(async ({ mode }: { mode: "recycle" | "permanent" }) =>
      makeExecutionResult(mode),
    ),
    getPrivilegedCleanupCapability: vi.fn(async () => makeCapability()),
    openPathInExplorer: vi.fn(async () => {}),
    subscribeToScanProgress: vi.fn(async () => () => {}),
    subscribeToDuplicateProgress: vi.fn(async () => () => {}),
  };

  return client;
}

describe("Space Sift cleanup workflow", () => {
  it("builds a cleanup preview from duplicate selections and enabled rules", async () => {
    const client = createCleanupClient();
    render(<App client={client} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /refresh cleanup preview/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByLabelText(/files in temp folders/i));
    fireEvent.click(screen.getByRole("button", { name: /refresh cleanup preview/i }));

    await waitFor(() => {
      expect(client.previewCleanup).toHaveBeenCalledWith({
        scanId: "scan-cleanup",
        duplicateDeletePaths: ["C:\\Users\\xiongxianfei\\Downloads\\left.bin"],
        enabledRuleIds: ["temp-folder-files"],
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/2 cleanup candidates/i)).toBeInTheDocument();
      expect(screen.getByText(/80 bytes/i)).toBeInTheDocument();
      expect(screen.getByText(/duplicate selection/i)).toBeInTheDocument();
      expect(screen.getByText(/files in temp folders/i)).toBeInTheDocument();
    });
  }, uiTestTimeout);

  it("executes recycle-bin-first cleanup by default and recommends a fresh scan", async () => {
    const client = createCleanupClient();
    render(<App client={client} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /refresh cleanup preview/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /refresh cleanup preview/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /move selected files to recycle bin/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /move selected files to recycle bin/i }));

    await waitFor(() => {
      expect(client.executeCleanup).toHaveBeenCalledWith({
        previewId: "preview-1",
        actionIds: ["action-duplicate", "action-temp"],
        mode: "recycle",
      });
      expect(screen.getByText(/cleanup completed/i)).toBeInTheDocument();
      expect(screen.getByText(/fresh scan is recommended/i)).toBeInTheDocument();
    });
  }, uiTestTimeout);

  it("keeps permanent delete behind an explicit advanced confirmation path", async () => {
    const client = createCleanupClient();
    render(<App client={client} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /refresh cleanup preview/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /refresh cleanup preview/i }));

    await waitFor(() => {
      expect(screen.getByLabelText(/i understand permanent delete cannot be undone/i)).toBeInTheDocument();
    });

    expect(
      screen.queryByRole("button", { name: /permanently delete selected files/i }),
    ).not.toBeInTheDocument();

    fireEvent.click(screen.getByLabelText(/i understand permanent delete cannot be undone/i));
    fireEvent.click(screen.getByRole("button", { name: /permanently delete selected files/i }));

    await waitFor(() => {
      expect(client.executeCleanup).toHaveBeenCalledWith({
        previewId: "preview-1",
        actionIds: ["action-duplicate", "action-temp"],
        mode: "permanent",
      });
    });
  }, uiTestTimeout);

  it("requires a fresh scan before cleanup preview when file-entry data is missing", async () => {
    render(<App client={createCleanupClient({ scan: makeSummaryOnlyScan("scan-legacy") })} />);

    await waitFor(() => {
      expect(screen.getByText(/fresh scan is required before cleanup preview/i)).toBeInTheDocument();
    });

    expect(
      screen.queryByRole("button", { name: /refresh cleanup preview/i }),
    ).not.toBeInTheDocument();
  }, uiTestTimeout);
});
