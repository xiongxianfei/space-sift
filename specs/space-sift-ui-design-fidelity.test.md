# Space Sift UI Design Fidelity Test Spec

## Status

active

## Related spec and plan

- Feature spec: [space-sift-ui-design-fidelity.md](/D:/Data/20260415-space-sift/specs/space-sift-ui-design-fidelity.md:1)
- Execution plan: [2026-04-24-space-sift-ui-design-fidelity.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-24-space-sift-ui-design-fidelity.md:1)
- Existing behavior spec: [space-sift-workspace-navigation.md](/D:/Data/20260415-space-sift/specs/space-sift-workspace-navigation.md:1)
- Existing architecture: [2026-04-22-workspace-navigation-ui.md](/D:/Data/20260415-space-sift/docs/architecture/2026-04-22-workspace-navigation-ui.md:1)
- Design reference: [space-sift-tabbed-ui-prototype.html](/D:/Data/20260415-space-sift/docs/ui/space-sift-tabbed-ui-prototype.html:1)
- Design implementation note: [space-sift-ui-redesign-implementation.md](/D:/Data/20260415-space-sift/docs/ui/space-sift-ui-redesign-implementation.md:1)

## Testing strategy

- Unit and integration: use Vitest, React Testing Library, and existing
  `SpaceSiftClient` test doubles to assert workspace reachability, ARIA
  state, shell status, next safe action, workflow guardrails, degraded states,
  and that visual markup changes do not trigger backend work.
- Contract/static checks: verify the design pass does not change Tauri command
  names, persistence schemas, backend events, or `src-tauri/` behavior unless a
  later approved spec permits it.
- Responsive proof: automate DOM availability where feasible, and require
  browser or Tauri visual review at representative app-window widths above
  `1050px`, between `640px` and `1050px`, and below `640px`.
- Visual comparison: compare implementation screenshots against the prototype
  structure and hierarchy, not byte-identical or pixel-perfect output.
- Manual QA: use focused visual review to catch overlap, hidden required
  content, destructive-action prominence, focus styling, and card/table
  readability that jsdom cannot prove.

## Requirement coverage map

| IDs | Covered by | Notes |
| --- | --- | --- |
| `R1`, `R2`, `R3` | `T1`, `T20` | Source precedence, approved copy, and non-pixel-perfect visual target. |
| `R4`, `R5`, `R6`, `R7`, `R8`, `R9` | `T2`, `T3`, `T16`, `T20` | Shell, rail, selected state, and one active panel. |
| `R10`, `R11`, `R12` | `T4`, `T20` | Panel hierarchy and Overview metric cards. |
| `R13` | `T5`, `T20` | Scan command and progress grouping. |
| `R14`, `R33` | `T6`, `T16`, `T20` | History completed/interrupted separation and resume actionability. |
| `R15` | `T7`, `T20` | Explorer browseable and degraded result treatment. |
| `R16`, `R17` | `T8`, `T20` | Duplicate controls and grouped review cards. |
| `R18`, `R19`, `R31`, `R32` | `T9`, `T10`, `T16`, `T20` | Cleanup funnel, Recycle Bin priority, permanent-delete separation. |
| `R20` | `T11`, `T20` | Safety guidance. |
| `R21`, `R22`, `R23`, `R24`, `R25`, `R26`, `R27` | `T12`, `T13`, `T16`, `T20` | Desktop-first responsive behavior and required content retention. |
| `R28`, `R29` | `T3`, `T14`, `T16` | Global status and non-destructive next safe action. |
| `R30` | `T14`, `T19` | No behavior, command, event, or persistence contract change. |
| `R34`, `R35`, `R36` | `T16`, `T20` | Screenshot/manual visual evidence at three width bands. |
| `R37` | `T2`, `T3`, `T9`, `T14`, `T15` | Existing behavioral suites continue to cover affected contracts. |
| `C1`, `C2`, `C3`, `C4`, `C5` | `T7`, `T14`, `T19` | Compatibility, no migration, summary-only degraded Explorer. |
| `O1`, `O5` | `T3`, `T6`, `T9`, `T11` | User-visible status and shell notices remain visible. |
| `O2`, `O3`, `O4` | `T20` | Visual evidence and verification notes. |
| `S1`, `S2`, `S3`, `S4`, `S5` | `T9`, `T11`, `T14`, `T19` | Privacy, no telemetry/network dependency, no shell destructive action. |
| `A1`, `A2`, `A3`, `A4`, `A5`, `A6`, `A7` | `T2`, `T3`, `T12`, `T16`, `T20` | Accessibility and responsive UX. |
| `P1`, `P2`, `P3`, `P4` | `T14`, `T17`, `T18`, `T19`, `T20` | No backend work on navigation, state retention, no new polling, large lists. |

