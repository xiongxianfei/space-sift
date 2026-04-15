# Space Sift Results Explorer

## Status

- approved

## Goal and context

`Space Sift` Milestone 3 turns the scan summary shell into a browseable results
explorer. After a scan completes or a prior scan is reopened from history, the
user must be able to navigate the scanned tree, sort visible items, move
between folders with breadcrumbs, inspect a simple space map for the current
level, and hand the current path or a selected item off to Windows Explorer.

Milestone 3 builds on `specs/space-sift-scan-history.md`. It does not add
duplicate detection or cleanup execution yet.

Related plan:
- `docs/plans/2026-04-15-space-sift-win11-mvp.md`

## Examples

### Example 1: drill into a directory from a completed scan

Given a completed scan contains `C:\Users\xiongxianfei\Downloads\Games`, when
the user clicks that directory row from the root results table, then the
results view switches to that directory, the breadcrumb trail updates, and the
table plus space map show only that directory's immediate children.

### Example 2: sort visible entries

Given the current directory contains multiple files and folders, when the user
sorts by size descending or by name ascending, then the visible table order
changes deterministically without mutating the stored scan result.

### Example 3: reopen a saved scan and continue browsing

Given the user reopens a completed scan from local history, when the result
loads, then the explorer defaults to the scan root and offers the same
browsing, sorting, and Explorer handoff affordances as a freshly completed
scan.

### Example 4: open the current item in Windows Explorer

Given the user is viewing a folder row or the current breadcrumb location,
when they choose the Explorer handoff action, then the app requests Windows
Explorer to reveal that path without requiring administrator elevation.

### Example 5: older saved result without the browseable tree

Given a history entry was saved before Milestone 3 added per-entry tree data,
when the user reopens that entry, then the app still shows the saved summary
and history metadata but explains that folder drill-down requires a fresh
rescan.

## Inputs and outputs

Inputs:
- a completed scan result from a fresh scan or history reopen
- a user click on a directory row
- a user click on a breadcrumb segment
- a user sort choice for the visible table
- a user request to open a path in Windows Explorer

Outputs:
- a current explorer location within the scanned tree
- a sorted list of the immediate children beneath that location
- breadcrumb navigation for the current location
- a simple proportional space map for the current location
- a request to Windows Explorer for the chosen path

## Additive result model extension

Milestone 3 extends the completed scan payload with an additive per-entry tree
model. A browseable scan result MUST be able to provide, for each scanned file
and directory row:
- the full path
- the parent path, or `null` for the root
- the entry kind (`file` or `directory`)
- the total size in bytes for that row

This extension MUST be additive so older Milestone 2 history entries remain
readable. If a reopened history entry lacks this per-entry tree data, the app
MUST remain usable in summary mode and MUST explain that drill-down requires a
new scan.

## Requirements

- R1: A newly completed scan MUST expose enough per-entry tree data for the UI
  to render immediate children for any browsed directory in the scanned tree.
- R2: When the user first opens a completed scan result, the explorer MUST
  default to the scan root path.
- R3: The explorer MUST show breadcrumbs from the scan root to the current
  browsed directory, and clicking a breadcrumb segment MUST navigate back to
  that level.
- R4: The explorer MUST show a results table for the immediate children of the
  current browsed directory.
- R5: The results table MUST distinguish files from directories.
- R6: Clicking a directory row in the results table MUST navigate into that
  directory without rescanning.
- R7: The results table MUST support deterministic sorting by at least:
  - size
  - name
- R8: The UI MUST show a scan dashboard summarizing at least:
  - scanned root
  - total bytes
  - total files
  - total directories
  - completion time
- R9: The UI MUST show a space map for the current browsed directory using the
  visible child data from the stored scan result. The space map does not need
  to be a full treemap in Milestone 3, but it MUST visualize relative space
  usage for the current level.
- R10: The UI MUST offer a Windows Explorer handoff action for:
  - the current browsed path
  - at least one selected row from the current table
- R11: The Windows Explorer handoff MUST stay available without requiring the
  whole app to run as administrator.
- R12: Reopening a completed scan from local history MUST restore the explorer
  at the scan root with the same browseable data as a fresh completion when the
  stored result includes the additive tree model.
- R13: If a reopened history entry lacks the browseable tree model, the app
  MUST show the saved summary data, MUST NOT crash, and MUST clearly explain
  that folder drill-down is unavailable for that older entry.
- R14: History browsing and result navigation MUST remain local-only and MUST
  NOT require network access or cloud synchronization.
- R15: Browsing, sorting, and Explorer handoff in Milestone 3 MUST remain
  read-only with respect to the scanned files. These actions MUST NOT move,
  rename, or delete content.

## Invariants

- Milestone 3 result exploration is read-only.
- The current explorer location is always within the reopened or completed scan
  root.
- Sorting changes the current view only; it does not rewrite persisted scan
  data.

## Error handling and boundary behavior

- E1: If the current browsed directory has no immediate children, the explorer
  MUST show an explicit empty-state message instead of a blank panel.
- E2: If the user requests Explorer handoff for a path that no longer exists,
  the app MUST show a clean error instead of crashing.
- E3: If a history entry lacks the additive tree data, the app MUST degrade to
  summary mode rather than failing the reopen.
- E4: If the stored result includes skipped paths, the explorer SHOULD keep
  those skipped-path summaries visible somewhere in the result experience.

## Compatibility and migration

- C1: Milestone 3 targets Windows 11 only.
- C2: The additive tree model MUST preserve readability of older Milestone 2
  history payloads.
- C3: A future, faster scan backend such as an NTFS fast path SHOULD be able to
  populate the same browseable tree contract without changing the UI behavior.

## Observability expectations

- O1: Frontend tests MUST cover drill-down navigation, breadcrumb navigation,
  sorting behavior, and history reopen at the scan root.
- O2: Frontend tests MUST cover the degraded summary-only behavior for older
  saved scans that do not contain the additive tree model.
- O3: Verification for Milestone 3 MUST include a focused `results` test run
  plus the existing history-focused test run.

## Edge cases

- Edge 1: The root directory may contain no immediate children.
- Edge 2: A directory with only files or only folders still renders correctly.
- Edge 3: A history entry without the additive tree remains readable in summary
  mode.
- Edge 4: Sorting by size and then navigating deeper still shows only the new
  current directory's immediate children.
- Edge 5: Explorer handoff for a missing path returns a visible error.

## Non-goals

- Duplicate detection
- Cleanup preview or execution
- Privileged helper flows
- Full treemap geometry optimization
- Search or filter across the whole scan tree

## Acceptance criteria

- A reviewer can reopen a scan and browse from the root into nested folders
  using row clicks and breadcrumbs.
- A reviewer can sort the visible results by size and name and see the table
  update deterministically.
- A reviewer can use a Windows Explorer handoff action for the current path or
  a selected row.
- A reviewer reopening an older summary-only history entry sees a clear rescan
  prompt instead of a broken explorer.
