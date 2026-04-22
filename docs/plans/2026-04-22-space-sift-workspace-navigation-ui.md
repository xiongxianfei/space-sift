# Workspace Navigation UI Plan

## Status

active

## Purpose / big picture

Implement the first-pass workspace shell for `Overview`, `Scan`, `History`,
`Explorer`, `Duplicates`, `Cleanup`, and `Safety` without reopening the
existing Tauri, Rust, SQLite, and safety contracts. This plan exists to turn
the approved workspace-navigation spec and architecture into small reviewable
milestones that coordinate the active UI plans instead of silently replacing
them.

## Source artifacts

- Proposal: [2026-04-22-space-sift-advanced-ui-upgrade.md](/D:/Data/20260415-space-sift/docs/proposals/2026-04-22-space-sift-advanced-ui-upgrade.md:1)
- Spec: [space-sift-workspace-navigation.md](/D:/Data/20260415-space-sift/specs/space-sift-workspace-navigation.md:1)
- Spec review outcome: approved after restoration, global-status, and accessibility fixes
- Architecture: [2026-04-22-workspace-navigation-ui.md](/D:/Data/20260415-space-sift/docs/architecture/2026-04-22-workspace-navigation-ui.md:1)
- Architecture review outcome: approved after cleanup scan-scoping and restore-schema fallback were made explicit
- Test spec: [space-sift-workspace-navigation.test.md](/D:/Data/20260415-space-sift/specs/space-sift-workspace-navigation.test.md:1)
- Project map: [project-map.md](/D:/Data/20260415-space-sift/docs/project-map.md:1)
- Coordinated active plans:
  - [2026-04-16-history-and-duplicate-review-clarity.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-16-history-and-duplicate-review-clarity.md:1)
  - [2026-04-16-scan-progress-and-active-run-ux.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-16-scan-progress-and-active-run-ux.md:1)
  - [2026-04-15-space-sift-win11-mvp.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-15-space-sift-win11-mvp.md:1)

## Context and orientation

- [App.tsx](/D:/Data/20260415-space-sift/src/App.tsx:1) is the current shell and already owns scan, history, explorer, duplicate, cleanup, and notice state. The first slice should keep it as the orchestration boundary.
- [App.css](/D:/Data/20260415-space-sift/src/App.css:1) is the current shell styling surface and will likely absorb first-pass workspace layout and navigation styling.
- [spaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/spaceSiftClient.ts:1), [tauriSpaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/tauriSpaceSiftClient.ts:1), and [spaceSiftTypes.ts](/D:/Data/20260415-space-sift/src/lib/spaceSiftTypes.ts:1) will need additive restore-context and shell-model support.
- [shell.rs](/D:/Data/20260415-space-sift/src-tauri/src/commands/shell.rs:1) and [lib.rs](/D:/Data/20260415-space-sift/src-tauri/src/lib.rs:1) currently expose only shell-adjacent commands such as opening a path in Explorer. Restore-context commands should stay additive there.
- [app-db lib.rs](/D:/Data/20260415-space-sift/src-tauri/crates/app-db/src/lib.rs:1) is the right durable local storage seam for the minimal workspace restore context.
- Existing frontend coverage is already organized by workflow in [App.test.tsx](/D:/Data/20260415-space-sift/src/App.test.tsx:1), [scan-history.test.tsx](/D:/Data/20260415-space-sift/src/scan-history.test.tsx:1), [results-explorer.test.tsx](/D:/Data/20260415-space-sift/src/results-explorer.test.tsx:1), [duplicates.test.tsx](/D:/Data/20260415-space-sift/src/duplicates.test.tsx:1), and [cleanup.test.tsx](/D:/Data/20260415-space-sift/src/cleanup.test.tsx:1).
- `src/workspace-navigation.test.tsx` will be the focused shell-contract test file introduced by the matching test spec and used by M2-M4 targeted validation.
- [docs/workflows.md](/D:/Data/20260415-space-sift/docs/workflows.md:1) already requires operation-aware handling for Tauri command-plus-event flows and repeated terminal snapshots. The shell milestones must preserve that rule.
- The current app already clears cleanup session state when a different scan or duplicate result is opened. The shell should formalize that behavior rather than introducing a second conflicting cleanup-status source.

