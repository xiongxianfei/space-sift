# Space Sift Workspace Navigation Test Spec

## Status

approved

## Related spec and plan

- Feature spec: [space-sift-workspace-navigation.md](/D:/Data/20260415-space-sift/specs/space-sift-workspace-navigation.md:1)
- Execution plan: [2026-04-22-space-sift-workspace-navigation-ui.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-22-space-sift-workspace-navigation-ui.md:1)
- Governing architecture: [2026-04-22-workspace-navigation-ui.md](/D:/Data/20260415-space-sift/docs/architecture/2026-04-22-workspace-navigation-ui.md:1)
- Related existing test specs:
  - [space-sift-scan-history.test.md](/D:/Data/20260415-space-sift/specs/space-sift-scan-history.test.md:1)
  - [space-sift-results-explorer.test.md](/D:/Data/20260415-space-sift/specs/space-sift-results-explorer.test.md:1)
  - [space-sift-duplicates.test.md](/D:/Data/20260415-space-sift/specs/space-sift-duplicates.test.md:1)
  - [space-sift-cleanup.test.md](/D:/Data/20260415-space-sift/specs/space-sift-cleanup.test.md:1)
  - [space-sift-scan-run-continuity.test.md](/D:/Data/20260415-space-sift/specs/space-sift-scan-run-continuity.test.md:1)

## Testing strategy

- Frontend integration tests in `src/workspace-navigation.test.tsx` own the workspace shell contract:
  manual tab selection, selected-state semantics, startup resolution, global
  status, next-safe-action priority, contractual auto-switches, and no-focus-steal
  behavior.
- Existing workflow suites stay authoritative for deeper domain behavior and are
  extended only where the shell crosses their boundary:
  `src/scan-history.test.tsx`, `src/results-explorer.test.tsx`,
  `src/duplicates.test.tsx`, and `src/cleanup.test.tsx`.
- Rust integration and contract tests in `src-tauri/crates/app-db/src/lib.rs`
  and `src-tauri/src/commands/shell.rs` own the durable restore-context seam,
  additive compatibility, and minimal-field persistence contract.
- No new desktop E2E harness is required for the first slice. Windows-specific
  accessibility and performance expectations use manual smoke verification where
  the repo has no current automation surface.

## Requirement coverage map

| Requirement IDs | Covered by |
| --- | --- |
| `R1`, `R2`, `R3` | `T1`, `T2` |
| `R4`, `R4a`, `R4b`, `R4g` | `T3` |
| `R4c`, `R4d`, `R4e`, `R4f` | `T4` |
| `R5`, `R19` | `T5` |
| `R6`, `R7`, `R7d`, `R7e`, `R8`, `R9`, `R10` | `T7` |
| `R7a`, `R7b`, `R7c`, `R23` | `T6`, `T14` |
| `R11`, `R12`, `R13`, `R14`, `R15`, `R16` | `T8` |
| `R17` | `T9` |
| `R18` | `T10` |
| `R20`, `R24` | `T11` |
| `R21`, `R22` | `T5`, `T12`, `T14` |
| `R25`, `R26`, `R26a` | `T2`, `T15` |
| `C1` | `T15` |
| `C2`, `C5` | `T5`, `T12` |
| `C3`, `C3a`, `C6` | `T6`, `T7`, `T14` |
| `C4` | `T5`, `T7` |
| `O1` | `T3`, `T4`, `T13` |
| `O2` | `T1`, `T2` |
| `O3` | `T7` |
| `O4` | `T8` |
| `O5` | `T9`, `T11` |
| `O6` | `T10`, `T13` |
| `O7` | `T4`, `T12` |
| `O8` | `T2` |
| `O8a` | `T2`, `T15` |

## Example coverage map

| Example | Covered by |
| --- | --- |
| `E1` running scan wins startup priority | `T7` |
| `E2` interrupted runs beat last explorer context | `T7` |
| `E3` validated restore context restores Explorer | `T7` |
| `E4` session-only duplicate results do not cold-start Duplicates | `T7` |
| `E5` start scan leaves stale completed result behind | `T8` |
| `E6` completed scan opens Explorer only after persistence succeeds | `T8` |
| `E7` history reopen navigates into Explorer | `T8` |
| `E8` explicit interrupted-run review opens History | `T8` |
| `E9` background continuity refresh does not steal focus | `T9` |
| `E10` explicit cleanup review opens Cleanup | `T8` |
| `E11` repeated terminal events do not re-navigate | `T10` |

