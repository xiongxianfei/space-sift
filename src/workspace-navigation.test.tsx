import { readFileSync } from "node:fs";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import { idleDuplicateStatus, idleScanStatus, type SpaceSiftClient } from "./lib/spaceSiftClient";
import type {
  CleanupExecutionResult,
  CleanupPreview,
  CleanupRuleDefinition,
  CompletedDuplicateAnalysis,
  CompletedScan,
  DuplicateStatusSnapshot,
  ScanHistoryEntry,
  ScanRunSummary,
  ScanStatusSnapshot,
  WorkspaceRestoreContext,
  WorkspaceRestoreWorkspace,
} from "./lib/spaceSiftTypes";
import { workspaceShellLogger } from "./workspaceShellLogger";

const uiReadyTimeout = 5000;

function makeHistoryEntry(
  scanId = "scan-shell",
  rootPath = "C:\\Users\\xiongxianfei\\Downloads",
  totalBytes = 4096,
): ScanHistoryEntry {
  return {
    scanId,
    rootPath,
    completedAt: "2026-04-22T10:00:00Z",
    totalBytes,
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

function makeSecondBrowseableScan(scanId = "scan-shell-2"): CompletedScan {
  return {
    scanId,
    rootPath: "C:\\Users\\xiongxianfei\\Videos",
    startedAt: "2026-04-22T11:59:00Z",
    completedAt: "2026-04-22T12:00:00Z",
    totalBytes: 8192,
    totalFiles: 2,
    totalDirectories: 1,
    largestFiles: [],
    largestDirectories: [],
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
        sizeBytes: 8192,
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

function makeWorkspaceRestoreContext(
  lastWorkspace: WorkspaceRestoreWorkspace,
  lastOpenedScanId: string | null,
): WorkspaceRestoreContext {
  return {
    schemaVersion: 1,
    lastWorkspace,
    lastOpenedScanId,
    updatedAt: "2026-04-22T10:00:00Z",
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
  scansById?: Record<string, CompletedScan>;
  duplicateAnalysis?: CompletedDuplicateAnalysis | null;
  history?: ScanHistoryEntry[];
  scanRuns?: ScanRunSummary[];
  duplicateStatus?: DuplicateStatusSnapshot;
  cleanupRules?: CleanupRuleDefinition[];
  cleanupPreview?: CleanupPreview;
  cleanupExecutionResult?: CleanupExecutionResult;
  restoreContext?: WorkspaceRestoreContext | null;
  restoreContextError?: Error;
};

function createWorkspaceClient(options?: WorkspaceClientOptions) {
  const scanStatus = options?.scanStatus ?? idleScanStatus;
  const scan = options?.scan ?? null;
  const scansById =
    options?.scansById ??
    (scan
      ? {
          [scan.scanId]: scan,
        }
      : {});
  const duplicateAnalysis = options?.duplicateAnalysis ?? null;
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
  const restoreContext = options?.restoreContext ?? null;
  const restoreContextError = options?.restoreContextError ?? null;

  const client: SpaceSiftClient = {
    startScan: vi.fn(async () => ({ scanId: "scan-started" })),
    cancelActiveScan: vi.fn(async () => {}),
    cancelScanRun: vi.fn(async () => {}),
    getScanStatus: vi.fn(async () => scanStatus),
    getWorkspaceRestoreContext: vi.fn(async () => {
      if (restoreContextError) {
        throw restoreContextError;
      }

      return restoreContext;
    }),
    saveWorkspaceRestoreContext: vi.fn(async ({ lastWorkspace, lastOpenedScanId }) => ({
      schemaVersion: 1,
      lastWorkspace,
      lastOpenedScanId,
      updatedAt: "2026-04-22T10:00:00Z",
    })),
    listScanHistory: vi.fn(async () => history),
    openScanHistory: vi.fn(async (scanId: string) => {
      const storedScan = scansById[scanId];
      if (!storedScan) {
        throw new Error(`missing stored scan ${scanId}`);
      }

      return storedScan;
    }),
    listScanRuns: vi.fn(async () => scanRuns),
    openScanRun: vi.fn(async () => {
      throw new Error("no scan run");
    }),
    resumeScanRun: vi.fn(async () => ({ runId: "run-resumed" })),
    startDuplicateAnalysis: vi.fn(async () => ({ analysisId: "analysis-started" })),
    cancelDuplicateAnalysis: vi.fn(async () => {}),
    getDuplicateAnalysisStatus: vi.fn(async () => duplicateStatus),
    openDuplicateAnalysis: vi.fn(async (analysisId: string) => {
      if (!duplicateAnalysis || duplicateAnalysis.analysisId !== analysisId) {
        throw new Error("no duplicate result");
      }

      return duplicateAnalysis;
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

type WorkspaceHarnessOptions = WorkspaceClientOptions;

function createWorkspaceHarness(options?: WorkspaceHarnessOptions) {
  let scanListener: ((snapshot: ScanStatusSnapshot) => void) | null = null;
  let duplicateListener: ((snapshot: DuplicateStatusSnapshot) => void) | null = null;
  const client = createWorkspaceClient(options);

  client.subscribeToScanProgress = vi.fn(async (listener) => {
    scanListener = listener;
    return () => {
      scanListener = null;
    };
  });

  client.subscribeToDuplicateProgress = vi.fn(async (listener) => {
    duplicateListener = listener;
    return () => {
      duplicateListener = null;
    };
  });

  return {
    client,
    async emitScan(snapshot: ScanStatusSnapshot) {
      await act(async () => {
        scanListener?.(snapshot);
      });
    },
    async emitDuplicate(snapshot: DuplicateStatusSnapshot) {
      await act(async () => {
        duplicateListener?.(snapshot);
      });
    },
  };
}

function defer<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });

  return { promise, resolve, reject };
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

function setViewportWidth(width: number) {
  Object.defineProperty(window, "innerWidth", {
    configurable: true,
    value: width,
  });
  window.dispatchEvent(new Event("resize"));
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

  it("workspace_shell_uses_persistent_header_left_rail_and_active_panel_layout", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();

    const banner = screen.getByRole("banner");
    expect(within(banner).getByRole("heading", { name: "Space Sift" })).toBeInTheDocument();
    expect(within(banner).queryByText(/desktop bridge connected/i)).not.toBeInTheDocument();
    expect(within(banner).getByText(/recycle bin first/i)).toBeInTheDocument();
    expect(within(banner).getByText(/local sqlite history/i)).toBeInTheDocument();

    const workspaceNavigation = screen.getByRole("navigation", {
      name: /workspace navigation/i,
    });
    const tablist = within(workspaceNavigation).getByRole("tablist", {
      name: /workspace navigation/i,
    });
    expect(tablist).toHaveAttribute("aria-orientation", "vertical");

    for (const label of [
      "Overview",
      "Scan",
      "History",
      "Explorer",
      "Duplicates",
      "Cleanup",
      "Safety",
    ]) {
      const tab = within(tablist).getByRole("tab", { name: label });
      expect(tab).toHaveTextContent(label);
      expect(tab).toHaveTextContent(/\S/);
    }

    const contentRegion = screen.getByRole("region", { name: /active workspace content/i });
    expect(within(contentRegion).getByRole("tabpanel", { name: "Overview" })).toBeInTheDocument();
    expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
  });

  it("workspace_shell_styles_define_design_breakpoints", () => {
    const appCss = readFileSync("src/App.css", "utf8");

    expect(appCss).toMatch(/@media\s*\(\s*max-width:\s*1050px\s*\)/);
    expect(appCss).toMatch(/@media\s*\(\s*max-width:\s*640px\s*\)/);
    expect(appCss).toMatch(/grid-template-columns:\s*260px\s+minmax\(0,\s*1fr\)/);
  });

  it("workspace_shell_keeps_required_content_available_at_contract_widths", async () => {
    const originalWidth = window.innerWidth;
    const client = createWorkspaceClient({
      scanStatus: makeCompletedStatus(),
      scan: makeBrowseableScan(),
      cleanupRules: makeCleanupRules(),
    });

    render(<App client={client} />);

    await waitForWorkspaceShell();

    for (const width of [1280, 900, 560]) {
      setViewportWidth(width);

      for (const label of [
        "Overview",
        "Scan",
        "History",
        "Explorer",
        "Duplicates",
        "Cleanup",
        "Safety",
      ]) {
        expect(screen.getByRole("tab", { name: label })).toBeInTheDocument();
      }

      expect(getStatusRegion()).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /find duplicates|start a scan/i })).toBeInTheDocument();

      await activateWorkspace("Cleanup");
      expect(screen.getByRole("button", { name: /refresh cleanup preview/i })).toBeInTheDocument();
      expect(screen.getByText(/built-in cleanup rules/i)).toBeInTheDocument();
      expect(screen.getAllByText(/recycle bin/i).length).toBeGreaterThan(0);

      await activateWorkspace("Overview");
      expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(screen.getAllByRole("tabpanel")).toHaveLength(1);
    }

    setViewportWidth(originalWidth);

    expect(client.startScan).not.toHaveBeenCalled();
    expect(client.previewCleanup).not.toHaveBeenCalled();
    expect(client.executeCleanup).not.toHaveBeenCalled();
    expect(client.resumeScanRun).not.toHaveBeenCalled();
  });

  it("safety_panel_keeps_approved_durable_guidance", async () => {
    render(<App client={createWorkspaceClient()} />);

    await activateWorkspace("Safety");

    const safetyPanel = screen.getByRole("tabpanel", { name: "Safety" });
    const guidanceRegion = within(safetyPanel).getByRole("region", {
      name: /safety guidance/i,
    });

    expect(guidanceRegion).toHaveTextContent(/unprivileged/i);
    expect(guidanceRegion).toHaveTextContent(/recycle bin/i);
    expect(guidanceRegion).toHaveTextContent(/local sqlite history/i);
    expect(guidanceRegion).toHaveTextContent(/protected-path cleanup/i);
    expect(guidanceRegion).toHaveTextContent(/permanent delete/i);
    expect(guidanceRegion).toHaveTextContent(/resume actions use can_resume/i);
    expect(guidanceRegion).not.toHaveTextContent(/sample|prototype|cloud sync|telemetry/i);
  });

  it("overview_metrics_render_real_or_explicit_empty_states", async () => {
    render(<App client={createWorkspaceClient()} />);

    await waitForWorkspaceShell();

    const emptyMetrics = screen.getByRole("region", { name: /overview metrics/i });
    expect(within(emptyMetrics).getByRole("article", { name: /total bytes metric/i })).toHaveTextContent(
      /not yet run/i,
    );
    expect(within(emptyMetrics).getByRole("article", { name: /total files metric/i })).toHaveTextContent(
      /not yet run/i,
    );
    expect(within(emptyMetrics).getByRole("article", { name: /duplicate reclaimable metric/i })).toHaveTextContent(
      /not analyzed/i,
    );
    expect(within(emptyMetrics).getByRole("article", { name: /cleanup candidates metric/i })).toHaveTextContent(
      /preview not generated/i,
    );
    expect(emptyMetrics).not.toHaveTextContent(/1\.2 tb|47,000|sample|prototype/i);
  });

  it("overview_metrics_use_loaded_scan_values_when_available", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus(),
          scan: makeBrowseableScan(),
        })}
      />,
    );

    await waitForWorkspaceShell();

    const metrics = screen.getByRole("region", { name: /overview metrics/i });
    expect(within(metrics).getByRole("article", { name: /total bytes metric/i })).toHaveTextContent(
      /4096 bytes/i,
    );
    expect(within(metrics).getByRole("article", { name: /total files metric/i })).toHaveTextContent(
      /^total files\s*3/i,
    );
  });

  it("scan_panel_groups_command_progress_and_running_context", async () => {
    render(<App client={createWorkspaceClient({ scanStatus: makeRunningStatus() })} />);

    await activateWorkspace("Scan");

    const scanPanel = screen.getByRole("tabpanel", { name: "Scan" });
    const commandRegion = within(scanPanel).getByRole("region", {
      name: /scan command and progress/i,
    });
    expect(within(commandRegion).getByLabelText(/scan root/i)).toBeInTheDocument();
    expect(within(commandRegion).getByRole("button", { name: /start scan/i })).toBeInTheDocument();
    expect(within(commandRegion).getByRole("button", { name: /cancel scan/i })).toBeInTheDocument();
    expect(within(commandRegion).getByText(/running/i)).toBeInTheDocument();
    expect(within(commandRegion).getByText(/2048 bytes processed/i)).toBeInTheDocument();
    expect(within(commandRegion).getByText(/c:\\users\\xiongxianfei\\downloads/i)).toBeInTheDocument();

    expect(within(scanPanel).getByRole("region", { name: /active scan details/i })).toBeInTheDocument();
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

  it("global_status_ignores_cleanup_state_for_different_loaded_scan", async () => {
    const firstScan = makeBrowseableScan("scan-shell");
    const secondScan = makeSecondBrowseableScan("scan-shell-2");
    const client = createWorkspaceClient({
      scanStatus: makeCompletedStatus(firstScan.scanId),
      scan: firstScan,
      scansById: {
        [firstScan.scanId]: firstScan,
        [secondScan.scanId]: secondScan,
      },
      history: [
        makeHistoryEntry(firstScan.scanId, firstScan.rootPath, firstScan.totalBytes),
        makeHistoryEntry(secondScan.scanId, secondScan.rootPath, secondScan.totalBytes),
      ],
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
      expect(getStatusRegion()).toHaveTextContent(
        /cleanup execution completed with rescan recommended/i,
      );
      expect(screen.getByRole("button", { name: /start a fresh scan/i })).toBeInTheDocument();
    });

    await activateWorkspace("History");
    fireEvent.click(screen.getByRole("button", { name: /reopen scan scan-shell-2/i }));

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Explorer" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(getStatusRegion()).toHaveTextContent(/completed scan loaded/i);
      expect(screen.getByRole("button", { name: /find duplicates/i })).toBeInTheDocument();
    });

    expect(getStatusRegion()).not.toHaveTextContent(
      /cleanup execution completed with rescan recommended/i,
    );
    expect(
      screen.queryByRole("button", { name: /start a fresh scan/i }),
    ).not.toBeInTheDocument();
  });

  it("initial_workspace_prefers_running_scan", async () => {
    const client = createWorkspaceClient({
      scanStatus: makeRunningStatus(),
      scanRuns: [makeInterruptedRunSummary()],
      restoreContext: makeWorkspaceRestoreContext("explorer", "scan-shell"),
      scan: makeBrowseableScan(),
    });

    render(<App client={client} />);
    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Scan" })).toHaveAttribute("aria-selected", "true");
    });
  });

  it("initial_workspace_prefers_interrupted_runs_over_restore_context", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus(),
          scan: makeBrowseableScan(),
          scanRuns: [makeInterruptedRunSummary()],
          restoreContext: makeWorkspaceRestoreContext("explorer", "scan-shell"),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "History" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });
  });

  it("startup_restore_validation_failure_shows_shell_notice", async () => {
    const logger = vi.spyOn(workspaceShellLogger, "log").mockImplementation(() => {});

    render(
      <App
        client={createWorkspaceClient({
          restoreContext: makeWorkspaceRestoreContext("explorer", "missing-scan"),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(screen.getByText(/saved explorer context could not be restored/i)).toBeInTheDocument();
    });

    expect(logger).toHaveBeenCalledWith(
      "workspace_restore_context_validation_failed",
      expect.objectContaining({
        lastOpenedScanId: "missing-scan",
      }),
    );

    logger.mockRestore();
  });

  it("startup_restore_read_failure_shows_shell_notice", async () => {
    const logger = vi.spyOn(workspaceShellLogger, "log").mockImplementation(() => {});

    render(
      <App
        client={createWorkspaceClient({
          restoreContextError: new Error("restore read failed"),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(screen.getByText(/saved workspace context could not be read/i)).toBeInTheDocument();
    });

    expect(logger).toHaveBeenCalledWith(
      "workspace_restore_context_load_failed",
      expect.objectContaining({
        message: "restore read failed",
      }),
    );

    logger.mockRestore();
  });

  it("initial_workspace_restores_valid_explorer_context", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus(),
          scan: makeBrowseableScan(),
          restoreContext: makeWorkspaceRestoreContext("explorer", "scan-shell"),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Explorer" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(screen.getByRole("heading", { name: /current result/i })).toBeInTheDocument();
    });
  });

  it("manual_workspace_activation_during_startup_does_not_get_overwritten_in_restore_context", async () => {
    const client = createWorkspaceClient({
      scanStatus: makeCompletedStatus(),
      scan: makeBrowseableScan(),
      restoreContext: makeWorkspaceRestoreContext("explorer", "scan-shell"),
    });
    const openDeferred = defer<CompletedScan>();
    client.openScanHistory = vi.fn(async () => openDeferred.promise);

    render(<App client={client} />);
    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(client.openScanHistory).toHaveBeenCalledWith("scan-shell");
    });

    await activateWorkspace("History");

    await act(async () => {
      openDeferred.resolve(makeBrowseableScan());
    });

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "History" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });

    expect(client.saveWorkspaceRestoreContext).toHaveBeenCalledTimes(1);
    expect(client.saveWorkspaceRestoreContext).toHaveBeenCalledWith({
      lastWorkspace: "history",
      lastOpenedScanId: "scan-shell",
    });
  });

  it("initial_workspace_restores_summary_only_explorer_context_in_degraded_mode", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus("scan-summary"),
          scan: makeSummaryOnlyScan("scan-summary"),
          restoreContext: makeWorkspaceRestoreContext("explorer", "scan-summary"),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Explorer" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(
        screen.getByText(/saved before folder browsing support\. run a fresh scan/i),
      ).toBeInTheDocument();
    });
  });

  it("cold_start_does_not_restore_duplicates_or_cleanup_from_session_only_context", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus(),
          scan: makeBrowseableScan(),
          restoreContext: makeWorkspaceRestoreContext("duplicates", "scan-shell"),
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Overview" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });

    expect(screen.getByRole("tab", { name: "Duplicates" })).toHaveAttribute(
      "aria-selected",
      "false",
    );
  });

  it("N1_START_SCAN_switches_back_to_scan_for_the_matching_running_snapshot", async () => {
    const harness = createWorkspaceHarness();
    const startScanDeferred = defer<{ scanId: string }>();
    const logger = vi.spyOn(workspaceShellLogger, "log").mockImplementation(() => {});
    harness.client.startScan = vi.fn(async () => startScanDeferred.promise);

    render(<App client={harness.client} />);
    await waitForWorkspaceShell();
    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await activateWorkspace("History");

    await harness.emitScan(makeRunningStatus());

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Scan" })).toHaveAttribute("aria-selected", "true");
    });

    await act(async () => {
      startScanDeferred.resolve({ scanId: "scan-running" });
    });

    await waitFor(() => {
      expect(logger).toHaveBeenCalledWith(
        "workspace_auto_switch_applied",
        expect.objectContaining({
          phase: "running",
          reason: "N1_START_SCAN",
          operationId: "scan-running",
          target: "scan",
        }),
      );
      expect(logger).toHaveBeenCalledWith(
        "workspace_auto_switch_skipped_duplicate",
        expect.objectContaining({
          phase: "accepted",
          reason: "N1_START_SCAN",
          operationId: "scan-running",
          target: "scan",
        }),
      );
    });

    logger.mockRestore();
  });

  it("N1_START_SCAN_matching_running_snapshot_can_switch_once_after_accepted_start", async () => {
    const harness = createWorkspaceHarness();
    harness.client.startScan = vi.fn(async () => ({ scanId: "scan-running" }));

    render(<App client={harness.client} />);
    await waitForWorkspaceShell();
    await activateWorkspace("Scan");

    fireEvent.change(screen.getByLabelText(/scan root/i), {
      target: { value: "C:\\Users\\xiongxianfei\\Downloads" },
    });
    fireEvent.click(screen.getByRole("button", { name: /start scan/i }));

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Scan" })).toHaveAttribute("aria-selected", "true");
    });

    await activateWorkspace("History");
    await harness.emitScan(makeRunningStatus());

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Scan" })).toHaveAttribute("aria-selected", "true");
    });

    await activateWorkspace("History");
    await harness.emitScan(makeRunningStatus());

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "History" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });
  });

  it("N2_SCAN_COMPLETED_AND_OPENED_switches_to_explorer_after_persistence_and_open", async () => {
    const harness = createWorkspaceHarness({
      scan: makeBrowseableScan(),
    });

    render(<App client={harness.client} />);
    await waitForWorkspaceShell();
    await activateWorkspace("History");

    await harness.emitScan({
      ...makeCompletedStatus(),
      scanId: "scan-running",
      completedScanId: "scan-shell",
    });

    await waitFor(() => {
      expect(harness.client.openScanHistory).toHaveBeenCalledWith("scan-shell");
      expect(screen.getByRole("tab", { name: "Explorer" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });
  });

  it("N4_START_DUPLICATE_ANALYSIS_restores_duplicates_when_acceptance_finishes_after_manual_navigation", async () => {
    const harness = createWorkspaceHarness({
      scanStatus: makeCompletedStatus(),
      scan: makeBrowseableScan(),
      restoreContext: makeWorkspaceRestoreContext("explorer", "scan-shell"),
    });
    const duplicateDeferred = defer<{ analysisId: string }>();
    harness.client.startDuplicateAnalysis = vi.fn(async () => duplicateDeferred.promise);

    render(<App client={harness.client} />);
    await waitForWorkspaceShell();
    await activateWorkspace("Duplicates");

    fireEvent.click(screen.getByRole("button", { name: /analyze duplicates/i }));
    await activateWorkspace("Overview");

    await harness.emitDuplicate({
      analysisId: "analysis-running",
      scanId: "scan-shell",
      state: "running",
      stage: "grouping",
      itemsProcessed: 1,
      groupsEmitted: 0,
      message: null,
      completedAnalysisId: null,
    });

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Duplicates" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });

    await act(async () => {
      duplicateDeferred.resolve({ analysisId: "analysis-running" });
    });
  });

  it("background_refresh_does_not_steal_focus", async () => {
    const harness = createWorkspaceHarness({
      scanStatus: makeRunningStatus(),
      scan: makeBrowseableScan(),
      restoreContext: makeWorkspaceRestoreContext("explorer", "scan-shell"),
    });

    render(<App client={harness.client} />);
    await waitForWorkspaceShell();
    await activateWorkspace("Explorer");

    const explorerTab = screen.getByRole("tab", { name: "Explorer" });
    explorerTab.focus();
    expect(explorerTab).toHaveFocus();

    await harness.emitScan({
      ...makeRunningStatus(),
      state: "failed",
      message: "Background scan failed.",
    });

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Explorer" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(explorerTab).toHaveFocus();
      expect(screen.getAllByText(/background scan failed\./i)).toHaveLength(2);
    });
  });

  it("interrupted_run_notice_visible_without_focus_steal", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeCompletedStatus(),
          scan: makeBrowseableScan(),
          scanRuns: [makeInterruptedRunSummary()],
        })}
      />,
    );

    await waitForWorkspaceShell();
    await activateWorkspace("Explorer");

    const explorerTab = screen.getByRole("tab", { name: "Explorer" });
    explorerTab.focus();

    await waitFor(() => {
      expect(screen.getByText(/interrupted runs need review in history\./i)).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Explorer" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(explorerTab).toHaveFocus();
    });
  });

  it("active_live_task_notice_visible_from_non_overview_workspace", async () => {
    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeRunningStatus(),
        })}
      />,
    );

    await waitForWorkspaceShell();
    await activateWorkspace("Safety");

    const safetyTab = screen.getByRole("tab", { name: "Safety" });
    safetyTab.focus();

    await waitFor(() => {
      expect(screen.getByText(/a scan is running\. review progress in scan\./i)).toBeInTheDocument();
      expect(screen.getByRole("tab", { name: "Safety" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
      expect(safetyTab).toHaveFocus();
    });
  });

  it("shell_notices_and_next_safe_action_are_logged", async () => {
    const logger = vi.spyOn(workspaceShellLogger, "log").mockImplementation(() => {});

    render(
      <App
        client={createWorkspaceClient({
          scanStatus: makeRunningStatus(),
          scanRuns: [makeInterruptedRunSummary()],
        })}
      />,
    );

    await waitForWorkspaceShell();

    await waitFor(() => {
      expect(logger).toHaveBeenCalledWith(
        "workspace_next_safe_action_selected",
        expect.objectContaining({
          label: "View scan progress",
          primaryStateLabel: "Live scan running",
          target: "scan",
        }),
      );
      expect(logger).toHaveBeenCalledWith(
        "workspace_status_notice_rendered",
        expect.objectContaining({
          kind: "live_task",
          noticeKey: "live-scan-running",
        }),
      );
      expect(logger).toHaveBeenCalledWith(
        "workspace_status_notice_rendered",
        expect.objectContaining({
          kind: "interrupted_runs",
          noticeKey: "interrupted-runs-attention",
        }),
      );
    });

    logger.mockRestore();
  });

  it("replayed_terminal_event_does_not_reset_review_state", async () => {
    const harness = createWorkspaceHarness({
      scan: makeBrowseableScan(),
    });

    render(<App client={harness.client} />);
    await waitForWorkspaceShell();
    await activateWorkspace("History");

    await harness.emitScan({
      ...makeCompletedStatus(),
      scanId: "scan-running",
      completedScanId: "scan-shell",
    });

    await waitFor(() => {
      expect(screen.getByRole("tab", { name: "Explorer" })).toHaveAttribute(
        "aria-selected",
        "true",
      );
    });

    fireEvent.click(screen.getByRole("button", { name: /sort by name/i }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: /sort by name/i })).toHaveClass("is-active");
    });

    await harness.emitScan({
      ...makeCompletedStatus(),
      scanId: "scan-running",
      completedScanId: "scan-shell",
    });

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /sort by name/i })).toHaveClass("is-active");
      expect(screen.getByRole("button", { name: /sort by size/i })).not.toHaveClass("is-active");
    });
  });
});