## Non-goals

- No backend scan-engine, duplicate-analysis, cleanup-engine, or resume-behavior changes.
- No new global state management library.
- No router migration or full `App.tsx` rewrite in the first implementation slice.
- No durable duplicate-analysis result persistence.
- No durable cleanup-preview persistence.
- No cleanup capability expansion, auto-elevation, or release workflow changes.
- No contract changes for existing scan-history, results-explorer, duplicates, cleanup, or continuity behavior unless a later spec revision explicitly requires them.

## Requirements covered

| Requirement area | Planned milestone |
| --- | --- |
| `R1`-`R5`, `R25`-`R26a`, global-status acceptance criteria, and deterministic next-safe-action behavior | `M2` |
| `R6`-`R10` startup resolver and validated restoration | `M1`, `M3` |
| `R11`-`R18` contractual auto-switches and non-switch rules | `M3` |
| `R19`-`R24` existing workflow contract preservation and cleanup-specific shell-status constraints | `M3`, `M4` |
| Acceptance criteria for keyboard navigation and panel semantics | `M2` |

## Milestones

### M1. Restore context and shell data seam

- Goal: add the minimal durable workspace restore context and additive command/client/type seam needed for startup resolution.
- Requirements: `R6`-`R10`, compatibility fallback requirements, architecture durable-storage rules, and restore-context observability prerequisites.
- Files/components likely touched:
  - [spaceSiftTypes.ts](/D:/Data/20260415-space-sift/src/lib/spaceSiftTypes.ts:1)
  - [spaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/spaceSiftClient.ts:1)
  - [tauriSpaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/tauriSpaceSiftClient.ts:1)
  - [shell.rs](/D:/Data/20260415-space-sift/src-tauri/src/commands/shell.rs:1)
  - [lib.rs](/D:/Data/20260415-space-sift/src-tauri/src/lib.rs:1)
  - [app-db lib.rs](/D:/Data/20260415-space-sift/src-tauri/crates/app-db/src/lib.rs:1)
- Dependencies: approved spec, approved architecture, matching test spec before implementation.
- Tests to add/update:
  - `workspace_restore_context_round_trip`
  - `workspace_restore_context_overwrites_singleton_row`
  - `workspace_restore_context_rejects_unsupported_schema_version`
- Observability:
  - local UI log-event names reserved for later shell wiring:
    - `workspace_restore_context_load_failed`
    - `workspace_restore_context_save_failed`
  - shell notices are not applicable in `M1` because the visible shell does not consume the seam yet
- Implementation steps:
  1. Add workspace restore context types and client interface methods.
  2. Add additive local persistence support for `workspace_restore_context`.
  3. Add `get_workspace_restore_context` and `save_workspace_restore_context` Tauri commands.
  4. Register the new commands without widening other backend contracts.
  5. Preserve load and save failure distinctions so later shell milestones can map them to the approved local log events and notices.
- Validation commands:
  - `cargo test --manifest-path src-tauri/Cargo.toml -p app-db workspace_restore_context`
  - `cargo test --manifest-path src-tauri/Cargo.toml workspace_restore_context_command_boundary`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `npm run build`
  - not applicable for `M1`: `npm run test -- src/workspace-navigation.test.tsx`
- Expected observable result: the app can save and read a validated restore-context record locally, but no visible startup-routing behavior changes yet.
- Commit message: `M1: add workspace restore context persistence seam`
- Milestone closeout:
  - validation passed
  - progress updated
  - decision log updated if needed
  - validation notes updated
  - milestone committed
- Risks:
  - additive SQLite support drifts into a broader bootstrap redesign
  - schema-version handling fails open instead of safe fallback
- Rollback/recovery: remove the restore-context command registrations and ignore the additive storage row until a corrected seam lands.

### M2. Workspace shell, atomic global status, and accessible manual navigation

