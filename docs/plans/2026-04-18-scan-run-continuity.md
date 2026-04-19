# Implement Scan Run Continuity And Optional Resume

## Metadata

- Status: draft
- Owner: xiongxianfei / Codex
- Start date: 2026-04-18
- Last updated: 2026-04-18
- Related issue or PR: none yet
- Supersedes: none

## Purpose / big picture

Implement additive scan-run continuity so active scans become durable run records, restart recovery is explicit, and optional resume can be enabled without destabilizing the existing completed-scan history and results explorer flows.

## Source artifacts

- Proposal: [2026-04-18-scan-resumption-and-run-clarity-proposal.md](../proposals/2026-04-18-scan-resumption-and-run-clarity-proposal.md)
- Spec: [space-sift-scan-run-continuity.md](../../specs/space-sift-scan-run-continuity.md)
- Architecture: [2026-04-18-scan-run-continuity.md](../architecture/2026-04-18-scan-run-continuity.md)
- ADR: [2026-04-18-scan-run-persistence-and-resume.md](../adr/2026-04-18-scan-run-persistence-and-resume.md)
- Test spec: [space-sift-scan-run-continuity.test.md](../../specs/space-sift-scan-run-continuity.test.md)

## Why now

The repository already improved active-scan UX and bounded telemetry, but it still loses run state on restart because live scan progress only exists in memory. The new continuity contract adds persistence, recovery, and optional resume on the same local SQLite path already used for completed history.

This initiative is proceeding now because the user explicitly requested the continuity proposal, spec, architecture, and plan flow on 2026-04-18. That direct request allows this bounded O2 follow-up to move forward now even though the proposal recommended an O1-first sequence. The plan therefore assumes this work is approved to proceed, but it must still avoid destabilizing the active scan UX contract already covered by the existing scan-progress plan.

## Context and orientation

- The current live scan path is:
  - `src-tauri/crates/scan-core/src/lib.rs` emits `ScanStatusSnapshot` values during traversal.
  - `src-tauri/src/state/mod.rs` keeps one active in-memory scan and the latest live snapshot.
  - `src-tauri/src/commands/scan.rs` starts scans, cancels the active scan, finalizes completion, and emits `scan-progress`.
- The current durable scan history path is:
  - `src-tauri/crates/app-db/src/lib.rs` stores completed scan payloads in `scan_history`.
  - `src-tauri/src/commands/history.rs` lists and reopens completed history rows.
  - `src/lib/spaceSiftTypes.ts`, `src/lib/spaceSiftClient.ts`, and `src/lib/tauriSpaceSiftClient.ts` mirror the current command/event contract.
- Existing frontend tests already cover active-scan UI transitions and completed-history reopen behavior in `src/scan-history.test.tsx` and related test files.
- Key constraints from the spec and architecture:
  - one active scan at a time remains the runtime rule
  - `scan_history` remains the source of truth for completed explorer payloads
  - continuity storage must be additive and SQLite-backed
  - the latest persisted snapshot is authoritative; `scan_runs` mirrors it for indexed reads
  - heartbeat ownership lives outside `scan-core`
  - resume stays explicit and off by default
  - non-live cancellation is command-response plus explicit refresh in the first pass
  - `ABANDONED` is visible as a badge in the first pass, not a dedicated filter
  - implementation should not start until `plan-review` and `test-spec` are ready
- Coordination rule with active O1 work:
  - backend continuity milestones may proceed now
  - any overlapping UI file work must remain additive to the active-scan UX contract already implemented under `2026-04-16-scan-progress-and-active-run-ux.md`
  - continuity UI surfaces land only after the backend run contract is stable

## Non-goals

- Replacing or migrating away from the existing `scan_history` completed payload model
- Multi-scan concurrency or background scan queues
- Scan algorithm redesign, MFT parsing, or broader scan-performance refactors
- Cloud sync, remote persistence, or account-scoped run recovery
- Inventing a separate diagnostics product surface beyond the minimal run detail/read model
- Reworking the current active-scan framing rules already covered by the existing scan-progress plan unless the continuity spec explicitly requires it