## Edge case coverage

| Edge case | Covered by |
| --- | --- |
| Edge 1 running scan beats interrupted runs | `T7` |
| Edge 2 interrupted runs beat valid Explorer restore | `T7` |
| Edge 3 summary-only scan restores Explorer in degraded mode | `T5`, `T7` |
| Edge 4 prior Duplicates view does not cold-start Duplicates | `T7` |
| Edge 5 prior Cleanup view does not cold-start Cleanup | `T7` |
| Edge 6 replayed completion event does not re-run navigation | `T10` |
| Edge 7 background scan failure does not force `Scan` | `T9` |
| Edge 8 `canResume` updates do not move the user | `T9`, `T11` |
| Edge 9 interrupted-run review with no current runs stays in History | `T8`, `T11` |
| Edge 10 start duplicate analysis from another workspace activates Duplicates | `T8` |
| Edge 11 request cleanup review from Overview or Explorer activates Cleanup | `T8`, `T12` |
| Edge 12 cleanup execution changes next action to `Start a fresh scan` | `T4`, `T12` |
| Edge 13 explicit no-action state appears when no action is eligible | `T3` |
| Edge 14 vertical navigation exposes orientation and `Up`/`Down` behavior | `T2`, `T15` |

## Test cases

### T1. Manual workspace selection and one active panel

- Covers: `R1`, `R2`, `R3`, `O2`
- Level: integration
- Fixture/setup:
  - idle `SpaceSiftClient` test double
  - no loaded scan, no interrupted runs, no live duplicate analysis
- Steps:
  1. Render the app shell.
  2. Assert that exactly seven top-level workspaces are available.
  3. Activate multiple workspaces manually by click.
  4. Observe the active panel and spy on client actions.
- Expected result:
  - exactly one workspace is selected at a time
  - the visible primary panel changes with the selected workspace
  - manual navigation alone does not start scans, duplicate analysis, cleanup preview, cleanup execution, or resume
- Failure proves:
  - the shell cannot satisfy the top-level workspace contract
  - manual navigation is mutating backend state
- Automation location:
  - `src/workspace-navigation.test.tsx`

### T2. Accessible workspace navigation semantics

- Covers: `R25`, `R26`, `R26a`, `O2`, `O8`, `O8a`
- Level: integration
- Fixture/setup:
  - idle `SpaceSiftClient` test double
  - focusable rendered tablist or equivalent shell navigation
- Steps:
  1. Focus the selected workspace control.
  2. Move between controls with arrow keys.
  3. Use `Home` and `End` to jump to the first and last control.
  4. Activate a focused workspace with `Enter` and `Space`.
  5. If the implementation uses vertical navigation, verify `aria-orientation="vertical"` or equivalent and `Up`/`Down` movement.
- Expected result:
  - one selected control is exposed programmatically at a time
  - the active panel is programmatically associated with the selected control
  - inactive panels are not exposed as the active panel
  - keyboard navigation matches the chosen accessible pattern
- Failure proves:
  - keyboard users cannot navigate reliably
  - selected-state or panel association is incorrect
- Automation location:
  - `src/workspace-navigation.test.tsx`

### T3. Global status visibility and explicit no-action fallback

- Covers: `R4`, `R4a`, `R4b`, `R4g`, `O1`
- Level: integration
- Fixture/setup:
  - shell fixtures for idle state, loaded-scan state, interrupted-run state, and one explicit no-action scenario created through shell model inputs
- Steps:
  1. Render each shell fixture and navigate through all workspaces.
  2. Confirm the global status surface remains visible from every workspace.
  3. Verify the surface includes primary label, context summary, compact summary or progress when available, and a single action or explicit no-action state.
  4. Drive the no-action fixture and verify the control area does not silently disappear.
- Expected result:
  - the shell-level status surface is always visible
  - the surface shape is complete and deterministic
  - explicit no-action text appears when no eligible next action exists
- Failure proves:
  - status becomes hidden on non-Overview tabs
  - the shell omits required status fields or the no-action fallback
- Automation location:
  - `src/workspace-navigation.test.tsx`

### T4. Next-safe-action priority and non-destructive navigation

