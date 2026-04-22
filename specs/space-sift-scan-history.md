# Space Sift Scan And History

## Status

- approved

## Goal and context

`Space Sift` scan and history behavior must feel trustworthy on both small and
large Windows 11 folders. The app must let a user start a recursive scan, move
into a clearly labeled active-scan mode immediately, watch honest live progress
while the scan runs, cancel an in-flight scan, and reopen completed scans from
local history without rescanning.

This contract keeps the completed scan result model from Milestone 2 and adds a
clearer long-scan experience: a running scan is not the same thing as a loaded
completed result, and the UI must not fake an exact percent-complete value when
the total recursive workload is unknown. It also locks the fast-safe scan
boundary for future optimization work: ordinary scans stay metadata-first,
remain read-only, target local fixed-volume folders for primary performance
work, and keep the same user-facing contract on removable, network, or
cloud-backed filesystem paths through a safe fallback path when needed.

Related plans:
- `docs/plans/2026-04-15-space-sift-win11-mvp.md`
- `docs/plans/2026-04-16-fast-safe-scan-architecture.md`
- `docs/plans/2026-04-16-scan-progress-and-active-run-ux.md`
- `docs/plans/2026-04-16-history-and-duplicate-review-clarity.md`

## Examples

### Example 1: start a new large scan while an older result exists

Given the user previously opened a completed scan result, when they start a new
scan for a large folder, then the app immediately switches into a dedicated
active-scan experience and does not present the older completed result as the
current result of the running scan.

### Example 2: observe honest live progress during a long scan

Given a recursive scan is still discovering new folders and files, when the app
receives progress updates, then it shows live counters, running-state context,
and a recent activity heartbeat or current path without claiming an exact
percent complete for work whose total size is not yet known.

### Example 3: cancel a long scan

Given a user starts a long-running scan, when they cancel it before completion,
then the in-progress scan stops, the UI reports that it was cancelled, and the
app does not save that partial result as a completed history entry.

### Example 4: complete a scan and transition back to a stored result

Given a scan finishes successfully, when the history save succeeds, then the
app loads the newly completed stored result and shows the completed-scan
experience rather than leaving the user in the active-scan state.

### Example 5: narrow a long local history list

Given the user has many completed scans stored locally and one of them is
currently loaded, when they use history narrowing controls for root path or
scan identifier, then the visible history list stays newest-first, keeps the
currently loaded result visually distinct when it still matches, and narrows
without rescanning or mutating stored history.

### Example 6: reopen a prior scan later

Given a user already completed a scan, when they open the history list and
select that entry later, then the previously stored result loads from local
SQLite data without requiring a new scan.

### Example 7: scan a non-primary path class safely

Given a user selects a removable drive, network share, or cloud-backed folder
that is exposed as a normal filesystem path, when they start an ordinary scan,
then the app keeps the same read-only scan flow and result contract even if it
uses a safer fallback backend instead of the local fixed-volume optimized path.

## Inputs and outputs

Inputs:
- a root folder or drive path selected by the user
- a user request to start scanning
- an optional user request to cancel the active scan
- an optional previously loaded completed scan result
- a user request to open a completed scan from history
- optional user-entered text to narrow visible history entries by root path or
  scan identifier

Ordinary scan path classes considered by this contract:
- local fixed-volume folders on Windows 11
- removable storage paths exposed as normal filesystem paths
- network-share paths exposed as normal filesystem paths
- cloud-backed or sync-provider folders exposed as normal filesystem paths

Outputs:
- an active scan session state for the running scan
- progress snapshots for the active scan
- a completed scan result model with aggregate totals and ranked items
- a skipped-path list with explicit reasons
- a locally persisted history entry for each completed scan
- a newest-first local history view that can narrow visible entries by bounded
  metadata fields and distinguish the currently loaded result when present

## Progress model

A running scan progress snapshot MUST include at least:
- a stable scan identifier
- the scanned root path
- the current lifecycle state
- monotonically increasing file, directory, and processed-byte counters
- a scan start timestamp
- a timestamp representing the latest emitted progress snapshot
- a best-effort current path or current directory context, which may be `null`
  when no narrower path context is available yet

The progress model for recursive scans does not need to know the total
filesystem workload up front. The contract MUST stay truthful when that total
is unknown.

## Result model

A completed scan result MUST include at least:
- a stable scan identifier
- the scanned root path
- scan start and completion timestamps
- total bytes beneath the root that were successfully measured
- total counted files
- total counted directories
- a ranked list of the largest files
- a ranked list of the largest directories
- a skipped-path list where each entry includes the path, reason code, and
  summary text

The result model for this milestone does not need to expose treemap geometry,
duplicate groups, or cleanup recommendations yet.

## Requirements

- R1: The app MUST let the user start a recursive scan for a selected Windows
  folder or drive path without requiring administrator elevation for normal
  user-accessible locations.
- R2: While a scan is running, the backend MUST expose progress updates that
  include the active root, the current lifecycle state, monotonic item or byte
  counters, a scan start timestamp, a latest-progress timestamp, and a
  best-effort current-path context.