## Requirements covered

| Requirement | Planned implementation area |
| --- | --- |
| `R1` lifecycle continuity | M1 schema/repository primitives, M2 live write-through |
| `R2` deterministic ordering | M1 `(run_id, seq)` schema + repository validation, M2 runtime sequencing |
| `R3` bounded progress metrics | M2 shared snapshot fields, M3 no-progress observability |
| `R4` heartbeat and staleness | M1 time seam, M3 heartbeat timer, M4 reconciliation |
| `R5` crash/shutdown continuation | M4 startup reconciliation and run read model |
| `R6` optional resumability | M6 resume token/fingerprint flow and explicit UI affordance |
| `R7` privacy/security/retention | M1 schema guardrails, M5 purge and audit behavior |
| `R8` API and UX consistency | M4 run commands/read model, M6 TypeScript and UI integration |
| `R9` failure handling | M2 atomic completion/fallback failure handling, M5 cancel semantics |
| `R10` migration compatibility | M1 additive schema, M4 legacy-empty behavior, M6 UI compatibility |

## Milestones

### M1. Continuity schema, DTOs, and time-control seam

- Goal:
  - Add additive SQLite tables, Rust DTOs, repository methods, and explicit time-control seams needed for deterministic heartbeat, stale, abandon, purge, and token-expiry tests.
- Requirements:
  - `R1`, `R2`, `R4`, `R7`, `R10`
- Files/components likely touched:
  - `src-tauri/crates/app-db/src/lib.rs`
  - `src-tauri/src/state/mod.rs`
  - `src-tauri/src/commands/scan.rs`
  - `src-tauri/crates/scan-core/src/lib.rs`
- Dependencies:
  - reviewed spec and architecture
- Tests to add/update:
  - `app-db` tests for table creation, ordered snapshot append, header mirror invariants, legacy completed-history compatibility, and finalization scaffolding
  - command/state tests for injected clock or timer-provider seams
- Implementation steps:
  - add `scan_runs`, `scan_run_snapshots`, and `scan_run_audit` schema plus indices
  - define Rust DTOs for run headers, snapshots, summaries, details, and resume eligibility
  - implement repository methods for create-run, append-snapshot, finalize-completed-run, list/open runs, and audit writes
  - introduce an injected clock or timer seam in the runtime/command layer so later milestones do not depend on real multi-minute waits
  - lock planning defaults in code-facing constants where appropriate: page size `20`, privacy scope constant, stable persistence error code names
