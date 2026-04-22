Status: active

# Space Sift scan run continuity test specification

## Related spec and plan

- Feature spec: [space-sift-scan-run-continuity.md](/D:/Data/20260415-space-sift/specs/space-sift-scan-run-continuity.md)
- Architecture: [2026-04-18-scan-run-continuity.md](/D:/Data/20260415-space-sift/docs/architecture/2026-04-18-scan-run-continuity.md)
- ADR: [2026-04-18-scan-run-persistence-and-resume.md](/D:/Data/20260415-space-sift/docs/adr/2026-04-18-scan-run-persistence-and-resume.md)
- Execution plan: [2026-04-18-scan-run-continuity.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-18-scan-run-continuity.md)

## Testing strategy

- `unit`: repository invariants, DTO normalization, monotonic counter rules, resume fingerprint comparison, purge eligibility logic, clock-driven heartbeat scheduling.
- `integration`: SQLite-backed continuity persistence, command-layer state transitions, restart reconciliation, non-live cancellation, resume creation, purge execution, audit and telemetry signals.
- `frontend integration`: React/Vitest flows around active runs, recovered runs, resume affordances, terminal transitions, and history compatibility using mocked `SpaceSiftClient`.
- `contract`: command payload shape, error code mapping, ordering rules, and continuity read model consistency across header plus latest snapshot.
- `migration`: legacy `scan_history` rows without continuity records, legacy completed payload variants, and sequence fallback behavior for pre-seq continuity data if supported.
- `smoke`: focused desktop restart and resume flows that cross the Rust command layer, persistence, and UI wiring.
- `manual`: restart/crash-recovery scenarios, retention timing, and reopen behavior that are expensive or fragile to automate end-to-end in the first milestone.

## Requirement coverage map

| Requirement | Covered by | Notes |
| --- | --- | --- |
| `R1` Run identity, persistence, and continuity read model | `T1`, `T2`, `T19`, `T22` | Verifies run header creation, latest snapshot reads, and legacy completed history compatibility. |
| `R2` Ordered snapshots with deterministic sequencing | `T1`, `T2`, `T10`, `T13` | Includes monotonic `seq` enforcement and legacy fallback behavior where supported. |
| `R3` Progress snapshot semantics and bounded counters | `T3`, `T4`, `T6`, `T19` | Covers monotonic counts, normalized progress, and terminal totals. |
| `R4` Heartbeat cadence and stale detection inputs | `T4`, `T5`, `T9`, `T10` | Uses injected clock/timer seam and restart reconciliation. |
| `R5` Recovery semantics for stale and abandoned runs | `T9`, `T10`, `T11`, `T12`, `T19`, `T20` | Includes startup reconciliation, surfaced recovery state, and visible `ABANDONED` behavior. |
| `R6` Optional resumability and current unsupported-engine posture | `T11`, `T14`, `T15`, `T19`, `T20` | Covers default-off behavior, `UNSUPPORTED_ENGINE`, and `ABANDONED` plus visible resume metadata with disabled actionability. |
| `R7` Privacy, security, and retention behavior | `T15`, `T17`, `T18` | Verifies rejection safety, token non-exposure, and purge eligibility for resumable versus non-resumable runs. |
| `R8` API and UX consistency | `T12`, `T15`, `T19`, `T20`, `T21` | Covers machine-readable command errors, read-model consistency, and UI state handling. |
| `R9` Failure handling and cancel behavior | `T8`, `T12`, `T19`, `T21` | Covers live cancel, synthetic non-live cancel, and terminal rejection behavior. |
| `R10` Compatibility and migration behavior | `T13`, `T22` | Covers legacy rows, missing continuity data, and unchanged completed history reopen flows. |

## Example coverage map

| Example | Covered by | Notes |
| --- | --- | --- |
| `E1` `ABANDONED` run keeps resume metadata visible while engine resume is unsupported | `T11`, `T19`, `T20` | Verifies `ABANDONED` remains visible while `has_resume = true` and `can_resume = false`. |
| `E2` Old `ABANDONED` run becomes purge-eligible when resume is no longer valid | `T15`, `T18` | Verifies invalid or expired resume metadata no longer blocks retention purge. |

