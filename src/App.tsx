import {
  startTransition,
  useEffect,
  useEffectEvent,
  useState,
  type FormEvent,
} from "react";
import "./App.css";
import {
  idleScanStatus,
  type SpaceSiftClient,
  unsupportedClient,
} from "./lib/spaceSiftClient";
import type {
  CompletedScan,
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
    body: "Deletion workflows remain outside this milestone, with Recycle Bin as the default safety path once cleanup ships.",
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

function formatBytes(bytes: number) {
  return `${bytes} bytes`;
}

function formatTimestamp(value: string) {
  const parsed = new Date(value);
  return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleString();
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

function compareEntryNames(left: ScanEntry, right: ScanEntry) {
  return (
    getPathLabel(left.path).localeCompare(getPathLabel(right.path), undefined, {
      sensitivity: "base",
      numeric: true,
    }) ||
    left.path.localeCompare(right.path, undefined, {
      sensitivity: "base",
      numeric: true,
    })
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

function App({ client = unsupportedClient }: AppProps) {
  const [rootPath, setRootPath] = useState("");
  const [scanStatus, setScanStatus] = useState<ScanStatusSnapshot>(idleScanStatus);
  const [history, setHistory] = useState<ScanHistoryEntry[]>([]);
  const [currentScan, setCurrentScan] = useState<CompletedScan | null>(null);
  const [currentExplorerPath, setCurrentExplorerPath] = useState<string | null>(null);
  const [explorerSortMode, setExplorerSortMode] = useState<ExplorerSortMode>("size");
  const [notice, setNotice] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

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

  const openStoredScan = useEffectEvent(async (scanId: string, nextNotice?: string) => {
    try {
      const result = await client.openScanHistory(scanId);
      startTransition(() => {
        setCurrentScan(result);
        setCurrentExplorerPath(result.rootPath);
        setExplorerSortMode("size");
        setNotice(nextNotice ?? `Loaded ${scanId} from local history.`);
        setErrorMessage(null);
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

  useEffect(() => {
    let isActive = true;
    let unsubscribe = () => {};

    void (async () => {
      try {
        const [initialStatus, initialHistory, unlisten] = await Promise.all([
          client.getScanStatus(),
          client.listScanHistory(),
          client.subscribeToScanProgress((snapshot) => {
            if (isActive) {
              handleProgress(snapshot);
            }
          }),
        ]);

        if (!isActive) {
          unlisten();
          return;
        }

        unsubscribe = unlisten;
        startTransition(() => {
          setScanStatus(initialStatus);
          setHistory(initialHistory);
          if (initialStatus.rootPath) {
            setRootPath(initialStatus.rootPath);
          }
        });

        if (initialStatus.state === "completed" && initialStatus.completedScanId) {
          await openStoredScan(initialStatus.completedScanId);
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
      unsubscribe();
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

    try {
      const started = await client.startScan(normalizedRoot);
      startTransition(() => {
        setScanStatus({
          ...idleScanStatus,
          scanId: started.scanId,
          rootPath: normalizedRoot,
          state: "running",
        });
        setNotice(`Scanning ${normalizedRoot}`);
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

  const browseableScan = currentScan && hasBrowseableEntries(currentScan) ? currentScan : null;
  const resolvedExplorerPath =
    browseableScan && currentExplorerPath && isPathWithinRoot(currentExplorerPath, browseableScan.rootPath)
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
  const spaceMapTotal = visibleEntries.reduce((sum, entry) => sum + entry.sizeBytes, 0);

  return (
    <main className="shell">
      <section className="hero">
        <div className="eyebrow">Windows 11 results explorer live in Milestone 3</div>
        <h1>Space Sift</h1>
        <p className="lede">
          Scan a folder or drive, surface the largest space consumers, and reopen
          saved results from local history without rescanning every time.
        </p>
        <div className="hero-callouts" aria-label="Current scope">
          <span>Browseable folder drill-down</span>
          <span>Skipped-path reporting</span>
          <span>SQLite-backed history</span>
        </div>
      </section>

      <section className="workspace-grid">
        <section className="panel panel-tall">
          <div className="panel-header">
            <h2>Scan workspace</h2>
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

          {notice ? <p className="notice-banner">{notice}</p> : null}
          {errorMessage ? <p className="error-banner">{errorMessage}</p> : null}

          <div className="safety-grid" role="list" aria-label="Safety principles">
            {safetyPrinciples.map((principle) => (
              <article className="principle-card" key={principle.title} role="listitem">
                <h3>{principle.title}</h3>
                <p>{principle.body}</p>
              </article>
            ))}
          </div>
        </section>

        <section className="panel">
          <div className="panel-header">
            <h2>Recent scans</h2>
            <p>
              Completed scans reopen from local SQLite history so larger trees do
              not need a full rescan just to inspect the last result.
            </p>
          </div>

          {history.length === 0 ? (
            <p className="empty-state">No completed scans saved locally yet.</p>
          ) : (
            <div className="history-list" role="list">
              {history.map((entry) => (
                <article className="history-card" key={entry.scanId} role="listitem">
                  <div>
                    <h3>{entry.rootPath}</h3>
                    <p>{formatTimestamp(entry.completedAt)}</p>
                    <p>{formatBytes(entry.totalBytes)}</p>
                  </div>
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() => void handleReopenScan(entry.scanId)}
                  >
                    Reopen scan {entry.scanId}
                  </button>
                </article>
              ))}
            </div>
          )}
        </section>
      </section>

      <section className="panel">
        <div className="panel-header">
          <h2>Current scan result</h2>
          <p>
            Browse the stored scan tree from the root, sort the current folder,
            and hand the current location or a selected item off to Windows
            Explorer without rescanning.
          </p>
        </div>

        {currentScan ? (
          <div className="results-layout">
            <div className="result-summary">
              <article className="summary-card">
                <span>Scanned root</span>
                <strong>{currentScan.rootPath}</strong>
              </article>
              <article className="summary-card">
                <span>Total bytes</span>
                <strong>{formatBytes(currentScan.totalBytes)}</strong>
              </article>
              <article className="summary-card">
                <span>Files measured</span>
                <strong>{currentScan.totalFiles}</strong>
              </article>
              <article className="summary-card">
                <span>Directories measured</span>
                <strong>{currentScan.totalDirectories}</strong>
              </article>
              <article className="summary-card">
                <span>Completed</span>
                <strong>{formatTimestamp(currentScan.completedAt)}</strong>
              </article>
            </div>

            {browseableScan && resolvedExplorerPath ? (
              <div className="explorer-grid">
                <section className="result-card explorer-card">
                  <div className="explorer-header">
                    <div>
                      <h3>Results explorer</h3>
                      <p className="explorer-note">
                        The current folder view is read-only and reflects the
                        stored scan result without rescanning.
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
                      This folder has no immediate children in the stored scan
                      result.
                    </p>
                  ) : (
                    <table className="results-table" aria-label="Current folder contents">
                      <thead>
                        <tr>
                          <th scope="col">Name</th>
                          <th scope="col">Type</th>
                          <th scope="col">Size</th>
                          <th scope="col">Actions</th>
                        </tr>
                      </thead>
                      <tbody>
                        {visibleEntries.map((entry) => {
                          const label = getPathLabel(entry.path);

                          return (
                            <tr key={entry.path}>
                              <td>
                                <div className="entry-name">
                                  <strong>{label}</strong>
                                </div>
                              </td>
                              <td className="entry-kind">{entry.kind}</td>
                              <td>{formatBytes(entry.sizeBytes)}</td>
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

                <section className="result-card explorer-card">
                  <div className="panel-header compact-header">
                    <h3>Space map</h3>
                    <p>
                      Relative space usage for the immediate children of the
                      current folder.
                    </p>
                  </div>

                  {visibleEntries.length === 0 ? (
                    <p className="empty-state">
                      The current folder has no child items to visualize.
                    </p>
                  ) : (
                    <div className="space-map" aria-label="Space map">
                      {visibleEntries.map((entry) => {
                        const widthPercent =
                          spaceMapTotal === 0
                            ? 0
                            : Math.max((entry.sizeBytes / spaceMapTotal) * 100, 6);
                        const sharePercent =
                          spaceMapTotal === 0
                            ? 0
                            : Math.round((entry.sizeBytes / spaceMapTotal) * 100);

                        return (
                          <div
                            className="space-map-item"
                            key={entry.path}
                            aria-label={`${getPathLabel(entry.path)} ${formatBytes(entry.sizeBytes)}`}
                            title={getPathLabel(entry.path)}
                          >
                            <div className="space-map-meta">
                              <strong>{formatBytes(entry.sizeBytes)}</strong>
                              <span>{sharePercent}% of current level</span>
                            </div>
                            <div className="space-map-track">
                              <div
                                className="space-map-bar"
                                style={{ width: `${Math.min(widthPercent, 100)}%` }}
                              />
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </section>
              </div>
            ) : (
              <p className="notice-banner compatibility-note">
                This saved result was saved before folder browsing support. Run a
                fresh scan to browse folders again.
              </p>
            )}

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
          </div>
        ) : (
          <p className="empty-state">
            Start a scan or reopen a stored result to populate the current view.
          </p>
        )}
      </section>

      <section className="panel">
        <div className="panel-header">
          <h2>Next up</h2>
          <p>
            Milestone 3 makes the stored scan result browseable. The next major
            layers are duplicate verification and safe cleanup previews, both
            still gated behind later implementation milestones.
          </p>
        </div>
        <div className="roadmap-strip">
          <span>Duplicate hashing workflow</span>
          <span>Cleanup rules preview</span>
          <span>Recycle Bin execution later</span>
        </div>
      </section>

      <section className="footer-note">
        <p>
          Offline-first scanning, explicit skipped-path reporting, local-only
          history, and read-only result browsing today. No background cleanup.
          No surprise elevation on launch.
        </p>
      </section>
    </main>
  );
}

export default App;