- Goal: introduce the workspace shell, panel boundaries, manual tab switching, accessibility semantics, and the contract-complete global status surface with deterministic state labels and next safe action behavior.
- Requirements: `R1`-`R5`, `R25`-`R26a`, global-status acceptance criteria, next-safe-action priority, and architecture app-shell ownership rules.
- Files/components likely touched:
  - [App.tsx](/D:/Data/20260415-space-sift/src/App.tsx:1)
  - [App.css](/D:/Data/20260415-space-sift/src/App.css:1)
  - shell helper components or hooks under `src/` if they reduce risk without becoming a rewrite
  - `src/workspace-navigation.test.tsx`
- Dependencies: `M1` types available if shell code imports restore-context models; matching test spec before implementation.
- Tests to add/update:
  - `workspace_nav_exposes_single_selected_tab`
  - `workspace_nav_selected_tab_controls_visible_panel`
  - `workspace_nav_keyboard_arrows_move_between_tabs`
  - `workspace_nav_home_end_move_to_boundary_tabs`
  - `workspace_nav_enter_or_space_activates_focused_tab`
  - `workspace_nav_inactive_panels_are_not_exposed_as_active`
  - `global_status_visible_on_all_workspaces`
  - `global_status_exposes_deterministic_state_label`
  - `global_status_exposes_single_next_safe_action_or_no_action`
  - `next_safe_action_prioritizes_live_scan`
  - `next_safe_action_prioritizes_live_duplicate_analysis_after_scan`
  - `next_safe_action_points_interrupted_runs_to_history`
  - `next_safe_action_for_cleanup_preview_navigates_without_executing`
  - `next_safe_action_never_invokes_permanent_delete`
  - `next_safe_action_after_cleanup_execution_recommends_rescan`
- Implementation steps:
  1. Add `WorkspaceTab`, `activeWorkspace`, first-pass shell layout, and the shell-level `GlobalStatusModel`.
  2. Move existing visual sections into workspace panels with minimal logic changes.
  3. Add the deterministic shell-level primary state label and next safe action selection so the visible global-status contract is complete in this milestone.
  4. Add accessible workspace navigation semantics and keyboard handling.
  5. Keep the global status surface visible from every workspace and ensure the shell action only navigates or focuses the relevant workspace.
- Validation commands:
  - `npm run test -- src/workspace-navigation.test.tsx`
  - `npm run lint`
  - `npm run test`
  - `npm run build`
  - not applicable for `M2`: `cargo test --manifest-path src-tauri/Cargo.toml -p app-db workspace_restore_context` unless the milestone unexpectedly widens into `src-tauri/`
- Expected observable result: users can switch between workspaces manually with mouse or keyboard, only one workspace is active at a time, and the visible shell already exposes one deterministic global state label plus one next safe action or an explicit no-action state.
- Commit message: `M2: add workspace shell navigation and atomic global status`
- Milestone closeout:
  - validation passed
  - progress updated
  - decision log updated if needed
  - validation notes updated
  - milestone committed
- Risks:
  - panel extraction regresses existing controls or visibility
  - accessibility semantics become inconsistent with the chosen tab orientation
  - shell status appears before it is deterministic; this milestone must not ship a placeholder that looks contractual
- Rollback/recovery: revert to the current single-page shell while keeping any correct helper types or tests that still apply.

### M3. Startup resolution and contractual auto-switches

- Goal: wire startup restoration, validated Explorer reopen behavior, and the six approved contractual auto-switches into the workspace shell.
- Requirements: `R6`-`R18`, startup examples, no-focus-steal expectations, architecture operation-aware navigation rules.
- Files/components likely touched:
  - [App.tsx](/D:/Data/20260415-space-sift/src/App.tsx:1)
  - a focused workspace-navigation helper or hook under `src/` if needed
  - [tauriSpaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/tauriSpaceSiftClient.ts:1)
  - `src/workspace-navigation.test.tsx`