- R3: The scan lifecycle MUST distinguish at least these states:
  - `idle`
  - `running`
  - `completed`
  - `cancelled`
  - `failed`
- R4: When the user starts a new scan, the UI MUST enter a dedicated
  active-scan experience immediately and MUST NOT present a previously loaded
  completed scan as the current result for that running scan.
- R5: The active-scan experience MUST show at least:
  - scanned root
  - lifecycle state
  - live file, directory, or byte counters
  - cancellation affordance while the scan is running
  - elapsed time or a visible heartbeat derived from progress telemetry
- R6: For recursive scans whose total workload is unknown, the active-scan UI
  MUST use indeterminate progress language and MUST NOT claim an exact percent
  complete or exact ETA.
- R7: The scan pipeline MAY reduce the cadence of intermediate progress updates
  for performance, but it MUST preserve monotonic counters and MUST still emit
  terminal `completed`, `cancelled`, and `failed` states promptly.
- R8: The recursive scanner MUST aggregate file sizes upward so each completed
  result includes total bytes for the root and for each returned directory row.
- R9: The completed result MUST include ranked largest-file and
  largest-directory lists sorted by descending size.
- R10: If the scanner encounters a path it cannot or should not traverse, it
  MUST continue scanning the rest of the tree when safe to do so and MUST add a
  skipped-path entry instead of crashing the scan.
- R11: Every skipped-path entry MUST include:
  - the path that was skipped
  - a stable reason code
  - a short human-readable summary
- R12: The scanner MUST avoid infinite recursion caused by symlinks, junctions,
  or other reparse-point loops.
- R13: The user MUST be able to cancel the active scan from the UI. After
  cancellation is acknowledged, the scan MUST stop promptly and MUST NOT be
  stored as a completed history entry.
- R14: When a scan completes successfully, the app MUST persist it to local
  SQLite-backed history storage before presenting that scan as the current
  completed result.
- R15: The history list MUST show enough metadata to distinguish prior scans,
  including at least scan identifier, scanned root, completion time, and total
  bytes.
- R15a: The visible history list MUST default to newest-first ordering by
  completion time.
- R15b: If the currently loaded completed result is present in the visible
  history list, that entry MUST be visually distinguished from the other saved
  scans.
- R15c: History review MUST support local-only narrowing of the visible list by
  at least:
  - scanned root path text
  - scan identifier text
- R16: The user MUST be able to reopen a completed history entry and receive
  the same stored scan result model without rescanning.
- R17: Starting a new scan while another scan is already running MUST NOT start
  a second concurrent scan. The app MUST reject or disable that action clearly.
- R18: If a requested root path does not exist or cannot be read at scan start,
  the app MUST fail that scan cleanly with an error state and MUST NOT create a
  completed history entry.
- R19: Scan history persistence MUST remain local-only. The app MUST NOT
  require a network connection, cloud account, or remote API to run scans or
  reopen stored results.
- R20: Ordinary space scans MUST remain metadata-first. They MUST NOT fully
  read file contents, perform duplicate-confirmation hashing, or sample file
  bodies as part of directory-size discovery.
- R21: The ordinary scan flow MUST support local fixed-volume folders on
  Windows 11. It SHOULD also support removable storage, network shares, and
  cloud-backed or sync-provider folders when the OS exposes them as normal
  filesystem paths.
- R22: Local fixed-volume folders are the primary performance target for scan
  optimization on Windows 11. The app MAY use a different backend strategy for
  other path classes, but it MUST preserve the same result model, skipped-path
  behavior, cancellation behavior, and read-only guarantees.
- R23: If a supported non-primary path class cannot safely use an optimized
  backend, the app MUST fall back to a safe supported scan path rather than
  silently reading file contents, requiring elevation, or changing the result
  contract.

## Invariants

- Scan and history behavior is read-only.
- Scan and history behavior does not delete, move, hash for duplicate
  confirmation, or modify files.
- The normal app UI remains unprivileged during scan and history flows.
- A cancelled or failed scan is not treated as a completed reusable result.
- The active-scan experience and the completed-result experience remain
  distinct states, even if recent history stays visible elsewhere in the UI.
- History narrowing affects only the current local view. It does not mutate the
  stored scan data or reorder persisted history entries.

## Error handling and boundary behavior

- E1: Permission-denied children are recorded as skipped entries and do not
  crash the whole scan.
- E2: Reparse points that would create traversal loops are skipped with an
  explicit reason rather than recursed infinitely.
- E3: Empty folders still produce a valid completed result with zero-byte
  totals.
- E4: If the history database is unavailable or the save step fails after an
  otherwise successful scan, the app MUST surface that persistence failure
  instead of pretending the scan was saved.
- E5: Reopening a missing history entry by identifier MUST return a clean
  not-found style error rather than stale or placeholder data.
- E6: If best-effort current-path context is temporarily unavailable during a
  running scan, the active-scan UI MUST still remain usable through root,
  lifecycle, and counter telemetry rather than appearing blank or failed.
