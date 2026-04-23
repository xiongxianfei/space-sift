import {
  startTransition,
  useEffect,
  useEffectEvent,
  useRef,
  useState,
  type FormEvent,
  type KeyboardEvent,
} from "react";
import "./App.css";
import {
  idleDuplicateStatus,
  idleScanStatus,
  type SpaceSiftClient,
  unsupportedClient,
} from "./lib/spaceSiftClient";
import type {
  CleanupExecutionMode,
  CleanupExecutionResult,
  CleanupPreview,
  CleanupRuleDefinition,
  CompletedDuplicateAnalysis,
  CompletedScan,
  DuplicateAnalysisStage,
  DuplicateGroup,
  DuplicateGroupMember,
  DuplicateStatusSnapshot,
  PrivilegedCleanupCapability,
  ScanEntry,
  ScanHistoryEntry,
  ScanRunSummary,
  ScanStatusSnapshot,
  WorkspaceRestoreContext,
} from "./lib/spaceSiftTypes";
import {
  deriveGlobalStatus,
  getNextSafeActionReason,
  resolveInitialWorkspace,
  workspaceDefinitions,
  type WorkspaceNavigationReason,
  type WorkspaceTab,
} from "./workspaceNavigation";
import { workspaceShellLogger } from "./workspaceShellLogger";

const safetyPrinciples = [
  {
    title: "Unprivileged by default",
    body: "Normal scanning and history flows stay in the standard desktop process without elevating the whole UI.",
  },
  {
    title: "Recycle Bin first",
    body: "Cleanup defaults to the Recycle Bin path, with permanent delete kept behind an explicit advanced control.",
  },
  {
    title: "Local-only history",
    body: "Completed scans are cached in local SQLite storage so reopening a result never needs a network connection.",
  },
];

type AppProps = {
  client?: SpaceSiftClient;
};

type ExplorerSortMode = "size" | "name";

type BrowseableScan = CompletedScan & {
  entries: ScanEntry[];
};

type DuplicateKeepSelections = Record<string, string>;
type DuplicateDisclosureState = Record<string, boolean>;
type RestoreContextResult =
  | { kind: "loaded"; context: WorkspaceRestoreContext | null }
  | { kind: "error"; error: unknown };
type OpenStoredScanOptions = {
  nextNotice?: string;
  preservedDuplicateStatusSnapshot?: DuplicateStatusSnapshot;
  persistWorkspace?: WorkspaceTab;
  persistRestoreContext?: boolean;
};
type WorkspaceSwitchPhase = "default" | "accepted" | "running";
type CleanupExecutionState = {
  scanId: string;
  result: CleanupExecutionResult;
};
type ShellNoticeKind = "status" | "live_task" | "interrupted_runs";
type ShellNoticeEntry = {
  key: string;
  kind: ShellNoticeKind;
  message: string;
};

function formatBytes(bytes: number) {
  return `${bytes} bytes`;
}

function formatTimestamp(value: string) {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString();
}

function formatElapsedWindow(startedAt: string | null, updatedAt: string | null) {
  if (!startedAt) {
    return null;
  }

  const started = new Date(startedAt).getTime();
  const updated = updatedAt ? new Date(updatedAt).getTime() : started;
  if (Number.isNaN(started) || Number.isNaN(updated)) {
    return null;
  }

  const totalSeconds = Math.max(0, Math.floor((updated - started) / 1000));
  if (totalSeconds < 60) {
    return `${totalSeconds}s active`;
  }

  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  if (minutes < 60) {
    return seconds === 0 ? `${minutes}m active` : `${minutes}m ${seconds}s active`;
  }

  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  return remainingMinutes === 0
    ? `${hours}h active`
    : `${hours}h ${remainingMinutes}m active`;
}

function describeError(error: unknown) {
  if (error instanceof Error) {
    return error.message;
  }

  if (
    typeof error === "object" &&
    error != null &&
    "message" in error &&
    typeof error.message === "string"
  ) {
    return error.message;
  }

  if (typeof error === "string") {
    return error;
  }

  return "The requested operation did not complete.";
}

function buildWorkspaceSwitchKey(
  reason: WorkspaceNavigationReason,
  target: WorkspaceTab,
  operationId?: string | null,
  phase: WorkspaceSwitchPhase = "default",
) {
  return `${reason}:${target}:${phase}:${operationId ?? "none"}`;
}

function getPathLabel(path: string) {
  const normalized = path.replace(/[\\/]+$/, "");
  if (!normalized) {
    return path;
  }

  const segments = normalized.split(/[\\/]/).filter(Boolean);
  return segments[segments.length - 1] ?? normalized;
}

function comparePaths(left: string, right: string) {
  return left.localeCompare(right, undefined, {
    sensitivity: "base",
    numeric: true,
  });
}

function getRootRelativePath(path: string, rootPath: string) {
  const normalizedPath = path.replace(/[\\/]+$/, "");
  const normalizedRoot = rootPath.replace(/[\\/]+$/, "");
  if (!normalizedPath || !normalizedRoot) {
    return null;
  }

  if (!isPathWithinRoot(normalizedPath, normalizedRoot)) {
    return null;
  }

  if (normalizedPath.length === normalizedRoot.length) {
    return "";
  }

  return normalizedPath.slice(normalizedRoot.length + 1);
}

function getDuplicateMemberLocationLabel(path: string, rootPath: string) {
  const normalized = path.replace(/[\\/]+$/, "");
  const rootRelativePath = getRootRelativePath(normalized, rootPath);
  if (rootRelativePath != null) {
    const relativeSegments = rootRelativePath.split(/[\\/]/).filter(Boolean);
    if (relativeSegments.length <= 1) {
      return "Scan root";
    }

    return relativeSegments.slice(0, -1).join("\\");
  }

  const segments = normalized.split(/[\\/]/).filter(Boolean);
  if (segments.length <= 1) {
    return normalized;
  }

  return segments.slice(0, -1).join("\\");
}

function compareEntryNames(left: ScanEntry, right: ScanEntry) {
  return (
    comparePaths(getPathLabel(left.path), getPathLabel(right.path)) ||
    comparePaths(left.path, right.path)
  );
}

function sortExplorerEntries(entries: ScanEntry[], sortMode: ExplorerSortMode) {
  return [...entries].sort((left, right) => {
    if (sortMode === "name") {
      return compareEntryNames(left, right) || right.sizeBytes - left.sizeBytes;
    }

    return (
      right.sizeBytes - left.sizeBytes ||
      compareEntryNames(left, right) ||
      (left.kind === right.kind ? 0 : left.kind === "directory" ? -1 : 1)
    );
  });
}

function getSortableTimestamp(value: string) {
  const parsed = new Date(value).getTime();
  return Number.isNaN(parsed) ? Number.NEGATIVE_INFINITY : parsed;
}

function sortHistoryEntries(entries: ScanHistoryEntry[]) {
  return [...entries].sort((left, right) => {
    const completedDelta =
      getSortableTimestamp(right.completedAt) - getSortableTimestamp(left.completedAt);
    if (completedDelta !== 0) {
      return completedDelta;
    }

    return left.scanId.localeCompare(right.scanId, undefined, {
      sensitivity: "base",
      numeric: true,
    });
  });
}

function hasBrowseableEntries(scan: CompletedScan): scan is BrowseableScan {
  return (
    Array.isArray(scan.entries) &&
    scan.entries.length > 0 &&
    scan.entries.some(
      (entry) =>
        entry.path === scan.rootPath &&
        entry.kind === "directory" &&
        entry.parentPath == null,
    )
  );
}

function hasFileEntries(scan: CompletedScan) {
  return Array.isArray(scan.entries) && scan.entries.some((entry) => entry.kind === "file");
}

function isPathWithinRoot(path: string, rootPath: string) {
  const normalizedPath = path.toLowerCase();
  const normalizedRoot = rootPath.toLowerCase();
  return normalizedPath === normalizedRoot || normalizedPath.startsWith(`${normalizedRoot}\\`);
}

function listVisibleEntries(
  scan: BrowseableScan,
  currentPath: string,
  sortMode: ExplorerSortMode,
) {
  const visible = scan.entries.filter((entry) => entry.parentPath === currentPath);
  return sortExplorerEntries(visible, sortMode);
}

function buildBreadcrumbs(scan: BrowseableScan, currentPath: string) {
  const entryByPath = new Map(scan.entries.map((entry) => [entry.path, entry]));
  const breadcrumbs: { path: string; label: string }[] = [];
  let cursor: string | null = currentPath;

  while (cursor) {
    const entry = entryByPath.get(cursor);
    if (!entry) {
      break;
    }

    breadcrumbs.unshift({
      path: entry.path,
      label: getPathLabel(entry.path),
    });
    cursor = entry.parentPath;
  }

  if (breadcrumbs.length === 0 || breadcrumbs[0].path !== scan.rootPath) {
    return [
      {
        path: scan.rootPath,
        label: getPathLabel(scan.rootPath),
      },
    ];
  }

  return breadcrumbs;
}

function getDuplicateStageLabel(stage: DuplicateAnalysisStage | null) {
  switch (stage) {
    case "grouping":
      return "Grouping";
    case "partial_hash":
      return "Partial hash";
    case "full_hash":
      return "Full hash";
    case "completed":
      return "Completed";
    default:
      return "Waiting";
  }
}

function getMemberTimestamp(member: DuplicateGroupMember) {
  const parsed = new Date(member.modifiedAt).getTime();
  return Number.isNaN(parsed) ? 0 : parsed;
}

function chooseMemberByAge(
  members: DuplicateGroupMember[],
  direction: "newest" | "oldest",
) {
  const ordered = [...members].sort((left, right) => {
    const delta = getMemberTimestamp(left) - getMemberTimestamp(right);
    if (delta !== 0) {
      return direction === "newest" ? -delta : delta;
    }

    return comparePaths(left.path, right.path);
  });

  return ordered[0]?.path ?? "";
}

function getDuplicateGroupSortPath(group: DuplicateGroup) {
  return [...group.members]
    .map((member) => member.path)
    .sort(comparePaths)[0] ?? group.groupId;
}

function sortDuplicateGroups(groups: DuplicateGroup[]) {
  return [...groups].sort((left, right) => {
    return (
      right.reclaimableBytes - left.reclaimableBytes ||
      right.members.length - left.members.length ||
      comparePaths(getDuplicateGroupSortPath(left), getDuplicateGroupSortPath(right)) ||
      comparePaths(left.groupId, right.groupId)
    );
  });
}

function resolveKeptPath(group: DuplicateGroup, selections: DuplicateKeepSelections) {
  const candidate = selections[group.groupId];
  if (candidate && group.members.some((member) => member.path === candidate)) {
    return candidate;
  }

  return chooseMemberByAge(group.members, "newest");
}

