# Proposal: prioritize active-scan UX completion before resumable-session work

## 1. Status

draft

## 2. Problem

Current active initiatives cover large-scan UX clarity and history/duplicate review, but there is no current decision about whether to extend into resumable scans or scan-engine redesign in the same cycle. The project needs a clear sequencing decision so work is reviewable, safety-focused, and aligned with existing contracts.

## 3. Goals

- Complete the in-flight user-visible scan-state clarity work so running scans are explicitly distinguished from completed results.
- Reduce perceived staleness when multiple scans happen, especially on large directories.
- Preserve safety-first cleanup behavior and avoid changing unreviewed high-risk storage/event semantics.
- Keep scope reviewable by favoring a small incremental path with clear rollback points.
- Produce a clean handoff for the next contract/spec step after sequencing is agreed.

## 4. Non-goals

- No NTFS fast-path implementation, no background index daemon, and no service-level scan infrastructure in this cycle.
- No duplicate verification rewrites beyond what the active duplicate plans already define.
- No cleanup rule expansion beyond what current scopes already cover.
- No release workflow or packaging changes.

## 5. Context

- `AGENTS.md` and `CONSTITUTION.md` require behavior changes to flow through proposal/spec/test-spec/plan before implementation.
- `docs/plan.md` already shows two active plans that correspond to near-term scan UX work:
  - `docs/plans/2026-04-16-scan-progress-and-active-run-ux.md`
  - `docs/plans/2026-04-16-history-and-duplicate-review-clarity.md`
- The repository does not include `docs/project-map.md`; `docs/roadmap.md` currently shows no new unapproved work outside active plans.
- The exploration artifact [2026-04-18-next-workstream-explore.md](docs/proposals/2026-04-18-next-workstream-explore.md) already compared five options and recommended O1-first sequencing.

## 6. Options considered

- `O0` from exploration: do nothing / defer; low risk, but stalls user trust gains and leaves current scan clarity and recovery questions unresolved.
- `O1` from exploration: complete active-plan UX and telemetry work only (`scan-progress` + `history-and-duplicate-review`); low-to-moderate risk and aligns with the currently active plans.
- `O2` from exploration: add resumable scan queue and persisted active-run cards; medium risk because it introduces state transitions and potential stale-session behavior.
- `O3` from exploration: bounded producer-consumer scan architecture refactor; medium-high risk and broader backend impact than needed for this near-term cycle.
- `O4` from exploration: platform-aware pre-indexing daemon; high risk and highest migration/operational cost.

## 7. Recommended direction

Accept the following phased direction:

- First priority: execute the existing active O1 workstreams to completion as the immediate user-visible improvement set.
- Second priority (only if O1 completion reveals runtime pain): evaluate O2 as a bounded follow-up.
- Defer O3 and O4 to a later cycle unless strong evidence appears that UX and bounded telemetry changes are insufficient.

Rationale:
- `scan-progress` and `history-and-duplicate-review` already represent approved contract paths and provide immediate value.
- The current active plans already scope those boundaries, reducing planning overhead and minimizing architectural churn.
- O2 and O3 carry state and scheduling risk that is appropriate only after O1 impact is measured.

## 8. Expected behavior changes

- While a scan runs, the main screen and result surface should be clearly in an active-running mode, not framed as a stale completed result.
- Progress telemetry should be meaningful under load without becoming fake precision.
- Duplicate review and history readability improvements should stay visible and deterministic, and avoid changing the cleanup safety model.
- If O2 is added later, resumed runs should be explicit, bounded, and visibly distinct from completed runs.

## 9. Architecture impact

- Frontend/back-end boundary stays in the existing contract lanes already used by scan-core, Tauri command surface, and React state flow.
- Immediate O1 impact is mostly App shell logic, scan status interpretation, and existing telemetry shape.
- O2 would add session persistence/state replay logic and require a strict contract for terminal snapshot handling, so it should be treated as a separate follow-up.

## 10. Testing and verification strategy

- Update existing specs for any new behavior contract and map them through matching test specs before implementation.
- Add/adjust frontend regression tests for active-scan framing and history/duplicate presentation stability.
- Add/adjust targeted Rust/backend checks for bounded terminal snapshot integrity and event ordering once contract updates require command-layer behavior changes.
- Keep verification scoped to:
  - `npm run test`
  - `npm run lint`
  - `npm run build`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- If O2 is later approved, include scenario tests for interrupted scans, stale-session invalidation, and resume boundaries.

## 11. Rollout and rollback

- Rollout in two stages: O1 now, O2 only after explicit approval at a later review point.
- O1 can be rolled out without data migration.
- If O2 is adopted, add schema/contract guards and a kill switch path so users can disable resumable session state.
- Rollback from O1 is low cost because it touches currently active, separable plan scopes; regression gates stay unchanged.

## 12. Risks and mitigations

- Risk: active scan progress can still feel stalled on edge hardware.
  - Mitigation: define acceptance thresholds in spec and tune telemetry cadence against real large-folder validation.
- Risk: fast completion races against optimistic UI state transitions.
  - Mitigation: preserve freshest terminal snapshots for the same scan ID and avoid blind overwrites.
- Risk: O2 introduces stale-session confusion.
  - Mitigation: treat resumable sessions as optional/explicit and explicitly mark any reopened state.
- Risk: adding persistence or scheduling logic without tests.
  - Mitigation: require contract, test-spec, and regression fixtures before any O2 implementation starts.

## 13. Open questions

- What numeric/temporal telemetry thresholds should be considered good enough for O1 before concluding success?
- What maximum stale-session age is acceptable if O2 is later introduced?
- Should O2 use on-disk metadata only in the app data directory, or also include per-scan snapshot markers in history rows?
- What user-visible affordance should disable resume for long-running scans on constrained drives?
- What minimum large-folder dataset should be used before considering O3?

## 14. Decision log

- 2026-04-18: choose O1-first sequencing from the explored option set, with O2 as a deferred, conditional follow-up.
  - Reason: current active plans already contain O1 scope, enabling immediate value with bounded risk.
  - Alternative rejected: O3 and O4, because they increase architectural churn before O1 value is validated.
- 2026-04-18: defer any persistence-heavy session work to a spec-approved follow-up instead of implicit expansion inside O1 execution.
  - Reason: contract stability and fast feedback matter more for MVP trust than runtime refactors now.

## 15. Next artifacts

- New or revised feature spec: likely `specs/space-sift-scan-resilience.md` (or equivalent) if O2 is approved later.
- New or revised test spec: corresponding `specs/space-sift-scan-resilience.test.md`.
- Plan: keep existing active plan files for immediate O1 execution, and create a new follow-up plan only if O2 moves to implementation.
- Architecture note: only if O2 proceeds, document session persistence boundaries before implementation.
- After this proposal is accepted: run `proposal-review`.
- Before implementation: produce a spec.