| Acceptance example | Covered by |
| --- | --- |
| `AC1` Active run survives restart as recoverable continuity record | `T9`, `T19`, `T20`, `M1` |
| `AC2` Live progress persists with ordered snapshots and latest snapshot reads | `T1`, `T2`, `T3`, `T19` |
| `AC3` Old run becomes `ABANDONED` when recovery window expires | `T11`, `T19`, `M2` |
| `AC4` Resume disabled by default | `T14`, `T20` |
| `AC5` Engine-disabled resume remains unavailable through `can_resume = false` | `T15`, `T19`, `T20` |
| `AC6` Invalid resume returns explicit rejection code | `T15` |
| `AC7` Completed runs remain reopenable through existing history path | `T6`, `T19`, `T22` |
| `AC8` Purge removes expired continuity data and emits signal | `T18`, `M4` |

## Edge case coverage

- Fast completion race where a delayed progress event arrives after terminal completion: `T6`, `T22`.
- Startup reconciliation precedence when a `RUNNING` row is already older than abandon threshold: `T10`, `T11`.
- At most one synthetic reconciliation snapshot per startup pass: `T10`.
- Heartbeats continue while progress counters remain unchanged: `T4`, `T5`.
- Long-running but healthy traversal should not be marked stale solely because no files completed recently: `T4`, `T5`, `T9`.
- `cancel_scan_run(runId)` against an already terminal run should return the contractually defined rejection and avoid duplicate terminal rows: `T12`.
- Resume disabled path should omit resume token and expose `can_resume = false`: `T14`.
- While engine resume support is disabled, resume rejection must deterministically return `UNSUPPORTED_ENGINE`: `T15`.
- Legacy databases with no continuity rows must still reopen completed scan history correctly: `T13`, `T22`.
- Purge must skip active, stale-within-window, or resumable records that are still retained: `T18`.

## Test cases

### Persistence, ordering, and progress invariants

`T1. Start run persists header and initial snapshot`
- Covers: `R1`, `R2`, `AC2`
- Level: integration
- Fixture/setup: temporary SQLite database, real `HistoryStore` or equivalent continuity repository, fake run metadata for a new scan.
- Steps:
- Start a new scan run through the command or repository layer.
- Read the created run header and latest snapshot through the public read model.
- Expected result:
- A stable `runId` is created.
- The initial snapshot is persisted with `seq = 1`.
- The read model returns header fields plus the latest snapshot in one response.
- Failure proves:
- Continuity cannot survive process restart because the run was never durably materialized at start.
- Automation location:
- Rust integration tests near the continuity repository and command start path.

`T2. Snapshot append enforces monotonic ordering and latest-snapshot authority`
- Covers: `R1`, `R2`, `AC2`
- Level: integration
- Fixture/setup: persisted run with at least one snapshot in a temp SQLite database.
- Steps:
- Append additional progress snapshots with increasing `seq`.
- Attempt an out-of-order or duplicate `seq` append.
- Read list/detail APIs after valid writes.
- Expected result:
- Valid appends preserve ordering and latest snapshot state.
- Duplicate or out-of-order appends are rejected or ignored according to the architecture without regressing latest state.
- List/detail reads use the most recent valid snapshot as authoritative status.
- Failure proves:
- UI and recovery behavior could observe contradictory or non-deterministic run state.
- Automation location:
- Rust repository tests.

`T3. Progress counters remain monotonic and normalized`
- Covers: `R3`, `AC2`
- Level: unit
- Fixture/setup: progress snapshot builder or DTO normalization helper with representative counter inputs.
- Steps:
- Feed increasing progress updates.
- Feed a regressive count, inconsistent percentage, or terminal payload with mismatched totals.
- Expected result:
- Stored or emitted counters never move backwards.
- Percent or bounded progress fields are normalized according to the spec.
- Terminal snapshot uses completed totals rather than stale intermediate counts.
- Failure proves:
- The continuity view can misreport progress or regress visually across restarts.
- Automation location:
- Rust unit tests around snapshot conversion helpers and command-layer progress mapping.

`T4. Heartbeat timer emits bounded snapshots without requiring traversal progress`
- Covers: `R3`, `R4`
- Level: integration
- Fixture/setup: injected clock/timer seam, active run, mocked or paused traversal that makes no progress for multiple intervals.
- Steps:
- Advance fake time in heartbeat increments while keeping counters unchanged.
- Inspect persisted snapshots and latest read model.
- Expected result:
- Heartbeats are emitted at the configured cadence.
- The run remains `RUNNING`.
- Repeated heartbeat snapshots do not invent progress or violate bounded telemetry rules.
- Failure proves:
- Healthy long-running scans can be misclassified as stale because heartbeat ownership is wrong.
- Automation location:
- Rust command/runtime integration tests using the plan’s injected clock seam.