- Validation commands:
  - `cargo test -p app-db scan_run_`
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_time_`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Expected observable result:
  - the database layer can store and reopen continuity records while legacy `scan_history` behavior remains unchanged, and the runtime has a deterministic time seam for later tests
- Risks:
  - schema drift between Rust DTOs and SQL writes
  - the time seam becoming too invasive for a small bounded change
- Rollback/recovery:
  - the new tables and time seam are additive and can remain unused if later runtime wiring is reverted

### M2. Live write-through sequencing and atomic completion

- Goal:
  - Wire active scans into the repository with authoritative `seq` snapshots and atomic completed finalization.
- Requirements:
  - `R1`, `R2`, `R3`, `R8`, `R9`
- Files/components likely touched:
  - `src-tauri/src/commands/scan.rs`
  - `src-tauri/src/state/mod.rs`
  - `src-tauri/crates/scan-core/src/lib.rs`
  - `src-tauri/crates/app-db/src/lib.rs`
- Dependencies:
  - M1
- Tests to add/update:
  - command-layer tests for monotonic `seq`
  - finalization tests proving `COMPLETED` is not published before durable `scan_history` persistence
  - failure-path tests for snapshot write errors and fallback `FAILED` finalization
- Implementation steps:
  - extend `ScanStatusSnapshot` and related Rust contracts with additive continuity fields
  - persist initial run header plus `seq = 1` snapshot on `start_scan`
  - track next `seq`, last activity time, and last persisted counters in `ScanManager`
  - finalize completed runs through one transaction that writes both the terminal continuity state and the completed payload
  - preserve current active-scan cancellation semantics for the live worker path
- Validation commands:
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_seq_`
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_finalization_`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Expected observable result:
  - a running scan produces durable ordered snapshots and a completed scan is reopenable only after the final transaction commits
- Risks:
  - write-through persistence reintroducing too much overhead during large scans
  - same-`scanId` fast-finish races between live events and awaited command responses
- Rollback/recovery:
  - revert runtime wiring while keeping additive schema in place; completed-history reopen continues to work

### M3. Heartbeat timer and no-progress observability

- Goal:
  - Add command-owned heartbeats and explicit no-progress warning behavior without making `scan-core` responsible for timer liveness.
- Requirements:
  - `R3`, `R4`
- Files/components likely touched:
  - `src-tauri/src/commands/scan.rs`
  - `src-tauri/src/state/mod.rs`
  - `src-tauri/crates/app-db/src/lib.rs`
- Dependencies:
  - M1, M2
- Tests to add/update:
  - heartbeat cadence tests using the injected time seam
  - tests proving `last_progress_at` does not advance on heartbeat-only snapshots
  - tests proving `scan_run_no_progress_warning` is emitted after four unchanged heartbeat intervals
- Implementation steps:
  - add a command/runtime-owned heartbeat timer that emits liveness snapshots when traversal is quiet
  - keep `last_progress_at` tied to activity snapshots only
  - emit the named `scan_run_no_progress_warning` operational signal after four unchanged heartbeat intervals
- Validation commands:
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_heartbeat_`
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_no_progress_`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Expected observable result:
  - healthy but quiet scans stay live, and long no-progress streaks are observable without forcing terminal failure
- Risks:
  - heartbeat timer races with fast terminal transitions
  - heartbeat writes becoming too frequent under large scans
- Rollback/recovery:
  - disable the heartbeat timer path while keeping sequence-persisted activity snapshots intact

### M4. Restart reconciliation and run read APIs

- Goal:
  - Make continuity data recoverable across restarts and expose run-oriented read behavior without breaking existing history commands.
- Requirements:
  - `R4`, `R5`, `R8`, `R10`
- Files/components likely touched:
  - `src-tauri/src/lib.rs`
  - `src-tauri/src/commands/history.rs`
  - `src-tauri/src/commands/scan.rs` or a new run-focused command module
  - `src-tauri/crates/app-db/src/lib.rs`
- Dependencies:
  - M1, M2, M3
- Tests to add/update:
  - reconciliation precedence tests proving at most one synthetic transition per startup pass
  - run-detail invariant tests proving header status and latest snapshot stay aligned
  - legacy database tests proving empty run lists do not break older completed history
- Implementation steps:
  - invoke `reconcile_scan_runs` during app startup
  - implement `list_scan_runs` and `open_scan_run`
  - append synthetic `STALE` and `ABANDONED` snapshots through repository primitives with explicit precedence
  - surface `last_progress_at` in the run read model
- Validation commands:
  - `cargo test --manifest-path src-tauri/Cargo.toml -p app-db scan_run_reconcile_`
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_open_run_`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Expected observable result:
  - restarting the app surfaces stale or abandoned runs as durable records while legacy completed history remains intact
- Risks:
  - duplicate reconciliation snapshots on repeated startup
  - read APIs drifting from the authoritative latest snapshot contract
- Rollback/recovery:
  - stop calling reconciliation and hide run APIs while leaving existing completed history untouched

### M5. Non-live cancellation, retention, and purge signals

- Goal:
  - Add safe non-live run cancellation plus retention and purge behavior with explicit audit and operational signals.
- Requirements:
  - `R7`, `R8`, `R9`
- Files/components likely touched:
  - `src-tauri/src/commands/scan.rs` or the new run-focused command module
  - `src-tauri/crates/app-db/src/lib.rs`
  - `src-tauri/src/state/mod.rs`
- Dependencies:
  - M1, M2, M4