- E7: If a supported path class cannot use an optimized backend safely, the app
  MUST continue through a supported fallback path or fail cleanly at scan
  start. It MUST NOT silently switch ordinary scanning into a content-reading
  workflow.
- E8: If local history contains saved scans but the current history narrowing
  controls match none of them, the UI MUST show an explicit no-match state
  rather than the same message used for a genuinely empty history store.

## Compatibility and migration

- C1: This milestone targets Windows 11 only.
- C2: The backend implementation SHOULD stay behind a stable scan abstraction so
  a future NTFS metadata fast path can produce the same result contract.
- C3: Later milestones may extend the stored scan schema, but they SHOULD keep
  old history entries readable through additive migrations.
- C4: Progress-model additions SHOULD remain additive so newer scan telemetry
  does not require destructive history or result migrations.
- C5: Backend choice MAY vary by path class, but the ordinary scan contract
  SHOULD stay stable across optimized and fallback paths.

## Observability expectations

- O1: Rust tests MUST cover recursive aggregation, skipped-path handling, and
  cancellation behavior in the scan engine.
- O2: Rust or command-layer tests MUST cover long-scan progress behavior,
  including monotonic counters and reliable terminal-state emission when
  intermediate progress is rate-limited.
- O3: Rust tests MUST cover storing and reopening completed scans through the
  local history layer.
- O4: Frontend tests MUST cover the visible active-scan flow, including the
  dedicated running state, stale-result separation, cancellation, and reopening
  a stored scan result from history data.
- O4a: Frontend tests MUST cover many-entry history review, including
  newest-first ordering, current-result highlighting, and bounded history
  narrowing by root path and scan identifier.
- O5: Milestone verification MUST include the targeted Rust tests for
  `scan-core` and `app-db`, plus focused frontend tests for scan and history
  interactions.
- O6: Fast-safe scan verification MUST include evidence for both:
  - a local fixed-volume scan path
  - one non-primary fallback path class, or an explicit note that it was not
    available on the maintainer machine
- O7: Scan-engine tests for ordinary scans MUST verify that the scan path does
  not invoke duplicate-style hashing or full file-content reads.

## Edge cases

- Edge 1: Scanning an empty folder produces a zero-byte completed result.
- Edge 2: Scanning a tree with nested folders aggregates child sizes into
  parents correctly.
- Edge 3: A permission-denied child path is reported in skipped entries.
- Edge 4: A reparse point or symlink does not cause infinite recursion.
- Edge 5: Cancelling a scan prevents a completed history save.
- Edge 6: Reopening history works after the active in-memory result has been
  cleared or the app has been restarted.
- Edge 7: A second scan request while one is already running is rejected
  clearly.
- Edge 8: Starting a new scan while an older completed result was loaded still
  enters active-scan mode immediately.
- Edge 9: A large recursive scan with unknown total work still shows live
  progress without an exact percent-complete claim.
- Edge 10: Best-effort current-path context may be unavailable briefly without
  making the active-scan experience look stalled or broken.
- Edge 11: A removable, network, or cloud-backed path can still be scanned
  through the normal read-only contract even if it does not use the optimized
  local fixed-volume backend.
- Edge 12: Ordinary scan behavior stays separate from duplicate analysis even
  after future scan-performance optimizations are added.
- Edge 13: A long saved history list can be narrowed by root path or scan ID
  without mutating the stored order or forcing a rescan.
- Edge 14: The currently loaded result may be temporarily absent from the
  visible history view if the active narrowing text does not match it.

## Non-goals

- Duplicate detection or staged hashing
- Cleanup preview generation
- Recycle Bin execution
- Privileged helper flows
- Treemap rendering polish
- NTFS direct MFT scanning in v1
- Exact percent-complete or exact ETA claims for recursive scans whose total
  workload is unknown
- Partial results explorer browsing before scan completion
- Requiring every supported path class to use the same internal enumeration
  backend

## Acceptance criteria

- A reviewer can start a scan from the app, observe a dedicated active-scan
  experience, and receive a completed result with total bytes, largest files,
  largest directories, and skipped-path reporting.
- A reviewer running a large scan can see live scan state, counters, and a
  heartbeat or current-path context without the app claiming an exact percent
  complete.
- A reviewer can start a new scan while an older completed result exists and
  confirm that the older result is not presented as the current result of the
  running scan.
- A reviewer can cancel an in-flight scan and confirm it is not stored as a
  completed history entry.
- A reviewer can reopen a previously completed scan from local history without
  rescanning.
- A reviewer with many saved scans can identify the currently loaded result and
  narrow history by root path or scan identifier without changing stored data.
- A reviewer can confirm that ordinary scan behavior remains metadata-first and
  separate from duplicate hashing even as scan performance work changes the
  backend.
- A reviewer can confirm that local fixed-volume folders remain the primary
  optimization target while an available non-primary path class still honors
  the same read-only scan contract through a supported fallback path.
- Automated tests cover the scan contract, persistence contract, long-scan
  progress contract, and the active-scan UI flow.