- Covers: `R4c`, `R4d`, `R4e`, `R4f`, `O1`, `O7`
- Level: integration
- Fixture/setup:
  - shell-state fixtures for:
    - live scan running
    - live duplicate analysis running
    - cleanup preview ready
    - cleanup execution completed for loaded scan
    - interrupted runs present
    - browseable loaded scan
    - no loaded scan
- Steps:
  1. Render each fixture and assert the primary state label.
  2. Verify the selected next safe action label and target follow the fixed priority order.
  3. Click the next safe action and spy on destructive client APIs.
- Expected result:
  - the shell uses the spec priority order
  - clicking the action navigates or focuses the correct workspace
  - cleanup execution, permanent delete, and resume are never invoked directly from the shell action
- Failure proves:
  - the shell is using non-deterministic priority logic
  - the shell is exposing destructive behavior directly
- Automation location:
  - `src/workspace-navigation.test.tsx`

### T5. Workspace prerequisites, degraded states, and preserved feature contracts

- Covers: `R5`, `R19`, `R21`, `R22`, `C2`, `C4`, `C5`
- Level: integration
- Fixture/setup:
  - browseable completed scan
  - summary-only completed scan
  - no loaded scan
  - no duplicate-analysis result
  - no cleanup preview
- Steps:
  1. Open each workspace with missing prerequisites.
  2. Verify prerequisite or degraded messaging appears instead of stale content from another workflow.
  3. Restore a summary-only scan into Explorer and confirm degraded rescan-required behavior.
  4. Attempt cold-start Duplicates or Cleanup restoration with only session-scoped state.
- Expected result:
  - each workspace shows its own prerequisite or degraded state
  - older summary-only scans do not masquerade as browseable or cleanup-ready
  - no durable Duplicates or Cleanup restoration is invented
- Failure proves:
  - the shell is leaking stale cross-workflow content
  - existing explorer, duplicate, or cleanup contracts were reopened unintentionally
- Automation location:
  - `src/workspace-navigation.test.tsx`
  - extend `src/results-explorer.test.tsx`, `src/duplicates.test.tsx`, and `src/cleanup.test.tsx` only where shell transitions cross those boundaries

### T6. Restore-context persistence and command boundary

- Covers: `R7a`, `R7b`, `R7c`, `R23`, `C3`, `C3a`, `C6`
- Level: integration
- Fixture/setup:
  - temporary SQLite database
  - direct `HistoryStore` access
  - shell command boundary tests for `get_workspace_restore_context` and `save_workspace_restore_context`
- Steps:
  1. Save a minimal restore-context record with only `last_workspace`, `last_opened_scan_id`, and required metadata.
  2. Reload it through the store and through the shell command surface.
  3. Overwrite the singleton record with a later workspace change.
  4. Simulate missing row, missing table, and unsupported `schema_version`.
- Expected result:
  - persistence is additive and local
  - overwrites stay single-record
  - missing or unsupported storage is treated as `no restore context`
  - the command boundary remains backward-compatible and minimal
- Failure proves:
  - the durable restore seam is not additive-safe
  - startup compatibility or downgrade posture is broken
- Automation location:
  - `src-tauri/crates/app-db/src/lib.rs`
  - `src-tauri/src/commands/shell.rs`

### T7. Startup resolver priority and restoration fallback

- Covers: `R6`, `R7`, `R7d`, `R7e`, `R8`, `R9`, `R10`, `E1`, `E2`, `E3`, `E10`, `O3`
- Level: integration
- Fixture/setup:
  - startup client doubles for:
    - running scan
    - interrupted runs
    - valid Explorer restore context
    - missing or unreadable restore context
    - invalid `lastOpenedScanId`
    - invalid live duplicate status not tied to a valid loaded scan
    - summary-only restored scan
- Steps:
  1. Render startup with each fixture.
  2. Assert the chosen initial workspace.
  3. Verify invalid or absent restore context falls back cleanly.
  4. Verify Duplicates or Cleanup are not restored solely from session-only state.
  5. Verify summary-only Explorer restore stays degraded.
- Expected result:
  - startup follows the approved priority order
  - invalid restore context never blocks use
  - no cold-start Duplicates or Cleanup restoration is fabricated
- Failure proves:
  - the startup resolver is non-deterministic or guessing from history or session state
  - the restore validation contract is broken
