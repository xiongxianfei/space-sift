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
}) {
  let progressListener: ((snapshot: ScanStatusSnapshot) => void) | null = null;
  const defaultCompletedScan = makeCompletedScan();
  const defaultHistory = [makeHistoryEntry()];
  const history = options?.history ?? defaultHistory;
  const scansById = options?.scansById ?? {
    [defaultCompletedScan.scanId]: defaultCompletedScan,
  };

  const client: SpaceSiftClient = {
    startScan: vi.fn(async () => ({ scanId: "scan-running" })),
    cancelActiveScan: vi.fn(async () => {}),
    getScanStatus: vi.fn(async () => makeIdleSnapshot()),
    listScanHistory: vi.fn(async () => history),
    openScanHistory: vi.fn(async (scanId: string) => {
      const scan = scansById[scanId];
      if (!scan) {
        throw new Error(`missing stored scan ${scanId}`);
      }

      return scan;
    }),
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

describe("Space Sift scan and history flow", () => {
  it("switches from a loaded result into dedicated active-scan mode", async () => {
    const mock = createMockClient();
    render(<App client={mock.client} />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-1/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /reopen scan scan-1/i }));

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: /current result/i })).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledWith("C:\\Users\\xiongxianfei\\Downloads");
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

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-1/i })).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Videos" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledWith("C:\\Users\\xiongxianfei\\Videos");
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

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-1/i })).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Videos" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledWith("C:\\Users\\xiongxianfei\\Videos");
      expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-2");
    });

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

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-1/i })).toBeInTheDocument();
    });

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledWith("C:\\Users\\xiongxianfei\\Downloads");
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

    fireEvent.click(screen.getByRole("button", { name: /reopen scan scan-1/i }));
    expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-1");

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
      expect(screen.getByRole("heading", { name: /current result/i })).toBeInTheDocument();
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
      expect(screen.getByRole("heading", { name: /current result/i })).toBeInTheDocument();
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

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /reopen scan scan-1/i })).toBeInTheDocument();
    });

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
});
