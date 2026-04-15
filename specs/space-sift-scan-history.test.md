# Space Sift Scan And History Test Spec

This test spec maps `specs/space-sift-scan-history.md` to concrete tests for
Milestone 2.

## Test cases

| Test ID | Spec item | Test type | Scenario | Expected result |
| --- | --- | --- | --- | --- |
| T1 | R4, R5, Edge 1, Edge 2 | rust unit | Scan a fixture tree with nested directories and known file sizes | The result aggregates bytes correctly and returns largest files/directories sorted descending |
| T2 | R6, R7, E1, Edge 3 | rust unit | Scan a tree with an intentionally skipped child path fixture or injected walker error | The scan completes and records the skipped path with a stable reason code and summary |
| T3 | R8, E2, Edge 4 | rust unit | Scan a tree containing a reparse point or simulated loop candidate | The scan avoids infinite recursion and emits an explicit skipped entry |
| T4 | R2, R3, R9, Edge 5 | rust unit | Start a scan, observe progress snapshots, then trigger cancellation | Progress moves from running to cancelled and no completed result is returned for persistence |
| T5 | R10, R11, R12, E5, Edge 6 | rust integration | Save a completed scan in the history store and reopen it by identifier | The reopened result matches the stored data; a missing identifier returns a not-found error |
| T6 | R14, E4 | rust integration | Attempt to start a scan on a missing root or simulate persistence failure after scan completion | The command returns a clean failure and does not create a completed history entry |
| T7 | R1, R2, R3, R13, R15 | component | Render the Milestone 2 shell, start a scan, and attempt to start a second scan while one is active | The UI shows running progress and blocks or rejects the second scan clearly |
| T8 | R9, Invariants | component | Cancel an active scan from the UI | The UI shows cancelled state and the cancelled result does not appear in history |
| T9 | R10, R11, R12 | component | Load a seeded history list and reopen a completed scan | The scan summary renders from stored local data without invoking a rescan |

## Coverage by requirement

| Requirement | Covered by |
| --- | --- |
| R1 | T7 |
| R2 | T4, T7 |
| R3 | T4, T7, T8 |
| R4 | T1 |
| R5 | T1 |
| R6 | T2 |
| R7 | T2, T3 |
| R8 | T3 |
| R9 | T4, T8 |
| R10 | T5, T9 |
| R11 | T5, T9 |
| R12 | T5, T9 |
| R13 | T7 |
| R14 | T6 |
| R15 | T7, T9 |
| E1 | T2 |
| E2 | T3 |
| E3 | T1 |
| E4 | T6 |
| E5 | T5 |
| O1 | T1, T2, T3, T4 |
| O2 | T5, T6 |
| O3 | T7, T8, T9 |
| O4 | T1, T2, T3, T4, T5, T6, T7, T8, T9 |

## Fixtures and scenarios

- Rust scan tests should use deterministic fixture trees under
  `tests/fixtures/scan/`.
- History tests should use temporary SQLite databases or test-only database
  files under `tests/fixtures/history/` when durable fixture data is needed.
- Frontend tests may stub the Tauri invoke bridge and in-memory event payloads
  instead of launching the desktop shell.

## What not to test

- Duplicate hashing or duplicate-group UI
- Cleanup rule previews or deletion flows
- Elevated helper behavior
- Installer, signing, or winget distribution

## Gaps and follow-up

- Real Windows permission-denied and reparse-point behavior may still need a
  manual smoke test in addition to unit coverage.
- Later milestones must extend the test plan with results drill-down,
  duplicates, cleanup previews, and signed release verification.
