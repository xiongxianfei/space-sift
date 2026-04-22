# Scan run continuity: change rationale

## 1. Summary

This change introduced durable scan-run continuity for in-progress and interrupted scans without replacing the existing completed-scan history model. The implementation added additive SQLite-backed run persistence, ordered snapshots, restart reconciliation, non-live cancellation, retention and audit signals, and a run-oriented read model and UI.

The final release posture is intentionally conservative on resume:

- runs may persist resume metadata when the advanced opt-in is enabled;
- public read models expose only `has_resume` and `can_resume`;
- `can_resume` is currently `false` because the scan engine cannot yet continue from a persisted traversal cursor;
- `resume_scan_run` rejects with `UNSUPPORTED_ENGINE` and does not create a child run.

The change also repaired the repository CI parity path by normalizing `scripts/ci.sh` to LF, enforcing LF for shell scripts in `.gitattributes`, and proving the canonical CI script under native Windows Git Bash.

## 2. Problem

Before this initiative:

- live scan state existed only in memory;
- a restart or crash lost active-run progress;
- completed explorer history and live scan state were separate, but there was no durable continuity layer between them;
- the UI could not surface stale or abandoned interrupted runs after startup;
- there was no contract for ordered snapshots, heartbeat-driven liveness, non-live cancellation, or run-retention behavior.

The feature goal was to make interrupted scans durable and explainable while preserving the existing `scan_history` completed-result contract.

## 3. Decision trail

### Exploration and proposal

- The earlier proposal recommended finishing active-scan UX work first and treating persisted run continuity as a bounded follow-up.
- The user explicitly approved that bounded follow-up, and the plan records that direct override of the proposal’s O1-first sequencing preference.

### Governing requirements