- Automation location:
  - `src/workspace-navigation.test.tsx`

### T8. Contractual auto-switches N1 through N6

- Covers: `R11`, `R12`, `R13`, `R14`, `R15`, `R16`, `E4`, `E5`, `E6`, `E7`, `E8`
- Level: integration
- Fixture/setup:
  - shell client doubles with accepted and rejected scan start
  - accepted and rejected duplicate-analysis start
  - accepted cleanup preview request
  - history reopen action
  - interrupted-run review action
- Steps:
  1. Trigger each explicit user action and associated live event when required.
  2. Assert the target workspace changes only for the approved reasons.
  3. Verify rejection cases do not claim active review states they did not earn.
  4. Verify failed cleanup preview remains in Cleanup with error state.
- Expected result:
  - all six contractual reasons switch to the correct workspace
  - rejected or failed prerequisite cases do not fake success
  - cleanup preview failures stay in Cleanup
- Failure proves:
  - the shell is missing a required transition or over-switching on rejected work
- Automation location:
  - `src/workspace-navigation.test.tsx`
  - extend `src/scan-history.test.tsx` where the transition starts from History

### T9. Non-switching background events and focus preservation

- Covers: `R17`, `E9`, `O5`
- Level: integration
- Fixture/setup:
  - active shell with user focused on Explorer or History
  - event callbacks for scan failure, scan cancellation, interrupted-run refresh,
    duplicate-analysis completion, cleanup completion, and `canResume` changes
- Steps:
  1. Move the user to a non-origin workspace.
  2. Emit non-contractual background updates.
  3. Assert active workspace, selected control, and keyboard focus remain stable.
  4. Verify badges, inline states, or notices update.
- Expected result:
  - non-contractual background updates never steal focus
  - user-visible status still updates
- Failure proves:
  - background updates are violating the shell focus contract
- Automation location:
  - `src/workspace-navigation.test.tsx`
  - extend `src/scan-history.test.tsx` for `canResume` rendering updates

### T10. Operation-aware dedupe and stale-event resistance

- Covers: `R18`, `E11`, `O6`
- Level: integration
- Fixture/setup:
  - deferred command promises
  - replayable scan-progress and duplicate-progress callbacks
  - shell fixtures with preserved local review state such as duplicate disclosure or keep selections
- Steps:
  1. Start a scan or duplicate analysis and emit a fresher live event before the awaited command resolves.
  2. Replay the same terminal snapshot twice for the same operation.
  3. Move the user manually and then emit a stale event from an older operation.
  4. Assert local review state remains intact.
- Expected result:
  - fresher event-driven state wins over stale command completions
  - repeated terminal events do not re-run the same auto-switch
  - stale events do not reset current review state
- Failure proves:
  - command-plus-event ordering is unsafe
  - the shell can erase user review state through replayed events
- Automation location:
  - `src/workspace-navigation.test.tsx`
  - extend `src/duplicates.test.tsx` if disclosure or keep-selection state needs direct regression coverage

### T11. History continuity surface and resume semantics

- Covers: `R20`, `R24`, `O5`
- Level: integration
- Fixture/setup:
  - stale, abandoned, and terminal run summaries
  - combinations of `has_resume` and `can_resume`
- Steps:
  1. Render History with interrupted-run summaries.
  2. Assert `status`, `has_resume`, and `can_resume` are visible.
  3. Verify `has_resume = true` with `can_resume = false` does not surface an actionable resume path.
  4. Request interrupted-run review when current data has no matching interrupted runs.
- Expected result:
  - History reflects the continuity contract without inventing unsupported resume capability
  - empty or no-match state stays in History
- Failure proves:
  - the shell is misrepresenting continuity actionability
- Automation location:
  - `src/scan-history.test.tsx`

### T12. Cleanup-specific status hardening and rescan recommendation

- Covers: `R21`, `R22`, `O7`
- Level: integration
- Fixture/setup:
  - loaded scan A
  - cleanup preview for scan A
  - cleanup execution result for scan A
  - later loaded scan B without matching cleanup state
- Steps:
  1. Render the shell with cleanup preview or cleanup execution for the current scan.
  2. Verify `Review cleanup preview` and `Start a fresh scan` appear only when eligible for the currently loaded scan.
  3. Load a different scan and verify mismatched cleanup state is ignored.
  4. Verify completed cleanup changes the next safe action to `Start a fresh scan`.
