# Space Sift Scan And History

## Status

- approved

## Goal and context

`Space Sift` Milestone 2 turns the informational shell into a working scanner
for Windows 11 folders and drives. The app must let a user start a recursive
scan, see progress while it runs, understand which paths were skipped, cancel
an in-flight scan, and reopen completed scans from local history without
rescanning immediately.

Related plan:
- `docs/plans/2026-04-15-space-sift-win11-mvp.md`

## Examples

### Example 1: scan a folder and view the totals

Given a user selects a folder that contains nested directories and files, when
they start a scan and it completes, then the app shows the scanned root,
aggregated byte totals, the largest directories, and the largest files for that
result set.

### Example 2: a scan reports skipped paths

Given a scan encounters a permission-denied folder, a broken link, or a path
that the scanner intentionally avoids, when the scan finishes, then the result
shows those paths in a skipped list with a machine-stable reason code and a
human-readable summary.

### Example 3: cancel a long scan

Given a user starts a long-running scan, when they cancel it before completion,
then the in-progress scan stops, the UI reports that it was cancelled, and the
app does not save that partial result as a completed history entry.

### Example 4: reopen a prior scan

Given a user already completed a scan, when they open the history list and
select that entry later, then the previously stored result loads from local
SQLite data without requiring a new scan.

## Inputs and outputs

Inputs:
- a root folder or drive path selected by the user
- a user request to start scanning
- an optional user request to cancel the active scan
- a user request to open a completed scan from history

Outputs:
- progress state for the active scan
- a completed scan result model with aggregate totals and ranked items
- a skipped-path list with explicit reasons
- a locally persisted history entry for each completed scan

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

The result model for Milestone 2 does not need to expose treemap geometry,
duplicate groups, or cleanup recommendations yet.

## Requirements

- R1: The app MUST let the user start a recursive scan for a selected Windows
  folder or drive path without requiring administrator elevation for normal
  user-accessible locations.
- R2: While a scan is running, the backend MUST expose progress updates that
  include the active root, the current lifecycle state, and monotonically
  increasing counters for discovered items or processed bytes.
- R3: The scan lifecycle MUST distinguish at least these states:
  - `idle`
  - `running`
  - `completed`
  - `cancelled`
  - `failed`
- R4: The recursive scanner MUST aggregate file sizes upward so each completed
  result includes total bytes for the root and for each returned directory row.
- R5: The completed result MUST include ranked largest-file and
  largest-directory lists sorted by descending size.
- R6: If the scanner encounters a path it cannot or should not traverse, it
  MUST continue scanning the rest of the tree when safe to do so and MUST add a
  skipped-path entry instead of crashing the scan.
- R7: Every skipped-path entry MUST include:
  - the path that was skipped
  - a stable reason code
  - a short human-readable summary
- R8: The scanner MUST avoid infinite recursion caused by symlinks, junctions,
  or other reparse-point loops.
- R9: The user MUST be able to cancel the active scan from the UI. After
  cancellation is acknowledged, the scan MUST stop promptly and MUST NOT be
  stored as a completed history entry.
- R10: When a scan completes successfully, the app MUST persist it to local
  SQLite-backed history storage.
- R11: The history list MUST show enough metadata to distinguish prior scans,
  including at least scan identifier, scanned root, completion time, and total
  bytes.
- R12: The user MUST be able to reopen a completed history entry and receive
  the same stored scan result model without rescanning.
- R13: Starting a new scan while another scan is already running MUST NOT start
  a second concurrent scan in Milestone 2. The app MUST reject or disable that
  action clearly.
- R14: If a requested root path does not exist or cannot be read at scan start,
  the app MUST fail that scan cleanly with an error state and MUST NOT create a
  completed history entry.
- R15: Scan history persistence MUST remain local-only for Milestone 2. The app
  MUST NOT require a network connection, cloud account, or remote API to run
  scans or reopen stored results.

## Invariants

- Milestone 2 scanning is read-only.
- Milestone 2 scan/history behavior does not delete, move, hash for duplicate
  confirmation, or modify files.
- The normal app UI remains unprivileged during scan and history flows.
- A cancelled or failed scan is not treated as a completed reusable result.

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

## Compatibility and migration

- C1: Milestone 2 targets Windows 11 only.
- C2: The backend implementation SHOULD stay behind a stable scan abstraction so
  a future NTFS metadata fast path can produce the same result contract.
- C3: Later milestones may extend the stored scan schema, but they SHOULD keep
  old history entries readable through additive migrations.

## Observability expectations

- O1: Rust tests MUST cover recursive aggregation, skipped-path handling, and
  cancellation behavior in the scan engine.
- O2: Rust tests MUST cover storing and reopening completed scans through the
  local history layer.
- O3: Frontend tests MUST cover the visible scan-state flow and reopening a
  stored scan result from history data.
- O4: Milestone verification MUST include the targeted Rust tests for
  `scan-core` and `app-db`, plus focused frontend tests for scan and history
  interactions.

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

## Non-goals

- Duplicate detection or staged hashing
- Cleanup preview generation
- Recycle Bin execution
- Privileged helper flows
- Treemap rendering polish
- NTFS direct MFT scanning in v1

## Acceptance criteria

- A reviewer can start a scan from the app, observe progress, and receive a
  completed result with total bytes, largest files, largest directories, and
  skipped-path reporting.
- A reviewer can cancel an in-flight scan and confirm it is not stored as a
  completed history entry.
- A reviewer can reopen a previously completed scan from local history without
  rescanning.
- Automated tests cover the Milestone 2 scan contract, persistence contract,
  and the minimal UI flow for scanning and history.