- Dependencies: `M1` for restore-context reads and writes, `M2` for navigation structure, matching test spec before implementation.
- Tests to add/update:
  - `initial_workspace_prefers_running_scan`
  - `initial_workspace_prefers_interrupted_runs_over_restore_context`
  - `startup_restore_validation_failure_shows_shell_notice`
  - `startup_restore_read_failure_shows_shell_notice`
  - `N1_START_SCAN`
  - `N2_SCAN_COMPLETED_AND_OPENED`
  - `N3_OPEN_HISTORY_SCAN`
  - `N4_START_DUPLICATE_ANALYSIS`
  - `N5_REQUEST_CLEANUP_PREVIEW`
  - `N6_REVIEW_INTERRUPTED_RUNS`
  - `background_refresh_does_not_steal_focus`
  - `duplicate_auto_switch_event_is_logged_once`
  - `replayed_terminal_event_does_not_reset_review_state`
- Observability:
  - local UI log events:
    - `workspace_restore_context_load_failed`
    - `workspace_restore_context_validation_failed`
    - `workspace_auto_switch_applied`
    - `workspace_auto_switch_skipped_duplicate`
  - shell notices:
    - restore-context read failure notice
    - restore-context validation failure notice
- Implementation steps:
  1. Build `resolveInitialWorkspace(ctx)` from the approved startup rules.
  2. Persist `lastWorkspace` and `lastOpenedScanId` only at the approved write points.
  3. Add shell navigation reasons and restrict automatic switching to the six contractual cases.
  4. Prevent stale command responses or repeated terminal snapshots from overriding fresher state for the same operation id.
  5. Emit the approved local UI log events and shell notices for restore-context read and validation failures.
- Validation commands:
  - `npm run test -- src/workspace-navigation.test.tsx`
  - `npm run test -- src/scan-history.test.tsx`
  - `npm run lint`
  - `npm run test`
  - blocked for `M3` if `src/workspace-navigation.test.tsx` does not exist yet; create it from the approved test spec before starting the milestone
  - not applicable for `M3`: `cargo test --manifest-path src-tauri/Cargo.toml -p app-db workspace_restore_context` if the milestone stays in `src/` and `src/lib/`; if any `src-tauri/` file changes, also run `cargo check --manifest-path src-tauri/Cargo.toml`
- Expected observable result: cold startup falls back to `Overview` unless validated live work, interrupted recovery work, or a valid last-opened Explorer context justifies a different workspace. Background updates do not steal focus, and restore-context failures surface approved shell notices and local UI log events.
- Commit message: `M3: add startup resolver and contractual workspace switching`
- Milestone closeout:
  - validation passed
  - progress updated
  - decision log updated if needed
  - validation notes updated
  - milestone committed
- Risks:
  - command-plus-event races misroute workspace selection
  - restore validation is skipped or too permissive
- Rollback/recovery: disable restore-context reads and fall back to manual navigation while keeping the shell layout intact if startup routing proves unstable.

### M4. Global status hardening and observability

- Goal: harden the already-visible global-status contract against cleanup-specific, continuity-specific, and cross-workflow edge cases without changing the fact that status became contract-complete in `M2`.
- Requirements: `R19`-`R24`, cleanup scan-scoping rule, cross-workflow regression cases, and non-destructive shell-action retention.
- Files/components likely touched:
  - [App.tsx](/D:/Data/20260415-space-sift/src/App.tsx:1)
  - helper model files under `src/` if status derivation should be isolated for testing
  - `src/workspace-navigation.test.tsx`
  - [cleanup.test.tsx](/D:/Data/20260415-space-sift/src/cleanup.test.tsx:1)
  - [duplicates.test.tsx](/D:/Data/20260415-space-sift/src/duplicates.test.tsx:1)
- Dependencies: `M2` shell framing complete, `M3` navigation semantics complete enough that status actions navigate correctly, matching test spec before implementation.
- Tests to add/update:
  - `global_status_ignores_cleanup_state_for_different_loaded_scan`
  - `next_safe_action_after_cleanup_execution_recommends_rescan`
  - `interrupted_run_notice_visible_without_focus_steal`
  - `active_live_task_notice_visible_from_non_overview_workspace`
- Observability:
  - local UI log events:
    - `workspace_next_safe_action_selected`
    - `workspace_status_notice_rendered`
  - shell notices:
    - interrupted-run attention-state notice
    - active live-task presence notice
