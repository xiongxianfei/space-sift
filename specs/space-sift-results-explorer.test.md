# Space Sift Results Explorer Test Spec

This test spec maps `specs/space-sift-results-explorer.md` to concrete tests
for Milestone 3.

## Test cases

| Test ID | Spec item | Test type | Scenario | Expected result |
| --- | --- | --- | --- | --- |
| T1 | R1, R2, R4, R8 | component | Render a completed scan that includes the additive tree data | The explorer defaults to the scan root, shows dashboard totals, and renders the root's immediate children |
| T2 | R3, R6, Edge 4 | component | Click a directory row and then a breadcrumb segment | The current location navigates into the directory and back to the selected breadcrumb level without rescanning |
| T3 | R4, R5, R7 | component | Toggle the visible table sort between size and name | The current directory's rows reorder deterministically while the stored data remains unchanged |
| T4 | R9, R10, E1, Edge 6 | component | Render the unified results table for a directory with children and for an empty directory | Each visible row shows an inline relative-usage cue, and empty directories still show an explicit empty-state message |
| T5 | R11, R12, E2, Edge 5 | component | Invoke Explorer handoff for the current path and a selected row, then simulate a missing-path failure | The client is called with the right path, and failures surface a readable error |
| T6 | R13, R15 | component | Reopen a completed history entry with browseable tree data | The reopened result starts at the scan root and supports the same explorer behavior as a fresh result |
| T7 | R14, E3, Edge 3 | component | Reopen an older history entry whose payload lacks the additive tree data | The summary stays visible, drill-down controls are unavailable, and the UI explains that a rescan is required |
| T8 | R16, Invariants | component | Navigate, sort, and trigger Explorer handoff through the results view | No action mutates the stored result or starts any delete/cleanup workflow |

## Coverage by requirement

| Requirement | Covered by |
| --- | --- |
| R1 | T1 |
| R2 | T1, T6 |
| R3 | T2 |
| R4 | T1, T3 |
| R5 | T3 |
| R6 | T2 |
| R7 | T3 |
| R8 | T1 |
| R9 | T4 |
| R10 | T4 |
| R11 | T5 |
| R12 | T5 |
| R13 | T6 |
| R14 | T7 |
| R15 | T6 |
| R16 | T8 |
| E1 | T4 |
| E2 | T5 |
| E3 | T7 |
| E4 | T1, T6 |
| Edge 6 | T4 |
| O1 | T1, T2, T3, T6 |
| O2 | T7 |
| O3 | T1, T2, T3, T4, T5, T6, T7, T8 |

## Fixtures and scenarios

- Component tests should stub the desktop bridge client and use in-memory scan
  payloads.
- At least one fixture payload should include nested directories with both
  files and folders so breadcrumb navigation can be exercised.
- At least one fixture payload should omit the additive tree field entirely to
  represent a pre-Milestone-3 saved result.

## What not to test

- Duplicate hashing workflows
- Cleanup rule previews
- Recycle Bin execution
- Winget, signing, or installer behaviors
- A separate side-panel space map, because that layout is no longer part of
  the approved explorer contract

## Gaps and follow-up

- A later manual smoke test should verify the real Windows Explorer handoff on
  Windows 11 after `npm run tauri dev`.
- Future milestones should add result filtering, treemap polish, and Explorer
  shell integration tests if the UI grows beyond the current browse model.
