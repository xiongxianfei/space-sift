# Space Sift Workspace Navigation

## Status

- approved

## Related proposal

- `docs/proposals/2026-04-22-space-sift-advanced-ui-upgrade.md`

## Goal and context

`Space Sift` now exposes multiple shipped workflows: scan start and monitoring,
interrupted-run recovery review, completed-scan history reopen, result
exploration, duplicate verification, cleanup preview and execution, and safety
guidance. This spec defines the contract for presenting those workflows through
one task-focused workspace shell instead of one long stacked page.

This spec does not replace the detailed behavior already governed by:

- `specs/space-sift-scan-history.md`
- `specs/space-sift-results-explorer.md`
- `specs/space-sift-duplicates.md`
- `specs/space-sift-cleanup.md`
- `specs/space-sift-scan-run-continuity.md`

This spec defines:

- the top-level workspace navigation model
- startup workspace resolution
- the allowed contractual auto-switches
- the events that must not steal focus
- the minimum shell-level UX, safety, and accessibility rules

## Glossary

- **Workspace shell**: the top-level app layout that exposes the major product
  workflows as distinct selectable workspaces.
- **Workspace tab**: one of the top-level navigation choices:
  `Overview`, `Scan`, `History`, `Explorer`, `Duplicates`, `Cleanup`, or
  `Safety`.
- **Active workspace**: the currently selected top-level workspace whose panel
  is visible as the primary content area.
- **Loaded scan**: the completed scan result currently opened in the app,
  whether it came from a fresh completion or local history reopen.
- **Openable scan context**: a locally saved completed scan reference that can
  still be reopened successfully. It may be fully browseable or summary-only.
- **Interrupted run**: a scan run surfaced through the continuity contract with
  non-terminal recovery-facing state such as `STALE` or `ABANDONED`.
- **Workspace restore context**: the small locally persisted shell context used
  for startup restoration. In the first implementation slice it requires only
  `lastWorkspace` and `lastOpenedScanId`.
- **Contractual auto-switch**: a workspace change that this spec explicitly
  requires in response to a user action or a high-priority live task
  transition.
- **Global notice**: shell-level status or warning content that is visible
  without forcing a workspace switch.
- **Next safe action**: the single deterministic shell-level action suggested
  by the global status surface. It may navigate to a workflow, but it does not
  perform destructive or irreversible work directly.
- **Operation-aware navigation**: navigation logic that correlates user actions
  and backend state updates by operation identity so stale or repeated events
  do not override fresher UI state.

## Examples

### Example E1: running scan wins startup priority

Given startup context includes a backend-confirmed scan in `running` state,
when the workspace shell resolves its initial workspace, then the app opens the
`Scan` workspace even if completed history entries or interrupted runs also
exist.

### Example E2: interrupted runs beat last explorer context

Given no live scan or live duplicate analysis is present, but startup recovery
finds one or more `STALE` or `ABANDONED` runs and the user also has a valid
last-opened completed scan, when the app resolves its initial workspace, then
it opens `History` rather than `Explorer`.

### Example E3: validated restore context restores explorer context

Given no higher-priority live or interrupted work is present and the user has a
validated workspace restore context whose `lastWorkspace` points to
`Explorer` and whose `lastOpenedScanId` still opens successfully, when the app
starts, then it opens `Explorer` with that scan as the loaded result.

### Example E4: session-only duplicate results do not cold-start Duplicates

Given the user previously reviewed duplicates in a prior session but the app
does not durably persist completed duplicate-analysis results, when the app
cold-starts, then it does not choose `Duplicates` only because that was the
last viewed review workspace.

### Example E5: start scan leaves stale completed result behind

Given the user has a completed scan loaded in the app, when they start a new
scan and that start request is accepted, then the app switches to `Scan` and
does not keep presenting the older completed result as the current result of
the running scan.

### Example E6: completed scan opens Explorer only after persistence succeeds

Given a running scan reaches `completed`, when the app persists the completed
scan successfully and opens it as the current stored result, then the active
workspace switches to `Explorer`.

### Example E7: history reopen navigates into Explorer

Given the user is reviewing local history, when they click the explicit open
action for a completed scan, then the app loads that scan and switches to
`Explorer`.

### Example E8: explicit interrupted-run review opens History

