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

function makeTriageScan(scanId: string): CompletedScan {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T10:59:00Z",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 512,
    totalFiles: 7,
    totalDirectories: 5,
    largestFiles: [],
    largestDirectories: [],
    skippedPaths: [],
    entries: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads",
        parentPath: null,
        kind: "directory",
        sizeBytes: 512,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\A",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "directory",
        sizeBytes: 128,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\B",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "directory",
        sizeBytes: 128,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Clients",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "directory",
        sizeBytes: 128,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Clients\\North",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Clients",
        kind: "directory",
        sizeBytes: 64,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Clients\\South",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Clients",
        kind: "directory",
        sizeBytes: 64,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\triple-a.bin",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 32,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\triple-b.bin",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 32,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\triple-c.bin",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 32,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\A\\budget.xlsx",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\A",
        kind: "file",
        sizeBytes: 64,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\A\\budget-copy.xlsx",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\A",
        kind: "file",
        sizeBytes: 64,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\B\\budget.xlsx",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\B",
        kind: "file",
        sizeBytes: 64,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\B\\budget-copy.xlsx",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\B",
        kind: "file",
        sizeBytes: 64,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Clients\\North\\report.docx",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Clients\\North",
        kind: "file",
        sizeBytes: 64,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Clients\\South\\report.docx",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Clients\\South",
        kind: "file",
        sizeBytes: 64,
      },
    ],
  };
}

function makeTriageDuplicateAnalysis(scanId: string): CompletedDuplicateAnalysis {
  return {
    analysisId: "analysis-triage",
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T11:01:00Z",
    completedAt: "2026-04-15T11:01:10Z",
    groups: [
      {
        groupId: "analysis-triage-group-b",
        sizeBytes: 64,
        reclaimableBytes: 64,
        members: [
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\B\\budget.xlsx",
            sizeBytes: 64,
            modifiedAt: "2026-04-14T12:00:00Z",
          },
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\B\\budget-copy.xlsx",
            sizeBytes: 64,
            modifiedAt: "2026-04-15T12:00:00Z",
          },
        ],
      },
      {
        groupId: "analysis-triage-group-three",
        sizeBytes: 32,
        reclaimableBytes: 64,
        members: [
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\triple-a.bin",
            sizeBytes: 32,
            modifiedAt: "2026-04-13T09:00:00Z",
          },
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\triple-b.bin",
            sizeBytes: 32,
            modifiedAt: "2026-04-14T09:00:00Z",
          },
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\triple-c.bin",
            sizeBytes: 32,
            modifiedAt: "2026-04-15T09:00:00Z",
          },
        ],
      },
      {
        groupId: "analysis-triage-group-a",
        sizeBytes: 64,
        reclaimableBytes: 64,
        members: [
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\A\\budget.xlsx",
            sizeBytes: 64,
            modifiedAt: "2026-04-14T11:00:00Z",
          },
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\A\\budget-copy.xlsx",
            sizeBytes: 64,
            modifiedAt: "2026-04-15T11:00:00Z",
          },
        ],
      },
      {
        groupId: "analysis-triage-group-report",
        sizeBytes: 64,
        reclaimableBytes: 64,
        members: [
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\Clients\\North\\report.docx",
            sizeBytes: 64,
            modifiedAt: "2026-04-14T08:00:00Z",
          },
          {
            path: "C:\\Users\\xiongxianfei\\Downloads\\Clients\\South\\report.docx",
            sizeBytes: 64,
            modifiedAt: "2026-04-15T08:00:00Z",
          },
        ],
      },
    ],
    issues: [],
  };
}