function buildDefaultKeepSelections(
  analysis: CompletedDuplicateAnalysis,
): DuplicateKeepSelections {
  return Object.fromEntries(
    analysis.groups.map((group) => [group.groupId, chooseMemberByAge(group.members, "newest")]),
  );
}

function summarizeDuplicatePreview(
  analysis: CompletedDuplicateAnalysis,
  selections: DuplicateKeepSelections,
) {
  let filesMarkedForDeletion = 0;
  let reclaimableBytes = 0;

  for (const group of analysis.groups) {
    const keptPath = resolveKeptPath(group, selections);
    for (const member of group.members) {
      if (member.path !== keptPath) {
        filesMarkedForDeletion += 1;
        reclaimableBytes += member.sizeBytes;
      }
    }
  }

  return {
    filesMarkedForDeletion,
    reclaimableBytes,
  };
}

function buildDuplicateDeletePaths(
  analysis: CompletedDuplicateAnalysis | null,
  selections: DuplicateKeepSelections,
) {
  if (!analysis) {
    return [];
  }

  const paths = new Set<string>();
  for (const group of analysis.groups) {
    const keptPath = resolveKeptPath(group, selections);
    for (const member of group.members) {
      if (member.path !== keptPath) {
        paths.add(member.path);
      }
    }
  }

  return [...paths];
}