Given the app is showing Overview with a notice that interrupted runs exist,
when the user clicks `Review interrupted runs`, then the app switches to
`History`.

### Example E9: background continuity refresh does not steal focus

Given the user is reviewing `Explorer`, when a background refresh discovers a
new `STALE` or `ABANDONED` run, then the app updates a badge or notice but does
not force a switch to `History`.

### Example E10: explicit cleanup review opens Cleanup

Given the user is reviewing duplicate groups and requests cleanup preview or
review, when that request is accepted, then the app switches to `Cleanup`
before showing the preview result or preview error.

### Example E11: repeated terminal events do not re-navigate

Given a scan has already auto-switched from `Scan` to `Explorer` for its
successful completion, when the same terminal completion snapshot is replayed
or delivered again for the same operation, then the app does not re-run the
auto-switch or reset the current review state.

## Inputs and outputs

Inputs:

- a user click or keyboard activation on a top-level workspace tab
- a user request to start a scan
- a user request to open a completed scan from history
- a user request to start duplicate analysis from a loaded scan
- a user request to preview or review cleanup
- a user request to review interrupted runs
- live scan progress or terminal updates
- live duplicate-analysis progress or terminal updates
- local scan-history summaries
- local scan-run continuity summaries
- optional locally retained workspace restore context

Outputs:

- an active workspace choice
- one visible primary workspace panel
- a global status or notice surface that remains visible across workspaces
- badges, notices, or inline status updates for non-switching background events
- prerequisite, degraded, empty, or error states for workspaces whose detailed
  contracts are governed elsewhere
- optional locally retained workspace restore context for future startup
  restoration

## Workspace restoration

The first implementation slice persists a small local workspace restore
context. Its purpose is to restore a valid task context, not to invent startup
navigation from newest history or from session-only frontend state.

The minimum required restore fields for this slice are:

- `lastWorkspace`
- `lastOpenedScanId`

Startup restoration is allowed only after that context is validated against
real durable app state. Missing or invalid restore context never blocks app
startup and never causes the shell to guess from newest history.

## Global status surface

The workspace shell exposes a shell-level global status surface outside the
individual workspace panels so it remains visible from every workspace,
including non-Overview tabs.

The global status surface communicates what is active or loaded and exposes at
most one deterministic next safe action. It never performs destructive work
directly.

## Next safe action priority

The next safe action is the shell-level navigation suggestion shown by the
global status surface. It is selected from current live, loaded, and recovery
state using a fixed priority order.

## Requirements

- R1: The app MUST expose exactly these top-level workspaces:
  - `Overview`
  - `Scan`
  - `History`
  - `Explorer`
  - `Duplicates`
  - `Cleanup`
  - `Safety`
- R2: The workspace shell MUST allow the user to select any top-level
  workspace through explicit navigation, and it MUST keep exactly one primary
  workspace active at a time.
- R3: Selecting a workspace tab manually MUST NOT by itself start a new scan,
  duplicate analysis, cleanup preview, cleanup execution, or resume action.
- R4: The workspace shell MUST keep a global status surface visible from any
  workspace so live work, loaded-result context, and the next safe action do
  not become hidden when another workspace is active.
- R4a: The global status surface MUST remain visible from every workspace,
  including non-Overview workspaces, and it MUST live at the app-shell level
  rather than only inside one workspace panel.
- R4b: The global status surface MUST include, at minimum:
  - a primary state label
  - the active or loaded context, when available
  - a compact progress or summary value, when available
  - one deterministic next safe action, or an explicit no-action state
- R4c: The primary state label MUST distinguish at least:
  - live scan running
  - live duplicate analysis running
  - interrupted runs need review
  - completed scan loaded
  - cleanup preview ready
  - cleanup execution completed with rescan recommended
  - ready or no scan loaded
- R4d: The global status surface MUST NOT expose a destructive action as the
  next safe action. It MUST NOT execute cleanup, permanent delete, or resume
  directly from the shell-level status surface.
- R4e: Clicking the next safe action MUST navigate or focus the relevant
  workspace. It MUST NOT silently perform destructive or irreversible
  operations.