`T5. No-progress warning is observable after repeated unchanged heartbeats`
- Covers: `R4`
- Level: integration
- Fixture/setup: fake clock, active run, telemetry sink or structured log capture.
- Steps:
- Produce four or more heartbeat intervals with unchanged counters.
- Inspect emitted observability events.
- Expected result:
- `scan_run_no_progress_warning` or equivalent structured event is emitted once per threshold crossing.
- The run is still not marked stale while heartbeats continue.
- Failure proves:
- Operators cannot distinguish "alive but not progressing" from normal steady-state heartbeats.
- Automation location:
- Rust command/runtime observability tests.

`T6. Completion finalization is atomic with reopenable history payload`
- Covers: `R1`, `R3`, `R10`, `AC7`
- Level: integration
- Fixture/setup: temp SQLite database, active run with progress, fault injection seam around final write transaction if available.
- Steps:
- Complete the run successfully.
- Read continuity detail and the existing completed `scan_history` payload.
- Re-run with a forced failure in the finalization path if fault injection exists.
- Expected result:
- A completed run publishes `COMPLETED` only when the completed `scan_history` payload is durably reopenable.
- Final continuity state and `scan_history` state remain consistent.
- Delayed non-terminal updates after completion do not overwrite the terminal view.
- Failure proves:
- Users can see a completed run that cannot be reopened, or conflicting terminal states across read models.
- Automation location:
- Rust integration tests plus frontend regression extension of existing history completion-race tests.

`T7. Finalization failure never leaves an impossible terminal history`
- Covers: `R1`, `R3`
- Level: integration
- Fixture/setup: same as `T6`, but with deterministic failure on completed payload persistence or transaction commit.
- Steps:
- Trigger the failure path during completion finalization.
- Read run detail, audit records, and any persisted history payload.
- Expected result:
- The system records the contractually allowed failure state only.
- No combination of `COMPLETED` continuity state plus missing completed payload is visible.
- No second contradictory terminal snapshot is appended.
- Failure proves:
- The atomic completion boundary described in the architecture is not real.
- Automation location:
- Rust integration tests if a transaction or repository fault seam is available.

### Recovery, cancellation, and run APIs

`T8. Live cancel writes a terminal cancelled snapshot and preserves reopen history`
- Covers: `R6`
- Level: integration
- Fixture/setup: active run with persisted progress snapshots.
- Steps:
- Cancel the live run through the public command.
- Read run detail and any surrounding UI state or command response.
- Expected result:
- A terminal `CANCELLED` snapshot is appended exactly once.
- The run becomes terminal and future live progress updates do not supersede it.
- Existing completed history rows from other scans remain unaffected.
- Failure proves:
- Cancel is not durable and the run can reappear as active after restart.
- Automation location:
- Rust command tests and frontend integration tests that extend current cancel behavior coverage.

`T9. Startup reconciliation marks recently interrupted runs as stale`
- Covers: `R4`, `R5`, `AC1`
- Level: integration
- Fixture/setup: temp database containing a `RUNNING` run with last heartbeat older than stale threshold but younger than abandon threshold.
- Steps:
- Simulate application startup reconciliation.
- Read run detail, list output, and audit rows.
- Expected result:
- Exactly one synthetic `STALE` snapshot is appended.
- The run appears recoverable in read APIs.
- Reconciliation audit state is persisted.
- Failure proves:
- Restarted sessions cannot surface interrupted work consistently.
- Automation location:
- Rust startup/reconciliation integration tests.

`T10. Reconciliation precedence appends at most one synthetic terminality snapshot per pass`
- Covers: `R2`, `R4`, `R5`
- Level: integration
- Fixture/setup: persisted `RUNNING` run old enough to satisfy multiple reconciliation thresholds.
- Steps:
- Execute one reconciliation pass.
- Inspect appended snapshots and final read-model state.
- Expected result:
- Only one synthetic snapshot is appended for the pass.
- Precedence matches the architecture rule, with no `RUNNING -> STALE -> ABANDONED` double-append in a single pass.
- Failure proves:
- Recovery status is non-deterministic and can differ by timing or implementation order.
- Automation location:
- Rust reconciliation tests.

