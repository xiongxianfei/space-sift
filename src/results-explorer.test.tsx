import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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

const uiReadyTimeout = 5000;
const uiTestTimeout = 15000;

type ScanEntryFixture = {
  path: string;
  parentPath: string | null;
  kind: "file" | "directory";
  sizeBytes: number;
};

type BrowseableScanFixture = CompletedScan & {
  entries?: ScanEntryFixture[];
};

type ExplorerClient = SpaceSiftClient & {
  openPathInExplorer: (path: string) => Promise<void>;
};

function makeCompletedStatus(scanId: string): ScanStatusSnapshot {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    state: "completed",
    filesDiscovered: 6,
    directoriesDiscovered: 4,
    bytesProcessed: 6656,
    message: "Scan complete.",
    completedScanId: scanId,
  };
}

function makeHistoryEntry(scanId: string): ScanHistoryEntry {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 6656,
  };
}

function makeBrowseableScan(scanId: string): BrowseableScanFixture {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T10:59:00Z",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 6656,
    totalFiles: 6,
    totalDirectories: 4,
    largestFiles: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Games\\launcher.iso",
        sizeBytes: 3072,
      },
    ],
    largestDirectories: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Games",
        sizeBytes: 4096,
      },
    ],
    skippedPaths: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Locked",
        reasonCode: "permission_denied",
        summary: "Access denied",
      },
    ],
    entries: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads",
        parentPath: null,
        kind: "directory",
        sizeBytes: 6656,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Games",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "directory",
        sizeBytes: 4096,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Archive",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "directory",
        sizeBytes: 2048,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Zeta.bin",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 512,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Games\\Mods",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Games",
        kind: "directory",
        sizeBytes: 1024,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Games\\Empty",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Games",
        kind: "directory",
        sizeBytes: 0,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Games\\launcher.iso",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Games",
        kind: "file",
        sizeBytes: 3072,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Games\\Mods\\patch.zip",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Games\\Mods",
        kind: "file",
        sizeBytes: 1024,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\Archive\\Missing.iso",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads\\Archive",
        kind: "file",
        sizeBytes: 2048,
      },
    ],
  };
}

function makeSummaryOnlyScan(scanId: string): BrowseableScanFixture {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-15T10:59:00Z",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 4096,
    totalFiles: 3,
    totalDirectories: 2,
    largestFiles: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\legacy.iso",
        sizeBytes: 4096,
      },
    ],
    largestDirectories: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads",
        sizeBytes: 4096,
      },
    ],
    skippedPaths: [],
  };
}

function createExplorerClient(scan: BrowseableScanFixture, scanId = scan.scanId) {
  const historyEntry = makeHistoryEntry(scanId);
  const openPathInExplorer = vi.fn(async (path: string) => {
    if (path.includes("Missing")) {
      throw new Error("Path no longer exists.");
    }
  });

  const client: ExplorerClient = {
    startScan: vi.fn(async () => ({ scanId: "scan-running" })),
    cancelActiveScan: vi.fn(async () => {}),
    getScanStatus: vi.fn(async () => makeCompletedStatus(scanId)),
    listScanHistory: vi.fn(async () => [historyEntry]),
    openScanHistory: vi.fn(async () => scan),
    startDuplicateAnalysis: vi.fn(async () => ({ analysisId: "analysis-unused" })),
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
    subscribeToScanProgress: vi.fn(async () => () => {}),
    subscribeToDuplicateProgress: vi.fn(async () => () => {}),
    openPathInExplorer,
  };

  return client;
}

describe("Space Sift results explorer", () => {
  it("renders the root explorer, drills into directories, and navigates by breadcrumb", async () => {
    render(<App client={createExplorerClient(makeBrowseableScan("scan-explorer"))} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /browse games/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    expect(screen.getByRole("button", { name: /downloads/i })).toBeInTheDocument();
    expect(screen.getByRole("table", { name: /current folder contents/i })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /browse games/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /browse mods/i })).toBeInTheDocument();
    });

    expect(screen.getByRole("button", { name: /games/i })).toBeInTheDocument();
    expect(screen.queryByText(/archive/i)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /^downloads$/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /browse archive/i })).toBeInTheDocument();
    });
  }, uiTestTimeout);

  it("sorts the current directory and shows inline usage in the same table", async () => {
    render(<App client={createExplorerClient(makeBrowseableScan("scan-sort"))} />);

    const contentsTable = await screen.findByRole("table", {
      name: /current folder contents/i,
    });

    fireEvent.click(screen.getByRole("button", { name: /sort by name/i }));

    await waitFor(() => {
      expect(within(contentsTable).getByRole("columnheader", { name: /usage/i })).toBeInTheDocument();
      const rows = within(contentsTable).getAllByRole("row").slice(1);
      expect(rows[0]).toHaveTextContent(/archive/i);
      expect(rows[1]).toHaveTextContent(/games/i);
      expect(rows[0]).toHaveTextContent(/31% of current level/i);
      expect(rows[1]).toHaveTextContent(/62% of current level/i);
    });

    expect(screen.queryByLabelText(/space map/i)).not.toBeInTheDocument();
  }, uiTestTimeout);

  it("shows an empty-state when the current folder has no immediate children", async () => {
    render(<App client={createExplorerClient(makeBrowseableScan("scan-empty"))} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /browse games/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /browse games/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /browse empty/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /browse empty/i }));

    await waitFor(() => {
      expect(
        screen.getByText(/this folder has no immediate children in the stored scan result/i),
      ).toBeInTheDocument();
    });

    expect(
      screen.queryByRole("table", { name: /current folder contents/i }),
    ).not.toBeInTheDocument();
  });

  it("requests Explorer handoff for the current path and surfaces missing-path errors", async () => {
    const client = createExplorerClient(makeBrowseableScan("scan-shell"));
    render(<App client={client} />);

    expect(
      await screen.findByRole(
        "button",
        { name: /open current path in explorer/i },
        { timeout: uiReadyTimeout },
      ),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /open current path in explorer/i }));
    expect(client.openPathInExplorer).toHaveBeenCalledWith(
      "C:\\Users\\xiongxianfei\\Downloads",
    );

    fireEvent.click(screen.getByRole("button", { name: /browse archive/i }));

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: /open missing\.iso in explorer/i }),
      ).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /open missing\.iso in explorer/i }));

    await waitFor(() => {
      expect(screen.getByText(/path no longer exists/i)).toBeInTheDocument();
    });
  }, uiTestTimeout);

  it("keeps older summary-only scans readable and asks for a rescan to browse", async () => {
    render(<App client={createExplorerClient(makeSummaryOnlyScan("scan-legacy"))} />);

    expect(
      await screen.findByText(/legacy\.iso/i, undefined, { timeout: uiReadyTimeout }),
    ).toBeInTheDocument();

    expect(
      screen.getByText(/saved before folder browsing support/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("table", { name: /current folder contents/i }),
    ).not.toBeInTheDocument();
  }, uiTestTimeout);
});