- R4f: When multiple next actions are possible, the app MUST choose the first
  matching action from this priority order:
  1. live scan running
     - label: `View scan progress`
     - target: `Scan`
  2. live duplicate analysis running
     - label: `View duplicate analysis`
     - target: `Duplicates`
  3. cleanup preview exists and has not been executed
     - label: `Review cleanup preview`
     - target: `Cleanup`
  4. cleanup execution completed for the loaded scan
     - label: `Start a fresh scan`
     - target: `Scan`
  5. one or more `STALE` or `ABANDONED` runs exist
     - label: `Review interrupted runs`
     - target: `History`
  6. a completed scan is loaded and duplicate analysis is available but not
     running
     - label: `Find duplicates`
     - target: `Duplicates`
  7. a completed scan is loaded and cleanup preview prerequisites are available
     under the current cleanup contract
     - label: `Preview cleanup`
     - target: `Cleanup`
  8. a completed scan is loaded and browseable
     - label: `Browse results`
     - target: `Explorer`
  9. no completed scan is loaded
     - label: `Start a scan`
     - target: `Scan`
- R4g: If no next safe action in the priority order is eligible, the global
  status surface MUST show an explicit no-action state rather than silently
  omitting the control.
- R5: The minimum shell responsibilities for each workspace MUST be:
  - `Overview`: summarize current loaded state, active work, interrupted-run
    attention items, and the next safe action
  - `Scan`: host scan start, active-scan progress, cancellation, and scan error
    state
  - `History`: host completed-scan reopen and interrupted-run review
  - `Explorer`: host the currently loaded scan result or its degraded
    rescan-required state
  - `Duplicates`: host duplicate-analysis lifecycle and duplicate review or its
    prerequisite state
  - `Cleanup`: host cleanup source selection, preview, execution, and cleanup
    error state
  - `Safety`: host durable guidance about local-only behavior, privilege
    boundaries, and destructive-action safeguards
- R6: `Overview` MUST be the default initial workspace unless a higher-priority
  startup condition defined by this spec selects another workspace.
- R7: A non-Overview initial workspace MUST be chosen only for:
  - live work
  - interrupted recovery work
  - a validated workspace restore context tied to a specific durable identity
- R7a: The product MUST persist a small local workspace restore context. In the
  first implementation slice, only `lastWorkspace` and `lastOpenedScanId` are
  required.
- R7b: The shell MUST update `lastWorkspace` after:
  - a manual workspace activation
  - an approved contractual auto-switch
- R7c: The shell MUST update `lastOpenedScanId` only after a completed scan has
  been successfully opened from local durable history, including the persisted
  scan that is reopened after a successful fresh scan completion.
- R7d: Startup restoration MUST validate the persisted workspace restore
  context against real durable app state before selecting a restoration
  workspace. If validation fails, the shell MUST ignore the invalid context and
  fall back to `Overview` after applying any higher-priority live or
  interrupted-work rules.
- R7e: When persisted workspace restore context is absent or invalid, the shell
  MUST NOT infer a non-Overview startup workspace from newest history alone or
  from session-only frontend state.
- R8: Startup workspace resolution MUST use this priority order:
  - a backend-confirmed live running scan selects `Scan`
  - a backend-confirmed live duplicate analysis tied to the current loaded scan
    selects `Duplicates`
  - one or more `STALE` or `ABANDONED` scan runs select `History`
  - a valid openable completed scan plus a validated workspace restore context
    whose `lastWorkspace` is `Explorer` selects `Explorer`
  - otherwise the shell selects `Overview`
- R9: The following signals MUST NOT by themselves select a non-Overview
  initial workspace:
  - completed history exists
  - the latest scan was `cancelled`
  - the latest scan `failed`
  - cleanup executed previously
  - cleanup rules exist
  - `Safety` was the last viewed workspace
  - interrupted runs exist only in terminal non-recovery states
- R10: When startup restoration points to an openable completed scan that lacks
  browseable tree data, the shell MAY still select `Explorer`, but it MUST use
  the degraded rescan-required explorer behavior already defined by the results
  explorer and downstream duplicate or cleanup specs.
- R11: The contractual auto-switch `N1_START_SCAN` MUST switch the active
  workspace to `Scan` when:
  - the UI accepts a user request to start a scan, or
  - the matching running-state snapshot for that accepted scan start arrives
- R12: The contractual auto-switch `N2_SCAN_COMPLETED_AND_OPENED` MUST switch
  the active workspace to `Explorer` only when:
  - the active scan reaches `completed`
  - persistence succeeds
  - the newly completed stored scan is opened as the current loaded result
