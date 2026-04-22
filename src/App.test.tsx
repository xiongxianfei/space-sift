import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";
import {
  idleDuplicateStatus,
  type SpaceSiftClient,
} from "./lib/spaceSiftClient";

function createIdleClient(): SpaceSiftClient {
  return {
    async startScan() {
      return { scanId: "scan-idle" };
    },
    async cancelActiveScan() {},
    async cancelScanRun() {
      throw new Error("no scan run");
    },
    async getScanStatus() {
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
    },
    async getWorkspaceRestoreContext() {
      return null;
    },
    async saveWorkspaceRestoreContext({ lastWorkspace, lastOpenedScanId }) {
      return {
        schemaVersion: 1,
        lastWorkspace,
        lastOpenedScanId,
        updatedAt: "2026-04-22T10:00:00Z",
      };
    },
    async listScanHistory() {
      return [];
    },
    async openScanHistory() {
      throw new Error("no saved scans");
    },
    async listScanRuns() {
      return [];
    },
    async openScanRun() {
      throw new Error("no scan run");
    },
    async resumeScanRun() {
      throw new Error("no scan run");
    },
    async startDuplicateAnalysis() {
      throw new Error("no duplicate analysis");
    },
    async cancelDuplicateAnalysis() {
      throw new Error("no duplicate analysis");
    },
    async getDuplicateAnalysisStatus() {
      return idleDuplicateStatus;
    },
    async openDuplicateAnalysis() {
      throw new Error("no duplicate analysis");
    },
    async listCleanupRules() {
      return [];
    },
    async previewCleanup() {
      throw new Error("no cleanup preview");
    },
    async executeCleanup() {
      throw new Error("no cleanup execution");
    },
    async getPrivilegedCleanupCapability() {
      return {
        available: false,
        message:
          "Protected-path cleanup stays outside the unprivileged desktop flow in this milestone.",
      };
    },
    async openPathInExplorer() {},
    async subscribeToScanProgress() {
      return () => {};
    },
    async subscribeToDuplicateProgress() {
      return () => {};
    },
  };
}

describe("Space Sift milestone 2 shell", () => {
  it("shows the branded product and scan workspace", async () => {
    render(<App client={createIdleClient()} />);

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: /space sift/i })).toBeInTheDocument();
    });

    expect(screen.getByRole("heading", { name: /scan workspace/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/scan root/i)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /recent scans/i })).toBeInTheDocument();
  });

  it("communicates the safety model", async () => {
    render(<App client={createIdleClient()} />);

    await waitFor(() => {
      expect(screen.getByText(/without elevating the whole ui/i)).toBeInTheDocument();
      expect(screen.getAllByText(/recycle bin first/i).length).toBeGreaterThan(0);
      expect(screen.getByText(/local sqlite storage/i)).toBeInTheDocument();
    });
  });

  it("keeps interrupted-run resume behind an advanced opt-in", async () => {
    render(<App client={createIdleClient()} />);

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: /scan workspace/i })).toBeInTheDocument();
    });

    expect(screen.getByText(/advanced scan options/i)).toBeInTheDocument();
    expect(screen.queryByLabelText(/enable interrupted-run resume/i)).not.toBeChecked();
  });
});
