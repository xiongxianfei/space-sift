import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import {
  idleDuplicateStatus,
  type SpaceSiftClient,
} from "./lib/spaceSiftClient";
import type {
  CompletedScan,
  ScanHistoryEntry,
  ScanRunSummary,
  ScanStatusSnapshot,
} from "./lib/spaceSiftTypes";

function makeHistoryEntry(
  scanId = "scan-1",
  rootPath = "C:\\Users\\xiongxianfei\\Downloads",
  totalBytes = 4096,
  completedAt = "2026-04-15T11:00:00Z",
): ScanHistoryEntry {
  return {
    scanId,
    rootPath,
    completedAt,
    totalBytes,
  };
}

function makeCompletedScan(): CompletedScan {
  return {
    scanId: "scan-1",
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T10:59:00Z",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 4096,
    totalFiles: 3,
    totalDirectories: 2,
    largestFiles: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\big.iso",
        sizeBytes: 3072,
      },
    ],
    largestDirectories: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads",
        sizeBytes: 4096,
      },
    ],
    skippedPaths: [],
    entries: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads",
        parentPath: null,
        kind: "directory",
        sizeBytes: 4096,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\big.iso",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 3072,
      },
    ],
  };
}

function makeSecondCompletedScan(): CompletedScan {
  return {
    scanId: "scan-2",
    rootPath: "C:\\Users\\xiongxianfei\\Videos",
    startedAt: "2026-04-15T12:00:00Z",
    completedAt: "2026-04-15T12:05:00Z",
    totalBytes: 8192,
    totalFiles: 2,
    totalDirectories: 1,
    largestFiles: [
      {
        path: "C:\\Users\\xiongxianfei\\Videos\\movie.mkv",
        sizeBytes: 6144,
      },
    ],
    largestDirectories: [
      {
        path: "C:\\Users\\xiongxianfei\\Videos",
        sizeBytes: 8192,
      },
    ],
    skippedPaths: [],
    entries: [
      {
        path: "C:\\Users\\xiongxianfei\\Videos",
        parentPath: null,
        kind: "directory",
        sizeBytes: 8192,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Videos\\movie.mkv",
        parentPath: "C:\\Users\\xiongxianfei\\Videos",
        kind: "file",
        sizeBytes: 6144,
      },
    ],
  };
}

function makeSeededHistoryReviewFixture() {
  const history: ScanHistoryEntry[] = [];
  const scansById: Record<string, CompletedScan> = {};
  const rootOptions = [
    "C:\\Users\\xiongxianfei\\Downloads\\Projects",
    "C:\\Users\\xiongxianfei\\Videos\\Archive",
    "D:\\Archive\\Backups",
    "D:\\Projects\\Northwind",
  ];

  for (let index = 1; index <= 24; index += 1) {
    const scanId = `scan-${String(index).padStart(2, "0")}`;
    const rootBase = rootOptions[(index - 1) % rootOptions.length]!;
    const rootPath = `${rootBase}\\Batch-${index}`;
    const completedAt = new Date(Date.UTC(2026, 3, 15, 12, index, 0)).toISOString();
    const totalBytes = 2048 + index * 256;

    history.push(makeHistoryEntry(scanId, rootPath, totalBytes, completedAt));
    scansById[scanId] = {
      ...makeCompletedScan(),
      scanId,
      rootPath,
      startedAt: new Date(Date.parse(completedAt) - 60_000).toISOString(),
      completedAt,
      totalBytes,
      totalFiles: 1,
      totalDirectories: 1,
      largestFiles: [
        {
          path: `${rootPath}\\item-${index}.bin`,
          sizeBytes: totalBytes - 256,
        },
      ],
      largestDirectories: [
        {
          path: rootPath,
          sizeBytes: totalBytes,
        },
      ],
      entries: [
        {
          path: rootPath,
          parentPath: null,
          kind: "directory",
          sizeBytes: totalBytes,
        },
        {
          path: `${rootPath}\\item-${index}.bin`,
          parentPath: rootPath,
          kind: "file",
          sizeBytes: totalBytes - 256,
        },
      ],
    };
  }

  return {
    history,
    scansById,
  };
}