function makeSeededLargeDuplicateAnalysis(scanId: string): CompletedDuplicateAnalysis {
  return {
    analysisId: "analysis-large",
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T11:05:00Z",
    completedAt: "2026-04-15T11:05:30Z",
    groups: Array.from({ length: 22 }, (_, offset) => {
      const index = offset + 1;
      const groupId = `analysis-large-group-${String(index).padStart(2, "0")}`;
      const memberCount = index === 22 || index % 5 === 0 ? 3 : 2;
      const reclaimableBytes = index * 128;
      const sizeBytes = memberCount === 3 ? reclaimableBytes / 2 : reclaimableBytes;
      const basename = index >= 20 ? "report.docx" : `artifact-${String(index).padStart(2, "0")}.bin`;
      const locationBase =
        index === 20
          ? "Clients\\North"
          : index === 21
            ? "Clients\\South"
            : index === 22
              ? "Clients\\West"
              : `Sets\\Group-${String(index).padStart(2, "0")}`;

      return {
        groupId,
        sizeBytes,
        reclaimableBytes,
        members: Array.from({ length: memberCount }, (_, memberOffset) => ({
          path: `C:\\Users\\xiongxianfei\\Downloads\\${locationBase}\\Copy-${memberOffset + 1}\\${basename}`,
          sizeBytes,
          modifiedAt: `2026-04-${String((memberOffset % 9) + 10).padStart(2, "0")}T0${(index % 6) + 1}:00:00Z`,
        })),
      };
    }),
    issues: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Remote\\report.docx",
        code: "read_error",
        summary: "Remote paths were skipped.",
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Cloud\\placeholder.docx",
        code: "read_error",
        summary: "Cloud placeholder was skipped.",
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
    cancelScanRun: vi.fn(async () => {}),
    getScanStatus: vi.fn(async () => initialScanStatus),
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

    const group = await screen.findByTestId("duplicate-group-analysis-1-group-1");

    await waitFor(() => {
      expect(
        within(group).getByRole("button", { name: /show details/i }),
      ).toHaveAttribute("aria-expanded", "false");
      expect(screen.getByText(/1 files marked for later deletion/i)).toBeInTheDocument();
      expect(screen.getAllByText(/32 bytes/i).length).toBeGreaterThan(0);
    });

    fireEvent.click(within(group).getByRole("button", { name: /show details/i }));

    await waitFor(() => {
      expect(
        within(group).getByRole("button", { name: /hide details/i }),
      ).toHaveAttribute("aria-expanded", "true");
      expect(within(group).getByText(/left\.bin/i)).toBeInTheDocument();
      expect(within(group).getByText(/right\.bin/i)).toBeInTheDocument();
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
      expect(
        screen.getByRole("button", { name: /cancel analysis/i }),
      ).toBeInTheDocument();
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
      expect(
        screen.getByRole("button", { name: /analyze duplicates/i }),
      ).toBeEnabled();
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
    fireEvent.click(within(group).getByRole("button", { name: /show details/i }));

    await waitFor(() => {
      expect(
        within(group).getByRole("button", { name: /hide details/i }),
      ).toHaveAttribute("aria-expanded", "true");
    });

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

  it("orders duplicate groups deterministically and exposes disclosure state", async () => {
    const scan = makeTriageScan("scan-triage");
    const duplicateAnalysis = makeTriageDuplicateAnalysis(scan.scanId);
    const mock = createDuplicateClient({
      scan,
      duplicateAnalysis,
      initialScanStatus: makeCompletedScanStatus(scan.scanId),
    });
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
        analysisId: duplicateAnalysis.analysisId,
        scanId: scan.scanId,
        state: "completed",
        stage: "completed",
        itemsProcessed: 11,
        groupsEmitted: duplicateAnalysis.groups.length,
        message: "Duplicate analysis complete.",
        completedAnalysisId: duplicateAnalysis.analysisId,
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/4 duplicate groups/i)).toBeInTheDocument();
    });

    expect(
      screen
        .getAllByTestId(/duplicate-group-/)
        .map((element) => element.getAttribute("data-testid")),
    ).toEqual([
      "duplicate-group-analysis-triage-group-three",
      "duplicate-group-analysis-triage-group-a",
      "duplicate-group-analysis-triage-group-b",
      "duplicate-group-analysis-triage-group-report",
    ]);

    const highestImpactGroup = screen.getByTestId("duplicate-group-analysis-triage-group-three");
    const disclosureButton = within(highestImpactGroup).getByRole("button", {
      name: /show details/i,
    });

    expect(disclosureButton).toHaveAttribute("aria-expanded", "false");

    fireEvent.click(disclosureButton);

    await waitFor(() => {
      expect(
        within(highestImpactGroup).getByRole("button", { name: /hide details/i }),
      ).toHaveAttribute("aria-expanded", "true");
      expect(within(highestImpactGroup).getByText(/triple-a\.bin/i)).toBeInTheDocument();
    });
  }, uiTestTimeout);

  it("keeps duplicate details open when the same completed snapshot is replayed", async () => {
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

    const completedSnapshot: DuplicateStatusSnapshot = {
      analysisId: "analysis-1",
      scanId: "scan-duplicates",
      state: "completed",
      stage: "completed",
      itemsProcessed: 4,
      groupsEmitted: 1,
      message: "Duplicate analysis complete.",
      completedAnalysisId: "analysis-1",
    };

    await act(async () => {
      mock.emitDuplicate(completedSnapshot);
    });

    const group = await screen.findByTestId("duplicate-group-analysis-1-group-1");
    fireEvent.click(within(group).getByRole("button", { name: /show details/i }));

    await waitFor(() => {
      expect(
        within(group).getByRole("button", { name: /hide details/i }),
      ).toHaveAttribute("aria-expanded", "true");
      expect(within(group).getByText(/left\.bin/i)).toBeInTheDocument();
    });

    await act(async () => {
      mock.emitDuplicate(completedSnapshot);
    });

    await waitFor(() => {
      expect(
        within(group).getByRole("button", { name: /hide details/i }),
      ).toHaveAttribute("aria-expanded", "true");
      expect(within(group).getByText(/left\.bin/i)).toBeInTheDocument();
      expect(mock.client.openDuplicateAnalysis).toHaveBeenCalledTimes(1);
    });
  }, uiTestTimeout);

  it("shows visible location context for same-name files and keeps group order stable while reviewing", async () => {
    const scan = makeTriageScan("scan-triage");
    const duplicateAnalysis = makeTriageDuplicateAnalysis(scan.scanId);
    const mock = createDuplicateClient({
      scan,
      duplicateAnalysis,
      initialScanStatus: makeCompletedScanStatus(scan.scanId),
    });
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
        analysisId: duplicateAnalysis.analysisId,
        scanId: scan.scanId,
        state: "completed",
        stage: "completed",
        itemsProcessed: 11,
        groupsEmitted: duplicateAnalysis.groups.length,
        message: "Duplicate analysis complete.",
        completedAnalysisId: duplicateAnalysis.analysisId,
      });
    });

    const orderBefore = () =>
      screen
        .getAllByTestId(/duplicate-group-/)
        .map((element) => element.getAttribute("data-testid"));

    await waitFor(() => {
      expect(orderBefore()).toEqual([
        "duplicate-group-analysis-triage-group-three",
        "duplicate-group-analysis-triage-group-a",
        "duplicate-group-analysis-triage-group-b",
        "duplicate-group-analysis-triage-group-report",
      ]);
    });

    const sameNameGroup = screen.getByTestId("duplicate-group-analysis-triage-group-report");
    fireEvent.click(within(sameNameGroup).getByRole("button", { name: /show details/i }));

    await waitFor(() => {
      expect(within(sameNameGroup).getAllByText(/^report\.docx$/i)).toHaveLength(2);
      expect(within(sameNameGroup).getByText(/clients\\north/i)).toBeInTheDocument();
      expect(within(sameNameGroup).getByText(/clients\\south/i)).toBeInTheDocument();
    });

    fireEvent.click(within(sameNameGroup).getByRole("button", { name: /keep oldest/i }));

    await waitFor(() => {
      expect(
        within(sameNameGroup).getByRole("button", { name: /keep oldest/i }),
      ).toHaveAttribute("aria-pressed", "true");
      expect(orderBefore()).toEqual([
        "duplicate-group-analysis-triage-group-three",
        "duplicate-group-analysis-triage-group-a",
        "duplicate-group-analysis-triage-group-b",
        "duplicate-group-analysis-triage-group-report",
      ]);
    });
  }, uiTestTimeout);

  it("keeps a seeded large duplicate review state readable and focusable", async () => {
    const scan = makeBrowseableScan("scan-large-review");
    const duplicateAnalysis = makeSeededLargeDuplicateAnalysis(scan.scanId);
    const mock = createDuplicateClient({
      scan,
      duplicateAnalysis,
      initialScanStatus: makeCompletedScanStatus(scan.scanId),
    });
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
        analysisId: duplicateAnalysis.analysisId,
        scanId: scan.scanId,
        state: "completed",
        stage: "completed",
        itemsProcessed: 57,
        groupsEmitted: duplicateAnalysis.groups.length,
        message: "Duplicate analysis complete.",
        completedAnalysisId: duplicateAnalysis.analysisId,
      });
    });

    const currentOrder = () =>
      screen
        .getAllByTestId(/duplicate-group-/)
        .map((element) => element.getAttribute("data-testid"));

    await waitFor(() => {
      expect(screen.getByText(/22 duplicate groups/i)).toBeInTheDocument();
      expect(screen.getAllByRole("button", { name: /show details/i })).toHaveLength(22);
      expect(currentOrder().slice(0, 3)).toEqual([
        "duplicate-group-analysis-large-group-22",
        "duplicate-group-analysis-large-group-21",
        "duplicate-group-analysis-large-group-20",
      ]);
    });

    const topGroup = screen.getByTestId("duplicate-group-analysis-large-group-22");
    const disclosureButton = within(topGroup).getByRole("button", {
      name: /show details/i,
    });
    disclosureButton.focus();
    expect(disclosureButton).toHaveFocus();

    fireEvent.click(disclosureButton);

    await waitFor(() => {
      expect(
        within(topGroup).getByRole("button", { name: /hide details/i }),
      ).toHaveAttribute("aria-expanded", "true");
      expect(within(topGroup).getAllByText(/^report\.docx$/i)).toHaveLength(3);
      expect(within(topGroup).getByText(/clients\\west\\copy-1/i)).toBeInTheDocument();
      expect(within(topGroup).getByText(/clients\\west\\copy-2/i)).toBeInTheDocument();
      expect(within(topGroup).getByText(/clients\\west\\copy-3/i)).toBeInTheDocument();
    });

    fireEvent.click(within(topGroup).getByRole("button", { name: /keep oldest/i }));

    await waitFor(() => {
      expect(
        within(topGroup).getByRole("button", { name: /keep oldest/i }),
      ).toHaveAttribute("aria-pressed", "true");
      expect(currentOrder().slice(0, 3)).toEqual([
        "duplicate-group-analysis-large-group-22",
        "duplicate-group-analysis-large-group-21",
        "duplicate-group-analysis-large-group-20",
      ]);
    });
  }, uiTestTimeout);

  it("keeps real file names visible in the results table after duplicate analysis loads", async () => {
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

    const contentsTable = await screen.findByRole("table", {
      name: /current folder contents/i,
    });

    await waitFor(() => {
      expect(within(contentsTable).getByText(/left\.bin/i)).toBeInTheDocument();
      expect(within(contentsTable).getByText(/right\.bin/i)).toBeInTheDocument();
    });

    expect(within(contentsTable).queryByText(/^file item$/i)).not.toBeInTheDocument();
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
