import {
  startTransition,
  useEffect,
  useEffectEvent,
  useState,
  type FormEvent,
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
  ScanStatusSnapshot,
} from "./lib/spaceSiftTypes";

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

  if (typeof error === "string") {
    return error;
  }

  return "The requested operation did not complete.";
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
  const [scanStatus, setScanStatus] = useState<ScanStatusSnapshot>(idleScanStatus);
  const [duplicateStatus, setDuplicateStatus] =
    useState<DuplicateStatusSnapshot>(idleDuplicateStatus);
  const [history, setHistory] = useState<ScanHistoryEntry[]>([]);
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
  const [cleanupExecutionResult, setCleanupExecutionResult] =
    useState<CleanupExecutionResult | null>(null);
  const [privilegedCapability, setPrivilegedCapability] =
    useState<PrivilegedCleanupCapability | null>(null);
  const [permanentDeleteConfirmed, setPermanentDeleteConfirmed] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const resetCleanupState = useEffectEvent(() => {
    startTransition(() => {
      setSelectedCleanupRuleIds([]);
      setCleanupPreview(null);
      setCleanupExecutionResult(null);
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
      setCleanupExecutionResult(null);
      setPermanentDeleteConfirmed(false);
    });
  });

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
        setCleanupExecutionResult(null);
        setPermanentDeleteConfirmed(false);
        setErrorMessage(null);
      });
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
    }
  });

  const openStoredScan = useEffectEvent(async (scanId: string, nextNotice?: string) => {
    try {
      const result = await client.openScanHistory(scanId);
      startTransition(() => {
        setCurrentScan(result);
        setCurrentExplorerPath(result.rootPath);
        setExplorerSortMode("size");
        setNotice(nextNotice ?? `Loaded ${scanId} from local history.`);
        setErrorMessage(null);
        setDuplicateStatus(idleDuplicateStatus);
        setDuplicateAnalysis(null);
        setDuplicateKeepSelections({});
        setExpandedDuplicateGroups({});
        setSelectedCleanupRuleIds([]);
        setCleanupPreview(null);
        setCleanupExecutionResult(null);
        setPermanentDeleteConfirmed(false);
      });
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
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

    if (snapshot.state === "completed" && snapshot.completedScanId) {
      void openStoredScan(snapshot.completedScanId);
      void loadHistory();
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

    if (snapshot.state === "completed" && snapshot.completedAnalysisId) {
      if (duplicateAnalysis?.analysisId === snapshot.completedAnalysisId) {
        return;
      }

      void openStoredDuplicateAnalysis(snapshot.completedAnalysisId);
    }
  });

  useEffect(() => {
    let isActive = true;
    let unsubscribeScan = () => {};
    let unsubscribeDuplicate = () => {};

    void (async () => {
      try {
        const [
          initialStatus,
          initialHistory,
          initialDuplicateStatus,
          initialCleanupRules,
          capability,
          scanUnlisten,
          duplicateUnlisten,
        ] = await Promise.all([
          client.getScanStatus(),
          client.listScanHistory(),
          client.getDuplicateAnalysisStatus(),
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
        startTransition(() => {
          setScanStatus(initialStatus);
          setDuplicateStatus(initialDuplicateStatus);
          setHistory(initialHistory);
          setCleanupRules(initialCleanupRules);
          setPrivilegedCapability(capability);
          if (initialStatus.rootPath) {
            setRootPath(initialStatus.rootPath);
          }
        });

        if (initialStatus.state === "completed" && initialStatus.completedScanId) {
          await openStoredScan(initialStatus.completedScanId);
        }

        if (
          initialDuplicateStatus.state === "completed" &&
          initialDuplicateStatus.completedAnalysisId
        ) {
          await openStoredDuplicateAnalysis(initialDuplicateStatus.completedAnalysisId);
        }
      } catch (error) {
        if (isActive) {
          startTransition(() => {
            setErrorMessage(describeError(error));
          });
        }
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
    resetReviewState();

    try {
      const started = await client.startScan(normalizedRoot);
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
    } catch (error) {
      startTransition(() => {
        setErrorMessage(describeError(error));
      });
    } finally {
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
    await openStoredScan(scanId, `Loaded ${scanId} from local history.`);
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
    } catch (error) {
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
    setCleanupExecutionResult(null);
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
    setCleanupExecutionResult(null);
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
        setCleanupExecutionResult(null);
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
        setCleanupExecutionResult(result);
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

  return (
    <main className="shell">
      <section className="hero">
        <div className="eyebrow">Windows 11 results explorer, duplicates, and cleanup preview</div>
        <h1>Space Sift</h1>
        <p className="lede">
          Scan a folder or drive, surface the largest space consumers, verify duplicate
          files, preview a narrow cleanup set, and delete through a review-first desktop
          flow instead of blind cleanup.
        </p>
        <div className="hero-callouts" aria-label="Current scope">
          <span>Browseable folder drill-down</span>
          <span>Verified duplicate groups</span>
          <span>Recycle Bin first cleanup</span>
        </div>
      </section>

      <section className="workspace-grid">
        <section className="panel panel-tall">
          <div className="panel-header">
            <h2>Scan workspace</h2>
            <p>
              Point the scanner at a Windows folder or drive path. Progress stays visible
              while results are prepared for local history storage.
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

          {scanStatus.message ? <p className="notice-banner">{scanStatus.message}</p> : null}
          {notice ? <p className="notice-banner">{notice}</p> : null}
          {errorMessage ? <p className="error-banner">{errorMessage}</p> : null}
        </section>

        <section className="panel">
          <div className="panel-header">
            <h2>Recent scans</h2>
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
                            {isLoadedResult ? <span className="history-badge">Loaded result</span> : null}
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
      </section>

      <section className="panel">
        <div className="panel-header">
          <h2>Safety model</h2>
          <p>Review-first cleanup stays local, explicit, and unprivileged by default.</p>
        </div>
        <div className="principles-grid">
          {safetyPrinciples.map((principle) => (
            <article className="principle-card" key={principle.title}>
              <h3>{principle.title}</h3>
              <p>{principle.body}</p>
            </article>
          ))}
        </div>
      </section>

      {isScanRunning ? (
        <section className="panel active-scan-panel" aria-live="polite">
          <div className="panel-header">
            <h2>Active scan</h2>
            <p>
              Progress stays indeterminate while Space Sift discovers more files and
              folders. Previous completed scans stay in Recent scans until this run
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
              <strong>{activeScanPath ? getPathLabel(activeScanPath) : "Waiting for path context"}</strong>
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
              <p className="active-scan-meta">Live progress is emitted at a bounded cadence.</p>
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
      ) : currentScan ? (
        <section className="panel">
          <div className="panel-header">
            <h2>Current result</h2>
            <p>
              Loaded scan {currentScan.scanId} from {currentScan.rootPath}. Review the stored
              result, verify duplicates, and build a cleanup preview from approved sources.
            </p>
          </div>

          {browseableScan && resolvedExplorerPath ? (
            <div className="explorer-grid">
              <section className="result-card explorer-card">
                <div className="explorer-header">
                  <div>
                    <h3>Results explorer</h3>
                    <p className="explorer-note">
                      The current folder view is read-only, reflects the stored scan result
                      without rescanning, and shows each row's share of the current level.
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
                                <span className="usage-label">{sharePercent}% of current level</span>
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
              This saved result was saved before folder browsing support. Run a fresh scan to
              browse folders again.
            </p>
          )}

          <section className="result-card duplicate-card">
            <div className="panel-header compact-header">
              <h3>Duplicate analysis</h3>
              <p>
                Fully verified groups require a size match plus content verification. Cleanup
                stays separate until you explicitly build a cleanup preview.
              </p>
            </div>

            {!duplicateEligible ? (
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
                          {duplicatePreview.filesMarkedForDeletion} files marked for later deletion
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
                                <div
                                  className="duplicate-members"
                                  id={membersRegionId}
                                  role="list"
                                >
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

          <section className="result-card cleanup-card">
            <div className="panel-header compact-header">
              <h3>Safe cleanup</h3>
              <p>
                Build a preview from duplicate delete candidates and the repo-tracked cleanup
                rules, then execute the default Recycle Bin path only after review.
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
                      <div className="cleanup-candidates" role="list" aria-label="Cleanup candidates">
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
                    <p>A fresh scan is recommended because the stored scan result may now be stale.</p>
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
        </section>
      ) : (
        <section className="panel">
          <p className="empty-state">
            Start a scan or reopen a stored result to populate the current view.
          </p>
        </section>
      )}

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