The implementation follows the continuity feature spec in [space-sift-scan-run-continuity.md](../..//specs/space-sift-scan-run-continuity.md), especially:

- `R1` lifecycle continuity
- `R2` deterministic snapshot ordering
- `R3` bounded progress metrics
- `R4` heartbeat cadence and stale detection
- `R5` crash/shutdown continuation
- `R6` explicit, gated resumability with current `UNSUPPORTED_ENGINE` posture
- `R7` privacy, retention, and token non-exposure
- `R8` API and UX consistency
- `R9` failure handling
- `R10` compatibility with existing completed history

### Architecture and ADR decisions

The architecture and ADR drove these structural decisions:

- keep `scan_history` as the completed-result store;
- add additive `scan_runs`, `scan_run_snapshots`, and `scan_run_audit` tables in the same SQLite database;
- treat the latest persisted snapshot as authoritative and mirror it into `scan_runs` for indexed reads;
- keep heartbeat ownership outside `scan-core`;
- finalize completed continuity state and `scan_history` together in one SQLite transaction;
- treat true child-run resume as deferred target design until the engine can continue from a persisted cursor.

### Plan milestones

The implementation followed the milestone structure in [2026-04-18-scan-run-continuity.md](../plans/2026-04-18-scan-run-continuity.md):

- `M1` schema, DTOs, and time seam
- `M2` live write-through and atomic completion
- `M3` heartbeat and no-progress observability
- `M4` restart reconciliation and run read APIs
- `M5` non-live cancellation, retention, and purge
- `M6` resume-gated flow and frontend integration
- `M7` integrated verification and closeout

## 4. Diff rationale by area

| File(s) | Change | Reason | Source artifact | Test / evidence |
| --- | --- | --- | --- | --- |
| [src-tauri/crates/scan-core/src/lib.rs](/D:/Data/20260415-space-sift/src-tauri/crates/scan-core/src/lib.rs) | Added shared run DTOs and hid raw resume fields with `#[serde(skip_serializing)]`; introduced `SCAN_RESUME_ENGINE_SUPPORTED = false`. | The backend and frontend needed one continuity contract, but raw resume metadata could not leak through public read models. The public resume contract also needed one actionability signal only. | `R6`, `R7`, `R8`; architecture current implementation posture; code review correction | `cargo check`, `commands::scan::tests::`, serialization assertions in repository tests |
| [src-tauri/crates/app-db/src/lib.rs](/D:/Data/20260415-space-sift/src-tauri/crates/app-db/src/lib.rs) | Added additive run tables; implemented run creation, ordered snapshot append, atomic completion, reconciliation, purge, audit writes, resume-rejection audit, and read-model helpers including `scan_run_resume_flags(...)`. | `HistoryStore` is the approved persistence boundary. Continuity needed durable, ordered, restart-safe storage without changing completed-history payload behavior. | `R1`-`R5`, `R7`-`R10`; architecture/ADR additive SQLite decision | `cargo test --manifest-path src-tauri/Cargo.toml -p app-db scan_run_` |
| [src-tauri/src/state/mod.rs](/D:/Data/20260415-space-sift/src-tauri/src/state/mod.rs) | Added injected clock seam, continuity cursor state, heartbeat scheduling, and no-progress warning tracking in `ScanManager`. | Heartbeats had to be owned outside `scan-core`, and later milestones needed deterministic timing in tests. | `R3`, `R4`; architecture heartbeat ownership | continuity time and heartbeat tests recorded in the plan |
| [src-tauri/src/commands/scan.rs](/D:/Data/20260415-space-sift/src-tauri/src/commands/scan.rs) | Persisted runs at start; emitted ordered live snapshots; added heartbeat loop, non-live cancel handling, structured purge and resume-rejection signals, and current unsupported-engine `resume_scan_run` behavior. | The command layer owns orchestration between live scan execution, repository writes, UI events, and defensive error semantics. | `R1`, `R4`, `R6`, `R8`, `R9`; architecture command/runtime ownership | `commands::scan::tests::`; `npm run tauri dev`; `cargo check` |
| [src-tauri/src/commands/history.rs](/D:/Data/20260415-space-sift/src-tauri/src/commands/history.rs) | Added additive `list_scan_runs` and `open_scan_run` commands with machine-readable `NOT_FOUND` mapping and snapshot preview paging. | Continuity required separate run-oriented APIs without overloading the legacy completed-history command surface. | `R5`, `R8`, `R10`; architecture additive API rule | continuity open/list run tests recorded in the plan |
| [src/lib/spaceSiftTypes.ts](/D:/Data/20260415-space-sift/src/lib/spaceSiftTypes.ts), [src/lib/spaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/spaceSiftClient.ts), [src/lib/tauriSpaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/tauriSpaceSiftClient.ts) | Added run detail/summary types and continuity commands, and removed the temporary public `resumeSupported` field. | The frontend needed typed access to the new run APIs, but the public contract had to stay honest: `canResume` is the only actionability field. | `R6`, `R8`, `R10`; code-review correction | frontend tests and TypeScript build |
| [src/App.tsx](/D:/Data/20260415-space-sift/src/App.tsx) | Added advanced scan option for interrupted-run metadata, surfaced interrupted-run cards, rendered required summary fields, and gated Resume solely on `canResume`. | The UI had to surface recoverable runs and their state without displacing the existing completed-results explorer. | `R6`, `R8`; spec UI requirements; M6 plan result | [scan-history.test.tsx](/D:/Data/20260415-space-sift/src/scan-history.test.tsx), `npm run test`, `npm run build` |
| [src/scan-history.test.tsx](/D:/Data/20260415-space-sift/src/scan-history.test.tsx) | Added frontend regressions for interrupted-run cards, required metrics, and disabled resume under engine-disabled posture. | The UI contract is behavior, not implementation detail; it needed direct proof. | `T20`, `T21`, `AC5` | `npm run test` |
| [src/App.test.tsx](/D:/Data/20260415-space-sift/src/App.test.tsx), [src/cleanup.test.tsx](/D:/Data/20260415-space-sift/src/cleanup.test.tsx), [src/duplicates.test.tsx](/D:/Data/20260415-space-sift/src/duplicates.test.tsx), [src/results-explorer.test.tsx](/D:/Data/20260415-space-sift/src/results-explorer.test.tsx) | Updated shared client mocks to satisfy the expanded `SpaceSiftClient` contract. | These files were not changed for new feature behavior; they were kept compiling and behavior-stable after the client contract expanded. | Compatibility fallout from `M6` | `npm run test` |
| [specs/space-sift-scan-run-continuity.md](/D:/Data/20260415-space-sift/specs/space-sift-scan-run-continuity.md), [specs/space-sift-scan-run-continuity.test.md](/D:/Data/20260415-space-sift/specs/space-sift-scan-run-continuity.test.md), [docs/architecture/2026-04-18-scan-run-continuity.md](/D:/Data/20260415-space-sift/docs/architecture/2026-04-18-scan-run-continuity.md), [docs/plans/2026-04-18-scan-run-continuity.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-18-scan-run-continuity.md), [docs/plan.md](/D:/Data/20260415-space-sift/docs/plan.md) | Kept the contract and lifecycle artifacts aligned with the shipped behavior, especially the late correction from “future child-run resume” to current `UNSUPPORTED_ENGINE` gating and final plan closeout. | The feature changed materially during review. Durable artifacts had to reflect the final release posture, not the earlier intended target design. | Constitution artifact-alignment rules; `M6`/`M7`; verify findings | final verify pass and plan closeout |
| [scripts/ci.sh](/D:/Data/20260415-space-sift/scripts/ci.sh), [.gitattributes](/D:/Data/20260415-space-sift/.gitattributes) | Normalized shell line endings and kept the canonical CI-parity script executable under native Windows Git Bash. | Verification found that the canonical CI gate itself had to be reliable before the branch could be treated as ready. | Constitution / AGENTS CI gate rules | `D:\Software\Git\bin\bash.exe scripts/ci.sh` passed |

## 5. Tests added or changed

### Repository and persistence tests

The `app-db` test suite was expanded to prove the persistence contract at the repository boundary:

- ordered snapshot append and latest-snapshot authority;
- counter regression rejection;
- atomic completion finalization;
- startup reconciliation to `STALE` and `ABANDONED`;
- purge eligibility and preserved audit evidence;
- public read-model metrics for interrupted-run summaries;
- `has_resume = true` and `can_resume = false` under the current unsupported-engine posture;
- no raw resume token exposure in serialized read models.

Why this level was appropriate:

- these behaviors are primarily SQLite and repository invariants;
- mocking them would miss the exact ordering, transaction, and schema guarantees the spec required.

### Command-layer tests

The command test module in [scan.rs](/D:/Data/20260415-space-sift/src-tauri/src/commands/scan.rs) proves:

- live sequencing and finalization behavior;
- heartbeat serialization and fatal-stop behavior;
- non-live cancel conflict and terminal rules;
- structured purge signal payloads;
- structured resume-rejection payloads;
- unsupported-engine resume rejection with matching audit evidence and no child run.

Why this level was appropriate:

- the behavior under test spans runtime state, persistence, and command error shaping;
- repository-only tests would not prove the public command contract or observability side effects.

### Frontend tests

The React/Vitest coverage proves:

- interrupted runs render with the required fields (`seq`, `created_at`, `items_scanned`, `errors_count`, percent, rate);
- resume is disabled from `canResume`, not from any hidden capability field;
- existing screens remain stable after the shared client contract expanded.

Why this level was appropriate:

- the spec explicitly defines UI-visible behavior;
- these are user-facing contract checks, not implementation trivia.

## 6. Verification evidence

### Targeted validation

The following commands were run successfully during implementation and correction:

- `cargo test --manifest-path src-tauri/Cargo.toml -p app-db scan_run_`
- `cargo test --manifest-path src-tauri/Cargo.toml commands::scan::tests::`
- `npm run lint`
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run test`
- `npm run tauri dev`

Observed results:

- Rust repository tests passed;
- command-layer tests passed;
- full Vitest suite passed with `37` tests;
- TypeScript build passed;
- desktop dev startup launched successfully.

### Canonical CI parity

Because this repository treats `bash scripts/ci.sh` as the canonical CI-parity gate, the change also verified:

- `D:\Software\Git\bin\bash.exe scripts/ci.sh`

Result:

- passed

Important context:

- the default `bash` on this workstation resolves to WSL and does not expose `npm` or `cargo` on `PATH`;
- that is an environment limitation, not a repository-script failure;
- the explanation and plan therefore record the compatible proof surface explicitly.

## 7. Alternatives rejected

- **Expose a public `resumeSupported` boolean**
  - rejected after review because it created a contradictory truth-table alongside `has_resume` and `can_resume`.
- **Fake resume by starting a fresh root scan**
  - rejected because it would falsely advertise executable resume while not actually continuing from persisted state.
- **Replace or overload `scan_history`**
  - rejected by the architecture and ADR because completed explorer payloads are already stable consumers of that table.
- **Header-only run state with no ordered snapshots**
  - rejected because the feature required deterministic history, preview, and reconciliation auditability.
- **Mutate the original run in place on resume**
  - rejected by the architecture/ADR because it would erase stale/failed lineage and make history harder to reason about.

## 8. Scope control

The implementation preserved the agreed non-goals:

- no multi-scan concurrency or run queue;
- no scan-engine redesign or MFT fast path;
- no cloud or account-scoped persistence;
- no replacement of the completed `scan_history` model;
- no automatic resume;
- no executable child-run resume while engine cursor continuation is still missing.

The only workflow-level change outside the feature itself was the CI-script portability fix needed to satisfy the project’s required branch-readiness gate.

## 9. Risks and follow-ups

- **Deferred capability**: true child-run resume remains future work until `scan-core` can continue from a persisted traversal cursor.
- **Environment note**: on this workstation, plain `bash` resolves to WSL, so canonical CI proof should continue to use native Windows Git Bash or hosted CI.
- **Architecture posture**: the architecture doc intentionally keeps both the current unsupported-engine posture and the deferred target design visible; future resume work should start from that target section rather than reopening the current contract.

## PR-ready summary bullets

- Added additive SQLite-backed scan-run continuity with ordered snapshots, restart reconciliation, non-live cancellation, retention, and audit signals.
- Preserved existing completed explorer history by keeping `scan_history` as the completed-result store.
- Shipped a conservative public resume contract: `has_resume` plus `can_resume`, with current command behavior rejecting `UNSUPPORTED_ENGINE` and creating no child run.
- Updated the frontend to surface interrupted runs and required continuity metrics without displacing existing completed-history flows.
- Aligned the feature spec, test spec, architecture note, plan, and plan index with the shipped behavior and closed the initiative through `M7`.
- Repaired and proved the canonical CI-parity script under native Windows Git Bash.
