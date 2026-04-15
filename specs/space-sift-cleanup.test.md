# Space Sift Safe Cleanup Test Spec

This test spec maps `specs/space-sift-cleanup.md` to concrete tests for
Milestone 5.

## Test cases

| Test ID | Spec item | Test type | Scenario | Expected result |
| --- | --- | --- | --- | --- |
| T1 | R1, R2 | unit/integration | Load the built-in cleanup rule catalog | The catalog exposes the repo-tracked `temp-folder-files` and `download-partials` rules with stable IDs and labels |
| T2 | R3, R4, R6, R7, R8, Edge 1 | unit | Build a cleanup preview from a loaded scan, one duplicate delete candidate, and enabled cleanup rules | The preview includes only file candidates inside the scan root, deduplicates repeated paths, preserves source labels, and reports reclaimable bytes |
| T3 | R9, E4, Edge 2, Edge 5 | unit | Preview candidates include a missing file, an out-of-root file, and a protected system path | Invalid candidates are excluded and surfaced as clean issues; the preview still completes and can show an empty state |
| T4 | R10, R12, R13, R14, O3, Edge 3 | unit | Execute a preview in `recycle` mode with one candidate succeeding and one candidate failing revalidation | The executor attempts Recycle-Bin-first cleanup, only previewed candidates are used, aggregate counts are correct, and failures are reported per file |
| T5 | R11, E7 | component/unit | Request permanent delete through the advanced execution path | Permanent delete uses a separate explicit mode and is blocked until the advanced confirmation path is active |
| T6 | R15, O4 | integration | Persist a cleanup execution result to the local SQLite store | The execution record is saved with mode, aggregate counts, and per-file outcomes |
| T7 | R16, R17, E6, O5 | unit | Evaluate the privilege boundary for user-profile paths and protected Windows paths | User paths stay in the unprivileged flow, protected paths are marked as requiring elevation, and the UI capability response stays fail-closed |
| T8 | R18, O6 | component | Run a cleanup preview and execute the default recycle action from the UI | The UI renders the preview, calls recycle execution by default, shows execution results, and recommends a fresh scan afterward |
| T9 | R3, E2, Edge 6 | component | Open a summary-only saved scan without file-entry data | Cleanup preview is unavailable and the UI shows a rescan-required message |
| T10 | E1, E3, E4 | component | Attempt cleanup preview with no loaded scan, then with no enabled source, then with zero valid candidates | The UI shows prerequisite messaging, does not silently create a preview, and renders an explicit empty state |

## Coverage by requirement

| Requirement | Covered by |
| --- | --- |
| R1 | T1 |
| R2 | T1 |
| R3 | T2, T9 |
| R4 | T2 |
| R5 | T8 |
| R6 | T2 |
| R7 | T2 |
| R8 | T2, T8 |
| R9 | T3 |
| R10 | T4, T8 |
| R11 | T5 |
| R12 | T4, T8 |
| R13 | T4 |
| R14 | T4, T8 |
| R15 | T6 |
| R16 | T7 |
| R17 | T3, T7 |
| R18 | T8 |
| E1 | T10 |
| E2 | T9 |
| E3 | T10 |
| E4 | T3, T10 |
| E5 | T4 |
| E6 | T7 |
| E7 | T5 |
| O1 | T2 |
| O2 | T3 |
| O3 | T4, T5 |
| O4 | T6 |
| O5 | T7 |
| O6 | T8, T9, T10 |

## Fixtures and scenarios

- Rust cleanup-preview tests should use a small temporary scan fixture that
  includes:
  - a file under a `Temp` folder
  - a partial-download file such as `installer.crdownload`
  - a duplicate delete candidate
  - a repeated path that is matched by more than one source
  - a path outside the scan root
  - a missing path
- Rust execution tests should use a recording executor or other test-safe
  boundary rather than relying on the operating system Recycle Bin.
- SQLite logging tests should use a temporary database fixture and validate the
  stored execution JSON or equivalent durable record.
- Frontend tests should stub the desktop bridge and drive the cleanup flow from
  in-memory scan and duplicate-analysis payloads.
- At least one frontend fixture should represent a summary-only history entry
  with no file-entry data.

## What not to test

- Winget, signing, or installer behavior
- Background scheduling
- Registry or driver-store cleaning
- Cross-platform cleanup semantics outside the Windows 11 target
- Actual OS-level Recycle Bin behavior in unit tests

## Gaps and follow-up

- A later manual smoke test should verify that the default cleanup path really
  lands files in the Windows Recycle Bin on a disposable Windows profile.
- A later milestone can extend this test plan with a true elevated helper flow
  if protected-path cleanup becomes supported.