- Tests to add/update:
  - `cancel_scan_run` tests for `404`, `409`, and live-run delegation
  - purge tests excluding active or recoverable runs
  - audit-row tests for reconciliation and non-live cancellation
  - tests asserting the named purge signal `scan_run_purged` and deletion verification behavior
- Implementation steps:
  - implement `cancel_scan_run`
  - append synthetic non-live `CANCELLED` snapshots for `STALE` and `ABANDONED` runs
  - add retention and purge behavior
  - emit and test the named operational purge signal and audit rows
- Validation commands:
  - `cargo test -p app-db scan_run_purge_`
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_cancel_run_`
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_audit_`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Expected observable result:
  - stale or abandoned runs can be explicitly cancelled, and old terminal rows are purged with verifiable deletion and named operational signals
- Risks:
  - purge behavior accidentally removing recoverable rows
  - cancellation semantics for active and non-live runs diverging unexpectedly
- Rollback/recovery:
  - disable non-live cancel and purge entrypoints while keeping read APIs and continuity rows intact

### M6. Resume-gated flow and frontend/client integration

- Goal:
  - Wire continuity and resume into the TypeScript client and UI while preserving the current active-scan and completed-history experience.
- Requirements:
  - `R6`, `R8`, `R10`
- Files/components likely touched:
  - `src/lib/spaceSiftTypes.ts`
  - `src/lib/spaceSiftClient.ts`
  - `src/lib/tauriSpaceSiftClient.ts`
  - `src/App.tsx`
  - `src/App.css`
  - `src/scan-history.test.tsx`
  - `src/App.test.tsx`
- Dependencies:
  - M1 through M5
- Tests to add/update:
  - client contract tests for new run commands
  - backend tests for resume rejection codes and child-run creation
  - UI regressions for recoverable run summaries, `ABANDONED` badge display, and non-live cancel refresh
  - UI regressions proving resume is off by default and enabled only through the advanced checkbox
  - legacy history regressions proving completed-scan reopen behavior still wins for explorer flows
- Implementation steps:
  - expose run continuity types and commands in the shared TS client contract
  - add continuity summaries and details to the main app shell without displacing the completed results explorer
  - add the advanced `resume_enabled` checkbox on scan start, collapsed by default
  - wire `resume_scan_run` and non-live cancel flows through explicit refresh
  - keep `ABANDONED` as a badge/state, not a dedicated filter, in the first pass