`T11. Reconciliation marks expired interrupted runs as abandoned`
- Covers: `R5`, `R6`, `E1`, `AC3`
- Level: integration
- Fixture/setup: persisted `RUNNING` run older than abandon threshold with no active process owner.
- Steps:
- Run startup reconciliation.
- Read run detail and list output.
- Expected result:
- The run becomes `ABANDONED` according to the architecture precedence.
- It no longer appears as an active live run.
- Resume affordance reflects whether the persisted run is still eligible.
- Failure proves:
- Old interrupted runs remain misleadingly active or recoverable forever.
- Automation location:
- Rust reconciliation tests and frontend list/detail tests.

`T12. Cancel stale or recovered run follows non-live cancel contract`
- Covers: `R5`, `R8`, `R9`
- Level: integration
- Fixture/setup: one `STALE` run, one `ABANDONED` run, and one already-terminal run.
- Steps:
- Call `cancel_scan_run(runId)` for each case.
- Inspect snapshots, API response codes, and audit rows.
- Expected result:
- Non-live cancellable runs append a synthetic terminal `CANCELLED` snapshot exactly once.
- Reconciliation or cancel audit rows are written as specified.
- Already-terminal runs return the contractually defined rejection behavior without mutating state.
- Failure proves:
- The non-live cancel API is underspecified in code and can corrupt terminal history.
- Automation location:
- Rust command integration tests.

`T13. Legacy history compatibility remains intact without continuity rows`
- Covers: `R2`, `R10`
- Level: integration
- Fixture/setup: legacy `scan_history` fixtures including payloads without `entries` and databases with no continuity tables or rows.
- Steps:
- Open legacy completed scans through existing history paths.
- Read continuity-aware list/detail endpoints where applicable.
- Expected result:
- Legacy completed history still reopens successfully.
- Missing continuity records do not break completed history reads.
- Any fallback ordering behavior for legacy continuity rows without `seq` matches the documented compatibility contract when supported.
- Failure proves:
- The migration path breaks existing users before they interact with the new continuity feature.
- Automation location:
- Rust repository tests and extensions of existing `app-db` compatibility tests.

`T14. Resume remains disabled by default`
- Covers: `R6`, `AC4`
- Level: integration
- Fixture/setup: default configuration with no explicit resume enablement, interrupted run persisted in the database, frontend mock client exposing read model.
- Steps:
- Read run detail and render the corresponding UI state.
- Attempt to invoke resume through command surface if exposed.
- Expected result:
- No resume token is issued.
- API read model returns `has_resume = false` and `can_resume = false` or equivalent default-off indicators.
- UI does not show an enabled resume action by default.
- Failure proves:
- Resume behavior escaped its rollout gate and changed product behavior without an explicit opt-in.
- Automation location:
- Rust command tests plus React/Vitest UI tests.

`T15. Resume rejects unsupported engine without creating a child run`
- Covers: `R6`, `R7`, `R8`, `E2`, `AC5`
- Level: integration
- Fixture/setup: resume-enabled configuration and persisted interrupted run with valid metadata while engine resume capability is disabled.
- Steps:
- Attempt resume through the public command.
- Inspect the returned error code, original run state, and audit evidence.
- Expected result:
- The command returns `UNSUPPORTED_ENGINE`.
- No child run is created.
- The original run remains unchanged.
- A resume-rejected audit/log event is recorded with the stable reason code.
- Failure proves:
- The public contract still advertises executable resume when the engine cannot continue from a persisted cursor.
- Automation location:
- Rust command integration tests.

`T16. Future engine-supported resume creates a child run and preserves the original run record`
- Deferred until the scan engine can continue traversal from a persisted cursor.

`T17. Persisted continuity data excludes file contents and secret resume material`
- Covers: `R7`
- Level: integration
- Fixture/setup: persisted run with representative file names, counts, resume metadata, audit rows, and any structured log capture.
- Steps:
- Inspect stored snapshots, run headers, audit rows, and emitted telemetry payloads.
- Expected result:
- File contents are never persisted in continuity payloads.
- Sensitive resume token material is not exposed in logs, UI read models, or audit payloads beyond the minimum contract.
- Privacy mismatch paths do not leak rejected scope details beyond the specified error code and safe metadata.
- Failure proves:
- Continuity persistence violates the feature’s privacy boundary.
- Automation location:
- Rust integration tests with direct DB inspection and telemetry capture.

