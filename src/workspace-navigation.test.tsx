import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import { idleDuplicateStatus, idleScanStatus, type SpaceSiftClient } from "./lib/spaceSiftClient";
import type {
  CleanupExecutionResult,
  CleanupPreview,
  CleanupRuleDefinition,
  CompletedScan,
  DuplicateStatusSnapshot,
  ScanHistoryEntry,
  ScanRunSummary,
  ScanStatusSnapshot,
} from "./lib/spaceSiftTypes";

const uiReadyTimeout = 5000;

function makeHistoryEntry(scanId = "scan-shell"): ScanHistoryEntry {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    completedAt: "2026-04-22T10:00:00Z",
    totalBytes: 4096,
  };
}

function makeBrowseableScan(scanId = "scan-shell"): CompletedScan {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-22T09:59:00Z",
    completedAt: "2026-04-22T10:00:00Z",
    totalBytes: 4096,
    totalFiles: 3,
    totalDirectories: 2,
    largestFiles: [],
    largestDirectories: [],
    skippedPaths: [],
    entries: [
      {
        path: "C:\\Users\\xiongxianfei\\Downloads",
        parentPath: null,
        kind: "directory",
        sizeBytes: 4096,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\left.bin",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 2048,
      },
      {
        path: "C:\\Users\\xiongxianfei\\Downloads\\right.bin",
        parentPath: "C:\\Users\\xiongxianfei\\Downloads",
        kind: "file",
        sizeBytes: 2048,
      },
    ],
  };
}

function makeSummaryOnlyScan(scanId = "scan-summary"): CompletedScan {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    startedAt: "2026-04-22T09:59:00Z",
    completedAt: "2026-04-22T10:00:00Z",
    totalBytes: 4096,
    totalFiles: 3,
    totalDirectories: 2,
    largestFiles: [],
    largestDirectories: [],
    skippedPaths: [],
  };
}

function makeCompletedStatus(scanId = "scan-shell"): ScanStatusSnapshot {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    state: "completed",
    filesDiscovered: 3,
    directoriesDiscovered: 2,
    bytesProcessed: 4096,
    startedAt: "2026-04-22T09:59:00Z",
    updatedAt: "2026-04-22T10:00:00Z",
    currentPath: "C:\\Users\\xiongxianfei\\Downloads",
    message: "Scan complete.",
    completedScanId: scanId,
  };
}

function makeRunningStatus(): ScanStatusSnapshot {
  return {
    scanId: "scan-running",
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    state: "running",
    filesDiscovered: 2,
    directoriesDiscovered: 1,
    bytesProcessed: 2048,
    startedAt: "2026-04-22T09:59:00Z",
    updatedAt: "2026-04-22T10:00:05Z",
    currentPath: "C:\\Users\\xiongxianfei\\Downloads\\left.bin",
    message: null,
    completedScanId: null,
  };
}

function makeRunningDuplicateStatus(scanId = "scan-shell"): DuplicateStatusSnapshot {
  return {
    analysisId: "analysis-running",
    scanId,
    state: "running",
    stage: "full_hash",
    itemsProcessed: 2,
    groupsEmitted: 0,
    message: null,
    completedAnalysisId: null,
  };
}

function makeInterruptedRunSummary(): ScanRunSummary {
  return {
    header: {
      runId: "run-stale",
      targetId: "target-stale",
      rootPath: "D:\\Archive",
      status: "stale",
      startedAt: "2026-04-22T09:30:00Z",
      lastSnapshotAt: "2026-04-22T09:40:00Z",
      lastProgressAt: "2026-04-22T09:39:00Z",
      staleSince: "2026-04-22T09:39:30Z",
      terminalAt: null,
      completedScanId: null,
      resumedFromRunId: null,
      createdAt: "2026-04-22T09:30:00Z",
      updatedAt: "2026-04-22T09:40:00Z",
      latestSeq: 4,
      errorCode: null,
      errorMessage: null,
    },
    latestSnapshot: {
      runId: "run-stale",
      seq: 4,
      snapshotAt: "2026-04-22T09:40:00Z",
      createdAt: "2026-04-22T09:40:00Z",
      status: "stale",
      filesDiscovered: 4,
      directoriesDiscovered: 2,
      itemsDiscovered: 6,
      itemsScanned: 5,
      errorsCount: 0,
      bytesProcessed: 1024,
      scanRateItemsPerSec: 2.5,
      progressPercent: 65,
      currentPath: "D:\\Archive\\nested",
      message: null,
    },
    snapshotPreview: [],
    seq: 4,
    createdAt: "2026-04-22T09:30:00Z",
    itemsScanned: 5,
    errorsCount: 0,
    progressPercent: 65,
    scanRateItemsPerSec: 2.5,
    hasResume: true,
    canResume: false,
  };
}