## Example coverage map

| Example | Covered by |
| --- | --- |
| `E1` desktop shell follows prototype structure | `T2`, `T16`, `T20` |
| `E2` mid-width keeps all workspaces reachable | `T12`, `T16`, `T20` |
| `E3` narrow Cleanup preserves safety content | `T9`, `T16`, `T20` |
| `E4` approved copy wins over sample copy | `T1`, `T11`, `T20` |
| `E5` screenshot review checks structure, not pixels | `T20` |
| `E6` visual polish does not add behavior | `T14`, `T19` |

## Edge case coverage

| Edge case | Covered by |
| --- | --- |
| Edge 1 wider than `1050px` shell | `T2`, `T16`, `T20` |
| Edge 2 resize to `640px`-`1050px` | `T12`, `T16`, `T20` |
| Edge 3 resize below `640px` | `T12`, `T16`, `T20` |
| Edge 4 Overview has no loaded scan | `T4` |
| Edge 5 summary-only scan loaded | `T7` |
| Edge 6 `has_resume = true`, `can_resume = false` | `T6` |
| Edge 7 Cleanup has no preview | `T9` |
| Edge 8 Cleanup preview has validation issues | `T10`, `T16`, `T20` |
| Edge 9 destructive warning and Recycle Bin visible | `T9`, `T16`, `T20` |
| Edge 10 sample copy conflicts with approved wording | `T1`, `T11` |
| Edge 11 focus-visible conflict | `T2`, `T20` |
| Edge 12 screenshot capture blocked | `T20` |

## Test cases

T1. Source precedence and no prototype sample data
- Covers: `R1`, `R2`, `R3`, `E4`, Edge 10
- Level: integration
- Fixture/setup: render `App` with no loaded scan, no duplicate result, and
  no cleanup preview.
- Steps: inspect Overview, global status, Safety, and Cleanup empty states.
- Expected result: approved product copy and empty/degraded states render;
  prototype sample values, fake paths, fake metrics, and sample safety copy do
  not appear.
- Failure proves: the design pass copied prototype content instead of using
  approved product state and copy.
- Automation location: new `src/ui-design-fidelity.test.tsx`, with supporting
  assertions in `src/workspace-navigation.test.tsx` if already present.

T2. Workspace shell exposes all seven workspaces accessibly
- Covers: `R4`-`R9`, `A1`-`A4`, `A6`, Edge 1, Edge 11
- Level: integration
- Fixture/setup: render `App` with default idle client.
- Steps: assert seven tabs with labels, selected state, keyboard navigation,
  visible active tab treatment classes or attributes, and exactly one
  tabpanel.
- Expected result: Overview, Scan, History, Explorer, Duplicates, Cleanup, and
  Safety remain reachable and programmatically associated with the visible
  panel.
- Failure proves: shell restyling broke the authoritative workspace-navigation
  contract.
- Automation location: update `src/workspace-navigation.test.tsx`.

T3. Global status and next safe action remain shell-level
- Covers: `R5`, `R28`, `R29`, `O1`, `O5`, `A1`
- Level: integration
- Fixture/setup: render states for idle, running scan, completed browseable
  scan, summary-only scan, interrupted runs, duplicate analysis, cleanup
  preview, and cleanup execution.
- Steps: switch through all workspaces and inspect the global status region.
- Expected result: the shell-level status remains visible from each workspace,
  and the next safe action is deterministic and non-destructive.
- Failure proves: visual layout hid or moved shell state into one workspace or
  introduced unsafe action exposure.
- Automation location: update `src/workspace-navigation.test.tsx`.

T4. Overview metric cards use real or explicit unavailable states
- Covers: `R10`, `R11`, `R12`, Edge 4
- Level: integration
- Fixture/setup: render no loaded scan, loaded browseable scan, and loaded
  summary-only scan states.