`T18. Purge removes expired continuity data, skips retained runs, and emits purge signal`
- Covers: `R7`, `E2`, `AC8`
- Level: integration
- Fixture/setup: mix of expired continuity runs, active/recent interrupted runs, and completed history rows in a temp database; fake clock if retention windows are time-based.
- Steps:
- Execute the purge routine.
- Inspect database state, deletion counts, and observability output.
- Expected result:
- Only eligible continuity data older than retention is deleted.
- Active, recent recoverable, or otherwise retained runs are preserved.
- Deletion is verified, and `scan_run_purged` or equivalent metric/log signal is emitted with the expected count.
- Failure proves:
- Retention either leaks stale continuity data indefinitely or deletes state that is still needed for recovery or history.
- Automation location:
- Rust repository or maintenance-job integration tests.

`T19. Run read APIs expose the continuity contract consistently`
- Covers: `R1`, `R3`, `R5`, `R6`, `R8`, `E1`, `AC1`, `AC5`, `AC7`
- Level: contract
- Fixture/setup: representative runs across `RUNNING`, `STALE`, `ABANDONED`, `CANCELLED`, `FAILED`, and `COMPLETED`.
- Steps:
- Call run list/detail commands for each state and for an unknown `runId`.
- Compare payload fields against the spec.
- Expected result:
- Responses include header plus latest snapshot fields needed by the UI.
- Preview and resume metadata are populated only when allowed, and current public actionability is represented by `can_resume` alone.
- Unknown `runId` returns the specified not-found behavior.
- Terminal and recoverable states do not conflict across payload fields.
- Failure proves:
- The public contract is not stable enough for the frontend or future clients.
- Automation location:
- Rust command contract tests and TypeScript client tests if a typed wrapper exists.

### Frontend compatibility and UX regressions

`T20. Scan history UI renders continuity state, recovery, and resume gating correctly`
- Covers: `R5`, `R6`, `R8`, `E1`, `AC1`, `AC4`, `AC5`
- Level: frontend integration
- Fixture/setup: React/Vitest test using mocked `SpaceSiftClient` responses for `STALE`, `ABANDONED`, resume-disabled, and engine-disabled resume scenarios.
- Steps:
- Render the history/run detail UI for each state.
- Trigger refresh, resume checkbox, and resume action where applicable.
- Expected result:
- Interrupted runs show the correct recovery badge or status.
- `ABANDONED` is visually distinct from `STALE`.
- Resume controls are hidden or disabled when default-off or engine-disabled, and enabled only when the read model allows it.
- Failure proves:
- Backend continuity work is not actually consumable in the existing UI flow.
- Automation location:
- [scan-history.test.tsx](/D:/Data/20260415-space-sift/src/scan-history.test.tsx) or adjacent React/Vitest coverage.

`T21. UI supports cancelling stale or recovered runs without corrupting history`
- Covers: `R8`, `R9`
- Level: frontend integration
- Fixture/setup: mocked client responses for stale and abandoned run detail plus cancel command outcomes.
- Steps:
- Render a recoverable run, trigger cancel, and refresh the view.
- Repeat against an already-terminal run rejection.
- Expected result:
- Successful cancel updates the run to terminal `CANCELLED`.
- Already-terminal rejection is surfaced without optimistic UI corruption.
- Completed history rows remain reopenable and unaffected.
- Failure proves:
- The frontend assumes only live cancel exists and mishandles the new command semantics.
- Automation location:
- React/Vitest history/run view tests.

`T22. Existing completed history behavior remains unchanged`
- Covers: `R1`, `R10`, `AC7`
- Level: frontend integration
- Fixture/setup: extend existing history tests that already cover switching between active scans and persisted completed results.
- Steps:
- Reproduce the current active-to-completed transition.
- Reopen a legacy completed scan.
- Exercise the fast-completion race path after continuity support exists.
- Expected result:
- Completed history continues to order, reopen, and render as before.
- Continuity state for an active run does not overwrite fresh terminal completed UI state.
- Failure proves:
- The new continuity feature regressed the current scan history contract.
- Automation location:
- [scan-history.test.tsx](/D:/Data/20260415-space-sift/src/scan-history.test.tsx)

## Fixtures and data

- Temporary SQLite database fixture shared across repository and command integration tests.
- Continuity schema fixture that includes:
- live `RUNNING` run with ordered snapshots,
- stale candidate with last heartbeat just beyond threshold,
- abandoned candidate older than the abandon window,
- completed run with matching `scan_history` payload,
- legacy completed run without continuity rows,
- optional legacy continuity row without `seq` if migration fallback is implemented.
- Fake clock or timer provider seam for heartbeat, stale, abandon, purge, and token-expiry tests.
- Resume fixture set containing:
- valid target fingerprint,
- engine-disabled variant for the current release posture.
- Deferred future engine-supported resume fixture set:
- changed target fingerprint,
- privacy-scope mismatch variant,
- expired token variant.
- Frontend mocked `SpaceSiftClient` responses matching the command payloads returned by the Rust layer.
- Telemetry/audit capture fixture for `scan_run_no_progress_warning`, reconciliation audit events, cancel audit events, and `scan_run_purged`.