function makeCleanupRules(): CleanupRuleDefinition[] {
  return [
    {
      ruleId: "temp-folder-files",
      label: "Files in Temp folders",
      description: "Files under directories named Temp or TMP within the current scan root.",
    },
  ];
}

function makeCleanupPreview(scanId = "scan-shell"): CleanupPreview {
  return {
    previewId: "preview-shell",
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Downloads",
    generatedAt: "2026-04-22T10:02:00Z",
    totalBytes: 2048,
    duplicateCandidateCount: 0,
    ruleCandidateCount: 1,
    candidates: [
      {
        actionId: "action-temp",
        path: "C:\\Users\\xiongxianfei\\Downloads\\Temp\\cache.tmp",
        sizeBytes: 2048,
        sourceLabels: ["Files in Temp folders"],
      },
    ],
    issues: [],
  };
}

function makeCleanupExecutionResult(): CleanupExecutionResult {
  return {
    executionId: "execution-shell",
    previewId: "preview-shell",
    mode: "recycle",
    completedAt: "2026-04-22T10:03:00Z",
    completedCount: 1,
    failedCount: 0,
    entries: [
      {
        actionId: "action-temp",
        path: "C:\\Users\\xiongxianfei\\Downloads\\Temp\\cache.tmp",
        status: "completed",
        summary: "Moved to the Recycle Bin.",
      },
    ],
  };
}

type WorkspaceClientOptions = {
  scanStatus?: ScanStatusSnapshot;
  scan?: CompletedScan | null;
  history?: ScanHistoryEntry[];
  scanRuns?: ScanRunSummary[];
  duplicateStatus?: DuplicateStatusSnapshot;
  cleanupRules?: CleanupRuleDefinition[];
  cleanupPreview?: CleanupPreview;
  cleanupExecutionResult?: CleanupExecutionResult;
};

function createWorkspaceClient(options?: WorkspaceClientOptions) {
  const scanStatus = options?.scanStatus ?? idleScanStatus;
  const scan = options?.scan ?? null;
  const history =
    options?.history ??
    (scanStatus.completedScanId && scan
      ? [makeHistoryEntry(scanStatus.completedScanId)]
      : []);
  const scanRuns = options?.scanRuns ?? [];
  const duplicateStatus = options?.duplicateStatus ?? idleDuplicateStatus;
  const cleanupRules = options?.cleanupRules ?? [];
  const cleanupPreview = options?.cleanupPreview ?? makeCleanupPreview(scan?.scanId ?? "scan-shell");
  const cleanupExecutionResult =
    options?.cleanupExecutionResult ?? makeCleanupExecutionResult();

  const client: SpaceSiftClient = {
    startScan: vi.fn(async () => ({ scanId: "scan-started" })),
    cancelActiveScan: vi.fn(async () => {}),
    cancelScanRun: vi.fn(async () => {}),
    getScanStatus: vi.fn(async () => scanStatus),
    getWorkspaceRestoreContext: vi.fn(async () => null),
    saveWorkspaceRestoreContext: vi.fn(async ({ lastWorkspace, lastOpenedScanId }) => ({
      schemaVersion: 1,
      lastWorkspace,
      lastOpenedScanId,
      updatedAt: "2026-04-22T10:00:00Z",
    })),
    listScanHistory: vi.fn(async () => history),
    openScanHistory: vi.fn(async (scanId: string) => {
      if (!scan || scan.scanId !== scanId) {
        throw new Error(`missing stored scan ${scanId}`);
      }

      return scan;
    }),
    listScanRuns: vi.fn(async () => scanRuns),
    openScanRun: vi.fn(async () => {
      throw new Error("no scan run");
    }),
    resumeScanRun: vi.fn(async () => ({ runId: "run-resumed" })),
    startDuplicateAnalysis: vi.fn(async () => ({ analysisId: "analysis-started" })),
    cancelDuplicateAnalysis: vi.fn(async () => {}),
    getDuplicateAnalysisStatus: vi.fn(async () => duplicateStatus),
    openDuplicateAnalysis: vi.fn(async () => {
      throw new Error("no duplicate result");
    }),
    listCleanupRules: vi.fn(async () => cleanupRules),
    previewCleanup: vi.fn(async () => cleanupPreview),
    executeCleanup: vi.fn(async () => cleanupExecutionResult),
    getPrivilegedCleanupCapability: vi.fn(async () => ({
      available: false,
      message:
        "Protected-path cleanup stays outside the unprivileged desktop flow in this milestone.",
    })),
    openPathInExplorer: vi.fn(async () => {}),
    subscribeToScanProgress: vi.fn(async () => () => {}),
    subscribeToDuplicateProgress: vi.fn(async () => () => {}),
  };

  return client;
}