function App({ client = unsupportedClient }: AppProps) {
  const [rootPath, setRootPath] = useState("");
  const [activeWorkspace, setActiveWorkspace] = useState<WorkspaceTab>("overview");
  const [scanStatus, setScanStatus] = useState<ScanStatusSnapshot>(idleScanStatus);
  const [duplicateStatus, setDuplicateStatus] =
    useState<DuplicateStatusSnapshot>(idleDuplicateStatus);
  const [history, setHistory] = useState<ScanHistoryEntry[]>([]);
  const [scanRuns, setScanRuns] = useState<ScanRunSummary[]>([]);
  const [currentScan, setCurrentScan] = useState<CompletedScan | null>(null);
  const [currentExplorerPath, setCurrentExplorerPath] = useState<string | null>(null);
  const [explorerSortMode, setExplorerSortMode] = useState<ExplorerSortMode>("size");
  const [duplicateAnalysis, setDuplicateAnalysis] =
    useState<CompletedDuplicateAnalysis | null>(null);
  const [duplicateKeepSelections, setDuplicateKeepSelections] =
    useState<DuplicateKeepSelections>({});
  const [expandedDuplicateGroups, setExpandedDuplicateGroups] =
    useState<DuplicateDisclosureState>({});
  const [historyRootFilter, setHistoryRootFilter] = useState("");
  const [historyScanIdFilter, setHistoryScanIdFilter] = useState("");
  const [cleanupRules, setCleanupRules] = useState<CleanupRuleDefinition[]>([]);
  const [selectedCleanupRuleIds, setSelectedCleanupRuleIds] = useState<string[]>([]);
  const [cleanupPreview, setCleanupPreview] = useState<CleanupPreview | null>(null);
  const [cleanupExecutionState, setCleanupExecutionState] =
    useState<CleanupExecutionState | null>(null);
  const [privilegedCapability, setPrivilegedCapability] =
    useState<PrivilegedCleanupCapability | null>(null);
  const [permanentDeleteConfirmed, setPermanentDeleteConfirmed] = useState(false);
  const [resumeEnabled, setResumeEnabled] = useState(false);
  const [shellNotice, setShellNotice] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const workspaceRestoreContextRef = useRef<WorkspaceRestoreContext | null>(null);
  const lastOpenedScanIdRef = useRef<string | null>(null);
  const appliedWorkspaceSwitchesRef = useRef<Set<string>>(new Set());
  const pendingScanStartRef = useRef<{ rootPath: string } | null>(null);
  const pendingDuplicateStartRef = useRef<{ scanId: string } | null>(null);
  const startupResolutionPendingRef = useRef(true);
  const manualWorkspaceDuringStartupRef = useRef<WorkspaceTab | null>(null);
  const visibleShellNoticeKeysRef = useRef<Set<string>>(new Set());
  const lastNextSafeActionKeyRef = useRef<string | null>(null);
  const workspaceTabRefs = useRef<Record<WorkspaceTab, HTMLButtonElement | null>>({
    overview: null,
    scan: null,
    history: null,
    explorer: null,
    duplicates: null,
    cleanup: null,
    safety: null,
  });

  const resetCleanupState = useEffectEvent(() => {
    startTransition(() => {
      setSelectedCleanupRuleIds([]);
      setCleanupPreview(null);
      setCleanupExecutionState(null);
      setPermanentDeleteConfirmed(false);
    });
  });

  const resetReviewState = useEffectEvent(() => {
    startTransition(() => {
      setDuplicateStatus(idleDuplicateStatus);
      setDuplicateAnalysis(null);
      setDuplicateKeepSelections({});
      setExpandedDuplicateGroups({});
      setSelectedCleanupRuleIds([]);
      setCleanupPreview(null);
      setCleanupExecutionState(null);
      setPermanentDeleteConfirmed(false);
    });
  });

  const persistWorkspaceRestoreContext = useEffectEvent(
    async (lastWorkspace: WorkspaceTab, lastOpenedScanId: string | null) => {
      const currentContext = workspaceRestoreContextRef.current;
      if (
        currentContext?.lastWorkspace === lastWorkspace &&
        currentContext.lastOpenedScanId === lastOpenedScanId
      ) {
        return;
      }

      try {
        const savedContext = await client.saveWorkspaceRestoreContext({
          lastWorkspace,
          lastOpenedScanId,
        });
        workspaceRestoreContextRef.current = savedContext;
      } catch (error) {
        workspaceShellLogger.log("workspace_restore_context_save_failed", {
          lastWorkspace,
          lastOpenedScanId,
          message: describeError(error),
        });
      }
    },
  );

  const activateWorkspace = useEffectEvent((tab: WorkspaceTab) => {
    if (startupResolutionPendingRef.current) {
      manualWorkspaceDuringStartupRef.current = tab;
    }
    setActiveWorkspace(tab);
    void persistWorkspaceRestoreContext(tab, lastOpenedScanIdRef.current);
  });

  const applyContractualWorkspaceSwitch = useEffectEvent(
    (
      target: WorkspaceTab,
      reason: Exclude<WorkspaceNavigationReason, "manual" | "startup">,
      operationId?: string | null,
      options?: {
        phase?: WorkspaceSwitchPhase;
        blockedByKeys?: string[];
      },
    ) => {
      const phase = options?.phase ?? "default";
      const key = buildWorkspaceSwitchKey(reason, target, operationId, phase);
      const isBlocked =
        options?.blockedByKeys?.some((blockedKey) =>
          appliedWorkspaceSwitchesRef.current.has(blockedKey),
        ) ?? false;
      if (isBlocked || appliedWorkspaceSwitchesRef.current.has(key)) {
        workspaceShellLogger.log("workspace_auto_switch_skipped_duplicate", {
          reason,
          target,
          operationId: operationId ?? null,
          phase,
        });
        return false;
      }

      appliedWorkspaceSwitchesRef.current.add(key);
      setActiveWorkspace(target);
      void persistWorkspaceRestoreContext(target, lastOpenedScanIdRef.current);
      workspaceShellLogger.log("workspace_auto_switch_applied", {
        reason,
        target,
        operationId: operationId ?? null,
        phase,
      });
      return true;
    },
  );

  function setWorkspaceTabRef(tab: WorkspaceTab, element: HTMLButtonElement | null) {
    workspaceTabRefs.current[tab] = element;
  }

  function focusWorkspaceAtIndex(index: number) {
    const nextWorkspace = workspaceDefinitions[index];
    if (!nextWorkspace) {
      return;
    }

    workspaceTabRefs.current[nextWorkspace.value]?.focus();
  }

  function handleWorkspaceKeyDown(
    event: KeyboardEvent<HTMLButtonElement>,
    workspace: WorkspaceTab,
  ) {
    const currentIndex = workspaceDefinitions.findIndex(
      (definition) => definition.value === workspace,
    );
    if (currentIndex < 0) {
      return;
    }

    switch (event.key) {
      case "ArrowRight":
        event.preventDefault();
        focusWorkspaceAtIndex((currentIndex + 1) % workspaceDefinitions.length);
        return;
      case "ArrowDown":
        event.preventDefault();
        focusWorkspaceAtIndex((currentIndex + 1) % workspaceDefinitions.length);
        return;
      case "ArrowLeft":
        event.preventDefault();
        focusWorkspaceAtIndex(
          (currentIndex - 1 + workspaceDefinitions.length) % workspaceDefinitions.length,
        );
        return;
      case "ArrowUp":
        event.preventDefault();
        focusWorkspaceAtIndex(
          (currentIndex - 1 + workspaceDefinitions.length) % workspaceDefinitions.length,
        );
        return;
      case "Home":
        event.preventDefault();
        focusWorkspaceAtIndex(0);
        return;
      case "End":
        event.preventDefault();
        focusWorkspaceAtIndex(workspaceDefinitions.length - 1);
        return;
      case "Enter":
      case " ":
        event.preventDefault();
        activateWorkspace(workspace);
        return;
      default:
        return;
    }
  }

  const loadHistory = useEffectEvent(async () => {
    try {
      const nextHistory = await client.listScanHistory();
      startTransition(() => {
        setHistory(nextHistory);
      });
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
    }
  });

  const loadScanRuns = useEffectEvent(async () => {
    try {
      const nextRuns = await client.listScanRuns();
      startTransition(() => {
        setScanRuns(nextRuns);
      });
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
    }
  });

  const openStoredDuplicateAnalysis = useEffectEvent(async (analysisId: string) => {
    try {
      const result = await client.openDuplicateAnalysis(analysisId);
      startTransition(() => {
        setDuplicateAnalysis(result);
        setDuplicateKeepSelections(buildDefaultKeepSelections(result));
        setExpandedDuplicateGroups({});
        setDuplicateStatus((currentValue) => ({
          ...currentValue,
          analysisId: result.analysisId,
          scanId: result.scanId,
          state: "completed",
          stage: "completed",
          groupsEmitted: result.groups.length,
          message: currentValue.message ?? "Duplicate analysis complete.",
          completedAnalysisId: result.analysisId,
        }));
        setCleanupPreview(null);
        setCleanupExecutionState(null);
        setPermanentDeleteConfirmed(false);
        setErrorMessage(null);
      });
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
    }
  });

  const applyOpenedScanResult = useEffectEvent(
    (result: CompletedScan, options?: OpenStoredScanOptions) => {
      startTransition(() => {
        const preservedDuplicateStatus =
          options?.preservedDuplicateStatusSnapshot?.state === "running" &&
          options.preservedDuplicateStatusSnapshot.scanId === result.scanId
            ? options.preservedDuplicateStatusSnapshot
            : duplicateStatus.state === "running" && duplicateStatus.scanId === result.scanId
              ? duplicateStatus
              : idleDuplicateStatus;
        setCurrentScan(result);
        setCurrentExplorerPath(result.rootPath);
        setExplorerSortMode("size");
        setNotice(options?.nextNotice ?? `Loaded ${result.scanId} from local history.`);
        setErrorMessage(null);
        setDuplicateStatus(preservedDuplicateStatus);
        setDuplicateAnalysis(null);
        setDuplicateKeepSelections({});
        setExpandedDuplicateGroups({});
        setSelectedCleanupRuleIds([]);
        setCleanupPreview(null);
        setCleanupExecutionState(null);
        setPermanentDeleteConfirmed(false);
      });

      lastOpenedScanIdRef.current = result.scanId;
      if (options?.persistRestoreContext !== false) {
        void persistWorkspaceRestoreContext(
          options?.persistWorkspace ?? activeWorkspace,
          result.scanId,
        );
      }
      return result;
    },
  );

  const openStoredScan = useEffectEvent(async (scanId: string, options?: OpenStoredScanOptions) => {
    try {
      const result = await client.openScanHistory(scanId);
      return applyOpenedScanResult(result, options);
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
      return null;
    }
  });

  const handleProgress = useEffectEvent((snapshot: ScanStatusSnapshot) => {
    startTransition(() => {
      setScanStatus(snapshot);
      if (snapshot.rootPath) {
        setRootPath((currentValue) => currentValue || snapshot.rootPath || "");
      }
      if (snapshot.state === "cancelled") {
        setNotice("Scan cancelled before history save.");
      }
      if (snapshot.state === "failed" && snapshot.message) {
        setErrorMessage(snapshot.message);
      }
    });

    if (
      snapshot.state === "running" &&
      snapshot.scanId &&
      ((pendingScanStartRef.current &&
        snapshot.rootPath &&
        snapshot.rootPath === pendingScanStartRef.current.rootPath) ||
        appliedWorkspaceSwitchesRef.current.has(
          buildWorkspaceSwitchKey("N1_START_SCAN", "scan", snapshot.scanId, "accepted"),
        ) ||
        appliedWorkspaceSwitchesRef.current.has(
          buildWorkspaceSwitchKey("N1_START_SCAN", "scan", snapshot.scanId, "running"),
        ))
    ) {
      applyContractualWorkspaceSwitch("scan", "N1_START_SCAN", snapshot.scanId, {
        phase: "running",
      });
    }

    if (snapshot.state === "completed" && snapshot.completedScanId) {
      pendingScanStartRef.current = null;
      const completedScanId = snapshot.completedScanId;
      const operationId = completedScanId;
      const completionSwitchKey = buildWorkspaceSwitchKey(
        "N2_SCAN_COMPLETED_AND_OPENED",
        "explorer",
        operationId,
      );
      if (appliedWorkspaceSwitchesRef.current.has(completionSwitchKey)) {
        workspaceShellLogger.log("workspace_auto_switch_skipped_duplicate", {
          reason: "N2_SCAN_COMPLETED_AND_OPENED",
          target: "explorer",
          operationId,
          phase: "default",
        });
        return;
      }

      void (async () => {
        const openedScan = await openStoredScan(completedScanId, {
          persistWorkspace: activeWorkspace,
        });
        if (openedScan) {
          applyContractualWorkspaceSwitch(
            "explorer",
            "N2_SCAN_COMPLETED_AND_OPENED",
            operationId,
          );
        }
      })();
      void loadHistory();
      void loadScanRuns();
    }

    if (snapshot.state === "cancelled" || snapshot.state === "failed") {
      pendingScanStartRef.current = null;
      void loadScanRuns();
    }
  });

  const handleDuplicateProgress = useEffectEvent((snapshot: DuplicateStatusSnapshot) => {
    startTransition(() => {
      setDuplicateStatus(snapshot);
      if (snapshot.state === "cancelled" && snapshot.message) {
        setNotice(snapshot.message);
        setErrorMessage(null);
      }
      if (snapshot.state === "failed" && snapshot.message) {
        setErrorMessage(snapshot.message);
      }
    });

    if (
      snapshot.state === "running" &&
      snapshot.scanId &&
      ((pendingDuplicateStartRef.current &&
        snapshot.scanId === pendingDuplicateStartRef.current.scanId) ||
        appliedWorkspaceSwitchesRef.current.has(
          buildWorkspaceSwitchKey(
            "N4_START_DUPLICATE_ANALYSIS",
            "duplicates",
            snapshot.analysisId ?? snapshot.scanId,
          ),
        ))
    ) {
      applyContractualWorkspaceSwitch(
        "duplicates",
        "N4_START_DUPLICATE_ANALYSIS",
        snapshot.analysisId ?? snapshot.scanId,
      );
    }

    if (snapshot.state === "completed" && snapshot.completedAnalysisId) {
      pendingDuplicateStartRef.current = null;
      if (duplicateAnalysis?.analysisId === snapshot.completedAnalysisId) {
        return;
      }

      void openStoredDuplicateAnalysis(snapshot.completedAnalysisId);
    }

    if (snapshot.state === "cancelled" || snapshot.state === "failed") {
      pendingDuplicateStartRef.current = null;
    }
  });

  useEffect(() => {
    let isActive = true;
    let unsubscribeScan = () => {};
    let unsubscribeDuplicate = () => {};
    startupResolutionPendingRef.current = true;
    manualWorkspaceDuringStartupRef.current = null;

    void (async () => {
      try {
        const [
          initialStatus,
          initialHistory,
          initialScanRuns,
          initialDuplicateStatus,
          restoreContextResult,
          initialCleanupRules,
          capability,
          scanUnlisten,
          duplicateUnlisten,
        ] = await Promise.all([
          client.getScanStatus(),
          client.listScanHistory(),
          client.listScanRuns(),
          client.getDuplicateAnalysisStatus(),
          client
            .getWorkspaceRestoreContext()
            .then<RestoreContextResult>((context) => ({ kind: "loaded", context }))
            .catch<RestoreContextResult>((error) => ({ kind: "error", error })),
          client.listCleanupRules(),
          client.getPrivilegedCleanupCapability(),
          client.subscribeToScanProgress((snapshot) => {
            if (isActive) {
              handleProgress(snapshot);
            }
          }),
          client.subscribeToDuplicateProgress((snapshot) => {
            if (isActive) {
              handleDuplicateProgress(snapshot);
            }
          }),
        ]);

        if (!isActive) {
          scanUnlisten();
          duplicateUnlisten();
          return;
        }

        unsubscribeScan = scanUnlisten;
        unsubscribeDuplicate = duplicateUnlisten;
        let restoreContext: WorkspaceRestoreContext | null = null;
        let startupNotice: string | null = null;
        let startupLoadedScan: CompletedScan | null = null;

        if (restoreContextResult.kind === "loaded") {
          restoreContext = restoreContextResult.context;
          workspaceRestoreContextRef.current = restoreContext;
          lastOpenedScanIdRef.current = restoreContext?.lastOpenedScanId ?? null;
        } else {
          startupNotice = "Saved workspace context could not be read. Opening Overview instead.";
          workspaceRestoreContextRef.current = null;
          lastOpenedScanIdRef.current = null;
          workspaceShellLogger.log("workspace_restore_context_load_failed", {
            message: describeError(restoreContextResult.error),
          });
        }

        if (restoreContext?.lastWorkspace === "explorer" && restoreContext.lastOpenedScanId) {
          try {
            startupLoadedScan = await client.openScanHistory(restoreContext.lastOpenedScanId);
          } catch (error) {
            startupNotice =
              "Saved Explorer context could not be restored. Opening Overview instead.";
            workspaceRestoreContextRef.current = null;
            lastOpenedScanIdRef.current = null;
            workspaceShellLogger.log("workspace_restore_context_validation_failed", {
              lastOpenedScanId: restoreContext.lastOpenedScanId,
              message: describeError(error),
            });
          }
        }

        if (!startupLoadedScan && initialStatus.state === "completed" && initialStatus.completedScanId) {
          try {
            startupLoadedScan = await client.openScanHistory(initialStatus.completedScanId);
          } catch (error) {
            if (isActive) {
              startTransition(() => {
                setErrorMessage(describeError(error));
              });
            }
          }
        }

        const initialWorkspace = resolveInitialWorkspace({
          scanStatus: initialStatus,
          duplicateStatus: initialDuplicateStatus,
          interruptedRuns: initialScanRuns,
          restoreContext,
          loadedScan: startupLoadedScan,
        });
        const startupWorkspace = manualWorkspaceDuringStartupRef.current ?? initialWorkspace;

        startTransition(() => {
          setScanStatus(initialStatus);
          setDuplicateStatus(initialDuplicateStatus);
          setHistory(initialHistory);
          setScanRuns(initialScanRuns);
          setCleanupRules(initialCleanupRules);
          setPrivilegedCapability(capability);
          setShellNotice(startupNotice);
          setActiveWorkspace(startupWorkspace);
          if (initialStatus.rootPath) {
            setRootPath(initialStatus.rootPath);
          }
        });

        if (startupLoadedScan) {
          applyOpenedScanResult(startupLoadedScan, {
            nextNotice: `Loaded ${startupLoadedScan.scanId} from local history.`,
            preservedDuplicateStatusSnapshot: initialDuplicateStatus,
            persistRestoreContext: false,
          });

          if (
            initialDuplicateStatus.state === "completed" &&
            initialDuplicateStatus.completedAnalysisId &&
            initialDuplicateStatus.scanId === startupLoadedScan.scanId
          ) {
            await openStoredDuplicateAnalysis(initialDuplicateStatus.completedAnalysisId);
          }
        }
        startupResolutionPendingRef.current = false;
      } catch (error) {
        if (isActive) {
          startTransition(() => {
            setErrorMessage(describeError(error));
          });
        }
        startupResolutionPendingRef.current = false;
      }
    })();

    return () => {
      isActive = false;
      unsubscribeScan();
      unsubscribeDuplicate();
    };
    // `useEffectEvent` keeps the handlers fresh without resubscribing on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [client]);

  async function handleStartScan(event: FormEvent) {
    event.preventDefault();

    if (scanStatus.state === "running") {
      setNotice("One scan at a time. Cancel the current scan before starting another.");
      return;
    }

    const normalizedRoot = rootPath.trim();
    if (!normalizedRoot) {
      setErrorMessage("Enter a folder or drive path before starting a scan.");
      return;
    }

    setIsSubmitting(true);
    setErrorMessage(null);
    setNotice(null);
    setCurrentScan(null);
    setCurrentExplorerPath(null);
    setExplorerSortMode("size");
    pendingScanStartRef.current = { rootPath: normalizedRoot };
    resetReviewState();

    try {
      const started = await client.startScan(normalizedRoot, { resumeEnabled });
      const startedAt = new Date().toISOString();
      startTransition(() => {
        setScanStatus((currentValue) => {
          if (currentValue.scanId === started.scanId && currentValue.state !== "idle") {
            return currentValue;
          }

          return {
            ...idleScanStatus,
            scanId: started.scanId,
            rootPath: normalizedRoot,
            state: "running",
            startedAt,
            updatedAt: startedAt,
            currentPath: normalizedRoot,
          };
        });
        setNotice((currentValue) => currentValue ?? `Scanning ${normalizedRoot}`);
      });
      applyContractualWorkspaceSwitch("scan", "N1_START_SCAN", started.scanId, {
        phase: "accepted",
        blockedByKeys: [
          buildWorkspaceSwitchKey("N1_START_SCAN", "scan", started.scanId, "running"),
        ],
      });
      void loadScanRuns();
    } catch (error) {
      pendingScanStartRef.current = null;
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
    } finally {
      pendingScanStartRef.current = null;
      setIsSubmitting(false);
    }
  }

  async function handleCancelScan() {
    if (scanStatus.state !== "running") {
      setNotice("No scan is running.");
      return;
    }

    try {
      await client.cancelActiveScan();
      setNotice("Cancellation requested.");
      setErrorMessage(null);
    } catch (error) {
      setErrorMessage(describeError(error));
    }
  }

  async function handleReopenScan(scanId: string) {
    setErrorMessage(null);
    const reopenedScan = await openStoredScan(scanId, {
      nextNotice: `Loaded ${scanId} from local history.`,
      persistWorkspace: "history",
    });
    if (reopenedScan) {
      applyContractualWorkspaceSwitch("explorer", "N3_OPEN_HISTORY_SCAN", scanId);
    }
  }

  async function handleResumeRun(run: ScanRunSummary) {
    if (!run.canResume) {
      setErrorMessage("Interrupted-run resume is unavailable for this run.");
      return;
    }

    setErrorMessage(null);
    setNotice(null);
    setCurrentScan(null);
    setCurrentExplorerPath(null);
    setExplorerSortMode("size");
    resetReviewState();

    try {
      const resumed = await client.resumeScanRun(run.header.runId);
      const resumedAt = new Date().toISOString();
      startTransition(() => {
        setRootPath(run.header.rootPath);
        setScanStatus({
          ...idleScanStatus,
          scanId: resumed.runId,
          rootPath: run.header.rootPath,
          state: "running",
          startedAt: resumedAt,
          updatedAt: resumedAt,
          currentPath: run.latestSnapshot.currentPath ?? run.header.rootPath,
          message: `Resuming ${run.header.runId}`,
        });
        setNotice(`Resuming ${run.header.rootPath}`);
      });
      void loadScanRuns();
    } catch (error) {
      setErrorMessage(describeError(error));
    }
  }

  async function handleCancelRun(runId: string) {
    try {
      await client.cancelScanRun(runId);
      setNotice(`Cancelled interrupted run ${runId}.`);
      setErrorMessage(null);
      void loadScanRuns();
    } catch (error) {
      setErrorMessage(describeError(error));
    }
  }

  async function handleOpenInExplorer(path: string) {
    try {
      await client.openPathInExplorer(path);
      setErrorMessage(null);
    } catch (error) {
      setErrorMessage(describeError(error));
    }
  }

  async function handleStartDuplicateAnalysis() {
    if (!currentScan) {
      setNotice("Load a stored scan before starting duplicate analysis.");
      return;
    }

    if (!hasFileEntries(currentScan)) {
      setNotice("A fresh scan is required before duplicate analysis.");
      return;
    }

    if (duplicateStatus.state === "running") {
      setNotice("Duplicate analysis is already running.");
      return;
    }

    setErrorMessage(null);
    setNotice("Running duplicate analysis on the current scan.");
    setDuplicateAnalysis(null);
    setDuplicateKeepSelections({});
    setExpandedDuplicateGroups({});
    pendingDuplicateStartRef.current = { scanId: currentScan.scanId };
    resetCleanupState();
    setDuplicateStatus({
      analysisId: null,
      scanId: currentScan.scanId,
      state: "running",
      stage: "grouping",
      itemsProcessed: 0,
      groupsEmitted: 0,
      message: null,
      completedAnalysisId: null,
    });

    try {
      const started = await client.startDuplicateAnalysis(currentScan.scanId);
      startTransition(() => {
        setDuplicateStatus((currentValue) => ({
          analysisId: started.analysisId,
          scanId: currentScan.scanId,
          state: currentValue.state,
          stage: currentValue.stage,
          itemsProcessed: currentValue.itemsProcessed,
          groupsEmitted: currentValue.groupsEmitted,
          message: currentValue.message,
          completedAnalysisId: currentValue.completedAnalysisId,
        }));
      });
      applyContractualWorkspaceSwitch(
        "duplicates",
        "N4_START_DUPLICATE_ANALYSIS",
        started.analysisId,
      );
    } catch (error) {
      pendingDuplicateStartRef.current = null;
      startTransition(() => {
        setDuplicateStatus(idleDuplicateStatus);
        setErrorMessage(describeError(error));
      });
    }
  }

  async function handleCancelDuplicateAnalysis() {
    if (duplicateStatus.state !== "running") {
      setNotice("No duplicate analysis is running.");
      return;
    }

    try {
      await client.cancelDuplicateAnalysis();
      setNotice("Duplicate analysis cancellation requested.");
      setErrorMessage(null);
    } catch (error) {
      setErrorMessage(describeError(error));
    }
  }

  function handleBrowsePath(path: string) {
    if (!currentScan || !hasBrowseableEntries(currentScan)) {
      return;
    }

    if (!isPathWithinRoot(path, currentScan.rootPath)) {
      setErrorMessage("Folder browsing is limited to the current scan root.");
      return;
    }

    setCurrentExplorerPath(path);
    setErrorMessage(null);
  }

  function handleSelectKeepPath(group: DuplicateGroup, path: string) {
    if (!group.members.some((member) => member.path === path)) {
      return;
    }

    setDuplicateKeepSelections((currentValue) => ({
      ...currentValue,
      [group.groupId]: path,
    }));
    setCleanupPreview(null);
    setCleanupExecutionState(null);
    setPermanentDeleteConfirmed(false);
  }

  function handleToggleDuplicateGroup(groupId: string) {
    setExpandedDuplicateGroups((currentValue) => ({
      ...currentValue,
      [groupId]: !currentValue[groupId],
    }));
  }

  function handleToggleCleanupRule(ruleId: string) {
    setSelectedCleanupRuleIds((currentValue) =>
      currentValue.includes(ruleId)
        ? currentValue.filter((currentRuleId) => currentRuleId !== ruleId)
        : [...currentValue, ruleId],
    );
    setCleanupPreview(null);
    setCleanupExecutionState(null);
    setPermanentDeleteConfirmed(false);
  }

  async function handleRefreshCleanupPreview() {
    if (!currentScan) {
      setNotice("Load a stored scan before cleanup preview.");
      return;
    }

    if (!hasFileEntries(currentScan)) {
      setNotice("A fresh scan is required before cleanup preview.");
      return;
    }

    const duplicateDeletePaths = buildDuplicateDeletePaths(
      duplicateAnalysis,
      duplicateKeepSelections,
    );
    if (duplicateDeletePaths.length === 0 && selectedCleanupRuleIds.length === 0) {
      setNotice("Select at least one cleanup source before previewing.");
      return;
    }

    try {
      const preview = await client.previewCleanup({
        scanId: currentScan.scanId,
        duplicateDeletePaths,
        enabledRuleIds: selectedCleanupRuleIds,
      });
      startTransition(() => {
        setCleanupPreview(preview);
        setCleanupExecutionState(null);
        setPermanentDeleteConfirmed(false);
        setNotice(
          preview.candidates.length === 0
            ? "No valid cleanup candidates remain after validation."
            : "Cleanup preview refreshed.",
        );
        setErrorMessage(null);
      });
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
    }
  }

  async function handleExecuteCleanup(mode: CleanupExecutionMode) {
    if (!cleanupPreview) {
      setNotice("Refresh cleanup preview before executing cleanup.");
      return;
    }

    if (cleanupPreview.candidates.length === 0) {
      setNotice("No valid cleanup candidates remain after validation.");
      return;
    }

    try {
      const result = await client.executeCleanup({
        previewId: cleanupPreview.previewId,
        actionIds: cleanupPreview.candidates.map((candidate) => candidate.actionId),
        mode,
      });
      startTransition(() => {
        setCleanupExecutionState({
          scanId: cleanupPreview.scanId,
          result,
        });
        setNotice("Cleanup execution finished.");
        setErrorMessage(null);
      });
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
    }
  }

  const browseableScan = currentScan && hasBrowseableEntries(currentScan) ? currentScan : null;
  const duplicateEligible = currentScan ? hasFileEntries(currentScan) : false;
  const duplicateDeletePaths = buildDuplicateDeletePaths(
    duplicateAnalysis,
    duplicateKeepSelections,
  );
  const resolvedExplorerPath =
    browseableScan &&
    currentExplorerPath &&
    isPathWithinRoot(currentExplorerPath, browseableScan.rootPath)
      ? currentExplorerPath
      : browseableScan?.rootPath ?? null;
  const explorerBreadcrumbs =
    browseableScan && resolvedExplorerPath
      ? buildBreadcrumbs(browseableScan, resolvedExplorerPath)
      : [];
  const visibleEntries =
    browseableScan && resolvedExplorerPath
      ? listVisibleEntries(browseableScan, resolvedExplorerPath, explorerSortMode)
      : [];
  const currentLevelTotal = visibleEntries.reduce((sum, entry) => sum + entry.sizeBytes, 0);
  const duplicatePreview = duplicateAnalysis
    ? summarizeDuplicatePreview(duplicateAnalysis, duplicateKeepSelections)
    : { filesMarkedForDeletion: 0, reclaimableBytes: 0 };
  const cleanupExecutionResult = cleanupExecutionState?.result ?? null;
  const orderedDuplicateGroups = duplicateAnalysis
    ? sortDuplicateGroups(duplicateAnalysis.groups)
    : [];
  const isScanRunning = scanStatus.state === "running";
  const activeScanRoot =
    (scanStatus.rootPath ?? rootPath.trim()) || "Waiting for scan root";
  const activeScanPath = scanStatus.currentPath ?? scanStatus.rootPath;
  const activeScanHeartbeat = scanStatus.updatedAt
    ? formatTimestamp(scanStatus.updatedAt)
    : "Waiting for the first live update.";
  const activeScanStarted = scanStatus.startedAt ? formatTimestamp(scanStatus.startedAt) : null;
  const activeScanElapsed = formatElapsedWindow(scanStatus.startedAt, scanStatus.updatedAt);
  const normalizedHistoryRootFilter = historyRootFilter.trim().toLowerCase();
  const normalizedHistoryScanIdFilter = historyScanIdFilter.trim().toLowerCase();
  const visibleHistory = sortHistoryEntries(history).filter((entry) => {
    const matchesRoot =
      normalizedHistoryRootFilter.length === 0 ||
      entry.rootPath.toLowerCase().includes(normalizedHistoryRootFilter);
    const matchesScanId =
      normalizedHistoryScanIdFilter.length === 0 ||
      entry.scanId.toLowerCase().includes(normalizedHistoryScanIdFilter);
    return matchesRoot && matchesScanId;
  });
  const interruptedRuns = [...scanRuns]
    .filter((run) => run.header.status === "stale" || run.header.status === "abandoned")
    .sort(
      (left, right) =>
        getSortableTimestamp(right.header.lastSnapshotAt) -
        getSortableTimestamp(left.header.lastSnapshotAt),
    );
  const cleanupPreviewAvailable = Boolean(
    currentScan &&
      hasFileEntries(currentScan) &&
      (duplicateDeletePaths.length > 0 || selectedCleanupRuleIds.length > 0),
  );
  const globalStatus = deriveGlobalStatus({
    scanStatus,
    duplicateStatus,
    currentScan,
    interruptedRuns,
    duplicateEligible,
    browseableScan: Boolean(browseableScan),
    cleanupPreview,
    cleanupExecutionResult,
    cleanupExecutionScanId: cleanupExecutionState?.scanId ?? null,
    cleanupPreviewAvailable,
  });
  const activeWorkspaceDefinition =
    workspaceDefinitions.find((definition) => definition.value === activeWorkspace) ??
    workspaceDefinitions[0]!;
  const nextSafeAction = globalStatus.nextSafeAction;
  const nextSafeActionTarget = nextSafeAction?.target ?? null;
  const nextSafeActionText = nextSafeAction?.label ?? null;
  const nextSafeActionLabel =
    nextSafeActionText ??
    globalStatus.noActionLabel ??
    "No safe next action right now.";
  const activeLiveTaskNotice =
    scanStatus.state === "running"
      ? "A scan is running. Review progress in Scan."
      : duplicateStatus.state === "running" &&
          currentScan &&
          duplicateStatus.scanId === currentScan.scanId
        ? "Duplicate analysis is running for the loaded scan. Review progress in Duplicates."
        : null;
  const shellNoticeEntries = [
    ...(activeLiveTaskNotice
      ? [
          {
            key:
              scanStatus.state === "running"
                ? "live-scan-running"
                : `live-duplicate-running:${duplicateStatus.analysisId ?? duplicateStatus.scanId ?? "unknown"}`,
            kind: "live_task" as const,
            message: activeLiveTaskNotice,
          },
        ]
      : []),
    ...(interruptedRuns.length > 0
      ? [
          {
            key: "interrupted-runs-attention",
            kind: "interrupted_runs" as const,
            message: "Interrupted runs need review in History.",
          },
        ]
      : []),
    ...([scanStatus.message, notice, shellNotice]
      .filter((value): value is string => Boolean(value))
      .map((message, index) => ({
        key: `status-${index}:${message}`,
        kind: "status" as const,
        message,
      })) as ShellNoticeEntry[]),
  ];
  const shellNotices = Array.from(
    new Map(shellNoticeEntries.map((entry) => [entry.key, entry])),
  ).map(([, entry]) => entry);

  useEffect(() => {
    const nextActionKey = nextSafeActionText
      ? `${nextSafeActionText}:${nextSafeActionTarget}:${globalStatus.primaryStateLabel}`
      : `no-action:${globalStatus.noActionLabel ?? "none"}:${globalStatus.primaryStateLabel}`;
    if (lastNextSafeActionKeyRef.current === nextActionKey) {
      return;
    }

    lastNextSafeActionKeyRef.current = nextActionKey;
    workspaceShellLogger.log("workspace_next_safe_action_selected", {
      label: nextSafeActionText,
      noActionLabel: nextSafeActionText ? null : globalStatus.noActionLabel,
      primaryStateLabel: globalStatus.primaryStateLabel,
      target: nextSafeActionTarget,
    });
  }, [
    globalStatus.noActionLabel,
    globalStatus.primaryStateLabel,
    nextSafeActionTarget,
    nextSafeActionText,
  ]);

  useEffect(() => {
    const previousKeys = visibleShellNoticeKeysRef.current;
    const nextKeys = new Set(shellNotices.map((entry) => entry.key));

    for (const entry of shellNotices) {
      if (previousKeys.has(entry.key)) {
        continue;
      }

      workspaceShellLogger.log("workspace_status_notice_rendered", {
        kind: entry.kind,
        message: entry.message,
        noticeKey: entry.key,
      });
    }

    visibleShellNoticeKeysRef.current = nextKeys;
  }, [shellNotices]);

  function handleGlobalStatusAction() {
    if (!nextSafeAction) {
      return;
    }

    const navigationReason = getNextSafeActionReason(nextSafeAction);
    if (navigationReason === "N5_REQUEST_CLEANUP_PREVIEW") {
      applyContractualWorkspaceSwitch("cleanup", navigationReason, currentScan?.scanId ?? null);
      return;
    }

    if (navigationReason === "N6_REVIEW_INTERRUPTED_RUNS") {
      applyContractualWorkspaceSwitch(
        "history",
        navigationReason,
        interruptedRuns[0]?.header.runId ?? null,
      );
      return;
    }

    activateWorkspace(nextSafeAction.target);
  }

  function renderOverviewPanel() {
    return (
      <section
        className="panel workspace-panel"
        role="tabpanel"
        id="workspace-panel-overview"
        aria-labelledby="workspace-tab-overview"
      >
        <div className="panel-header">
          <h2>Overview</h2>
          <p>
            Keep the current loaded result, live work, interrupted runs, and the next
            safe action visible while you move between the major workflows.
          </p>
        </div>

        <div className="result-summary">
          <article className="summary-card">
            <span>Global state</span>
            <strong>{globalStatus.primaryStateLabel}</strong>
            <p className="current-path">{globalStatus.contextLabel}</p>
          </article>
          <article className="summary-card">
            <span>Loaded scan</span>
            <strong>{currentScan ? currentScan.scanId : "No loaded scan"}</strong>
            <p className="current-path">
              {currentScan ? currentScan.rootPath : "Use Scan or History to load a result."}
            </p>
          </article>
          <article className="summary-card">
            <span>Interrupted runs</span>
            <strong>{interruptedRuns.length}</strong>
            <p className="current-path">
              {interruptedRuns.length > 0
                ? "Recovery-facing scan runs remain visible in History."
                : "No interrupted runs currently need review."}
            </p>
          </article>
          <article className="summary-card">
            <span>Next safe action</span>
            <strong>{nextSafeActionLabel}</strong>
            <p className="current-path">{activeWorkspaceDefinition.description}</p>
          </article>
        </div>

        <div className="results-columns">
          <section className="result-card">
            <h3>Current task context</h3>
            <ul>
              <li>
                <span>Active workspace</span>
                <strong>{activeWorkspaceDefinition.label}</strong>
              </li>
              <li>
                <span>History entries</span>
                <strong>{history.length} stored scans</strong>
              </li>
              <li>
                <span>Duplicate review</span>
                <strong>
                  {duplicateAnalysis
                    ? `${duplicateAnalysis.groups.length} verified groups loaded`
                    : "No duplicate review is loaded"}
                </strong>
              </li>
            </ul>
          </section>

          <section className="result-card">
            <h3>Loaded result summary</h3>
            {currentScan ? (
              <ul>
                <li>
                  <span>Root path</span>
                  <strong>{currentScan.rootPath}</strong>
                </li>
                <li>
                  <span>Files</span>
                  <strong>{currentScan.totalFiles}</strong>
                </li>
                <li>
                  <span>Total bytes</span>
                  <strong>{formatBytes(currentScan.totalBytes)}</strong>
                </li>
              </ul>
            ) : (
              <p className="empty-state">
                No completed scan is loaded yet. Start a scan or reopen one from local
                history.
              </p>
            )}
          </section>
        </div>

        <section className="result-card">
          <div className="panel-header compact-header">
            <h3>Safety highlights</h3>
            <p>Cleanup remains review-first, local-only, and explicit by design.</p>
          </div>
          <div className="safety-grid">
            {safetyPrinciples.map((principle) => (
              <article className="principle-card" key={principle.title}>
                <h3>{principle.title}</h3>
                <p>{principle.body}</p>
              </article>
            ))}
          </div>
        </section>
      </section>
    );
  }

  function renderScanPanel() {
    return (
      <section
        className="panel workspace-panel"
        role="tabpanel"
        id="workspace-panel-scan"
        aria-labelledby="workspace-tab-scan"
      >
        <div className="panel-header">
          <h2>Scan</h2>
          <p>
            Start a scan, monitor live progress, and keep the running-scan state
            distinct from any previously loaded completed result.
          </p>
        </div>

        <section className="result-card">
          <div className="panel-header compact-header">
            <h3>Scan workspace</h3>
            <p>
              Point the scanner at a Windows folder or drive path. Progress stays
              visible while results are prepared for local history storage.
            </p>
          </div>

          <form className="scan-form" onSubmit={handleStartScan}>
            <label className="field">
              <span>Scan root</span>
              <input
                name="scan-root"
                type="text"
                value={rootPath}
                onChange={(event) => setRootPath(event.target.value)}
                placeholder="C:\\Users\\xiongxianfei\\Downloads"
              />
            </label>
            <details className="field">
              <summary>Advanced scan options</summary>
              <label>
                <input
                  type="checkbox"
                  checked={resumeEnabled}
                  onChange={(event) => setResumeEnabled(event.target.checked)}
                />{" "}
                Enable interrupted-run resume
              </label>
            </details>
            <div className="action-row">
              <button type="submit" className="primary-button" disabled={isSubmitting}>
                Start scan
              </button>
              <button type="button" className="secondary-button" onClick={handleCancelScan}>
                Cancel scan
              </button>
            </div>
          </form>

          <div className="status-strip" aria-live="polite">
            <div>
              <span className="status-label">State</span>
              <strong>{scanStatus.state}</strong>
            </div>
            <div>
              <span className="status-label">Files</span>
              <strong>{scanStatus.filesDiscovered}</strong>
            </div>
            <div>
              <span className="status-label">Directories</span>
              <strong>{scanStatus.directoriesDiscovered}</strong>
            </div>
            <div>
              <span className="status-label">Progress</span>
              <strong>{formatBytes(scanStatus.bytesProcessed)} processed</strong>
            </div>
          </div>
        </section>

        {isScanRunning ? (
          <section className="result-card active-scan-panel" aria-live="polite">
            <div className="panel-header compact-header">
              <h3>Active scan</h3>
              <p>
                Progress stays indeterminate while Space Sift discovers more files and
                folders. Previous completed scans stay in History until this run
                finishes.
              </p>
            </div>

            <div className="active-scan-grid">
              <article className="summary-card">
                <span>Scan root</span>
                <strong>{activeScanRoot}</strong>
              </article>
              <article className="summary-card">
                <span>Current activity</span>
                <strong>
                  {activeScanPath ? getPathLabel(activeScanPath) : "Waiting for path context"}
                </strong>
                <p className="current-path active-scan-path">
                  {activeScanPath ?? "The scanner has not emitted a narrower path yet."}
                </p>
              </article>
              <article className="summary-card">
                <span>Started</span>
                <strong>{activeScanStarted ?? "Starting scan"}</strong>
                <p className="active-scan-meta">
                  {activeScanElapsed ?? "Elapsed time will appear after progress updates."}
                </p>
              </article>
              <article className="summary-card">
                <span>Last update</span>
                <strong>{activeScanHeartbeat}</strong>
                <p className="active-scan-meta">
                  Live progress is emitted at a bounded cadence.
                </p>
              </article>
            </div>

            <div className="active-scan-toolbar">
              <div className="result-summary">
                <article className="summary-card">
                  <span>State</span>
                  <strong>{scanStatus.state}</strong>
                </article>
                <article className="summary-card">
                  <span>Files discovered</span>
                  <strong>{scanStatus.filesDiscovered}</strong>
                </article>
                <article className="summary-card">
                  <span>Directories discovered</span>
                  <strong>{scanStatus.directoriesDiscovered}</strong>
                </article>
                <article className="summary-card">
                  <span>Bytes processed</span>
                  <strong>{formatBytes(scanStatus.bytesProcessed)} processed</strong>
                </article>
              </div>

              <div className="action-row">
                <button type="button" className="secondary-button" onClick={handleCancelScan}>
                  Cancel scan
                </button>
              </div>
            </div>
          </section>
        ) : (
          <section className="result-card">
            <h3>Current scan context</h3>
            <p className="empty-state">
              No live scan is currently running. Use Start scan to create a new active
              scan session.
            </p>
          </section>
        )}
      </section>
    );
  }

  function renderHistoryPanel() {
    return (
      <section
        className="panel workspace-panel"
        role="tabpanel"
        id="workspace-panel-history"
        aria-labelledby="workspace-tab-history"
      >
        <div className="panel-header">
          <h2>History</h2>
          <p>
            Reopen stored scans from local history and review interrupted runs without
            guessing from session-only state.
          </p>
        </div>

        <section className="result-card">
          <div className="panel-header compact-header">
            <h3>Recent scans</h3>
            <p>Completed scan results stay local so you can reopen them without rescanning.</p>
          </div>

          {history.length === 0 ? (
            <p className="empty-state">No completed scans are stored yet.</p>
          ) : (
            <>
              <div className="history-filters">
                <div className="history-filter-grid">
                  <label className="field">
                    <span>Filter by root path</span>
                    <input
                      type="text"
                      value={historyRootFilter}
                      onChange={(event) => setHistoryRootFilter(event.target.value)}
                      placeholder="C:\\Users\\xiongxianfei\\Downloads"
                    />
                  </label>
                  <label className="field">
                    <span>Filter by scan ID</span>
                    <input
                      type="text"
                      value={historyScanIdFilter}
                      onChange={(event) => setHistoryScanIdFilter(event.target.value)}
                      placeholder="scan-2"
                    />
                  </label>
                </div>
              </div>

              {visibleHistory.length === 0 ? (
                <p className="empty-state">No saved scans match the current filters.</p>
              ) : (
                <ul className="history-list">
                  {visibleHistory.map((entry) => {
                    const isLoadedResult = currentScan?.scanId === entry.scanId;

                    return (
                      <li
                        key={entry.scanId}
                        className={`history-entry ${isLoadedResult ? "is-loaded" : ""}`}
                        aria-current={isLoadedResult ? "true" : undefined}
                      >
                        <div className="history-meta">
                          <div className="history-title-row">
                            <strong>{entry.rootPath}</strong>
                            {isLoadedResult ? (
                              <span className="history-badge">Loaded result</span>
                            ) : null}
                          </div>
                          <span>{formatTimestamp(entry.completedAt)}</span>
                          <span>{formatBytes(entry.totalBytes)}</span>
                          <span className="history-id">Scan ID: {entry.scanId}</span>
                        </div>
                        <button
                          type="button"
                          className="secondary-button"
                          onClick={() => void handleReopenScan(entry.scanId)}
                        >
                          Reopen scan {entry.scanId}
                        </button>
                      </li>
                    );
                  })}
                </ul>
              )}
            </>
          )}
        </section>

        <section className="result-card">
          <div className="panel-header compact-header">
            <h3>Interrupted runs</h3>
            <p>Recovered runs stay visible until you resume them or cancel them.</p>
          </div>

          {interruptedRuns.length > 0 ? (
            <ul className="history-list">
              {interruptedRuns.map((run) => (
                <li key={run.header.runId} className="history-entry">
                  <div className="history-meta">
                    <div className="history-title-row">
                      <strong>{run.header.rootPath}</strong>
                      <span className="history-badge">{run.header.status}</span>
                      {run.hasResume ? (
                        <span className="history-badge">
                          {run.canResume ? "Resume available" : "Resume unavailable"}
                        </span>
                      ) : null}
                    </div>
                    <span>Created {formatTimestamp(run.createdAt)}</span>
                    <span>{formatTimestamp(run.header.lastSnapshotAt)}</span>
                    <span>Seq {run.seq}</span>
                    <span>{run.itemsScanned} items scanned</span>
                    <span>{run.errorsCount} errors</span>
                    <span>
                      {run.progressPercent == null
                        ? "Progress pending"
                        : `${Math.round(run.progressPercent)}% progress`}
                    </span>
                    <span>{run.scanRateItemsPerSec.toFixed(1)} items/s</span>
                    <span className="history-id">Run ID: {run.header.runId}</span>
                  </div>
                  <div className="action-row">
                    {run.hasResume ? (
                      <button
                        type="button"
                        className="secondary-button"
                        disabled={!run.canResume || scanStatus.state === "running"}
                        onClick={() => void handleResumeRun(run)}
                      >
                        Resume run {run.header.runId}
                      </button>
                    ) : null}
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => void handleCancelRun(run.header.runId)}
                    >
                      Cancel run {run.header.runId}
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p className="empty-state">No interrupted runs currently need review.</p>
          )}
        </section>
      </section>
    );
  }

  function renderExplorerPanel() {
    return (
      <section
        className="panel workspace-panel"
        role="tabpanel"
        id="workspace-panel-explorer"
        aria-labelledby="workspace-tab-explorer"
      >
        <div className="panel-header">
          <h2>Explorer</h2>
          <p>
            Browse the currently loaded result, reopen the stored root in Explorer, and
            keep summary-only scans in a clean degraded state.
          </p>
        </div>

        {currentScan ? (
          <>
            <section className="result-card">
              <div className="panel-header compact-header">
                <h3>Current result</h3>
                <p>
                  Loaded scan {currentScan.scanId} from {currentScan.rootPath}. Review the
                  stored result without rescanning.
                </p>
              </div>

              {browseableScan && resolvedExplorerPath ? (
                <div className="explorer-grid">
                  <section className="result-card explorer-card">
                    <div className="explorer-header">
                      <div>
                        <h3>Results explorer</h3>
                        <p className="explorer-note">
                          The current folder view is read-only, reflects the stored scan
                          result without rescanning, and shows each row&apos;s share of the
                          current level.
                        </p>
                      </div>
                      <div className="sort-controls" aria-label="Sort controls">
                        <button
                          type="button"
                          className={`secondary-button ${explorerSortMode === "size" ? "is-active" : ""}`}
                          onClick={() => setExplorerSortMode("size")}
                        >
                          Sort by size
                        </button>
                        <button
                          type="button"
                          className={`secondary-button ${explorerSortMode === "name" ? "is-active" : ""}`}
                          onClick={() => setExplorerSortMode("name")}
                        >
                          Sort by name
                        </button>
                      </div>
                    </div>

                    <div className="breadcrumbs" aria-label="Current folder breadcrumbs">
                      {explorerBreadcrumbs.map((breadcrumb, index) => (
                        <span className="breadcrumb-segment" key={breadcrumb.path}>
                          {index > 0 ? <span className="breadcrumb-divider">\</span> : null}
                          <button
                            type="button"
                            className="breadcrumb-button"
                            onClick={() => handleBrowsePath(breadcrumb.path)}
                          >
                            {breadcrumb.label}
                          </button>
                        </span>
                      ))}
                    </div>

                    <div className="explorer-toolbar">
                      <div>
                        <span className="status-label">Current location</span>
                        <p className="current-path">{resolvedExplorerPath}</p>
                      </div>
                      <button
                        type="button"
                        className="primary-button"
                        onClick={() => void handleOpenInExplorer(resolvedExplorerPath)}
                      >
                        Open current path in Explorer
                      </button>
                    </div>

                    {visibleEntries.length === 0 ? (
                      <p className="empty-state">
                        This folder has no immediate children in the stored scan result.
                      </p>
                    ) : (
                      <table className="results-table" aria-label="Current folder contents">
                        <thead>
                          <tr>
                            <th scope="col">Name</th>
                            <th scope="col">Type</th>
                            <th scope="col">Size</th>
                            <th scope="col">Usage</th>
                            <th scope="col">Actions</th>
                          </tr>
                        </thead>
                        <tbody>
                          {visibleEntries.map((entry) => {
                            const label = getPathLabel(entry.path);
                            const widthPercent =
                              currentLevelTotal === 0
                                ? 0
                                : Math.max((entry.sizeBytes / currentLevelTotal) * 100, 6);
                            const sharePercent =
                              currentLevelTotal === 0
                                ? 0
                                : Math.round((entry.sizeBytes / currentLevelTotal) * 100);

                            return (
                              <tr key={entry.path}>
                                <td>
                                  <strong>{label}</strong>
                                </td>
                                <td className="entry-kind">{entry.kind}</td>
                                <td>{formatBytes(entry.sizeBytes)}</td>
                                <td>
                                  <div className="usage-cell">
                                    <div className="usage-track" aria-hidden="true">
                                      <div
                                        className="usage-bar"
                                        style={{ width: `${Math.min(widthPercent, 100)}%` }}
                                      />
                                    </div>
                                    <span className="usage-label">
                                      {sharePercent}% of current level
                                    </span>
                                  </div>
                                </td>
                                <td>
                                  <div className="table-actions">
                                    {entry.kind === "directory" ? (
                                      <button
                                        type="button"
                                        className="secondary-button"
                                        aria-label={`Browse ${label}`}
                                        onClick={() => handleBrowsePath(entry.path)}
                                      >
                                        Browse folder
                                      </button>
                                    ) : null}
                                    <button
                                      type="button"
                                      className="secondary-button"
                                      aria-label={`Open ${label} in Explorer`}
                                      onClick={() => void handleOpenInExplorer(entry.path)}
                                    >
                                      Open in Explorer
                                    </button>
                                  </div>
                                </td>
                              </tr>
                            );
                          })}
                        </tbody>
                      </table>
                    )}
                  </section>
                </div>
              ) : (
                <p className="notice-banner compatibility-note">
                  This saved result was saved before folder browsing support. Run a fresh
                  scan to browse folders again.
                </p>
              )}
            </section>

            <div className="results-columns">
              {!browseableScan ? (
                <section className="result-card">
                  <h3>Largest files</h3>
                  <ul>
                    {currentScan.largestFiles.map((item) => (
                      <li key={item.path}>
                        <span>{item.path}</span>
                        <strong>{formatBytes(item.sizeBytes)}</strong>
                      </li>
                    ))}
                  </ul>
                </section>
              ) : null}

              {!browseableScan ? (
                <section className="result-card">
                  <h3>Largest directories</h3>
                  <ul>
                    {currentScan.largestDirectories.map((item) => (
                      <li key={item.path}>
                        <span>{item.path}</span>
                        <strong>{formatBytes(item.sizeBytes)}</strong>
                      </li>
                    ))}
                  </ul>
                </section>
              ) : null}

              <section className="result-card">
                <h3>Skipped paths</h3>
                {currentScan.skippedPaths.length === 0 ? (
                  <p className="empty-state">No skipped paths were recorded for this scan.</p>
                ) : (
                  <ul>
                    {currentScan.skippedPaths.map((item) => (
                      <li key={`${item.path}-${item.reasonCode}`}>
                        <span>{item.path}</span>
                        <strong>{item.reasonCode}</strong>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            </div>
          </>
        ) : (
          <section className="result-card">
            <p className="empty-state">
              Start a scan or reopen a stored result to populate Explorer.
            </p>
          </section>
        )}
      </section>
    );
  }

  function renderDuplicatesPanel() {
    return (
      <section
        className="panel workspace-panel"
        role="tabpanel"
        id="workspace-panel-duplicates"
        aria-labelledby="workspace-tab-duplicates"
      >
        <div className="panel-header">
          <h2>Duplicates</h2>
          <p>
            Run duplicate analysis for the loaded scan, keep review state local, and keep
            cleanup separate until preview is explicitly requested.
          </p>
        </div>

        <section className="result-card duplicate-card">
          <div className="panel-header compact-header">
            <h3>Duplicate analysis</h3>
            <p>
              Fully verified groups require a size match plus content verification.
              Cleanup stays separate until you explicitly build a cleanup preview.
            </p>
          </div>

          {!currentScan ? (
            <p className="notice-banner">
              Load a stored scan before starting duplicate analysis.
            </p>
          ) : !duplicateEligible ? (
            <p className="notice-banner duplicate-note">
              A fresh scan is required before duplicate analysis because this saved result
              does not include file-entry data.
            </p>
          ) : (
            <div className="duplicate-layout">
              <div className="duplicate-toolbar">
                <div>
                  <span className="status-label">Analysis state</span>
                  <p className="current-path">{duplicateStatus.state}</p>
                </div>
                <div className="action-row">
                  <button
                    type="button"
                    className="primary-button"
                    onClick={() => void handleStartDuplicateAnalysis()}
                    disabled={duplicateStatus.state === "running"}
                  >
                    Analyze duplicates
                  </button>
                  {duplicateStatus.state === "running" ? (
                    <button
                      type="button"
                      className="secondary-button"
                      onClick={() => void handleCancelDuplicateAnalysis()}
                    >
                      Cancel analysis
                    </button>
                  ) : null}
                </div>
              </div>

              {duplicateStatus.state === "running" ? (
                <div className="duplicate-progress" aria-live="polite">
                  <span>{getDuplicateStageLabel(duplicateStatus.stage)}</span>
                  <strong>{duplicateStatus.itemsProcessed} items processed</strong>
                </div>
              ) : null}

              {duplicateAnalysis ? (
                <>
                  <div className="duplicate-summary-grid">
                    <article className="summary-card">
                      <span>Duplicate groups</span>
                      <strong>{duplicateAnalysis.groups.length} duplicate groups</strong>
                    </article>
                    <article className="summary-card">
                      <span>Marked for later deletion</span>
                      <strong>
                        {duplicatePreview.filesMarkedForDeletion} files marked for later
                        deletion
                      </strong>
                    </article>
                    <article className="summary-card">
                      <span>Reclaimable bytes</span>
                      <strong>{formatBytes(duplicatePreview.reclaimableBytes)}</strong>
                    </article>
                  </div>

                  {duplicateAnalysis.groups.length === 0 ? (
                    <p className="empty-state">
                      No fully verified duplicate groups remain in this scan result.
                    </p>
                  ) : (
                    <div className="duplicate-groups" role="list" aria-label="Duplicate groups">
                      {orderedDuplicateGroups.map((group) => {
                        const keptPath = resolveKeptPath(group, duplicateKeepSelections);
                        const newestPath = chooseMemberByAge(group.members, "newest");
                        const oldestPath = chooseMemberByAge(group.members, "oldest");
                        const isExpanded = Boolean(expandedDuplicateGroups[group.groupId]);
                        const membersRegionId = `duplicate-members-${group.groupId}`;

                        return (
                          <article
                            className="duplicate-group"
                            key={group.groupId}
                            data-testid={`duplicate-group-${group.groupId}`}
                            role="listitem"
                          >
                            <div className="duplicate-group-header">
                              <div className="duplicate-group-summary">
                                <h4>{group.members.length} verified copies</h4>
                                <p>
                                  {formatBytes(group.sizeBytes)} each and{" "}
                                  {formatBytes(group.reclaimableBytes)} maximum reclaimable
                                </p>
                              </div>
                              <div className="duplicate-actions">
                                <button
                                  type="button"
                                  className="secondary-button"
                                  aria-expanded={isExpanded}
                                  aria-controls={membersRegionId}
                                  onClick={() => handleToggleDuplicateGroup(group.groupId)}
                                >
                                  {isExpanded ? "Hide details" : "Show details"}
                                </button>
                                <button
                                  type="button"
                                  className={`secondary-button ${keptPath === newestPath ? "is-active" : ""}`}
                                  aria-pressed={keptPath === newestPath}
                                  onClick={() => handleSelectKeepPath(group, newestPath)}
                                >
                                  Keep newest
                                </button>
                                <button
                                  type="button"
                                  className={`secondary-button ${keptPath === oldestPath ? "is-active" : ""}`}
                                  aria-pressed={keptPath === oldestPath}
                                  onClick={() => handleSelectKeepPath(group, oldestPath)}
                                >
                                  Keep oldest
                                </button>
                              </div>
                            </div>

                            {isExpanded ? (
                              <div className="duplicate-members" id={membersRegionId} role="list">
                                {group.members.map((member) => {
                                  const label = getPathLabel(member.path);
                                  const locationLabel = getDuplicateMemberLocationLabel(
                                    member.path,
                                    duplicateAnalysis.rootPath,
                                  );
                                  const isKept = member.path === keptPath;

                                  return (
                                    <article
                                      className="duplicate-member"
                                      key={member.path}
                                      role="listitem"
                                      title={member.path}
                                    >
                                      <div className="duplicate-member-meta">
                                        <strong>{label}</strong>
                                        <span className="duplicate-member-location">
                                          {locationLabel}
                                        </span>
                                        <span>{formatTimestamp(member.modifiedAt)}</span>
                                        <span>{formatBytes(member.sizeBytes)}</span>
                                      </div>
                                      <div className="duplicate-member-actions">
                                        <button
                                          type="button"
                                          className={`selection-pill selection-toggle ${isKept ? "is-kept" : "is-delete"}`}
                                          aria-label={`${isKept ? "Kept copy" : "Delete candidate"} for ${label}`}
                                          aria-pressed={isKept}
                                          onClick={() => handleSelectKeepPath(group, member.path)}
                                        >
                                          {isKept ? "Kept copy" : "Delete candidate"}
                                        </button>
                                      </div>
                                    </article>
                                  );
                                })}
                              </div>
                            ) : null}
                          </article>
                        );
                      })}
                    </div>
                  )}

                  {duplicateAnalysis.issues.length > 0 ? (
                    <section className="duplicate-issues">
                      <h4>Excluded paths</h4>
                      <ul>
                        {duplicateAnalysis.issues.map((issue) => (
                          <li key={`${issue.path}-${issue.code}`}>
                            <span>{issue.path}</span>
                            <strong>{issue.summary}</strong>
                          </li>
                        ))}
                      </ul>
                    </section>
                  ) : null}
                </>
              ) : duplicateStatus.state === "cancelled" && duplicateStatus.message ? (
                <p className="notice-banner">{duplicateStatus.message}</p>
              ) : duplicateStatus.state === "failed" && duplicateStatus.message ? (
                <p className="error-banner">{duplicateStatus.message}</p>
              ) : (
                <p className="empty-state">
                  Run duplicate analysis to verify groups from the stored scan result.
                </p>
              )}
            </div>
          )}
        </section>
      </section>
    );
  }

  function renderCleanupPanel() {
    return (
      <section
        className="panel workspace-panel"
        role="tabpanel"
        id="workspace-panel-cleanup"
        aria-labelledby="workspace-tab-cleanup"
      >
        <div className="panel-header">
          <h2>Cleanup</h2>
          <p>
            Build a cleanup preview from approved sources, keep Recycle Bin as the
            default execution path, and preserve the separate confirmation flow for
            permanent delete.
          </p>
        </div>

        <section className="result-card cleanup-card">
          <div className="panel-header compact-header">
            <h3>Safe cleanup</h3>
            <p>
              Build a preview from duplicate delete candidates and the repo-tracked
              cleanup rules, then execute the default Recycle Bin path only after
              review.
            </p>
          </div>

          {privilegedCapability ? (
            <p className="notice-banner cleanup-capability">{privilegedCapability.message}</p>
          ) : null}

          {!currentScan ? (
            <p className="notice-banner">Load a stored scan before cleanup preview.</p>
          ) : !hasFileEntries(currentScan) ? (
            <p className="notice-banner">
              A fresh scan is required before cleanup preview.
            </p>
          ) : (
            <div className="cleanup-layout">
              <div className="cleanup-source-grid">
                <article className="summary-card">
                  <span>Duplicate delete candidates</span>
                  <strong>{duplicateDeletePaths.length}</strong>
                </article>
                <article className="summary-card">
                  <span>Enabled cleanup rules</span>
                  <strong>{selectedCleanupRuleIds.length}</strong>
                </article>
              </div>

              {cleanupRules.length > 0 ? (
                <fieldset className="cleanup-rules">
                  <legend>Built-in cleanup rules</legend>
                  {cleanupRules.map((rule) => {
                    const checked = selectedCleanupRuleIds.includes(rule.ruleId);
                    const visibleRuleLabel =
                      rule.ruleId === "temp-folder-files"
                        ? "Temp folder rule"
                        : rule.ruleId === "download-partials"
                          ? "Partial download rule"
                          : rule.label;
                    return (
                      <label className="cleanup-rule-option" key={rule.ruleId}>
                        <input
                          type="checkbox"
                          aria-label={rule.label}
                          checked={checked}
                          onChange={() => handleToggleCleanupRule(rule.ruleId)}
                        />
                        <span>
                          <strong>{visibleRuleLabel}</strong>
                          <small>{rule.description}</small>
                        </span>
                      </label>
                    );
                  })}
                </fieldset>
              ) : (
                <p className="empty-state">No cleanup rules are available in this build.</p>
              )}

              <div className="action-row">
                <button
                  type="button"
                  className="primary-button"
                  onClick={() => void handleRefreshCleanupPreview()}
                >
                  Refresh cleanup preview
                </button>
              </div>

              {cleanupPreview ? (
                <>
                  <div className="duplicate-summary-grid">
                    <article className="summary-card">
                      <span>Preview</span>
                      <strong>{cleanupPreview.candidates.length} cleanup candidates</strong>
                    </article>
                    <article className="summary-card">
                      <span>Reclaimable bytes</span>
                      <strong>{formatBytes(cleanupPreview.totalBytes)}</strong>
                    </article>
                    <article className="summary-card">
                      <span>Sources</span>
                      <strong>
                        {cleanupPreview.duplicateCandidateCount} duplicate /{" "}
                        {cleanupPreview.ruleCandidateCount} rule
                      </strong>
                    </article>
                  </div>

                  {cleanupPreview.candidates.length === 0 ? (
                    <p className="empty-state">
                      No valid cleanup candidates remain after validation.
                    </p>
                  ) : (
                    <div
                      className="cleanup-candidates"
                      role="list"
                      aria-label="Cleanup candidates"
                    >
                      {cleanupPreview.candidates.map((candidate) => (
                        <article
                          className="cleanup-candidate"
                          key={candidate.actionId}
                          role="listitem"
                        >
                          <div className="cleanup-candidate-meta">
                            <strong>{candidate.path}</strong>
                            <span>{formatBytes(candidate.sizeBytes)}</span>
                          </div>
                          <div className="cleanup-source-tags">
                            {candidate.sourceLabels.map((label) => (
                              <span
                                className="selection-pill is-delete"
                                key={`${candidate.actionId}-${label}`}
                              >
                                {label}
                              </span>
                            ))}
                          </div>
                        </article>
                      ))}
                    </div>
                  )}

                  {cleanupPreview.issues.length > 0 ? (
                    <section className="duplicate-issues">
                      <h4>Excluded cleanup paths</h4>
                      <ul>
                        {cleanupPreview.issues.map((issue) => (
                          <li key={`${issue.path}-${issue.code}`}>
                            <span>{issue.path}</span>
                            <strong>{issue.summary}</strong>
                          </li>
                        ))}
                      </ul>
                    </section>
                  ) : null}

                  <div className="action-row cleanup-actions">
                    <button
                      type="button"
                      className="primary-button"
                      onClick={() => void handleExecuteCleanup("recycle")}
                      disabled={cleanupPreview.candidates.length === 0}
                    >
                      Move selected files to Recycle Bin
                    </button>
                  </div>

                  <label className="cleanup-rule-option advanced-toggle">
                    <input
                      type="checkbox"
                      checked={permanentDeleteConfirmed}
                      onChange={(event) => setPermanentDeleteConfirmed(event.target.checked)}
                    />
                    <span>
                      <strong>I understand permanent delete cannot be undone</strong>
                      <small>Use only when the Recycle Bin path is not appropriate.</small>
                    </span>
                  </label>

                  {permanentDeleteConfirmed ? (
                    <div className="action-row cleanup-actions">
                      <button
                        type="button"
                        className="secondary-button danger-button"
                        onClick={() => void handleExecuteCleanup("permanent")}
                        disabled={cleanupPreview.candidates.length === 0}
                      >
                        Permanently delete selected files
                      </button>
                    </div>
                  ) : null}
                </>
              ) : (
                <p className="empty-state">
                  Enable one or more cleanup sources, then refresh the preview.
                </p>
              )}

              {cleanupExecutionResult ? (
                <section className="cleanup-result">
                  <h4>Cleanup completed</h4>
                  <p>
                    A fresh scan is recommended because the stored scan result may now be
                    stale.
                  </p>
                  <div className="duplicate-summary-grid">
                    <article className="summary-card">
                      <span>Completed</span>
                      <strong>{cleanupExecutionResult.completedCount}</strong>
                    </article>
                    <article className="summary-card">
                      <span>Failed</span>
                      <strong>{cleanupExecutionResult.failedCount}</strong>
                    </article>
                    <article className="summary-card">
                      <span>Mode</span>
                      <strong>{cleanupExecutionResult.mode}</strong>
                    </article>
                  </div>
                  <ul className="cleanup-result-list">
                    {cleanupExecutionResult.entries.map((entry) => (
                      <li key={`${entry.actionId}-${entry.path}`}>
                        <span>{entry.path}</span>
                        <strong>{entry.summary}</strong>
                      </li>
                    ))}
                  </ul>
                </section>
              ) : null}
            </div>
          )}
        </section>
      </section>
    );
  }

  function renderSafetyPanel() {
    return (
      <section
        className="panel workspace-panel"
        role="tabpanel"
        id="workspace-panel-safety"
        aria-labelledby="workspace-tab-safety"
      >
        <div className="panel-header">
          <h2>Safety</h2>
          <p>
            Space Sift stays local-only, review-first, and unprivileged by default even
            when cleanup flows are available.
          </p>
        </div>

        <section className="result-card">
          <div className="panel-header compact-header">
            <h3>Safety model</h3>
            <p>Review-first cleanup stays local, explicit, and unprivileged by default.</p>
          </div>
          <div className="safety-grid">
            {safetyPrinciples.map((principle) => (
              <article className="principle-card" key={principle.title}>
                <h3>{principle.title}</h3>
                <p>{principle.body}</p>
              </article>
            ))}
          </div>
        </section>
      </section>
    );
  }

  function renderActiveWorkspacePanel() {
    switch (activeWorkspace) {
      case "scan":
        return renderScanPanel();
      case "history":
        return renderHistoryPanel();
      case "explorer":
        return renderExplorerPanel();
      case "duplicates":
        return renderDuplicatesPanel();
      case "cleanup":
        return renderCleanupPanel();
      case "safety":
        return renderSafetyPanel();
      case "overview":
      default:
        return renderOverviewPanel();
    }
  }

  return (
    <main className="shell">
      <header className="topbar" role="banner">
        <div className="brand">
          <div className="logo-mark" aria-hidden="true">
            SS
          </div>
          <div>
            <h1>Space Sift</h1>
            <p className="lede">
              A local Windows workflow for scanning space usage, reopening history,
              reviewing duplicates, and executing cleanup only after a safe preview.
            </p>
          </div>
        </div>

        <div className="utility" aria-label="Application safety status">
          <span className="status-pill">Desktop bridge connected</span>
          <span className="status-pill">Recycle Bin first</span>
          <span className="status-pill">Local SQLite history</span>
        </div>
      </header>

      <div className="workspace-layout">
        <nav className="workspace-sidebar" aria-label="Workspace navigation">
          <div className="nav-label">Workspace</div>
          <div
            className="workspace-tablist"
            role="tablist"
            aria-label="Workspace navigation"
            aria-orientation="vertical"
          >
            {workspaceDefinitions.map((definition, index) => {
              const isSelected = activeWorkspace === definition.value;

              return (
                <button
                  key={definition.value}
                  ref={(element) => setWorkspaceTabRef(definition.value, element)}
                  type="button"
                  id={`workspace-tab-${definition.value}`}
                  role="tab"
                  aria-label={definition.label}
                  aria-selected={isSelected}
                  aria-controls={`workspace-panel-${definition.value}`}
                  className={`workspace-tab ${isSelected ? "is-active" : ""}`}
                  onClick={() => activateWorkspace(definition.value)}
                  onKeyDown={(event) => handleWorkspaceKeyDown(event, definition.value)}
                >
                  <span className="workspace-tab-icon" aria-hidden="true">
                    {index + 1}
                  </span>
                  <span className="workspace-tab-copy">
                    <span className="workspace-tab-label">{definition.label}</span>
                    <span className="workspace-tab-description">
                      {definition.description}
                    </span>
                  </span>
                </button>
              );
            })}
          </div>
        </nav>

        <div className="workspace-main" role="region" aria-label="Active workspace content">
          <section className="panel workspace-status-panel" role="region" aria-label="Global status">
            <div className="workspace-status-layout">
              <div className="workspace-status-copy">
                <span className="status-label">Global status</span>
                <h2>{globalStatus.primaryStateLabel}</h2>
                <p className="current-path">{globalStatus.contextLabel}</p>
              </div>

              <div className="workspace-status-summary">
                <article className="summary-card">
                  <span>Summary</span>
                  <strong>{globalStatus.summaryLabel ?? "No additional summary yet."}</strong>
                </article>
                <article className="summary-card">
                  <span>Active workspace</span>
                  <strong>{activeWorkspaceDefinition.label}</strong>
                  <p className="current-path">{activeWorkspaceDefinition.description}</p>
                </article>
              </div>

              <div className="workspace-status-action">
                {nextSafeAction ? (
                  <button
                    type="button"
                    className="primary-button"
                    onClick={handleGlobalStatusAction}
                  >
                    {nextSafeAction.label}
                  </button>
                ) : (
                  <p className="empty-state">{globalStatus.noActionLabel}</p>
                )}
              </div>
            </div>
          </section>

          {shellNotices.map((entry) => (
            <p className="notice-banner shell-banner" key={entry.key}>
              {entry.message}
            </p>
          ))}
          {errorMessage ? <p className="error-banner shell-banner">{errorMessage}</p> : null}

          {renderActiveWorkspacePanel()}
        </div>
      </div>

      <section className="footer-note">
        <p>
          Offline-first scanning, explicit skipped-path reporting, local-only history,
          verified duplicate analysis, and review-first cleanup now live in the desktop
          shell. No background cleanup. No surprise elevation on launch.
        </p>
      </section>
    </main>
  );
}

export default App;