## Mocking and stubbing policy

- Prefer real SQLite-backed repository and command integration tests over pure mocks for continuity persistence boundaries.
- Mock only nondeterministic seams:
- wall clock and timers,
- filesystem scan progression when testing heartbeat without real traversal,
- telemetry sinks or structured log collectors,
- frontend transport client.
- Do not mark a requirement covered by snapshot-only UI assertions; assert concrete status text, actions, and payload-driven behavior.
- Use direct database inspection only for persistence/privacy assertions that are not observable through the public contract.
- Avoid mocking the continuity repository in command tests where ordering, atomicity, or reconciliation semantics are the behavior under test.

## Migration or compatibility tests

- `T13` verifies existing completed history rows and payload variants remain readable without continuity data.
- `T22` verifies the current frontend history flow is not regressed by continuity state.
- If the implementation supports legacy continuity rows created before `seq` was introduced, add repository fixtures that prove deterministic fallback ordering exactly as documented.
- If the implementation intentionally omits legacy continuity-row support because the schema is new, that decision must be reflected back into the feature spec before implementation.

## Observability verification

- Assert `scan_run_no_progress_warning` after the documented unchanged-heartbeat threshold in `T5`.
- Assert reconciliation audit rows or structured events in `T9`, `T10`, and `T11`.
- Assert non-live cancel audit output in `T12`.
- Assert purge telemetry or metric event such as `scan_run_purged` and verified deletion counts in `T18`.
- Verify observability payloads use safe identifiers and counts rather than sensitive path contents in `T17`.

## Security and privacy verification

- `T15` verifies unsupported-engine resume rejection safety and confirms no child run is created in the current release posture.
- Deferred future engine-supported resume tests should verify target mismatch, privacy-scope mismatch, and token-expiry rejection codes.
- `T17` verifies snapshots, headers, audit rows, and telemetry do not persist file contents or leak sensitive resume material.
- Add negative assertions that UI-facing read models do not expose raw resume tokens.
- If preview metadata is user-visible, assert it contains only allowed safe summary data rather than file-content payloads.

## Performance checks

- `T4` and `T5` verify heartbeat cadence is bounded and does not require per-file persistence to prove liveness.
- `T18` verifies purge operates on bounded eligible sets and reports deletion counts.
- No microbenchmark is required for this feature before the first implementation slice because the spec focuses on bounded telemetry rather than throughput optimization.

## Manual QA checklist

- Start a scan, kill or restart the app, reopen, and verify the run appears as recoverable continuity state rather than disappearing.
- Leave an interrupted run beyond the stale threshold but short of abandon threshold and verify it surfaces as `STALE`.
- Leave an interrupted run beyond the abandon threshold and verify it surfaces as `ABANDONED`.
- With resume disabled, verify no resume action is shown.
- With resume metadata present while engine resume support is disabled, verify the UI keeps resume unavailable and the command rejects with `UNSUPPORTED_ENGINE`.
- Cancel a stale or abandoned run and verify it transitions once to `CANCELLED`.
- Complete a scan after continuity persistence is enabled and verify the completed result still reopens through the existing history path.
- Run retention or purge flow against expired continuity data and verify the UI no longer lists purged runs while retained runs remain visible.

## What not to test

- Do not add cloud-sync or cross-device recovery tests; the feature is local continuity only.
- Do not redesign or benchmark the underlying scan algorithm in this test spec; this feature only requires bounded continuity telemetry.
- Do not add broad end-to-end desktop automation for every state transition before the repository and command seams exist; targeted integration tests are the primary proof for the first implementation slices.
- Do not treat unrelated scan-history sorting or duplicate-detection behavior as part of continuity coverage unless the new work changes those contracts.

## Uncovered gaps

- None at this time. The previously blocking `ABANDONED` versus resume-eligibility conflict, legacy ordering fallback concern, and terminal cancel-response ambiguity are now resolved in the governing spec and aligned with the architecture note.

## Readiness for implement

Ready to resume `implement` for the remaining continuity milestones under the current unsupported-engine resume posture. Successful child-run resume remains a deferred future slice until the scan engine can continue from a persisted cursor.