- Validation commands:
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_resume_`
  - `npm run test -- scan`
  - `npm run test -- history`
  - `npm run lint`
  - `npm run build`
- Expected observable result:
  - the user can view recoverable runs, explicitly enable resume for new runs, and resume or cancel stale runs without losing the existing completed-history experience
- Risks:
  - UI state collisions between active live scans, recovered runs, and completed result panels
  - overexposing resume before the backend invariants are proven
- Rollback/recovery:
  - hide continuity UI surfaces while keeping backend APIs additive and dormant

### M7. Integrated verification and closeout

- Goal:
  - Verify cross-layer behavior against the spec, record real validation evidence, and close the plan with durable notes.
- Requirements:
  - `R1` through `R10`
- Files/components likely touched:
  - `docs/plans/2026-04-18-scan-run-continuity.md`
  - any fallout fixes discovered during validation
- Dependencies:
  - M1 through M6
- Tests to add/update:
  - only fallout regressions discovered by integrated verification
- Implementation steps:
  - run the smallest relevant checks first, then the repo-wide gates
  - perform manual restart/recovery smoke tests through the desktop flow
  - explicitly verify operational signals and audit evidence, not just functional UI state
  - capture validation notes, surprises, and any deferred follow-up work directly in this plan
- Validation commands:
  - `npm run test`
  - `npm run lint`
  - `npm run build`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `npm run tauri dev`
- Expected observable result:
  - a reviewer can exercise continuity flows end to end without contradictory run states or broken legacy history, and can point to named no-progress, audit, and purge signals
- Risks:
  - long real-time thresholds making manual smoke validation slow if time seams are not used consistently
  - platform-specific Tauri prerequisites missing locally
- Rollback/recovery:
  - if integrated verification shows cross-layer instability, keep the schema and backend scaffolding while hiding resume/recovery UI until fixed

## Validation plan

- Pre-implementation gates:
  - run `plan-review`
  - write a matching `test-spec`
- Backend milestone checks:
  - `cargo test -p app-db scan_run_`
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Frontend milestone checks:
  - `npm run test -- scan`
  - `npm run test -- history`
  - `npm run lint`
  - `npm run build`
- Final integrated checks:
  - `npm run test`
  - `npm run lint`
  - `npm run build`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `npm run tauri dev`
- Operational assertions that must be covered by targeted tests before M7 is complete:
  - `scan_run_no_progress_warning`
  - `scan_run_purged`
  - reconciliation audit rows
  - non-live cancellation audit rows
  - purge deletion verification behavior
- Manual smoke checks for M7:
  - start a scan and confirm durable `seq` progress is visible
  - restart during an active run and confirm a recoverable `STALE` or `ABANDONED` record appears according to threshold or test-harness conditions
  - cancel a stale recovered run and confirm it becomes terminal without a duplicate active-run event
  - complete a run and confirm the completed explorer payload is reopenable only after terminal continuity state is durable
  - verify legacy completed history entries still reopen without continuity metadata

## Risks and recovery

- The biggest technical risk is dual-write finalization drift between continuity rows and `scan_history`; mitigate by making completion atomic and by testing contradictory-terminal-state failures directly.
- Heartbeat logic can regress into false liveness or excessive writes; mitigate with command-owned timers, `last_progress_at`, the explicit time seam, and named no-progress warnings.
- Resume can couple too tightly to current traversal internals; mitigate by keeping it opt-in, off by default, and validated through explicit fingerprint and privacy-scope checks.
- Recovery logic can append duplicate synthetic snapshots across restarts; mitigate with explicit reconciliation precedence and idempotence tests.
- UI overlap with the active scan-progress plan can create churn in shared files; mitigate by landing backend milestones first and deferring continuity UI surfaces until M6.
- The safest rollback path is feature-surface rollback, not schema rollback: keep additive tables, hide or stop using the new APIs/UI, and preserve legacy completed history behavior.

## Dependencies

- Internal sequencing:
  - `plan-review` before implementation
  - `test-spec` before implementation
  - M2 depends on M1
  - M3 depends on M1 and M2
  - M4 depends on M1 through M3
  - M5 depends on M1, M2, and M4
  - M6 depends on M1 through M5
  - M7 depends on all prior milestones
- Product sequencing:
  - this plan is allowed to proceed now by direct user request despite the proposal's O1-first preference
  - active-scan UX behavior from `2026-04-16-scan-progress-and-active-run-ux.md` remains the baseline contract for overlapping UI files until M6 explicitly extends it
- Tooling/runtime:
  - Rust/Tauri workspace remains buildable with current local prerequisites
  - desktop smoke validation depends on a working local Tauri runtime
- Testing support:
  - the injected clock or timer seam from M1 is a prerequisite for deterministic stale, abandon, retention, heartbeat, and resume-expiry tests

## Progress

- [x] M1. Continuity schema, DTOs, and time-control seam
- [x] M2. Live write-through sequencing and atomic completion
- [x] M3. Heartbeat timer and no-progress observability
- [x] M4. Restart reconciliation and run read APIs
- [ ] M5. Non-live cancellation, retention, and purge signals
- [ ] M6. Resume-gated flow and frontend/client integration
- [ ] M7. Integrated verification and closeout

## Decision log

- 2026-04-18: proceed with this continuity follow-up now by direct user request despite the proposal's O1-first sequencing preference.
  - Rationale: direct user intent is the highest-priority source, and the spec/architecture now define a bounded path for the work.
- 2026-04-18: plan backend-first and keep the existing completed-history contract intact.
  - Rationale: the highest-risk work is persistence, finalization, and restart recovery. UI should follow the stable backend contract rather than lead it.
- 2026-04-18: keep continuity APIs additive instead of overloading `list_scan_history` and `open_scan_history`.
  - Rationale: completed explorer payloads and continuity runs serve different user tasks and have different compatibility needs.
- 2026-04-18: make the injected clock or timer seam explicit milestone work rather than an assumed test helper.
  - Rationale: heartbeat, stale, abandon, purge, and expiry behavior are all time-based and need deterministic coverage.
- 2026-04-18: keep resume opt-in and surface it as an advanced checkbox in the first pass.
  - Rationale: this satisfies the explicit-per-run contract while minimizing accidental scope expansion and support burden.
- 2026-04-18: keep `ABANDONED` visible as a badge but defer dedicated filtering.
  - Rationale: the spec requires visibility, not a full workflow taxonomy.
- 2026-04-18: use command-response plus explicit refresh for non-live cancellation instead of a new event stream.
  - Rationale: stale or recovered runs are not backed by an active worker, so command semantics are simpler and easier to verify.

## Surprises and discoveries

- 2026-04-18: planning draft created after architecture review exposed that status authority, terminal finalization, and heartbeat ownership had to be made explicit before implementation sequencing was safe.
- 2026-04-18: plan review found that the original M2 and M3 slices were still too broad and that time-control infrastructure could not stay implicit.

## Validation notes

- 2026-04-18: planning-only turn. No implementation, build, lint, or test commands were run while drafting and revising this plan.
- 2026-04-18: planning inputs reviewed included:
  - `AGENTS.md`
  - `.codex/CONSTITUTION.md`
  - `.codex/PLANS.md`
  - `docs/plan.md`
  - `docs/proposals/2026-04-18-scan-resumption-and-run-clarity-proposal.md`
  - `specs/space-sift-scan-run-continuity.md`
  - `docs/architecture/2026-04-18-scan-run-continuity.md`
  - `docs/adr/2026-04-18-scan-run-persistence-and-resume.md`
  - `docs/workflows.md`
  - existing scan runtime, state, history-store, and TypeScript client modules previously reviewed during proposal and architecture work

## Outcome and retrospective

Expected outcome:

- Space Sift can recover or explicitly abandon interrupted scan runs without confusing them with completed history, and optional resume remains bounded, explicit, and locally durable.

Retrospective focus:

- whether the command-owned heartbeat stays honest without excessive writes
- whether the additive run API remains simpler than overloading completed history
- whether the first-pass resume affordance is appropriately cautious or too hidden
- whether the operational signals (`scan_run_no_progress_warning`, `scan_run_purged`, audit rows) are useful enough for support and debugging

## 2026-04-18 implementation update: M1 schema and repository groundwork

- Progress: completed the first implementation slice for continuity persistence groundwork.
- Completed:
  - added shared continuity DTOs in `scan-core` for run headers and ordered snapshots with the continuity fields needed by the approved schema baseline;
  - added additive SQLite tables in `HistoryStore` for `scan_runs`, `scan_run_snapshots`, and `scan_run_audit` with `created_at`, resume/privacy placeholders, and mirrored latest-state fields;
  - added a deterministic repository clock seam via `HistoryStore::with_now(...)` for timestamp-sensitive tests;
  - added repository methods to record a started run, append ordered snapshots, validate monotonic counters, and reopen the run detail view;
  - added a runtime clock seam in `ScanManager::with_now(...)` so command-owned timing behavior can be tested without wall-clock waits.
- Decisions:
  - `scan_runs` mirrors the latest snapshot status and `latest_seq`, while `scan_run_snapshots` remains the ordered event history;
  - `scan_run_snapshots.created_at` was included now so the documented legacy ordering fallback has a real persisted column to target later.
- Surprises:
  - the existing persistence layer and runtime state had no reusable time seam, so this milestone had to introduce both repository and runtime clock hooks before heartbeat work can land cleanly.
- Validation:
  - `cargo test --manifest-path src-tauri/crates/app-db/Cargo.toml` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_time_` passed.
  - `cargo check --manifest-path src-tauri/Cargo.toml` passed.
