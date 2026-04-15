import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import type { SpaceSiftClient } from "./lib/spaceSiftClient";
import type {
  CompletedScan,
  ScanHistoryEntry,
  ScanStatusSnapshot,
} from "./lib/spaceSiftTypes";

function makeHistoryEntry(): ScanHistoryEntry {
  return {
    scanId: "scan-1",
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    completedAt: "2026-04-15T11:00:00Z",
    totalBytes: 4096,
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

function makeIdleSnapshot(): ScanStatusSnapshot {
  return {
    scanId: null,
    rootPath: null,
    state: "idle",
    filesDiscovered: 0,
    directoriesDiscovered: 0,
    bytesProcessed: 0,
    message: null,
    completedScanId: null,
  };
}

function createMockClient() {
  let progressListener: ((snapshot: ScanStatusSnapshot) => void) | null = null;
  const historyEntry = makeHistoryEntry();
  const completedScan = makeCompletedScan();

  const client: SpaceSiftClient = {
    startScan: vi.fn(async () => ({ scanId: "scan-running" })),
    cancelActiveScan: vi.fn(async () => {}),
    getScanStatus: vi.fn(async () => makeIdleSnapshot()),
    listScanHistory: vi.fn(async () => [historyEntry]),
    openScanHistory: vi.fn(async () => completedScan),
    openPathInExplorer: vi.fn(async () => {}),
    subscribeToScanProgress: vi.fn(async (listener) => {
      progressListener = listener;
      return () => {
        progressListener = null;
      };
    }),
  };

  return {
    client,
    emitProgress(snapshot: ScanStatusSnapshot) {
      progressListener?.(snapshot);
    },
  };
}

describe("Space Sift scan and history flow", () => {
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
      mock.emitProgress({
        scanId: "scan-running",
        rootPath: "C:\\Users\\xiongxianfei\\Downloads",
        state: "running",
        filesDiscovered: 2,
        directoriesDiscovered: 1,
        bytesProcessed: 2048,
        message: null,
        completedScanId: null,
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/^running$/i)).toBeInTheDocument();
      expect(screen.getByText(/2048 bytes processed/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /cancel scan/i }));
    expect(mock.client.cancelActiveScan).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: /reopen scan scan-1/i }));
    expect(mock.client.openScanHistory).toHaveBeenCalledWith("scan-1");

    await waitFor(() => {
      expect(screen.getByText(/big\.iso/i)).toBeInTheDocument();
      expect(screen.getAllByText(/4096 bytes/i).length).toBeGreaterThan(0);
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
      mock.emitProgress({
        scanId: "scan-running",
        rootPath: "C:\\Users\\xiongxianfei\\Downloads",
        state: "running",
        filesDiscovered: 3,
        directoriesDiscovered: 2,
        bytesProcessed: 3072,
        message: null,
        completedScanId: null,
      });
    });

    await waitFor(() => {
      expect(screen.getByText(/^running$/i)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(mock.client.startScan).toHaveBeenCalledTimes(1);
      expect(screen.getByText(/one scan at a time/i)).toBeInTheDocument();
    });
  });
});