- Steps: inspect Overview metrics and high-level loaded/active state.
- Expected result: up to four metric cards render when real data exists;
  missing values render zero, unavailable, or not-yet-run states, never sample
  values.
- Failure proves: the Overview is visually filled with unsupported data or
  drops the approved loaded-state summary.
- Automation location: new `src/ui-design-fidelity.test.tsx`.

T5. Scan panel keeps command and progress content grouped
- Covers: `R10`, `R13`
- Level: integration
- Fixture/setup: render idle scan, running scan, canceling or failed scan
  states using the existing client double.
- Steps: inspect scan root input, start action, cancel action, progress
  metrics, current path or heartbeat context, and notices.
- Expected result: command and live progress information are close in the same
  panel hierarchy and all approved actions remain available.
- Failure proves: visual structure weakened the scan workflow or hid running
  context.
- Automation location: new `src/ui-design-fidelity.test.tsx` or existing scan
  tests if a scan-specific suite is created later.

T6. History separates completed scans from interrupted runs
- Covers: `R14`, `R25`, `R33`, `O1`, Edge 6
- Level: integration
- Fixture/setup: completed scan history plus interrupted run with
  `hasResume: true` and `canResume: false`.
- Steps: activate History, inspect completed rows, interrupted section,
  continuity fields, and resume affordance state.
- Expected result: completed scans expose reopen actions; interrupted runs keep
  run id or root, progress, timestamps/status, and resume actionability visible;
  resume stays disabled or non-actionable when `canResume` is false.
- Failure proves: layout conflated completed and interrupted history or used
  the wrong actionability field.
- Automation location: update `src/scan-history.test.tsx` and
  `src/workspace-navigation.test.tsx`.

T7. Explorer preserves browseable and degraded result treatment
- Covers: `R15`, `C5`, Edge 5
- Level: integration
- Fixture/setup: one browseable scan with entries and one summary-only scan
  without entries.
- Steps: activate Explorer for both states.
- Expected result: browseable scans show current-location context,
  breadcrumbs or equivalent, sorting affordances, usage visualization, and
  current-level table content; summary-only scans show the approved rescan
  required degraded state.
- Failure proves: prototype sample Explorer rows replaced real compatibility
  handling.
- Automation location: update `src/results-explorer.test.tsx`.

T8. Duplicate review cards keep identity and decisions readable
- Covers: `R16`, `R17`, `P4`
- Level: integration
- Fixture/setup: completed duplicate analysis with verified groups, same-name
  files in different directories, and selected keep paths.
- Steps: activate Duplicates, inspect analysis controls, status, group cards,
  member paths, keep-selection affordances, and delete-candidate summary.
- Expected result: each group is visually grouped and preserves file identity,
  keep/delete context, and readable path disambiguation.
- Failure proves: card treatment made duplicate decisions ambiguous.
- Automation location: update `src/duplicates.test.tsx`.

T9. Cleanup funnel preserves Recycle Bin priority and permanent-delete friction
- Covers: `R18`, `R19`, `R31`, `R32`, `S3`, Edge 7, Edge 9
- Level: integration
- Fixture/setup: loaded browseable scan, cleanup rules, duplicate selection,
  no preview, generated preview, and permanent-delete confirmation.
- Steps: activate Cleanup, inspect source selection, preview generation,
  candidate review, Recycle Bin action, advanced permanent-delete controls,
  and execution calls.
- Expected result: permanent delete is unavailable before preview and explicit
  confirmation; Recycle Bin remains primary when both paths are visible.
- Failure proves: visual funnel weakened destructive-action safeguards.
- Automation location: update `src/cleanup.test.tsx`.

T10. Cleanup validation issues remain visible
- Covers: `R18`, `R25`, `R31`, `O1`, Edge 8
- Level: integration and manual
- Fixture/setup: cleanup preview with validation issues and candidates.
- Steps: inspect Cleanup at normal width in automated tests, then include the
  same state in responsive visual review.
- Expected result: validation issues stay visible near candidate review and
  execution controls and are not hidden as secondary metadata.
- Failure proves: layout hides fail-closed cleanup information.
- Automation location: update `src/cleanup.test.tsx`; manual coverage in
  `T20`.