- Expected result:
  - cleanup-derived shell status is scan-scoped
  - the shell does not reuse cleanup state for the wrong loaded scan
  - rescan recommendation replaces stale "results are current" framing after cleanup execution
- Failure proves:
  - cleanup shell state is leaking across scan contexts
- Automation location:
  - `src/workspace-navigation.test.tsx`
  - extend `src/cleanup.test.tsx`

### T13. Shell notices and local log events

- Covers: `O1`, `O6`
- Level: integration
- Fixture/setup:
  - shell logger spy or equivalent local structured log sink
  - fixtures that trigger:
    - restore-context read failure
    - restore-context validation failure
    - applied contractual auto-switch
    - duplicate contractual auto-switch replay skip
    - next-safe-action selection
    - interrupted-run attention notice
    - active live-task presence notice
- Steps:
  1. Trigger each failure or notice path.
  2. Assert shell notices are visible and non-blocking.
  3. Assert the logger receives the named shell events.
- Expected result:
  - user-visible shell notices appear for restore failures, interrupted-run attention, active live work, and current next action
  - local structured log events are emitted once per path:
    - `workspace_restore_context_load_failed`
    - `workspace_restore_context_save_failed`
    - `workspace_restore_context_validation_failed`
    - `workspace_auto_switch_applied`
    - `workspace_auto_switch_skipped_duplicate`
    - `workspace_next_safe_action_selected`
    - `workspace_status_notice_rendered`
- Failure proves:
  - observability is too weak to debug shell-state failures safely
- Automation location:
  - `src/workspace-navigation.test.tsx`

### T14. Local-only restore context and compatibility safety

- Covers: `R23`, `C3`, `C3a`, `C6`
- Level: integration
- Fixture/setup:
  - temporary SQLite store
  - restore-context command boundary
  - optional schema inspection helper in Rust tests
- Steps:
  1. Save restore context and inspect persisted fields.
  2. Assert no duplicate groups, cleanup candidates, raw resume tokens, or network-specific fields are stored.
  3. Clear or downgrade the restore context and start the shell again.
- Expected result:
  - restore context stays minimal and local-only
  - absent or downgraded storage does not break manual navigation
- Failure proves:
  - the shell is persisting unnecessary or sensitive data
  - compatibility fallback is unsafe
- Automation location:
  - `src-tauri/crates/app-db/src/lib.rs`
  - `src-tauri/src/commands/shell.rs`

### T15. Windows manual QA and performance smoke

- Covers: `C1`, `R26a`
- Level: manual
- Fixture/setup:
  - Windows 11 desktop with repo prerequisites installed
  - disposable local app-data profile
- Steps:
  1. Launch the shell on Windows 11 and verify it becomes usable without waiting for optional background work.
  2. Switch workspaces repeatedly and confirm no scan, duplicate analysis, or cleanup execution starts solely because a panel was viewed.
  3. If the chosen navigation is vertical, verify `Up` and `Down` movement and exposed orientation with an accessibility inspection tool.
  4. Confirm automatic background updates do not steal keyboard focus.
- Expected result:
  - startup remains usable and workspace switches are lightweight
  - Windows-specific navigation semantics are correct when vertical orientation is used
- Failure proves:
  - the shell is violating performance or Windows accessibility expectations that the current automation surface cannot fully cover
- Automation location:
  - manual QA only

## Fixtures and data

- Frontend shell fixtures should build on the existing `SpaceSiftClient` test-double pattern already used in:
  - [App.test.tsx](/D:/Data/20260415-space-sift/src/App.test.tsx:1)
  - [scan-history.test.tsx](/D:/Data/20260415-space-sift/src/scan-history.test.tsx:1)
  - [results-explorer.test.tsx](/D:/Data/20260415-space-sift/src/results-explorer.test.tsx:1)
  - [duplicates.test.tsx](/D:/Data/20260415-space-sift/src/duplicates.test.tsx:1)
  - [cleanup.test.tsx](/D:/Data/20260415-space-sift/src/cleanup.test.tsx:1)
- Required frontend fixture families:
  - idle shell with no loaded scan
  - live running scan
  - live duplicate analysis
  - browseable completed scan
  - summary-only completed scan
  - stale and abandoned run summaries
  - cleanup preview and cleanup execution state for a specific loaded scan
  - invalid and missing restore-context variants
  - stale or replayed operation-event sequences