- Follow-ups:
  - live write-through from the scan runtime, atomic completion integration, and command/API exposure remain in later milestones;
  - the previously documented continuity contract gaps around `ABANDONED` versus valid resume and already-terminal cancel responses still need to be resolved before the affected milestones.

## 2026-04-19 implementation update: M2 live write-through and atomic completion

- Progress: completed the second implementation slice for continuity write-through and atomic completion.
- Completed:
  - persisted the initial continuity run record on `start_scan` before the worker is spawned;
  - added command-layer conversion from live `ScanStatusSnapshot` values into ordered persisted `ScanRunSnapshot` rows;
  - used `ScanManager` continuity cursor state to allocate `seq`, track last persisted snapshot timing, and compute bounded scan-rate values;
  - added atomic `HistoryStore.finalize_completed_scan_run(...)` so the terminal `COMPLETED` continuity snapshot and `scan_history` payload commit in one SQLite transaction;
  - added rollback coverage proving failed completed-payload writes do not leave visible `COMPLETED` continuity state;
  - appended best-effort terminal `FAILED` or `CANCELLED` continuity snapshots for live terminal paths.
- Decisions:
  - command-layer progress events update the in-memory UI state only after the continuity snapshot write succeeds, keeping the persisted record authoritative for later recovery work;
  - the repository owns the final atomic boundary for completed scans rather than trying to coordinate dual writes in the command layer.
