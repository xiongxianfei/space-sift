import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";
import type { SpaceSiftClient } from "./lib/spaceSiftClient";

function createIdleClient(): SpaceSiftClient {
  return {
    async startScan() {
      return { scanId: "scan-idle" };
    },
    async cancelActiveScan() {},
    async getScanStatus() {
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
    },
    async listScanHistory() {
      return [];
    },
    async openScanHistory() {
      throw new Error("no saved scans");
    },
    async openPathInExplorer() {},
    async subscribeToScanProgress() {
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
});