T11. Safety panel keeps approved durable guidance
- Covers: `R20`, `O5`, `S1`-`S5`, Edge 10
- Level: integration
- Fixture/setup: render Safety with default client capability response.
- Steps: activate Safety and inspect unprivileged mode, Recycle Bin-first,
  local-only history, protected-path behavior, destructive safeguards, and
  resume actionability guidance.
- Expected result: approved guidance remains visible and does not expose new
  data, telemetry, raw tokens, secrets, or sample copy.
- Failure proves: design copy or information architecture changed a safety
  contract.
- Automation location: new `src/ui-design-fidelity.test.tsx` or
  `src/workspace-navigation.test.tsx`.

T12. Responsive DOM availability across width bands
- Covers: `R21`-`R27`, `A5`-`A7`, Edge 2, Edge 3
- Level: integration
- Fixture/setup: render all key workflow states under test-controlled viewport
  widths of `1280`, `900`, and `560` CSS pixels where the test harness can set
  `window.innerWidth` and dispatch `resize`.
- Steps: assert all seven tabs, global status, next safe action, continuity
  fields, cleanup guardrails, and required action labels remain in the DOM.
- Expected result: responsive logic does not remove required content.
- Failure proves: narrow-window behavior drops required workflow or safety
  content.
- Automation location: new `src/ui-design-fidelity.test.tsx`; actual visual
  layout remains manually verified by `T20`.

T13. Responsive stylesheet exposes the approved breakpoint contract
- Covers: `R21`, `R22`, `R23`, `R24`, `R27`
- Level: smoke/static
- Fixture/setup: inspect the frontend stylesheet source after implementation.
- Steps: verify responsive rules are based on app-window CSS width and include
  behavior at `1050px` and `640px`, or record an approved replacement
  threshold in the plan decision log before accepting different breakpoints.
- Expected result: implementation follows the spec's baseline breakpoints or
  has a documented approved deviation.
- Failure proves: responsive behavior drifted from the accepted design source.
- Automation location: optional static assertion in
  `src/ui-design-fidelity.test.tsx`; otherwise manual checklist in `T20`.

T14. Manual workspace activation does not start backend work
- Covers: `R29`, `R30`, `P1`, `P3`, `E6`
- Level: integration
- Fixture/setup: client with spies for scan, duplicate analysis, cleanup
  preview, cleanup execution, resume, subscriptions, and Explorer handoff.
- Steps: click each workspace tab without clicking workflow commands.
- Expected result: no scan, duplicate-analysis, cleanup-preview,
  cleanup-execution, or resume command is called solely because a workspace was
  viewed.
- Failure proves: visual shell changes introduced behavior.
- Automation location: update `src/workspace-navigation.test.tsx` or new
  `src/ui-design-fidelity.test.tsx`.

T15. Existing behavioral suites remain authoritative
- Covers: `R37`
- Level: integration
- Fixture/setup: existing suites and updated focused assertions.
- Steps: run targeted suites for workspace navigation, history, explorer,
  duplicates, and cleanup.
- Expected result: the same approved behavior remains covered through the new
  layout.
- Failure proves: visual changes regressed an existing product contract.
- Automation location: `src/workspace-navigation.test.tsx`,
  `src/scan-history.test.tsx`, `src/results-explorer.test.tsx`,
  `src/duplicates.test.tsx`, `src/cleanup.test.tsx`.

T16. Required content remains available at three width bands
- Covers: `R23`-`R26`, `R34`, `R35`, `A5`-`A7`, Edge 1-3, Edge 8-9
- Level: manual
- Fixture/setup: run the app with states for no scan, completed scan,
  interrupted run, duplicate groups, and cleanup preview with issues.
- Steps: inspect representative widths above `1050px`, between `640px` and
  `1050px`, and below `640px`.
- Expected result: no required status, continuity field, cleanup guardrail, or
  next-safe-action content disappears; text and controls do not overlap
  incoherently.
- Failure proves: responsive visual behavior violates the safety or continuity
  contract.
- Automation location: plan validation notes and final verification evidence.

T17. Responsive reflow preserves local review state
- Covers: `P2`, `P4`
- Level: integration and manual
- Fixture/setup: duplicate group with selected keep path, Explorer sort state,
  cleanup selected rules, and active workspace state.