- R13: The contractual auto-switch `N3_OPEN_HISTORY_SCAN` MUST switch the
  active workspace to `Explorer` when the user explicitly opens a completed
  scan from `History`.
- R14: The contractual auto-switch `N4_START_DUPLICATE_ANALYSIS` MUST switch
  the active workspace to `Duplicates` when the user explicitly starts
  duplicate analysis from a loaded scan that satisfies the duplicate-analysis
  prerequisites.
- R15: The contractual auto-switch `N5_REQUEST_CLEANUP_PREVIEW` MUST switch the
  active workspace to `Cleanup` when the user explicitly requests cleanup
  preview or cleanup review from another workspace and that request is accepted
  for evaluation.
- R16: The contractual auto-switch `N6_REVIEW_INTERRUPTED_RUNS` MUST switch the
  active workspace to `History` when the user explicitly requests interrupted
  run review from `Overview` or from a shell-level notice.
- R17: The following events MUST NOT switch the active workspace automatically
  and MUST instead use badges, notices, or inline updates:
  - background history refresh
  - background discovery of new `STALE` or `ABANDONED` runs
  - scan cancellation
  - scan failure
  - duplicate-analysis completion
  - duplicate-analysis cancellation
  - duplicate-analysis failure
  - keep-selection changes inside duplicate review
  - cleanup preview completion
  - cleanup preview failure
  - cleanup execution completion
  - cleanup execution failure
  - safety warnings
  - `canResume` changes
- R18: Automatic navigation MUST be operation-aware:
  - a stale, duplicated, or replayed backend event for one operation MUST NOT
    override fresher UI state for that same operation
  - the same qualifying terminal event MUST NOT trigger the same contractual
    auto-switch more than once for the same operation
  - an awaited command response MUST NOT overwrite a fresher event snapshot for
    the same operation
- R19: When a workspace requires a loaded scan, completed duplicate result, or
  cleanup preview that is unavailable under the current feature contract, the
  shell MUST show that workspace's prerequisite or degraded state rather than
  stale content from another workflow.
- R20: `History` MUST surface interrupted-run status using the current
  continuity contract, including visible `status`, `has_resume`, and
  `can_resume` state where applicable.
- R21: This workspace shell MUST preserve the existing behavior contracts from
  the related specs, including:
  - a completed scan becomes the current result only after persistence succeeds
  - history remains local-only
  - older summary-only saved scans degrade to rescan-required flows instead of
    broken Explorer, Duplicates, or Cleanup behavior
  - duplicate analysis remains read-only and deterministic
  - cleanup remains preview-first with `Recycle Bin` as the default execution
    mode
  - protected-path cleanup remains fail-closed and the normal UI does not
    auto-elevate
- R22: This spec does not introduce durable completed duplicate-analysis
  persistence or durable unexecuted cleanup-preview persistence. Therefore,
  cold app startup MUST NOT restore `Duplicates` or `Cleanup` solely from prior
  completed duplicate review or prior cleanup preview state.
- R23: Any locally retained workspace restore context used by startup
  restoration MUST remain local-only and additive. If that context is missing,
  unreadable, outdated, or inconsistent with current stored data, the shell
  MUST fall back to `Overview` after applying any higher-priority live or
  interrupted-work rules, without blocking app use.
- R24: Scan resume remains governed by the continuity spec. This shell spec
  MUST NOT treat `has_resume = true` as actionability. `can_resume` remains the
  actionability source of truth.
- R25: The workspace navigation MUST expose exactly one selected workspace at a
  time, programmatically expose the selected state, and programmatically
  associate the active workspace panel with the selected workspace control.
  Inactive workspace panels MUST NOT be exposed as the active panel to
  assistive technology.
- R26: Keyboard users MUST be able to move focus among workspace controls and
  activate a workspace without a mouse. The shell MUST support directional
  arrow-key navigation, `Home`, `End`, and activation through `Enter` or
  `Space` consistently with the chosen accessible navigation pattern.
- R26a: If the workspace navigation is rendered as a vertical tablist or
  equivalent vertical navigation control, the shell MUST expose that vertical
  orientation programmatically and MUST treat `Up` and `Down` as the
  directional workspace-navigation keys.

