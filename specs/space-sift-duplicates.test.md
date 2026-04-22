# Space Sift Duplicate Detection Test Spec

This test spec maps `specs/space-sift-duplicates.md` to concrete tests for
Milestone 4 and the follow-on fast-safe duplicate-performance work.

## Test cases

| Test ID | Spec item | Test type | Scenario | Expected result |
| --- | --- | --- | --- | --- |
| T1 | R1, R3, R4, R7, R8 | component | Start duplicate analysis from a currently loaded scan that contains file entries | The UI enters a running state, exposes progress, and completes using only file candidates from the loaded scan |
| T2 | R5, R6, R9, Edge 2 | unit | Analyze same-size files where only some pairs match after full hashing | Only the files with matching full hashes are emitted as duplicate groups |
| T3 | R5, E6, Edge 1 | unit | Analyze tiny or zero-byte files that do not need a meaningful partial-hash step | Full-hash confirmation still governs whether the final duplicate group is emitted |
| T4 | R9, R9a, R9b, R9c, O4a, O4b | component | Render a completed duplicate result with many groups and disclosure controls | The UI shows only verified groups, orders them by reclaimable bytes then member count then path tie-breaker, and exposes keyboard-operable disclosure state instead of one unbroken wall of fully visible groups |
| T5 | R10, R10a, R10b, Edge 10, O4a | component | Expand a group containing repeated basenames from different folders | Member rows show basename plus visible location context and same-name copies remain distinguishable without relying on hover-only full paths |
| T6 | R11, R12, R13, R13a, Edge 6, Edge 11, O4, O4a | component | Render ordered duplicate groups, apply `keep newest`, `keep oldest`, and a manual keep override | The preview updates correctly, one file always remains kept, and keep-selection changes do not reshuffle the visible group order |
| T7 | R14, R15, E4, Edge 3 | unit | Candidate files are missing, unreadable, or changed after the scan result was created | Those files are excluded, issues are recorded, and no under-sized candidate set is emitted as a duplicate group |
| T8 | R16, O4 | component | Reopen a browseable history entry and run duplicate analysis from that loaded result | The duplicate workflow runs from the reopened result without requiring a rescan first |
| T9 | R2, E2, Edge 5 | component | Open a summary-only history entry that lacks file-entry data | Duplicate analysis is unavailable and the UI shows a rescan-required message |
| T10 | R17, O3 | unit/integration | Reuse cached hash metadata for an unchanged file and invalidate the cache when the file metadata changes | Valid cache entries are reused, stale entries are ignored or recomputed, and correctness is preserved |
| T11 | R18, Invariants | component | Run duplicate analysis and adjust preview selections | No filesystem mutation command is triggered; the workflow remains preview-only |
| T12 | E1, E3, E5 | component | Attempt duplicate analysis without a loaded scan, then run an analysis that finds no verified groups, then simulate a failure | The UI shows a prerequisite message, an explicit empty state, and a clean failure state respectively |
| T13 | R7, R8, R8a, R19, E7, Edge 7, O4 | component | Start duplicate analysis, observe a running stage, request cancellation, and receive a cancelled terminal snapshot | The UI exposes a cancel action during the running state, stops showing running progress after cancellation, and does not show partial duplicate groups as completed output |
| T14 | R8, O7 | unit | Run duplicate analysis across a large duplicate candidate set while collecting progress snapshots | Progress emission stays bounded rather than emitting once per processed item, while the terminal snapshot still reports the final totals |
| T15 | R19, E7, O7 | unit | Trigger cancellation just before or during full-hash verification of a candidate set | Duplicate analysis returns a cancelled outcome promptly and does not emit completed groups |
| T16 | R3, R3a, R3b, E8, Edge 8, Edge 9, O9 | unit/integration | Simulate candidate files on a non-primary path class where hashing would require hidden network-backed reads or placeholder hydration | The analyzer preserves the local-only contract by skipping or reporting the affected files, or by routing them through a safer fallback, and never emits them as verified duplicates without full safe verification |
| T17 | R3b, C4, O8 | manual/integration | Validate one large local fixed-volume candidate set, one warm rerun on the same set, and one non-primary path class if locally available | Validation records show the local fixed-volume baseline and warm rerun, and show that non-primary paths stay on a safe fallback or reduced-impact policy while preserving duplicate correctness |

## Coverage by requirement

| Requirement | Covered by |
| --- | --- |
| R1 | T1 |
| R2 | T7 |
| R3 | T1, T14 |
| R3a | T14 |
| R3b | T14, T15 |
| R4 | T1 |
| R5 | T2, T3 |
| R6 | T2 |
| R7 | T1 |
| R8 | T1 |
| R8a | T11 |
| R9 | T2, T4 |
| R9a | T4 |
| R9b | T4 |
| R9c | T4 |
| R10 | T4, T5 |
| R10a | T5 |
| R10b | T5 |
| R11 | T6 |
| R12 | T6 |
| R13 | T6 |
| R13a | T6 |
| R14 | T7 |
| R15 | T7 |
| R16 | T8 |
| R17 | T10 |
| R18 | T11 |
| R19 | T13, T15 |
| E1 | T12 |
| E2 | T9 |
| E3 | T12 |
| E4 | T7 |
| E5 | T12 |
| E6 | T3 |
| E7 | T13, T15 |
| E8 | T16 |
| O1 | T2, T3 |
| O2 | T7 |
| O3 | T10 |
| O4 | T1, T6, T8, T11, T12, T13 |
| O4a | T4, T5, T6 |
| O4b | T4 |
| O5 | T9 |
| O6 | T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17 |
| O7 | T14, T15 |
| O8 | T17 |
| O9 | T16 |
| C4 | T17 |

## Fixtures and scenarios

- Rust tests should use small temporary fixture trees containing:
  - exact duplicates
  - same-size but different-content files
  - zero-byte files
  - a file that is changed or removed after the scan payload is created
- Integration tests for caching should use a local SQLite fixture and record
  cached partial and full hash results against file metadata that can be
  changed between assertions.
- Integration fixtures for path-class fallback should include a backend stub or
  seam that can simulate files which would require placeholder hydration or
  hidden network-backed reads if opened normally.
- Frontend tests should stub the desktop bridge and drive duplicate-analysis
  state from in-memory scan payloads based on the Milestone 3 `entries` model.
- At least one frontend duplicate fixture should contain more than one verified
  group so deterministic ordering and stable post-selection ordering can be
  asserted.
- At least one frontend duplicate fixture should contain repeated basenames in
  different folders so visible location context can be asserted without using
  hover-only path text.
- At least one frontend fixture should represent a pre-Milestone-3 summary-only
  history entry with no file-entry data.
- Manual performance validation should reuse the same candidate set for the
  cold and warm local fixed-volume runs so cache reuse claims are comparable.

## What not to test

- Recycle Bin or permanent-delete execution
- Cleanup-rule previews
- Winget, signing, or installer behavior
- Cross-platform filesystem behavior outside the Windows 11 target

## Gaps and follow-up

- A later manual smoke test should verify duplicate analysis against a real
  Windows folder tree with large files and the actual Tauri desktop shell.
- If the local validation machine does not expose a suitable non-primary path
  class or placeholder-backed path, record that explicitly instead of claiming
  those cases were exercised.
- Milestone 5 should extend this coverage with execution-path tests once
  duplicate preview actions can hand off to cleanup or Recycle Bin flows.