- Implementation steps:
  1. Keep the `M2` global-status contract intact while integrating cleanup-specific and continuity-specific edge states.
  2. Associate cleanup execution shell status with the originating scan, or clear it when the loaded scan changes.
  3. Add the remaining shell notices and local UI log events for interrupted-run attention and active live-task presence.
  4. Extend workflow-specific suites so cleanup and duplicate review regressions cannot silently break shell-level status.
- Validation commands:
  - `npm run test -- src/workspace-navigation.test.tsx`
  - `npm run test -- src/cleanup.test.tsx`
  - `npm run test -- src/duplicates.test.tsx`
  - `npm run lint`
  - `npm run test`
  - `npm run build`
  - not applicable for `M4`: `cargo test --manifest-path src-tauri/Cargo.toml -p app-db workspace_restore_context` unless this milestone changes the restore-context seam
- Expected observable result: users can always see the app's current global state and the next safe action from any workspace, cleanup-derived shell state stays tied to the correct loaded scan, and shell notices remain visible without stealing focus.
- Commit message: `M4: harden shell status and observability`
- Milestone closeout:
  - validation passed
  - progress updated
  - decision log updated if needed
  - validation notes updated
  - milestone committed
- Risks:
  - priority drift between spec and implementation
  - stale cleanup session state leaks into shell status for the wrong scan
- Rollback/recovery: keep the shell layout but remove only the last-added hardening or notice behavior if a cleanup-specific regression appears, rather than re-opening the atomic shell contract from `M2`.

## Validation plan

- `M1`: run `cargo test --manifest-path src-tauri/Cargo.toml -p app-db workspace_restore_context`, `cargo test --manifest-path src-tauri/Cargo.toml workspace_restore_context_command_boundary`, `cargo check --manifest-path src-tauri/Cargo.toml`, and `npm run build`. `npm run test -- src/workspace-navigation.test.tsx` is not applicable.
- `M2`: run `npm run test -- src/workspace-navigation.test.tsx`, `npm run lint`, `npm run test`, and `npm run build`. Rust validation is not applicable unless the milestone edits `src-tauri/`.
- `M3`: run `npm run test -- src/workspace-navigation.test.tsx`, `npm run test -- src/scan-history.test.tsx`, `npm run lint`, and `npm run test`. If `src/workspace-navigation.test.tsx` does not exist yet, the milestone is blocked until it is created from the approved test spec.
- `M4`: run `npm run test -- src/workspace-navigation.test.tsx`, `npm run test -- src/cleanup.test.tsx`, `npm run test -- src/duplicates.test.tsx`, `npm run lint`, `npm run test`, and `npm run build`. Rust validation is not applicable unless the milestone edits `src-tauri/`.
- Branch-ready claim: run `bash scripts/ci.sh`.
- If any command is blocked by missing prerequisites, file locks, or environment issues, record the exact command and error here and in the final report.

## Risks and recovery

- `App.tsx` concentration makes shell changes regression-prone. Prefer helper extraction only when it reduces local complexity without turning into a rewrite.
- Tauri command-plus-event races can misroute tab state or overwrite fresher data. Operation-aware dedupe must be implemented and tested before broad rollout.
- Cleanup execution status is session-scoped and not keyed in the existing type model. The frontend must preserve scan association or invalidate the state on scan change.
- The new shell-level test file must not become a dumping ground for every workflow assertion. Keep shell-navigation contract coverage in `src/workspace-navigation.test.tsx` and leave workflow-specific assertions in their existing domain suites where possible.
- This umbrella plan overlaps active UI plans. Each milestone implementation must state which active contract it leaves unchanged, extends, or supersedes before code lands.

## Dependencies

- Approved proposal, approved spec, approved architecture
- Matching test spec before implementation
- `src/workspace-navigation.test.tsx` created from the approved test spec before `M2`, `M3`, or `M4` implementation starts
- `M1` before `M3` because startup restoration depends on the new persistence seam
- `M2` before `M3` and `M4` because workspace navigation must exist before auto-switches and shell-level actions can be verified
- Existing active UI plans remain in force unless a later artifact explicitly supersedes a contract they currently govern

## Progress