- Restore-context boundary fixtures should use a temporary SQLite database and explicit unsupported-`schema_version` rows rather than mocking persistence in JavaScript.
- Observability fixtures should use a spyable shell logger or equivalent local structured sink instead of asserting raw `console` output.

## Mocking/stubbing policy

- Prefer `SpaceSiftClient` test doubles over mocking Tauri `invoke` directly in React tests.
- Use explicit event subscription callbacks to simulate ordered progress events, stale events, and repeated terminal snapshots.
- Do not rely on snapshots alone for behavioral coverage; assert roles, selected state, visible notices, and client-call boundaries.
- For restore-context persistence, prefer direct Rust store and command tests over JavaScript-only mocks.
- If the implementation chooses a shell logger adapter, tests may stub that adapter. If it emits only through raw console calls, revisit observability coverage before claiming `O1` or `O6`.

## Migration or compatibility tests

- `T6` covers additive persistence, single-record overwrite behavior, and unsupported-schema fallback.
- `T7` covers startup without restore context, invalid restore context, and no fabricated Duplicates or Cleanup restoration.
- `T5` covers degraded summary-only Explorer behavior across old scan data.
- `T14` covers cleared or downgraded local restore-context storage and minimal-field persistence.

## Observability verification

- User-visible shell notices must be verified for:
  - active scan presence
  - active duplicate-analysis presence
  - interrupted-run attention state
  - currently loaded scan context
  - next safe action or explicit no-action state
  - non-blocking restore-context failures
- Local structured shell events must be verified through a spyable logger or equivalent sink:
  - `workspace_restore_context_load_failed`
  - `workspace_restore_context_save_failed`
  - `workspace_restore_context_validation_failed`
  - `workspace_auto_switch_applied`
  - `workspace_auto_switch_skipped_duplicate`
  - `workspace_next_safe_action_selected`
  - `workspace_status_notice_rendered`

## Security/privacy verification

- `T4` proves the shell action never executes destructive cleanup or resume directly.
- `T6` and `T14` prove restore context is additive, local-only, and minimal.
- Manual QA must confirm no cloud account, sync prompt, or network dependency is introduced by startup restoration.
- No test should rely on or store raw resume tokens, cleanup candidate lists, or other secret-like backend state in shell fixtures.

## Performance checks

- `T1` and `T15` verify manual workspace changes do not trigger scans, duplicate re-analysis, or cleanup execution.
- `T8` verifies contractual auto-switches move directly to the relevant workspace without requiring an extra confirmatory navigation step.
- `T15` manually verifies startup remains usable without waiting for optional background work.

## Manual QA checklist

- On Windows 11, verify the shell opens to the correct initial workspace for:
  - running scan
  - interrupted runs
  - valid Explorer restore
  - invalid restore context
- Confirm background status updates do not steal keyboard focus.
- If the chosen navigation is vertical, inspect orientation semantics and `Up` or `Down` key behavior.
- Confirm restore-context failure notices are non-blocking and do not prevent manual navigation.
- Confirm the shell still recommends `Start a fresh scan` after cleanup execution completes.

## What not to test

- Pixel-perfect parity with the frozen `docs/ui` prototype snapshot
- OS-level Explorer handoff behavior beyond existing results-explorer coverage
- Duplicate-analysis and cleanup engine correctness beyond the already-governing test specs
- Real Recycle Bin, elevation-helper, or protected-path execution behavior in this shell test spec
- Tauri desktop chrome or installer behavior

## Uncovered gaps

- No spec-level or architecture-level blocker is currently uncovered.
- If the implementation omits a spyable shell logger or equivalent local log sink, `T13` will need a small implementation seam before observability coverage can be claimed honestly.

## Next artifacts

- `implement` for the remaining milestones in [2026-04-22-space-sift-workspace-navigation-ui.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-22-space-sift-workspace-navigation-ui.md:1)
- later `verify` once the planned implementation milestones are complete

## Follow-on artifacts

None yet.

## Readiness

This test spec is `approved` and active as the governing proof surface for the
workspace-navigation milestones.

It is ready for downstream `implement`, `code-review`, and `verify` work under
the active execution plan.