- Steps: change viewport width or manually resize the app window after making
  review selections.
- Expected result: selected workspace and local review decisions remain intact
  and lists remain usable through layout, scrolling, wrapping, or disclosure.
- Failure proves: responsive reflow disrupts in-progress review work.
- Automation location: focused assertions in related suites where feasible;
  manual visual review in `T20`.

T18. Large tables and review lists remain usable
- Covers: `E2`, `P4`
- Level: manual
- Fixture/setup: seeded scan with wide paths, many files, and multiple
  duplicate groups.
- Steps: inspect Explorer, History, Duplicates, and Cleanup at all width
  bands.
- Expected result: required fields remain readable by wrapping, stacking,
  horizontal scrolling, or equivalent accessible treatment.
- Failure proves: design fidelity made real data review impractical.
- Automation location: manual checklist; add automated DOM assertions only for
  required fields that can be selected reliably.

T19. No backend, persistence, migration, telemetry, or network drift
- Covers: `R30`, `C1`-`C4`, `S1`-`S5`, `P3`
- Level: contract/static
- Fixture/setup: inspect changed files and run frontend validation.
- Steps: verify no unexpected `src-tauri/` changes, no command/event schema
  changes, no migration files, no new network dependency for UI state, and no
  new always-running timers or polling loops for presentation.
- Expected result: the change is a frontend presentation pass only.
- Failure proves: implementation exceeded the approved scope.
- Automation location: code review, `git diff -- src-tauri`, `npm run test`,
  `npm run build`; add `cargo check --manifest-path src-tauri/Cargo.toml` if
  any Tauri file changes.

T20. Screenshot and human-visible prototype-structure review
- Covers: `R1`-`R36`, `O2`-`O4`, `A3`-`A7`, Edge 1-3, Edge 8-12
- Level: manual
- Fixture/setup: launch the Vite or Tauri app and open the prototype HTML for
  side-by-side structural comparison.
- Steps:
  1. Capture or inspect app screenshots at representative widths above
     `1050px`, between `640px` and `1050px`, and below `640px`.
  2. Compare implementation against prototype structure: persistent header,
     left workspace rail or reflowed navigation, panel/card hierarchy,
     history/explorer tables, duplicate-review cards, cleanup funnel, and
     destructive-action separation.
  3. Confirm screenshots are evaluated for close visual similarity, not
     pixel-perfect or byte-identical matching.
  4. Record whether required status content, continuity fields, cleanup
     guardrails, and next-safe-action content remained available at each width.
  5. If capture is blocked, record the exact command and error and do not claim
     visual review passed.
- Expected result: visual evidence supports close structural similarity and
  no required content disappears at required width bands.
- Failure proves: the shipped UI does not satisfy the accepted design-fidelity
  contract or the local environment cannot verify it yet.
- Automation location: plan validation notes, final verification summary, and
  any screenshot artifacts generated during implementation.

## Fixtures and data

- `idleClient`: no scan loaded, no duplicate result, no cleanup preview.
- `browseableScan`: completed scan with `entries`, total bytes, files,
  directories, current root, and Explorer table rows.
- `summaryOnlyScan`: completed scan without `entries` to prove degraded
  Explorer and cleanup-prerequisite handling.
- `runningScan`: active scan with current path, heartbeat/progress metrics,
  and cancel affordance.
- `completedHistory`: completed scan rows with reopen actions.
- `interruptedRunNonActionable`: run summary with `hasResume: true` and
  `canResume: false`.
- `duplicateAnalysisWithSameNames`: duplicate groups where members share file
  names but differ by path, plus keep-selection state.
- `cleanupNoPreview`: cleanup sources available, no generated preview.
- `cleanupPreviewWithCandidates`: duplicate and rule candidates with Recycle
  Bin eligible.
- `cleanupPreviewWithIssues`: preview containing validation issues and
  candidates.
- `viewportBands`: `1280px`, `900px`, and `560px` representative widths for
  automated DOM checks and manual screenshot review.

## Mocking/stubbing policy

- Frontend tests MUST use `SpaceSiftClient` test doubles and MUST NOT touch
  the real filesystem, real Tauri shell, or real Recycle Bin.
