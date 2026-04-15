# Space Sift MVP Foundation Test Spec

This test spec maps `specs/space-sift-mvp.md` to concrete tests for Milestone 1.

## Test cases

| Test ID | Spec item | Test type | Scenario | Expected result |
| --- | --- | --- | --- | --- |
| T1 | R2, R4 | component | Render the initial app shell | The landing screen shows `Space Sift` and capability areas for large-file discovery, duplicate detection, and cleanup rules |
| T2 | R3 | component | Render the initial app shell | The landing screen includes messaging about unprivileged UI and Recycle Bin first deletion |
| T3 | R5, R6, E1, Edge 3 | component | Render planned action controls before feature implementation | Placeholder scan or cleanup actions are visibly unavailable and cannot be triggered |
| T4 | R8, Invariants | component | Render the initial app shell in a normal test runtime without network mocking | The initial content renders from local data only and does not require remote fetches |
| T5 | R1, R7, O1 | manual smoke | Run the documented desktop development command on Windows 11 | A `Space Sift` desktop window opens without requiring the whole app to run as administrator |

## Coverage by requirement

| Requirement | Covered by |
| --- | --- |
| R1 | T5 |
| R2 | T1 |
| R3 | T2 |
| R4 | T1 |
| R5 | T3 |
| R6 | T3 |
| R7 | T5 |
| R8 | T4 |
| E1 | T3 |
| E2 | T5 |
| E3 | T1 |
| O1 | T5 |
| O2 | T1, T2, T3, T4 |

## Fixtures and scenarios

- No filesystem fixtures are required for Milestone 1 frontend tests.
- Component tests should render the landing screen in `jsdom`.
- The manual smoke test should use the documented desktop development command
  from the repository root after dependencies are installed.

## What not to test

- Real scan traversal
- Duplicate hashing
- Cleanup preview generation
- Recycle Bin integration
- NTFS fast-path behavior

## Gaps and follow-up

- T5 depends on a working Rust + Tauri toolchain on the local machine.
- Later milestones must extend this test spec with scan, duplicate, cleanup,
  persistence, and release-path coverage.