async function waitForWorkspaceShell() {
  await waitFor(() => {
    expect(screen.getByRole("tab", { name: "Overview" })).toBeInTheDocument();
  }, { timeout: uiReadyTimeout });
}

function getSelectedTabs() {
  return screen
    .getAllByRole("tab")
    .filter((tab) => tab.getAttribute("aria-selected") === "true");
}

function getStatusRegion() {
  return screen.getByRole("region", { name: /global status/i });
}

async function activateWorkspace(label: string) {
  fireEvent.click(screen.getByRole("tab", { name: label }));
  await waitFor(() => {
    expect(screen.getByRole("tab", { name: label })).toHaveAttribute("aria-selected", "true");
  });
}

describe("Space Sift workspace navigation shell", () => {
  it("workspace_nav_exposes_single_selected_tab", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();

    expect(screen.getAllByRole("tab")).toHaveLength(7);
    expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(getSelectedTabs()).toHaveLength(1);

    fireEvent.click(screen.getByRole("tab", { name: "Cleanup" }));

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Cleanup" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(getSelectedTabs()).toHaveLength(1);
    });
  });

  it("workspace_nav_selected_tab_controls_visible_panel", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();
    await activateWorkspace("History");

    expect(screen.getByRole("tabpanel", { name: "History" })).toBeInTheDocument();
    expect(screen.queryByRole("tabpanel", { name: "Overview" })).not.toBeInTheDocument();
    expect(screen.getByRole("heading", { name: /recent scans/i })).toBeInTheDocument();
  });

  it("workspace_nav_keyboard_arrows_move_between_tabs", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();

    const overviewTab = screen.getByRole("tab", { name: "Overview" });
    overviewTab.focus();
    fireEvent.keyDown(overviewTab, { key: "ArrowRight" });

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Scan" })).toHaveFocus();
    });

    expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("workspace_nav_home_end_move_to_boundary_tabs", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();

    const duplicatesTab = screen.getByRole("tab", { name: "Duplicates" });
    duplicatesTab.focus();
    fireEvent.keyDown(duplicatesTab, { key: "Home" });

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Overview" })).toHaveFocus();
    });

    fireEvent.keyDown(screen.getByRole("tab", { name: "Overview" }), { key: "End" });

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Safety" })).toHaveFocus();
    });
  });

  it("workspace_nav_enter_or_space_activates_focused_tab", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();

    const historyTab = screen.getByRole("tab", { name: "History" });
    historyTab.focus();
    fireEvent.keyDown(historyTab, { key: "Enter" });

    await waitFor(() => {
      expect(historyTab).toHaveAttribute("aria-selected", "true");
    });

    const safetyTab = screen.getByRole("tab", { name: "Safety" });
    safetyTab.focus();
    fireEvent.keyDown(safetyTab, { key: " " });

    await waitFor(() => {
      expect(safetyTab).toHaveAttribute("aria-selected", "true");
      expect(screen.getByRole("tabpanel", { name: "Safety" })).toBeInTheDocument();
    });
  });

  it("workspace_nav_inactive_panels_are_not_exposed_as_active", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();

    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
    expect(screen.getByRole("tabpanel", { name: "Overview" })).toBeInTheDocument();

    await activateWorkspace("Safety");

    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
    expect(screen.getByRole("tabpanel", { name: "Safety" })).toBeInTheDocument();
    expect(screen.queryByRole("tabpanel", { name: "Overview" })).not.toBeInTheDocument();
  });

  it("global_status_visible_on_all_workspaces", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();

    for (const label of [
      "Overview",
      "Scan",
      "History",
      "Explorer",
      "Duplicates",
      "Cleanup",
      "Safety",
    ]) {
      await activateWorkspace(label);
      expect(getStatusRegion()).toBeInTheDocument();
    }
  });

  it("global_status_exposes_deterministic_state_label", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus(),
          scan: makeBrowseableScan(),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(getStatusRegion()).toHaveTextContent(/completed scan loaded/i);
      expect(getStatusRegion()).toHaveTextContent(/scan-shell/i);
    });
  });

  it("global_status_exposes_single_next_safe_action_or_no_action", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus("scan-summary"),
          scan: makeSummaryOnlyScan("scan-summary"),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(getStatusRegion()).toHaveTextContent(/completed scan loaded/i);
      expect(getStatusRegion()).toHaveTextContent(/no safe next action right now/i);
    });

    expect(
      screen.queryByRole("button", { name: /view scan progress|find duplicates|browse results/i }),
    ).not.toBeInTheDocument();
  });

  it("next_safe_action_prioritizes_live_scan", async () => {
    const client = createWorkspaceClient({
      scanStatus: makeRunningStatus(),
      scanRuns: [makeInterruptedRunSummary()],
    });
    render(<App client={client} />);

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(getStatusRegion()).toHaveTextContent(/live scan running/i);
    });

    const action = screen.getByRole("button", { name: /view scan progress/i });
    fireEvent.click(action);

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Scan" })).toHaveAttribute("aria-selected", "true");
      expect(client.startScan).not.toHaveBeenCalled();
    });
  });

  it("next_safe_action_prioritizes_live_duplicate_analysis_after_scan", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus(),
          scan: makeBrowseableScan(),
          duplicateStatus: makeRunningDuplicateStatus(),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(getStatusRegion()).toHaveTextContent(/live duplicate analysis running/i);
      expect(screen.getByRole("button", { name: /view duplicate analysis/i })).toBeInTheDocument();
    });
  });

  it("next_safe_action_points_interrupted_runs_to_history", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanRuns: [makeInterruptedRunSummary()],
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(getStatusRegion()).toHaveTextContent(/interrupted runs need review/i);
    });

    fireEvent.click(screen.getByRole("button", { name: /review interrupted runs/i }));

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "History" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(
        within(screen.getByRole("tabpanel", { name: "History" })).getByRole("heading", {
          name: /interrupted runs/i,
        }),
      ).toBeInTheDocument();
    });
  });

  it("next_safe_action_for_cleanup_preview_navigates_without_executing", async () => {
    const client = createWorkspaceClient({
      scanStatus: makeCompletedStatus(),
      scan: makeBrowseableScan(),
      cleanupRules: makeCleanupRules(),
    });
    render(<App client={client} />);

    await waitForWorkspaceShell();
    await activateWorkspace("Cleanup");

    fireEvent.click(screen.getByLabelText(/files in temp folders/i));
    fireEvent.click(screen.getByRole("button", { name: /refresh cleanup preview/i }));

    await waitFor(() => {
      expect(screen.getByText(/1 cleanup candidates/i)).toBeInTheDocument();
    });

    await activateWorkspace("Overview");

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review cleanup preview/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review cleanup preview/i }));

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Cleanup" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(client.executeCleanup).not.toHaveBeenCalled();
    });
  });

  it("next_safe_action_never_invokes_permanent_delete", async () => {
    const client = createWorkspaceClient({
      scanStatus: makeCompletedStatus(),
      scan: makeBrowseableScan(),
      cleanupRules: makeCleanupRules(),
    });
    render(<App client={client} />);

    await waitForWorkspaceShell();
    await activateWorkspace("Cleanup");

    fireEvent.click(screen.getByLabelText(/files in temp folders/i));
    fireEvent.click(screen.getByRole("button", { name: /refresh cleanup preview/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /review cleanup preview/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /review cleanup preview/i }));

    await waitFor(() => {
      expect(client.executeCleanup).not.toHaveBeenCalled();
      expect(client.resumeScanRun).not.toHaveBeenCalled();
    });
  });

  it("next_safe_action_after_cleanup_execution_recommends_rescan", async () => {
    const client = createWorkspaceClient({
      scanStatus: makeCompletedStatus(),
      scan: makeBrowseableScan(),
      cleanupRules: makeCleanupRules(),
    });
    render(<App client={client} />);

    await waitForWorkspaceShell();
    await activateWorkspace("Cleanup");

    fireEvent.click(screen.getByLabelText(/files in temp folders/i));
    fireEvent.click(screen.getByRole("button", { name: /refresh cleanup preview/i }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /move selected files to recycle bin/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /move selected files to recycle bin/i }));

    await waitFor(() => {
      expect(screen.getByText(/cleanup completed/i)).toBeInTheDocument();
    });

    await activateWorkspace("Overview");

    await waitFor(() => {
      expect(getStatusRegion()).toHaveTextContent(
        /cleanup execution completed with rescan recommended/i,
      );
      expect(screen.getByRole("button", { name: /start a fresh scan/i })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /start a fresh scan/i }));

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Scan" })).toHaveAttribute("aria-selected", "true");
    });
  });
});