## State and invariants

- Only one primary workspace is active at a time.
- Manual workspace navigation does not mutate scan history, duplicate results,
  cleanup execution history, or other durable product data by itself.
- The running-scan experience and the completed-result experience remain
  distinct states even when both are visible within the same app shell.
- Non-contractual background events never steal focus from the current
  workspace.
- Startup resolution is deterministic: the highest-priority eligible condition
  wins.
- This spec does not make completed duplicate-analysis results or cleanup
  previews durable across cold restart.
- `Safety` remains an informational workspace; it does not grant elevation or
  bypass existing cleanup guardrails.

## Error and boundary behavior

- E1: If locally retained workspace restore context is missing or unreadable at
  startup, the app MUST fall back to `Overview` unless a higher-priority live
  or interrupted-work condition is present.
- E2: If locally retained explorer context points to a completed scan that no
  longer exists or cannot be reopened, the app MUST ignore that context,
  surface a clean non-blocking notice, and fall back to `Overview` unless a
  higher-priority live or interrupted-work condition is present.
- E3: If startup sees a retained last workspace of `Duplicates` or `Cleanup`
  but the required durable duplicate or cleanup state does not exist, the app
  MUST NOT open those workspaces automatically.
- E4: If a user starts a scan but the request is rejected before acceptance,
  the app MUST NOT run `N1_START_SCAN` solely because the start action was
  attempted.
- E5: If a scan reaches `completed` but persistence fails, the app MUST NOT run
  `N2_SCAN_COMPLETED_AND_OPENED`.
- E6: If duplicate analysis start is rejected because no eligible loaded scan
  exists, the app MUST keep the current workspace or show the Duplicates
  prerequisite state, but it MUST NOT claim that a duplicate review is active.
- E7: If cleanup preview or cleanup review is requested and then fails, the app
  MUST remain in `Cleanup` and surface the preview error there rather than
  jumping back automatically.
- E8: If a user requests interrupted-run review and the backing data refresh
  finds no current interrupted runs, `History` MUST remain open and show a
  clean empty or no-match state rather than bouncing the user elsewhere.
- E9: If a background event arrives after the user has manually moved to a
  different workspace, the app MUST honor the user's current workspace unless
  that later event is itself one of the contractual auto-switches for a newer
  qualifying operation.
- E10: If startup exposes a live duplicate-analysis state that is not tied to a
  valid currently loaded scan, the app MUST ignore that candidate for startup
  workspace selection and continue evaluating the next eligible condition.

## Compatibility and migration

- C1: This workspace shell targets Windows 11 desktop behavior.
- C2: Existing scan, history, explorer, duplicate, cleanup, and continuity
  contracts remain in force unless a later approved spec changes them
  explicitly.
- C3: Existing installs or profiles that do not yet retain workspace restore
  context
  remain valid and MUST start in `Overview` or another higher-priority
  workspace chosen by live or interrupted work.
- C3a: The persisted workspace restore context is additive. Older installs that
  lack it remain valid and MUST continue working without migration before the
  shell becomes usable.
- C4: Summary-only saved scans from earlier milestones remain readable. If they
  are restored into `Explorer`, the app MUST use the existing degraded
  rescan-required behavior instead of treating them as browseable or duplicate-
  eligible data.
- C5: This spec does not require schema or command-surface changes for durable
  duplicate-analysis restoration or cleanup-preview restoration. A future
  approved spec MAY add those behaviors additively.
- C6: If local storage for workspace restore context is cleared, downgraded, or
  absent, the workspace shell MUST remain fully usable through manual
  navigation.

## Observability

- O1: The shell MUST expose user-visible status or notices for:
  - active scan presence
  - active duplicate-analysis presence
  - interrupted-run attention state
  - currently loaded scan context
  - the currently selected next safe action or an explicit no-action state
  - non-blocking startup restoration failures
- O2: Frontend tests MUST cover manual workspace selection and one-panel-active
  behavior.
- O3: Frontend tests MUST cover startup workspace resolution priority,
  including running scan, interrupted runs, valid explorer restoration, and the
  absence of cold-start Duplicates or Cleanup restoration in this spec.
- O4: Frontend tests MUST cover all six contractual auto-switches.
- O5: Frontend tests MUST cover non-switching events, including background
  continuity refresh, scan failure or cancellation away from `Scan`,
  duplicate-analysis completion away from `Duplicates`, and `canResume`
  updates.