- Tests SHOULD render the real `App` and real child components when asserting
  user-visible shell, panel, status, and safety behavior.
- Tests MAY stub backend responses, event subscriptions, and command results
  through the existing client interface.
- Tests MUST NOT mock away accessibility semantics or child panels when the
  assertion is about visible content, ARIA state, destructive-action gating, or
  workflow continuity.
- Screenshot/manual review MAY use local seeded state, but any environment
  blocker must be reported exactly.

## Migration or compatibility tests

- No data migration is expected.
- `T7` covers older summary-only scan compatibility.
- `T19` covers the no-migration and no-backend-contract-change claim.
- If implementation unexpectedly changes `src-tauri/`, command names, event
  names, storage schema, or persisted workspace context, this test spec must
  return to review before implementation proceeds.

## Observability verification

- `T3`, `T6`, `T9`, and `T11` verify user-visible status and notices remain
  available.
- `T20` verifies the final notes state whether visual similarity was checked
  against the prototype structure and whether required content remained
  available at each width band.
- Existing local shell notice logging in `workspace-navigation.test.tsx` should
  continue to pass unless the approved behavior spec changes.

## Security/privacy verification

- `T9` verifies destructive cleanup remains behind preview-first and explicit
  confirmation gates.
- `T11` verifies Safety guidance preserves local-only history, protected-path,
  Recycle Bin-first, and resume actionability language.
- `T14` verifies the shell next safe action remains non-destructive.
- `T19` verifies no cloud sync, telemetry, remote account, raw token exposure,
  secret exposure, or new network dependency is introduced.

## Performance checks

- `T14` verifies navigation does not start expensive backend work.
- `T17` verifies responsive reflow does not clear in-progress review state.
- `T18` verifies large tables and review lists remain usable.
- `T19` verifies the design pass does not add new always-running timers,
  polling loops, network calls, or backend commands solely for presentation.

## Manual QA checklist

- Launch the app with the implementation state using the smallest appropriate
  local command, usually `npm run dev`; use `npm run tauri dev` if Tauri shell
  behavior must be inspected.
- At a width above `1050px`, verify persistent header, left rail, right active
  panel, all seven workspaces, global status, and next safe action.
- At a width between `640px` and `1050px`, verify shell collapse/reflow, all
  seven workspaces, status, continuity fields, cleanup guardrails, and
  next-safe-action content.
- At a width below `640px`, verify stacked navigation, stacked panels, readable
  controls, no incoherent overlap, and no hidden required content.
- Compare screenshots or live views against the prototype's structure and
  hierarchy, not exact pixels.
- Inspect Cleanup with no preview, preview with candidates, validation issues,
  Recycle Bin action, and advanced permanent delete.
- Inspect History with completed scans and non-actionable interrupted runs.
- Inspect Duplicates with same-name files in different folders.
- Inspect Explorer with browseable and summary-only scans.
- Confirm focus-visible styling on tabs, buttons, links, form controls, table
  actions, and cleanup confirmations.
- Record exact command and error if browser, Tauri, or screenshot capture is
  blocked.

## What not to test

- Do not test pixel-perfect or byte-identical rendering against the prototype.
- Do not test prototype sample data as if it were product data.
- Do not add backend scan, duplicate-analysis, cleanup, persistence, or schema
  tests unless implementation changes those boundaries unexpectedly.
- Do not test real file deletion or real Recycle Bin behavior in frontend
  tests; use existing backend verification for those contracts.
- Do not treat mobile-phone breakpoints as a product requirement; the contract
  is desktop-first, responsive-required for resizable Tauri windows.

## Uncovered gaps

None accepted as uncovered.

Actual visual reflow, overlap, and close visual similarity cannot be fully
proved by jsdom. `T16`, `T18`, and `T20` are required manual or browser-visible
verification before branch-ready claims.

## Next artifacts

- Test-spec review.
- Implementation may begin only after this test spec is accepted or activated
  for implementation.

## Follow-on artifacts

None yet.

## Readiness

This test spec is active for implementation after the explicit `$implement`
request on 2026-04-24.

It remains the proof surface for the design-fidelity milestones. `M1` is ready
for code review after its implementation validation is recorded in the active
plan.