- Surprises:
  - the Tauri crate needed an explicit `chrono` dependency even though the workspace already defined it centrally; M2 rate calculation uses that crate directly.
- Validation:
  - `cargo test --manifest-path src-tauri/crates/app-db/Cargo.toml` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_time_` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_seq_` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_finalization_` passed.
  - `cargo check --manifest-path src-tauri/Cargo.toml` passed.
- Follow-ups:
  - M3 still needs command-owned heartbeat emission and no-progress observability; M2 only establishes the persistence and cursor path that heartbeat will reuse.
  - fallback terminal persistence on catastrophic repository failure remains best-effort; structured observability for that path belongs with later milestones.

## 2026-04-19 M2 follow-up corrections

- persisted explicit continuity `error_code` values for unrecoverable snapshot-write and finalization failure paths
- changed continuity header mirroring so `last_snapshot_at` tracks liveness while `last_progress_at` only advances on counter growth
- added structured local failure events for `scan_run_snapshot_write_failed` and `scan_run_finalization_failed`
- added regression tests for liveness-only snapshots, failed-snapshot error-code persistence, structured failure event payloads, and first-error retention
Validation follow-up:
- `cargo test --manifest-path src-tauri/crates/app-db/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml continuity_`
- `cargo test --manifest-path src-tauri/Cargo.toml commands::scan::tests::`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- Result: all passed
## 2026-04-19 M2 follow-up correction: truthful generic failure classification

- split generic scan-execution failure handling from snapshot-write failure handling so only persistence failures persist `SNAPSHOT_WRITE_FAILED`
- kept finalization failures on the dedicated `FINALIZATION_FAILED` path
- added a command-module regression test that persists an ordinary failed run and proves `error_code` remains `NULL`
- Validation:
  - `cargo test --manifest-path src-tauri/Cargo.toml commands::scan::tests::` passed.
  - `cargo check --manifest-path src-tauri/Cargo.toml` passed.
## 2026-04-19 implementation update: M3 heartbeat timer and no-progress observability

- Progress: completed the third implementation slice for command-owned heartbeat emission and explicit no-progress signaling.
- Completed:
  - added `ScanManager::due_heartbeat(...)` and `ScanManager::mark_heartbeat_persisted(...)` so heartbeat scheduling and warning state stay in the runtime layer rather than `scan-core`;
  - added a command-owned heartbeat thread in `scan.rs` that emits persisted liveness snapshots while traversal is quiet and stops before terminal finalization is processed;
  - kept heartbeat snapshots on the existing ordered continuity path so `seq` allocation and header mirroring stay authoritative in the repository layer;
  - emitted the named structured `scan_run_no_progress_warning` event after four unchanged heartbeat intervals;
  - added injected-clock tests for heartbeat cadence and no-progress warning thresholds.
- Decisions:
  - heartbeat cadence is enforced by comparing both `last_activity_at` and `last_persisted_snapshot_at` against the fixed interval, which prevents duplicate writes inside a single 30-second window;
  - no-progress warning state resets on real activity snapshots and stays local to the runtime state until restart reconciliation work lands in later milestones.
- Surprises:
  - the runtime needed explicit warning-state fields in `ActiveScanRuntime`; the earlier continuity cursor was not enough to model repeated unchanged heartbeats safely.
- Validation:
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_heartbeat_` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_no_progress_` passed.
  - `cargo check --manifest-path src-tauri/Cargo.toml` passed.
- Follow-ups:
  - restart reconciliation, `STALE`/`ABANDONED`, and run-read API exposure remain in M4;
  - the current no-progress signal is a structured local log event, while audit/read-model surfacing belongs to later recovery milestones.
## 2026-04-19 M3 follow-up correction: serialized heartbeat writes and fatal-stop behavior

- serialized activity and heartbeat continuity writes behind a shared command-layer persistence gate so only one writer can reserve and append the next `seq` at a time
- stopped heartbeat iterations immediately when cancel is set or a fatal persistence error is already recorded
- added command-module heartbeat integration tests that exercise the shared repository path and verify no additional heartbeats are appended after a recorded fatal error
- Validation:
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_heartbeat_` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_no_progress_` passed.
  - `cargo check --manifest-path src-tauri/Cargo.toml` passed.
## 2026-04-19 implementation update: M4 restart reconciliation and run read APIs

- Progress: completed the fourth implementation slice for startup reconciliation and additive run-history read commands.
- Completed:
  - invoked `HistoryStore.reconcile_scan_runs()` during app startup before the state is managed, so interrupted rows are reconciled before commands are served;
  - added additive backend commands `list_scan_runs` and `open_scan_run` without changing the legacy completed-history command surface;
  - implemented startup reconciliation precedence so `RUNNING` rows older than the abandon threshold become `ABANDONED`, otherwise stale `RUNNING` rows become `STALE`, and already-`STALE` rows can later become `ABANDONED`, with at most one synthetic snapshot per pass;
  - added reconciliation audit writes in the same repository transaction as the synthetic snapshot and read-model mirror update;
  - extended the run read model with preview paging, `last_progress_at`, and conservative `has_resume` / `can_resume` flags.
- Decisions:
  - kept reconciliation in the repository layer so status-transition persistence, header mirroring, and audit writes stay atomic;
  - preserved legacy completed-history behavior by keeping continuity reads on separate additive commands instead of overloading `list_scan_history` or `open_scan_history`.
- Surprises:
  - the M4 read-model work needed paging metadata in `ScanRunDetail` earlier than the frontend milestone because the backend contract already requires a stable snapshot preview shape.
- Validation:
  - `cargo test --manifest-path src-tauri/Cargo.toml -p app-db scan_run_reconcile_` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_open_run_` passed.
  - `cargo check --manifest-path src-tauri/Cargo.toml` passed.
## 2026-04-19 M4 follow-up correction: list/read contract alignment

- aligned `list_scan_runs` summaries with the approved read/list contract by including the default snapshot preview alongside the header, latest snapshot, and resume booleans
- changed `open_scan_run` to return a machine-readable `NOT_FOUND` error payload instead of flattening all repository errors into strings
- added regression tests for list-summary previews and command-level not-found semantics
- Known follow-up:
  - the older spec-vs-architecture gap around whether valid resume eligibility suppresses `ABANDONED` still remains unresolved and should be closed before deeper resume/recovery milestones
- Validation:
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_open_run_` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml continuity_list_run_` passed.
  - `cargo test --manifest-path src-tauri/Cargo.toml -p app-db scan_run_reconcile_` passed.
  - `cargo check --manifest-path src-tauri/Cargo.toml` passed.
