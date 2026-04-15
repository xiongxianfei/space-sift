# Space Sift Duplicate Detection Test Spec

This test spec maps `specs/space-sift-duplicates.md` to concrete tests for
Milestone 4.

## Test cases

| Test ID | Spec item | Test type | Scenario | Expected result |
| --- | --- | --- | --- | --- |
| T1 | R1, R3, R4, R7, R8 | component | Start duplicate analysis from a currently loaded scan that contains file entries | The UI enters a running state, exposes progress, and completes using only file candidates from the loaded scan |
| T2 | R5, R6, R9, Edge 2 | unit | Analyze same-size files where only some pairs match after full hashing | Only the files with matching full hashes are emitted as duplicate groups |
| T3 | R5, E6, Edge 1 | unit | Analyze tiny or zero-byte files that do not need a meaningful partial-hash step | Full-hash confirmation still governs whether the final duplicate group is emitted |
| T4 | R10, R11, R12, R13, Edge 6 | component | Render a verified duplicate group, apply `keep newest`, `keep oldest`, and a manual keep override | The preview updates correctly, one file always remains kept, and the aggregate reclaimable-byte summary updates |
| T5 | R14, R15, E4, Edge 3 | unit | Candidate files are missing, unreadable, or changed after the scan result was created | Those files are excluded, issues are recorded, and no under-sized candidate set is emitted as a duplicate group |
| T6 | R16, O4 | component | Reopen a browseable history entry and run duplicate analysis from that loaded result | The duplicate workflow runs from the reopened result without requiring a rescan first |
| T7 | R2, E2, Edge 5 | component | Open a summary-only history entry that lacks file-entry data | Duplicate analysis is unavailable and the UI shows a rescan-required message |
| T8 | R17, O3 | unit/integration | Reuse cached hash metadata for an unchanged file and invalidate the cache when the file metadata changes | Valid cache entries are reused, stale entries are ignored or recomputed, and correctness is preserved |
| T9 | R18, Invariants | component | Run duplicate analysis and adjust preview selections | No filesystem mutation command is triggered; the workflow remains preview-only |
| T10 | E1, E3, E5 | component | Attempt duplicate analysis without a loaded scan, then run an analysis that finds no verified groups, then simulate a failure | The UI shows a prerequisite message, an explicit empty state, and a clean failure state respectively |

## Coverage by requirement

| Requirement | Covered by |
| --- | --- |
| R1 | T1 |
| R2 | T7 |
| R3 | T1 |
| R4 | T1 |
| R5 | T2, T3 |
| R6 | T2 |
| R7 | T1 |
| R8 | T1 |
| R9 | T2 |
| R10 | T4 |
| R11 | T4 |
| R12 | T4 |
| R13 | T4 |
| R14 | T5 |
| R15 | T5 |
| R16 | T6 |
| R17 | T8 |
| R18 | T9 |
| E1 | T10 |
| E2 | T7 |
| E3 | T10 |
| E4 | T5 |
| E5 | T10 |
| E6 | T3 |
| O1 | T2, T3 |
| O2 | T5 |
| O3 | T8 |
| O4 | T1, T4, T6, T9, T10 |
| O5 | T7 |
| O6 | T1, T2, T3, T4, T5, T6, T7, T8, T9, T10 |

## Fixtures and scenarios

- Rust tests should use small temporary fixture trees containing:
  - exact duplicates
  - same-size but different-content files
  - zero-byte files
  - a file that is changed or removed after the scan payload is created
- Integration tests for caching should use a local SQLite fixture and record
  cached partial and full hash results against file metadata that can be
  changed between assertions.
- Frontend tests should stub the desktop bridge and drive duplicate-analysis
  state from in-memory scan payloads based on the Milestone 3 `entries` model.
- At least one frontend fixture should represent a pre-Milestone-3 summary-only
  history entry with no file-entry data.

## What not to test

- Recycle Bin or permanent-delete execution
- Cleanup-rule previews
- Winget, signing, or installer behavior
- Cross-platform filesystem behavior outside the Windows 11 target

## Gaps and follow-up

- A later manual smoke test should verify duplicate analysis against a real
  Windows folder tree with large files and the actual Tauri desktop shell.
- Milestone 5 should extend this coverage with execution-path tests once
  duplicate preview actions can hand off to cleanup or Recycle Bin flows.