- O6: Frontend or integration tests MUST cover operation-aware behavior so
  repeated or stale backend events do not re-run the same auto-switch or reset
  current review state.
- O7: The matching test spec MUST include at least these cases for the global
  status surface and next safe action:
  - `global_status_visible_on_all_workspaces`
  - `next_safe_action_prioritizes_live_scan`
  - `next_safe_action_prioritizes_live_duplicate_analysis_after_scan`
  - `next_safe_action_points_interrupted_runs_to_history`
  - `next_safe_action_for_cleanup_preview_navigates_without_executing`
  - `next_safe_action_never_invokes_permanent_delete`
  - `next_safe_action_after_cleanup_execution_recommends_rescan`
- O8: The matching test spec MUST include at least these workspace-navigation
  accessibility cases:
  - `workspace_nav_exposes_single_selected_tab`
  - `workspace_nav_selected_tab_controls_visible_panel`
  - `workspace_nav_keyboard_arrows_move_between_tabs`
  - `workspace_nav_home_end_move_to_boundary_tabs`
  - `workspace_nav_enter_or_space_activates_focused_tab`
  - `workspace_nav_inactive_panels_are_not_exposed_as_active`
- O8a: If the chosen workspace navigation uses a vertical tablist or equivalent
  vertical control, the matching test spec MUST also cover exposed orientation
  semantics and `Up` or `Down` directional movement.

## Security and privacy

- The workspace shell MUST preserve the product's local-only posture for scan
  history, interrupted-run data, duplicate analysis, and cleanup data.
- Locally retained workspace restore context MUST NOT require cloud sync,
  remote accounts, or network access.
- Locally retained workspace restore context MUST NOT store raw resume tokens,
  destructive cleanup candidate lists, or other secret-like backend state that
  is unnecessary for navigation restoration.
- Workspace navigation MUST NOT auto-elevate the app or weaken the protected-
  path fail-closed boundary.
- Global notices and shell-level summaries MUST NOT expose more sensitive data
  than the underlying feature contracts already allow.

## Accessibility and UX

- The workspace shell MUST expose accessible top-level navigation semantics
  using `tablist`/`tab`/`tabpanel` roles or an equivalent accessible pattern.
- The selected workspace state MUST be programmatically exposed.
- Each workspace tab MUST be keyboard reachable.
- The active workspace panel MUST be programmatically associated with the
  selected workspace control.
- Keyboard interaction for workspace tabs MUST support:
  - arrow-key movement between tabs
  - `Home` and `End` navigation
  - explicit activation through `Enter` or `Space`
- If the navigation is vertical, the shell MUST expose vertical orientation and
  directional `Up` or `Down` behavior consistent with that orientation.
- Automatic background status updates and non-contractual events MUST NOT steal
  keyboard focus.
- The shell MUST NOT rely on color alone to communicate running, interrupted,
  disabled, unsafe, or destructive states.
- The `Safety` workspace and destructive cleanup affordances MUST remain
  visually and textually distinct from neutral navigation.

## Performance expectations

- Startup workspace resolution MUST complete from existing startup inputs and
  MUST NOT wait for optional background refresh work before showing a usable
  shell.
- A manual workspace change MUST NOT trigger a rescan, duplicate re-analysis,
  or cleanup re-execution solely because the user viewed a different panel.
- Contractual auto-switches SHOULD move the user directly to the relevant
  workflow without requiring an extra confirmatory navigation step.
- Badges, notices, and non-switching status updates SHOULD update without
  blocking the user's current review task.

## Edge cases

- Edge 1: A running scan and one or more interrupted runs exist at the same
  startup time; `Scan` wins.
- Edge 2: Interrupted runs exist and a valid last-opened completed scan exists;
  `History` wins.
- Edge 3: A valid last-opened summary-only scan restores `Explorer` in degraded
  rescan-required mode instead of a broken view.
- Edge 4: The last viewed workspace was `Duplicates`, but there is no durable
  duplicate-analysis result; cold startup does not restore `Duplicates`.
- Edge 5: The last viewed workspace was `Cleanup`, but there is no durable
  unexecuted cleanup preview; cold startup does not restore `Cleanup`.