- [x] Architecture normalized to `approved`
- [x] Test spec created
- [x] Plan reviewed
- [x] M1 complete
- [ ] M2 complete
- [ ] M3 complete
- [ ] M4 complete
- [ ] Branch-wide verification complete

## Decision log

- 2026-04-22: This initiative is a coordinating umbrella and does not supersede active UI plans by default.
- 2026-04-22: The first implementation slice keeps `App.tsx` as the orchestration boundary and allows only additive helper extraction.
- 2026-04-22: Cold-start restoration remains limited to validated `Overview`, `Scan`, `History`, and `Explorer` cases. `Duplicates` and `Cleanup` stay session-scoped.
- 2026-04-22: The architecture artifact was normalized to `approved` before this plan relied on it.
- 2026-04-22: The visible global-status contract is atomic in `M2`; `M4` only hardens workflow-specific edge cases and observability around that already-visible shell behavior.
- 2026-04-22: `M1` persists only the approved minimal restore-context record: `schema_version`, `last_workspace`, `last_opened_scan_id`, and `updated_at`.
- 2026-04-22: Invalid or unsupported restore-context rows are treated as `no restore context` at the storage and shell-command boundary instead of surfacing a fatal startup dependency.

## Surprises and discoveries

- `.codex/PLANS.md` is not present in this repository, so this plan follows the repository's documented plan structure directly.
- The current app already clears cleanup state when a different scan or duplicate result is opened, which supports the approved scan-scoped cleanup-status rule.
- `CleanupExecutionResult` does not include `scanId`, so the shell must pair execution status with the originating scan or invalidate it on scan change.
- Because `M1` extended the shared TypeScript client interface, existing workflow test doubles had to grow no-op restore-context methods before `npm run build` would pass.

## Validation notes

- `cargo test --manifest-path src-tauri/Cargo.toml -p app-db workspace_restore_context`
  - first run blocked while Cargo tried to remove `src-tauri\\target\\debug\\deps\\libduplicates_core-70d3dfccb52ec320.rlib` and hit `拒绝访问。 (os error 5)`
  - rerun passed: `3 passed; 0 failed`
- `cargo test --manifest-path src-tauri/Cargo.toml workspace_restore_context_command_boundary`
  - passed: `2 passed; 0 failed`
- `cargo check --manifest-path src-tauri/Cargo.toml`
  - passed
- `npm run build`
  - first run failed because existing test doubles in `src/App.test.tsx`, `src/cleanup.test.tsx`, `src/duplicates.test.tsx`, `src/results-explorer.test.tsx`, and `src/scan-history.test.tsx` did not yet implement `getWorkspaceRestoreContext` and `saveWorkspaceRestoreContext`
  - rerun after updating the mocks passed
- `git diff --check -- src-tauri/crates/app-db/Cargo.toml src-tauri/crates/app-db/src/lib.rs src-tauri/src/commands/shell.rs src-tauri/src/lib.rs src/lib/spaceSiftTypes.ts src/lib/spaceSiftClient.ts src/lib/tauriSpaceSiftClient.ts src/App.test.tsx src/cleanup.test.tsx src/duplicates.test.tsx src/results-explorer.test.tsx src/scan-history.test.tsx`
  - passed with only CRLF conversion warnings
- Follow-up review resolution:
  - tracked the proposal, spec, test spec, architecture note, and project map that this active plan cites as source artifacts
  - added a direct restore-context compatibility regression test for missing or cleared storage

## Outcome and retrospective

- `M1` is complete. The app now has an additive local SQLite-backed restore-context seam, two shell commands, matching TypeScript client/types, and targeted Rust tests for persistence and shell command behavior.
- This milestone leaves the visible workspace shell, startup resolver, and auto-switch behavior unchanged. Those contracts remain for `M2` and `M3`.
- The coordinated active UI plans remain unchanged by `M1`; this slice only extends the persistence and client boundary needed by the later shell milestones.

## Readiness

This plan is `active`. `M1` is complete and ready for `code-review`. `M2`
remains the next implementation milestone, and it still depends on the focused
`src/workspace-navigation.test.tsx` shell test file from the approved test spec.
