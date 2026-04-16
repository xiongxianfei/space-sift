# Space Sift Scan And History Test Spec

This test spec maps `specs/space-sift-scan-history.md` to concrete tests for
the current scan, long-scan progress, and history contract.

## Test cases

| Test ID | Spec item | Test type | Scenario | Expected result |
| --- | --- | --- | --- | --- |
| T1 | R8, R9, Edge 1, Edge 2 | rust unit | Scan a fixture tree with nested directories and known file sizes | The result aggregates bytes correctly and returns largest files and directories sorted descending |
| T2 | R10, R11, E1, Edge 3 | rust unit | Scan a tree with an intentionally skipped child path fixture or injected walker error | The scan completes and records the skipped path with a stable reason code and summary |
| T3 | R12, E2, Edge 4 | rust unit | Scan a tree containing a reparse point or simulated loop candidate | The scan avoids infinite recursion and emits an explicit skipped entry |
| T4 | R2, R3, R7, Edge 9, Edge 10 | rust unit | Start a scan, collect progress snapshots from a long-running fixture, and inspect intermediate plus terminal events | Progress snapshots keep monotonic counters, include long-scan telemetry fields, and still emit the terminal state even if intermediate updates are rate-limited |
| T5 | R13, Invariants, Edge 5 | rust unit | Start a scan, observe progress snapshots, then trigger cancellation | Progress moves from running to cancelled and no completed result is produced for persistence |
| T6 | R14, R15, R16, E5, Edge 6 | rust integration | Save a completed scan in the history store and reopen it by identifier | The reopened result matches the stored data, and a missing identifier returns a not-found error |
| T7 | R18, E4 | rust integration | Attempt to start a scan on a missing root or simulate persistence failure after scan completion | The command returns a clean failure and does not create a completed history entry |
| T8 | R1, R2, R3, R4, R5, R6, R17, R19, E6, Edge 7, Edge 8, Edge 9, Edge 10 | component | Render the app with a previous completed result available, start a new scan, and simulate running progress | The UI enters dedicated active-scan mode immediately, shows root, running-state context, live counters, and heartbeat or path context, does not present the older result as the current result, blocks a second scan, and does not claim an exact percent complete |
| T9 | R13, Invariants | component | Cancel an active scan from the UI | The UI shows cancelled state and the cancelled result does not appear in history |
| T10 | R14, R15, R16 | component | Drive a scan from running to completed with a `completedScanId` and stored result | The UI leaves active-scan mode and loads the newly persisted completed result rather than stale placeholder content |
| T11 | R20, O7, Edge 12 | rust unit or integration | Run an ordinary scan through an instrumented scan backend or fixture seam that can detect duplicate-style hashing or file-body reads | The scan completes without invoking duplicate hashing, file sampling, or full content reads |
| T12 | R21, R22, R23, E7, Edge 11 | rust integration | Scan a fixture or injected path classified as a supported non-primary path and force the optimized backend to be unavailable | The scan uses the safe fallback path, preserves the normal result contract, and does not require elevation or content reads |
| T13 | O6, acceptance criteria | manual smoke | Compare one large local fixed-volume scan with one available non-primary path class | The maintainer records that the local fixed-volume path is the primary optimization target and that the fallback path still honors the same read-only contract |

## Coverage by requirement

| Requirement | Covered by |
| --- | --- |
| R1 | T8 |
| R2 | T4, T8 |
| R3 | T4, T8, T9 |
| R4 | T8 |
| R5 | T8 |
| R6 | T8 |
| R7 | T4 |
| R8 | T1 |
| R9 | T1 |
| R10 | T2 |
| R11 | T2 |
| R12 | T3 |
| R13 | T5, T9 |
| R14 | T6, T7, T10 |
| R15 | T6, T10 |
| R16 | T6, T10 |
| R17 | T8 |
| R18 | T7 |
| R19 | T8 |
| R20 | T11 |
| R21 | T12, T13 |
| R22 | T12, T13 |
| R23 | T12 |
| E1 | T2 |
| E2 | T3 |
| E3 | T1 |
| E4 | T7 |
| E5 | T6 |
| E6 | T8 |
| E7 | T12 |
| O1 | T1, T2, T3, T5 |
| O2 | T4 |
| O3 | T6, T7 |
| O4 | T8, T9, T10 |
| O5 | T1, T2, T3, T4, T5, T6, T7, T8, T9, T10 |
| O6 | T13 |
| O7 | T11 |

## Fixtures and scenarios

- Rust scan tests should use deterministic fixture trees under
  `tests/fixtures/scan/`.
- Long-scan telemetry tests should include a fixture or fake backend that emits
  multiple progress opportunities so bounded-cadence behavior can be asserted
  without relying on wall-clock flakiness.
- History tests should use temporary SQLite databases or test-only database
  files under `tests/fixtures/history/` when durable fixture data is needed.
- Fast-safe scan tests may use injected path classifiers or backend-selection
  seams so local fixed-volume and fallback-path behavior can be covered without
  depending on real network infrastructure in every automated run.
- Frontend tests may stub the Tauri invoke bridge and in-memory progress events
  instead of launching the desktop shell.
- At least one component fixture should begin with a previously loaded
  completed scan so the stale-result separation rules are exercised when a new
  scan starts.

## What not to test

- Duplicate analysis
- Cleanup rule previews or deletion flows
- Elevated helper behavior
- Installer, signing, or winget distribution
- A fake exact ETA model, because the spec does not promise one
- Partial results explorer browsing during an in-flight scan
- The exact internal Windows enumeration API choice, because the spec only
  fixes the user-visible contract and safety boundaries

## Notes

- Real Windows permission-denied and reparse-point behavior may still need a
  manual smoke test in addition to unit coverage.
- A manual Windows 11 smoke run on a genuinely large folder should confirm that
  the chosen progress cadence still feels alive outside synthetic fixtures.
- If the repo does not yet expose a backend-selection seam, Milestone 2 or 3 of
  the fast-safe scan plan should add one before T12 is implemented.
- Later milestones may extend the test plan with NTFS fast-path behavior only
  after that capability is explicitly added to the contract.
