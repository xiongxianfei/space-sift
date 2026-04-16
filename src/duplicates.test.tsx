import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import type { SpaceSiftClient } from "./lib/spaceSiftClient";
import type {
  CompletedDuplicateAnalysis,
  CompletedScan,
  DuplicateStatusSnapshot,
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
    filesDiscovered: 4,
    directoriesDiscovered: 2,
    bytesProcessed: 96,
    startedAt: "2026-04-15T10:59:00Z",
    updatedAt: "2026-04-15T11:00:00Z",
    currentPath: "C:\\Users\\xiongxianfei\\Downloads",
    message: "Scan complete.",
    completedScanId: scanId,
  };
}

function makeIdleDuplicateStatus(): DuplicateStatusSnapshot {
  return {
    analysisId: null,
    scanId: null,
    state: "idle",
    stage: null,
    itemsProcessed: 0,
    groupsEmitted: 0,
    message: null,
    completedAnalysisId: null,
  };
}

function makeHistoryEntry(scanId: string): ScanHistoryEntry {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 96,
  };
}

function makeBrowseableScan(scanId: string): CompletedScan {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T10:59:00Z",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 96,
    totalFiles: 4,
    totalDirectories: 2,
    largestFiles: [],
    largestDirectories: [],
    skippedPaths: [],
    entries: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads",
        parentPath: null,
        kind: "directory",
        sizeBytes: 96,
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
        path: "C:\\Users\\xiongxianfei\\Downloads\\notes.txt",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 16,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\other.txt",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 16,
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
    totalBytes: 96,
    totalFiles: 4,
    totalDirectories: 2,
    largestFiles: [],
    largestDirectories: [],
    skippedPaths: [],
  };
}

function makeDuplicateAnalysis(scanId: string): CompletedDuplicateAnalysis {
  return {
    analysisId: "analysis-1",
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T11:01:00Z",
    completedAt: "2026-04-15T11:01:10Z",
    groups: [
      {
        groupId: "analysis-1-group-1",
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
    issues: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\missing.bin",
        code: "missing_path",
        summary: "Path no longer exists.",
      },
    ],
  };
}

function createDuplicateClient(options?: {
  scan?: CompletedScan;
  duplicateAnalysis?: CompletedDuplicateAnalysis;
  initialScanStatus?: ScanStatusSnapshot;
}) {
  let duplicateListener: ((snapshot: DuplicateStatusSnapshot) => void) | null = null;
  const scan = options?.scan ?? makeBrowseableScan("scan-duplicates");
  const duplicateAnalysis = options?.duplicateAnalysis ?? makeDuplicateAnalysis(scan.scanId);
  const initialScanStatus = options?.initialScanStatus ?? makeCompletedScanStatus(scan.scanId);
  const client: SpaceSiftClient = {
    startScan: vi.fn(async () => ({ scanId: "scan-running" })),
    cancelActiveScan: vi.fn(async () => {}),
    getScanStatus: vi.fn(async () => initialScanStatus),
    listScanHistory: vi.fn(async () => [makeHistoryEntry(scan.scanId)]),
    openScanHistory: vi.fn(async () => scan),
    startDuplicateAnalysis: vi.fn(async () => ({ analysisId: duplicateAnalysis.analysisId })),
    cancelDuplicateAnalysis: vi.fn(async () => {}),
    getDuplicateAnalysisStatus: vi.fn(async () => makeIdleDuplicateStatus()),
    openDuplicateAnalysis: vi.fn(async () => duplicateAnalysis),
    listCleanupRules: vi.fn(async () => []),
    previewCleanup: vi.fn(async () => {
      throw new Error("no cleanup preview");
    }),
    executeCleanup: vi.fn(async () => {
      throw new Error("no cleanup execution");
    }),
    getPrivilegedCleanupCapability: vi.fn(async () => ({
      available: false,
      message:
        "Protected-path cleanup stays outside the unprivileged desktop flow in this milestone.",
    })),
    openPathInExplorer: vi.fn(async () => {}),
    subscribeToScanProgress: vi.fn(async () => () => {}),
    subscribeToDuplicateProgress: vi.fn(async (listener) => {
      duplicateListener = listener;
      return () => {
        duplicateListener = null;
      };
    }),
  };

  return {
    client,
    emitDuplicate(snapshot: DuplicateStatusSnapshot) {
      duplicateListener?.(snapshot);
    },
  };
}