function makeIdleSnapshot(): ScanStatusSnapshot {
  return {
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
}

function makeRunSummary(options?: {
  runId?: string;
  rootPath?: string;
  status?: "stale" | "abandoned" | "cancelled" | "failed" | "completed" | "running";
  hasResume?: boolean;
  canResume?: boolean;
  latestSeq?: number;
  itemsScanned?: number;
  errorsCount?: number;
  progressPercent?: number | null;
  scanRateItemsPerSec?: number;
}) {
  const runId = options?.runId ?? "run-1";
  const rootPath = options?.rootPath ?? "C:\\Users\\xiongxianfei\\Downloads";
  const status = options?.status ?? "stale";
  const latestSeq = options?.latestSeq ?? 3;
  const itemsScanned = options?.itemsScanned ?? 12;
  const errorsCount = options?.errorsCount ?? 0;
  const progressPercent = options?.progressPercent ?? 65;
  const scanRateItemsPerSec = options?.scanRateItemsPerSec ?? 3.5;
  const snapshot = {
    runId,
    seq: latestSeq,
    snapshotAt: "2026-04-19T10:00:00Z",
    createdAt: "2026-04-19T10:00:00Z",
    status,
    filesDiscovered: 10,
    directoriesDiscovered: 2,
    itemsDiscovered: 12,
    itemsScanned,
    errorsCount,
    bytesProcessed: 4096,
    scanRateItemsPerSec,
    progressPercent,
    currentPath: `${rootPath}\\nested`,
    message: null,
  };

  const summary: ScanRunSummary = {
    header: {
      runId,
      targetId: rootPath,
      rootPath,
      status,
      startedAt: "2026-04-19T09:55:00Z",
      lastSnapshotAt: "2026-04-19T10:00:00Z",
      lastProgressAt: "2026-04-19T09:59:30Z",
      staleSince: status === "stale" ? "2026-04-19T09:58:00Z" : null,
      terminalAt: status === "cancelled" || status === "failed" || status === "completed"
        ? "2026-04-19T10:00:00Z"
        : null,
      completedScanId: null,
      resumedFromRunId: null,
      createdAt: "2026-04-19T09:55:00Z",
      updatedAt: "2026-04-19T10:00:00Z",
      latestSeq,
      errorCode: null,
      errorMessage: null,
    },
    latestSnapshot: snapshot,
    snapshotPreview: [snapshot],
    seq: latestSeq,
    createdAt: "2026-04-19T09:55:00Z",
    itemsScanned,
    errorsCount,
    progressPercent,
    scanRateItemsPerSec,
    hasResume: options?.hasResume ?? false,
    canResume: options?.canResume ?? false,
  };

  return summary;
}

function makeRunningSnapshot(overrides: Partial<ScanStatusSnapshot> = {}): ScanStatusSnapshot {
  return {
    scanId: "scan-running",
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    state: "running",
    filesDiscovered: 2,
    directoriesDiscovered: 1,
    bytesProcessed: 2048,
    startedAt: "2026-04-15T10:59:00Z",
    updatedAt: "2026-04-15T10:59:05Z",
    currentPath: "C:\\Users\\xiongxianfei\\Downloads\\big.iso",
    message: null,
    completedScanId: null,
    ...overrides,
  };
}

function createMockClient(options?: {
  history?: ScanHistoryEntry[];
  scansById?: Record<string, CompletedScan>;
  scanRuns?: ScanRunSummary[];
}) {
  let progressListener: ((snapshot: ScanStatusSnapshot) => void) | null = null;
  const defaultCompletedScan = makeCompletedScan();
  const defaultHistory = [makeHistoryEntry()];
  const history = options?.history ?? defaultHistory;
  const scansById = options?.scansById ?? {
    [defaultCompletedScan.scanId]: defaultCompletedScan,
  };
  let scanRuns = options?.scanRuns ?? [];

  const client: SpaceSiftClient = {
    startScan: vi.fn(async () => ({ scanId: "scan-running" })),
    cancelActiveScan: vi.fn(async () => {}),
    cancelScanRun: vi.fn(async (runId: string) => {
      scanRuns = scanRuns.map((run) =>
        run.header.runId === runId
          ? {
              ...run,
              header: {
                ...run.header,
                status: "cancelled",
              },
              latestSnapshot: {
                ...run.latestSnapshot,
                status: "cancelled",
              },
            }
          : run,
      );
    }),
    getScanStatus: vi.fn(async () => makeIdleSnapshot()),
    getWorkspaceRestoreContext: vi.fn(async () => null),
    saveWorkspaceRestoreContext: vi.fn(async ({ lastWorkspace, lastOpenedScanId }) => ({
      schemaVersion: 1,
      lastWorkspace,
      lastOpenedScanId,
      updatedAt: "2026-04-22T10:00:00Z",
    })),
    listScanHistory: vi.fn(async () => history),
    openScanHistory: vi.fn(async (scanId: string) => {
      const scan = scansById[scanId];
      if (!scan) {
        throw new Error(`missing stored scan ${scanId}`);
      }

      return scan;
    }),
    listScanRuns: vi.fn(async () => scanRuns),
    openScanRun: vi.fn(async (runId: string) => {
      const run = scanRuns.find((entry) => entry.header.runId === runId);
      if (!run) {
        throw new Error(`missing run ${runId}`);
      }

      return {
        ...run,
        snapshotPreviewPage: 1,
        snapshotPreviewPageSize: run.snapshotPreview.length || 1,
        snapshotPreviewTotal: run.snapshotPreview.length,
      };
    }),
    resumeScanRun: vi.fn(async (runId: string) => ({ runId: `${runId}-child` })),
    startDuplicateAnalysis: vi.fn(async () => ({ analysisId: "analysis-unused" })),
    cancelDuplicateAnalysis: vi.fn(async () => {}),
    getDuplicateAnalysisStatus: vi.fn(async () => idleDuplicateStatus),
    openDuplicateAnalysis: vi.fn(async () => {
      throw new Error("no duplicate result");
    }),
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
    subscribeToScanProgress: vi.fn(async (listener) => {
      progressListener = listener;
      return () => {
        progressListener = null;
      };
    }),
    subscribeToDuplicateProgress: vi.fn(async () => () => {}),
  };

  return {
    client,
    emitProgress(snapshot: ScanStatusSnapshot) {
      progressListener?.(snapshot);
    },
  };
}

function getActiveScanPanel() {
  const heading = screen.getByRole("heading", { name: /active scan/i });
  const panel = heading.closest("section");
  if (!panel) {
    throw new Error("Active scan panel not found.");
  }

  return panel;
}

async function activateWorkspace(label: string) {
  await waitFor(() => {
    expect(screen.getByRole("tab", { name: label })).toBeInTheDocument();
  });

  fireEvent.click(screen.getByRole("tab", { name: label }));

  await waitFor(() => {
    expect(screen.getByRole("tab", { name: label })).toHaveAttribute("aria-selected", "true");
  });
}

describe("Space Sift scan and history flow", () => {
  it("switches from a loaded result into dedicated active-scan mode", async () => {
    const mock = createMockClient();
    render(<App client={mock.client} />);

    await activateWorkspace("History");

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-1/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /reopen scan scan-1/i }));

    await waitFor(() => {
      expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-1");
      expect(screen.getByText(/loaded scan-1 from local history/i)).toBeInTheDocument();
    });

    await activateWorkspace("Explorer");

    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledWith(
        "C:\\Users\\xiongxianfei\\Downloads",
        { resumeEnabled: false },
      );
    });

    await act(async () => {
      mock.emitProgress(makeRunningSnapshot());
    });

    await waitFor(() => {
      const activePanel = getActiveScanPanel();
      expect(within(activePanel).getByText(/progress stays indeterminate/i)).toBeInTheDocument();
      expect(
        within(activePanel).getByText(/c:\\users\\xiongxianfei\\downloads\\big\.iso/i),
      ).toBeInTheDocument();
      expect(within(activePanel).getByText(/2048 bytes processed/i)).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: /current result/i })).not.toBeInTheDocument();
    });
  });

  it("returns to the persisted completed result when the scan finishes", async () => {
    const secondCompletedScan = makeSecondCompletedScan();
    const mock = createMockClient({
      history: [makeHistoryEntry()],
      scansById: {
        "scan-1": makeCompletedScan(),
        "scan-2": secondCompletedScan,
      },
    });
    render(<App client={mock.client} />);

    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Videos" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledWith("C:\\Users\\xiongxianfei\\Videos", {
        resumeEnabled: false,
      });
    });

    await act(async () => {
      mock.emitProgress(
        makeRunningSnapshot({
          rootPath: "C:\\Users\\xiongxianfei\\Videos",
          currentPath: "C:\\Users\\xiongxianfei\\Videos\\movie.mkv",
        }),
      );
    });

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: /active scan/i })).toBeInTheDocument();
    });

    await act(async () => {
      mock.emitProgress({
        scanId: "scan-running",
        rootPath: "C:\\Users\\xiongxianfei\\Videos",
        state: "completed",
        filesDiscovered: 2,
        directoriesDiscovered: 1,
        bytesProcessed: 8192,
        startedAt: secondCompletedScan.startedAt,
        updatedAt: secondCompletedScan.completedAt,
        currentPath: "C:\\Users\\xiongxianfei\\Videos",
        message: "Scan complete.",
        completedScanId: "scan-2",
      });
    });

    await waitFor(() => {
      expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-2");
      expect(screen.getByText(/scan complete\./i)).toBeInTheDocument();
    });

    await activateWorkspace("Explorer");

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: /current result/i })).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: /active scan/i })).not.toBeInTheDocument();
      expect(
        screen.getByText(/loaded scan scan-2 from c:\\users\\xiongxianfei\\videos/i),
      ).toBeInTheDocument();
    });
  });

  it("does not overwrite a fast completed scan back to running after start resolves", async () => {
    const secondCompletedScan = makeSecondCompletedScan();
    const mock = createMockClient({
      history: [makeHistoryEntry()],
      scansById: {
        "scan-1": makeCompletedScan(),
        "scan-2": secondCompletedScan,
      },
    });

    mock.client.startScan = vi.fn(async (rootPath: string) => {
      mock.emitProgress({
        scanId: "scan-running",
        rootPath,
        state: "completed",
        filesDiscovered: 2,
        directoriesDiscovered: 1,
        bytesProcessed: 8192,
        startedAt: secondCompletedScan.startedAt,
        updatedAt: secondCompletedScan.completedAt,
        currentPath: rootPath,
        message: "Scan complete.",
        completedScanId: "scan-2",
      });

      return { scanId: "scan-running" };
    });

    render(<App client={mock.client} />);

    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Videos" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledWith("C:\\Users\\xiongxianfei\\Videos", {
        resumeEnabled: false,
      });
      expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-2");
    });

    await activateWorkspace("Explorer");

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: /current result/i })).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: /active scan/i })).not.toBeInTheDocument();
      expect(
        screen.getByText(/loaded scan scan-2 from c:\\users\\xiongxianfei\\videos/i),
      ).toBeInTheDocument();
    });
  });

  it("starts, cancels, and reopens scan history from local data", async () => {
    const mock = createMockClient();
    render(<App client={mock.client} />);

    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledWith(
        "C:\\Users\\xiongxianfei\\Downloads",
        { resumeEnabled: false },
      );
    });

    await act(async () => {
      mock.emitProgress(makeRunningSnapshot());
    });

    await waitFor(() => {
      const activePanel = getActiveScanPanel();
      expect(within(activePanel).getByText(/^running$/i)).toBeInTheDocument();
      expect(within(activePanel).getByText(/2048 bytes processed/i)).toBeInTheDocument();
    });

    fireEvent.click(within(getActiveScanPanel()).getByRole("button", { name: /cancel scan/i }));
    expect(mock.client.cancelActiveScan).toHaveBeenCalledTimes(1);

    await act(async () => {
      mock.emitProgress({
        ...makeRunningSnapshot(),
        state: "cancelled",
        message: "Scan cancelled before history save.",
      });
    });

    await activateWorkspace("History");
    fireEvent.click(screen.getByRole("button", { name: /reopen scan scan-1/i }));
    await waitFor(() => {
      expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-1");
    });

    await activateWorkspace("Explorer");

    const contentsTable = await screen.findByRole("table", {
      name: /current folder contents/i,
    });

    await waitFor(() => {
      expect(within(contentsTable).getByText(/big\.iso/i)).toBeInTheDocument();
      expect(screen.getAllByText(/4096 bytes/i).length).toBeGreaterThan(0);
    });
  });

  it("orders saved scans newest first, highlights the loaded result, and narrows the local history view", async () => {
    const completedScans = {
      "scan-1": makeCompletedScan(),
      "scan-2": makeSecondCompletedScan(),
      "scan-3": {
        ...makeCompletedScan(),
        scanId: "scan-3",
        rootPath: "D:\\Archive\\Backups",
        startedAt: "2026-04-14T21:58:00Z",
        completedAt: "2026-04-14T22:00:00Z",
        totalBytes: 2048,
      },
    };
    const history = [
      makeHistoryEntry("scan-1", "C:\\Users\\xiongxianfei\\Downloads", 4096, "2026-04-15T11:00:00Z"),
      makeHistoryEntry("scan-3", "D:\\Archive\\Backups", 2048, "2026-04-14T22:00:00Z"),
      makeHistoryEntry("scan-2", "C:\\Users\\xiongxianfei\\Videos", 8192, "2026-04-15T12:05:00Z"),
    ];
    const mock = createMockClient({
      history,
      scansById: completedScans,
    });

    render(<App client={mock.client} />);

    await activateWorkspace("History");

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-2/i })).toBeInTheDocument();
    });

    expect(
      screen
        .getAllByRole("button", { name: /reopen scan scan-/i })
        .map((button) => button.textContent),
    ).toEqual(["Reopen scan scan-2", "Reopen scan scan-1", "Reopen scan scan-3"]);

    fireEvent.change(screen.getByLabelText(/filter by root path/i), {
      target: { value: "videos" },
    });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-2/i })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /reopen scan scan-1/i })).not.toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /reopen scan scan-3/i })).not.toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /reopen scan scan-2/i }));

    await waitFor(() => {
      expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-2");
      expect(screen.getByText(/loaded scan-2 from local history/i)).toBeInTheDocument();
    });

    const loadedEntry = screen
      .getByRole("button", { name: /reopen scan scan-2/i })
      .closest("li");
    if (!loadedEntry) {
      throw new Error("Expected loaded history entry.");
    }

    expect(within(loadedEntry).getByText(/loaded result/i)).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText(/filter by root path/i), {
      target: { value: "" },
    });
    fireEvent.change(screen.getByLabelText(/filter by scan id/i), {
      target: { value: "scan-3" },
    });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-3/i })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /reopen scan scan-2/i })).not.toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText(/filter by scan id/i), {
      target: { value: "scan-9" },
    });

    await waitFor(() => {
      expect(screen.getByText(/no saved scans match the current filters/i)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /reopen scan scan-/i })).not.toBeInTheDocument();
    });
  });

  it("keeps a seeded large history review state readable and keyboard reachable", async () => {
    const fixture = makeSeededHistoryReviewFixture();
    const mock = createMockClient({
      history: fixture.history,
      scansById: fixture.scansById,
    });

    render(<App client={mock.client} />);

    await activateWorkspace("History");

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-24/i })).toBeInTheDocument();
    });

    expect(
      screen
        .getAllByRole("button", { name: /reopen scan scan-/i })
        .slice(0, 4)
        .map((button) => button.textContent),
    ).toEqual([
      "Reopen scan scan-24",
      "Reopen scan scan-23",
      "Reopen scan scan-22",
      "Reopen scan scan-21",
    ]);

    const rootFilter = screen.getByLabelText(/filter by root path/i);
    rootFilter.focus();
    expect(rootFilter).toHaveFocus();

    fireEvent.change(rootFilter, {
      target: { value: "northwind" },
    });

    await waitFor(() => {
      expect(screen.getAllByRole("button", { name: /reopen scan scan-/i })).toHaveLength(6);
      expect(screen.getByRole("button", { name: /reopen scan scan-24/i })).toBeInTheDocument();
    });

    const reopenButton = screen.getByRole("button", { name: /reopen scan scan-24/i });
    reopenButton.focus();
    expect(reopenButton).toHaveFocus();
    fireEvent.click(reopenButton);

    await waitFor(() => {
      expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-24");
      expect(screen.getByText(/loaded scan-24 from local history/i)).toBeInTheDocument();
    });

    const loadedEntry = screen
      .getByRole("button", { name: /reopen scan scan-24/i })
      .closest("li");
    if (!loadedEntry) {
      throw new Error("Expected loaded seeded history entry.");
    }

    expect(within(loadedEntry).getByText(/loaded result/i)).toBeInTheDocument();

    fireEvent.change(rootFilter, {
      target: { value: "" },
    });

    const scanIdFilter = screen.getByLabelText(/filter by scan id/i);
    scanIdFilter.focus();
    expect(scanIdFilter).toHaveFocus();
    fireEvent.change(scanIdFilter, {
      target: { value: "scan-05" },
    });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-05/i })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /reopen scan scan-24/i })).not.toBeInTheDocument();
      expect(screen.getAllByRole("button", { name: /reopen scan scan-/i })).toHaveLength(1);
    });
  });

  it("blocks a second scan request while one is already running", async () => {
    const mock = createMockClient();
    render(<App client={mock.client} />);

    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledTimes(1);
    });

    await act(async () => {
      mock.emitProgress(
        makeRunningSnapshot({
          filesDiscovered: 3,
          directoriesDiscovered: 2,
          bytesProcessed: 3072,
          updatedAt: "2026-04-15T10:59:08Z",
          currentPath: "C:\\Users\\xiongxianfei\\Downloads\\nested",
        }),
      );
    });

    await waitFor(() => {
      const activePanel = getActiveScanPanel();
      expect(within(activePanel).getByText(/^running$/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledTimes(1);
      expect(screen.getByText(/one scan at a time/i)).toBeInTheDocument();
    });
  });

  it("keeps resume opt-in off by default and only sends it when advanced mode is enabled", async () => {
    const firstMock = createMockClient();
    const { unmount } = render(<App client={firstMock.client} />);

    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(firstMock.client.startScan).toHaveBeenCalledWith(
        "C:\\Users\\xiongxianfei\\Downloads",
        { resumeEnabled: false },
      );
    });

    unmount();

    const secondMock = createMockClient();
    render(<App client={secondMock.client} />);

    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByText(/advanced scan options/i));
    fireEvent.click(screen.getByLabelText(/enable interrupted-run resume/i));
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(secondMock.client.startScan).toHaveBeenLastCalledWith(
        "C:\\Users\\xiongxianfei\\Downloads",
        { resumeEnabled: true },
      );
    });
  });

  it("shows interrupted runs with explicit resume and cancel actions", async () => {
    const mock = createMockClient({
      scanRuns: [
        makeRunSummary({
          runId: "run-abandoned",
          status: "abandoned",
          hasResume: true,
          canResume: false,
          latestSeq: 5,
        }),
        makeRunSummary({
          runId: "run-stale",
          status: "stale",
          hasResume: false,
          canResume: false,
          latestSeq: 4,
        }),
      ],
    });

    render(<App client={mock.client} />);

    await activateWorkspace("History");

    await waitFor(() => {
      expect(
        within(screen.getByRole("tabpanel", { name: "History" })).getByRole("heading", {
          name: /interrupted runs/i,
        }),
      ).toBeInTheDocument();
    });

    const historyPanel = screen.getByRole("tabpanel", { name: "History" });
    expect(within(historyPanel).getByText(/^abandoned$/i)).toBeInTheDocument();
    expect(within(historyPanel).getByText(/^stale$/i)).toBeInTheDocument();
    const abandonedCard = screen
      .getByText(/run id: run-abandoned/i)
      .closest(".history-entry");
    expect(abandonedCard).not.toBeNull();

    expect(within(abandonedCard as HTMLElement).getByText(/resume unavailable/i)).toBeInTheDocument();
    expect(within(abandonedCard as HTMLElement).getByText(/created .*2026/i)).toBeInTheDocument();
    expect(within(abandonedCard as HTMLElement).getByText(/seq 5/i)).toBeInTheDocument();
    expect(within(abandonedCard as HTMLElement).getByText(/12 items scanned/i)).toBeInTheDocument();
    expect(within(abandonedCard as HTMLElement).getByText(/^0 errors$/i)).toBeInTheDocument();
    expect(within(abandonedCard as HTMLElement).getByText(/65% progress/i)).toBeInTheDocument();
    expect(within(abandonedCard as HTMLElement).getByText(/3\.5 items\/s/i)).toBeInTheDocument();

    const resumeButton = within(abandonedCard as HTMLElement).getByRole("button", {
      name: /resume run run-abandoned/i,
    });
    expect(resumeButton).toBeDisabled();
    fireEvent.click(resumeButton);
    expect(mock.client.resumeScanRun).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /cancel run run-stale/i }));
    await waitFor(() => {
      expect(mock.client.cancelScanRun).toHaveBeenCalledWith("run-stale");
    });
  });

  it("disables resume from canResume when resume metadata exists but the engine is unsupported", async () => {
    const mock = createMockClient({
      scanRuns: [
        makeRunSummary({
          runId: "run-abandoned",
          status: "abandoned",
          hasResume: true,
          canResume: false,
          latestSeq: 5,
        }),
      ],
    });

    render(<App client={mock.client} />);

    await activateWorkspace("History");

    await waitFor(() => {
      expect(
        within(screen.getByRole("tabpanel", { name: "History" })).getByRole("heading", {
          name: /interrupted runs/i,
        }),
      ).toBeInTheDocument();
    });

    const resumeButton = screen.getByRole("button", { name: /resume run run-abandoned/i });
    expect(resumeButton).toBeDisabled();
    fireEvent.click(resumeButton);
    expect(mock.client.resumeScanRun).not.toHaveBeenCalled();
  });
});