- Edge 6: A scan completion event is replayed after the app already switched to
  `Explorer`; the second event does not re-run the navigation.
- Edge 7: The user is on `Explorer` when a scan fails in the background; the
  shell shows a notice and does not force `Scan`.
- Edge 8: The user is on `History` and `canResume` changes for a run; the run
  state updates without moving the user elsewhere.
- Edge 9: A user requests interrupted-run review, but current data shows none;
  `History` stays open with a clean empty or no-match state.
- Edge 10: A user starts duplicate analysis from an eligible loaded scan while
  another workspace is active; `Duplicates` becomes active.
- Edge 11: A user requests cleanup review from `Overview` or `Explorer`;
  `Cleanup` becomes active even if the preview later reports errors.
- Edge 12: Cleanup execution completed for the loaded scan changes the global
  next safe action to `Start a fresh scan` rather than presenting the old scan
  as fully current.
- Edge 13: If no safe next action is currently eligible, the global status
  surface shows an explicit no-action state instead of an empty action area.
- Edge 14: A vertical workspace navigation control exposes vertical orientation
  semantics and uses `Up` or `Down` for directional movement.

## Non-goals

- Changing scan, explorer, duplicate-analysis, cleanup, or continuity engine
  behavior beyond what is needed to navigate between their existing contracts
- Introducing backend API redesign, Tauri command renames, or event renames
- Enabling executable scan resume
- Adding durable completed duplicate-analysis persistence in this spec
- Adding durable unexecuted cleanup-preview persistence in this spec
- Replacing the first-pass app-shell orchestration boundary with a full routed
  frontend rewrite
- Defining final visual styling beyond the shell and accessibility contract

## Acceptance criteria

- A reviewer can see seven distinct top-level workspaces and switch among them
  manually without triggering unrelated backend work.
- From every workspace tab, a reviewer can see the global status surface.
- From every workspace tab, a reviewer can see either one next safe action or
  an explicit no-action state.
- A reviewer can start the app with a running scan, with interrupted runs, or
  with a valid last-opened completed scan and observe the correct initial
  workspace priority in each case.
- A reviewer can start a scan from another workspace and confirm the shell
  switches to `Scan` without continuing to present an older completed result as
  the current running result.
- A reviewer can complete and persist a scan, then observe the shell switch to
  `Explorer` only after the stored result is opened.
- A reviewer can open a completed scan from `History` and land in `Explorer`.
- A reviewer can start duplicate analysis from a loaded scan and land in
  `Duplicates`.
- A reviewer can request cleanup review and land in `Cleanup`.
- When a live scan is running, the next safe action is `View scan progress`.
- When a live duplicate analysis is running and no live scan is running, the
  next safe action is `View duplicate analysis`.
- When interrupted runs exist and no live task is running, the next safe action
  is `Review interrupted runs`.
- The global next safe action never executes cleanup, permanent delete, or
  resume directly.
- After cleanup execution completes, the global status recommends a fresh scan
  rather than presenting the old scan as fully current.
- A reviewer can see that background refreshes, failures, cancellations,
  duplicate completion, and `canResume` changes update notices or badges
  without stealing focus.
- A reviewer can verify that repeated or stale backend events do not re-run the
  same auto-switch or reset the current review state.
- A reviewer can verify that cold app startup does not restore `Duplicates` or
  `Cleanup` solely from session-scoped review state in this spec.
- The workspace navigation exposes a single selected workspace at a time.
- The selected workspace control exposes selected state programmatically.
- The active workspace panel is programmatically associated with the selected
  workspace control.
- Keyboard users can move focus through the workspace controls and activate a
  workspace without a mouse.
- Arrow-key navigation, `Home` or `End` navigation, and `Enter` or `Space`
  activation behave consistently with the chosen tab or navigation pattern.
- Inactive workspace panels are not presented as the active panel to assistive
  technology.

## Open questions

None.

## Next artifacts

- `docs/architecture/2026-04-22-workspace-navigation-ui.md`
- `specs/space-sift-workspace-navigation.test.md`
- `docs/plans/2026-04-22-space-sift-workspace-navigation-ui.md`

## Follow-on artifacts

- `docs/architecture/2026-04-22-workspace-navigation-ui.md`

## Readiness

This spec is approved and ready for architecture, test-spec, and downstream
planning.