describe("Space Sift duplicate workflow", () => {
  it("runs duplicate analysis from a loaded scan and renders verified groups", async () => {
    const mock = createDuplicateClient();
    render(<App client={mock.client} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /analyze duplicates/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /analyze duplicates/i }));

    expect(mock.client.startDuplicateAnalysis).toHaveBeenCalledWith("scan-duplicates");

    await act(async () => {
      mock.emitDuplicate({
        analysisId: "analysis-1",
        scanId: "scan-duplicates",
        state: "running",
        stage: "full_hash",
        itemsProcessed: 3,
        groupsEmitted: 0,
        message: null,
        completedAnalysisId: null,
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/full hash/i)).toBeInTheDocument();
      expect(screen.getByText(/3 items processed/i)).toBeInTheDocument();
    });

    await act(async () => {
      mock.emitDuplicate({
        analysisId: "analysis-1",
        scanId: "scan-duplicates",
        state: "completed",
        stage: "completed",
        itemsProcessed: 4,
        groupsEmitted: 1,
        message: "Duplicate analysis complete.",
        completedAnalysisId: "analysis-1",
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/left\.bin/i)).toBeInTheDocument();
      expect(screen.getByText(/right\.bin/i)).toBeInTheDocument();
      expect(screen.getByText(/1 files marked for later deletion/i)).toBeInTheDocument();
      expect(screen.getAllByText(/32 bytes/i).length).toBeGreaterThan(0);
    });
  }, uiTestTimeout);

  it("cancels a running duplicate analysis and returns to a clean review state", async () => {
    const mock = createDuplicateClient();
    render(<App client={mock.client} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /analyze duplicates/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /analyze duplicates/i }));

    await act(async () => {
      mock.emitDuplicate({
        analysisId: "analysis-1",
        scanId: "scan-duplicates",
        state: "running",
        stage: "full_hash",
        itemsProcessed: 3,
        groupsEmitted: 0,
        message: null,
        completedAnalysisId: null,
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/full hash/i)).toBeInTheDocument();
      expect(screen.getByText(/3 items processed/i)).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /cancel analysis/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /cancel analysis/i }));
    expect(mock.client.cancelDuplicateAnalysis).toHaveBeenCalledTimes(1);

    await act(async () => {
      mock.emitDuplicate({
        analysisId: "analysis-1",
        scanId: "scan-duplicates",
        state: "cancelled",
        stage: null,
        itemsProcessed: 3,
        groupsEmitted: 0,
        message: "Duplicate analysis cancelled before completion.",
        completedAnalysisId: null,
      });
    });

    await waitFor(() => {
      expect(
        screen.getByText(/duplicate analysis cancelled before completion/i),
      ).toBeInTheDocument();
      expect(screen.queryByText(/3 items processed/i)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /analyze duplicates/i })).toBeEnabled();
      expect(
        screen.queryByTestId("duplicate-group-analysis-1-group-1"),
      ).not.toBeInTheDocument();
    });
  }, uiTestTimeout);

  it("applies keep-selection helpers and manual keep selection", async () => {
    const mock = createDuplicateClient();
    render(<App client={mock.client} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /analyze duplicates/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /analyze duplicates/i }));

    await act(async () => {
      mock.emitDuplicate({
        analysisId: "analysis-1",
        scanId: "scan-duplicates",
        state: "completed",
        stage: "completed",
        itemsProcessed: 4,
        groupsEmitted: 1,
        message: "Duplicate analysis complete.",
        completedAnalysisId: "analysis-1",
      });
    });

    const group = await screen.findByTestId("duplicate-group-analysis-1-group-1");
    expect(
      within(group).getByRole("button", { name: /keep newest/i }),
    ).toHaveAttribute("aria-pressed", "true");
    expect(
      within(group).getByRole("button", { name: /keep oldest/i }),
    ).toHaveAttribute("aria-pressed", "false");
    expect(
      within(group).getByRole("button", {
        name: /delete candidate for left\.bin/i,
      }),
    ).toBeInTheDocument();

    fireEvent.click(within(group).getByRole("button", { name: /keep oldest/i }));

    await waitFor(() => {
      expect(
        within(group).getByRole("button", { name: /keep oldest/i }),
      ).toHaveAttribute("aria-pressed", "true");
      expect(
        within(group).getByRole("button", {
          name: /kept copy for left\.bin/i,
        }),
      ).toHaveAttribute("aria-pressed", "true");
      expect(
        within(group).getByRole("button", {
          name: /delete candidate for right\.bin/i,
        }),
      ).toBeInTheDocument();
    });

    fireEvent.click(
      within(group).getByRole("button", {
        name: /delete candidate for right\.bin/i,
      }),
    );

    await waitFor(() => {
      expect(screen.getByText(/1 files marked for later deletion/i)).toBeInTheDocument();
      expect(
        within(group).getByRole("button", { name: /keep newest/i }),
      ).toHaveAttribute("aria-pressed", "true");
      expect(
        within(group).getByRole("button", {
          name: /kept copy for right\.bin/i,
        }),
      ).toHaveAttribute("aria-pressed", "true");
    });
  }, uiTestTimeout);

  it("requires a fresh scan for summary-only history entries", async () => {
    const mock = createDuplicateClient({
      scan: makeSummaryOnlyScan("scan-legacy"),
      initialScanStatus: makeCompletedScanStatus("scan-legacy"),
    });
    render(<App client={mock.client} />);

    await waitFor(() => {
      expect(screen.getByText(/fresh scan is required before duplicate analysis/i)).toBeInTheDocument();
    });

    expect(screen.queryByRole("button", { name: /analyze duplicates/i })).not.toBeInTheDocument();
  }, uiTestTimeout);

  it("shows duplicate issues and an empty-state when no verified groups remain", async () => {
    const mock = createDuplicateClient({
      duplicateAnalysis: {
        analysisId: "analysis-empty",
        scanId: "scan-duplicates",
        rootPath: "C:\\Users\\xiongxianfei\\Downloads",
        startedAt: "2026-04-15T11:01:00Z",
        completedAt: "2026-04-15T11:01:10Z",
        groups: [],
        issues: [
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\missing.bin",
            code: "missing_path",
            summary: "Path no longer exists.",
          },
        ],
      },
    });
    render(<App client={mock.client} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /analyze duplicates/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /analyze duplicates/i }));

    await act(async () => {
      mock.emitDuplicate({
        analysisId: "analysis-empty",
        scanId: "scan-duplicates",
        state: "completed",
        stage: "completed",
        itemsProcessed: 4,
        groupsEmitted: 0,
        message: "Duplicate analysis complete.",
        completedAnalysisId: "analysis-empty",
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/no fully verified duplicate groups/i)).toBeInTheDocument();
      expect(screen.getByText(/missing\.bin/i)).toBeInTheDocument();
      expect(screen.getByText(/path no longer exists/i)).toBeInTheDocument();
    });
  }, uiTestTimeout);
});
