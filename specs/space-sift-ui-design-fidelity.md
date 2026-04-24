# Space Sift UI Design Fidelity

## Status

approved

## Related proposal

- `docs/proposals/2026-04-24-ui-design-fidelity-optimization.md`

## Goal and context

This spec defines the visual, layout, responsive, and review contract for
aligning the shipped Space Sift interface with the uploaded `docs/ui` design
direction.

The approved workspace-navigation spec already defines the shell behavior:
seven top-level workspaces, a shell-level global status surface, deterministic
next safe action, startup routing, contractual auto-switches, accessible
workspace navigation, and non-destructive shell actions. This spec does not
replace that behavior contract. It defines how the shipped UI should visually
present that contract so the app follows the uploaded prototype's component
structure and layout with close visual similarity.

The source design artifacts are:

- `docs/ui/space-sift-tabbed-ui-prototype.html`
- `docs/ui/space-sift-ui-redesign-implementation.md`

## Glossary

- **Design fidelity**: close alignment with the uploaded design's component
  structure, layout, hierarchy, spacing rhythm, responsive behavior, and visual
  emphasis.
- **Close visual similarity**: a screenshot-level match to the prototype's
  structure and visual hierarchy, not byte-level or pixel-perfect equality.
- **Prototype HTML**: `docs/ui/space-sift-tabbed-ui-prototype.html`, the
  primary source for layout, hierarchy, panel composition, and responsive
  behavior when those do not conflict with approved behavior or safety specs.
- **Implementation note**: `docs/ui/space-sift-ui-redesign-implementation.md`,
  the source for design intent, panel responsibilities, interaction notes, and
  suggested ownership patterns, but not final visual authority.
- **Approved behavior and safety sources**: the accepted proposal,
  `CONSTITUTION.md`, approved feature specs, and approved architecture or plan
  decisions that govern behavior, copy, safety, accessibility, and data
  boundaries.
- **Responsive width band**: one of the app-window width ranges used for visual
  verification: above `1050px`, between `640px` and `1050px`, and below
  `640px`.
- **Required safety content**: visible UI content or controls required by
  approved contracts for local-only history, preview-first cleanup, Recycle Bin
  default, protected-path fail-closed behavior, permanent-delete friction, and
  resume actionability.

## Examples first

### Example E1: desktop shell follows the prototype structure

Given the app window is wider than `1050px`, when a reviewer opens the app,
then the product shows a desktop-first shell with a persistent header area, a
left workspace rail, and right-side workspace content panels.

### Example E2: mid-width window keeps all workspaces reachable

Given the app window is between `640px` and `1050px`, when the reviewer
resizes the window, then the two-column shell collapses to one column, the
workspace navigation reflows, and all seven workspaces remain reachable without
removing the global status surface or next safe action.

### Example E3: narrow window preserves required safety content

Given the app window is below `640px`, when the reviewer opens Cleanup, then
the cleanup source selection, preview status, Recycle Bin default action,
permanent-delete warning or confirmation state, and relevant validation issues
remain available even if the layout stacks vertically.

### Example E4: approved copy wins over prototype sample copy

Given the prototype contains sample wording for a status or safety message,
when the approved product spec defines different status language or safety
copy, then the shipped UI uses the approved product language while preserving
the prototype's layout role for that content.

### Example E5: screenshot review checks structure, not pixels

Given implementation screenshots are captured at each responsive width band,
when they are compared against the prototype, then the review checks for
matching shell structure, component hierarchy, panel/card/table treatment, and
responsive reflow rather than exact pixel coordinates or byte-identical image
output.

### Example E6: visual polish does not add behavior

Given the reviewer activates a workspace tab after the design pass, when no
explicit scan, duplicate-analysis, cleanup-preview, cleanup-execution, or
resume action is requested, then the UI changes only the active workspace view
and does not start backend work.

## Requirements

### Source precedence

R1. The design-fidelity spec and implementation MUST resolve conflicts using
this precedence order:
1. approved behavior and safety sources, including proposals and specs
2. prototype HTML for layout, hierarchy, and responsive behavior
3. architecture and implementation notes for component ownership and wiring,
   not final visuals
4. screenshots or Figma exports as reference-only unless later checked in and
   explicitly designated authoritative

R2. The UI MUST preserve approved product text, status language, and safety
copy when prototype sample copy differs from approved behavior or safety
sources.

R3. The UI MUST aim for exact component and layout fidelity with close visual
similarity to the uploaded design, but MUST NOT require pixel-perfect or
byte-identical rendering.

### Shell and navigation layout

R4. At app-window widths above `1050px`, the UI MUST present a desktop-first
two-column shell with workspace navigation on the left and the active workspace
content on the right.

R5. The UI MUST keep a persistent header or equivalent top utility area that
supports product identity and global orientation without replacing the
workspace-navigation spec's shell-level global status contract.

R6. The workspace navigation MUST visually expose all seven top-level
workspaces: Overview, Scan, History, Explorer, Duplicates, Cleanup, and Safety.

R7. Each workspace navigation item MUST include a visible workspace label and
MUST provide enough adjacent text, iconography, or status treatment to support
the prototype's quick-scanning rail pattern without relying on color alone.

R8. The selected workspace MUST be visually distinct from unselected
workspaces and MUST remain programmatically exposed according to the approved
workspace-navigation accessibility contract.

R9. The UI MUST keep exactly one primary workspace panel visible as the active
content area.

### Panel and component fidelity

R10. Workspace panels MUST use a consistent panel/card hierarchy aligned with
the prototype's composition: panel header, explanatory or status copy,
workflow controls, metric cards where relevant, and table or review content
where relevant.

R11. Overview MUST emphasize current loaded state, active work, high-level
metrics, and the next safe action.

R12. Overview MUST include up to four high-level metric cards when data is
available. Missing data MUST render as an explicit unavailable, zero, or
not-yet-run state instead of placeholder sample values.

R13. Scan MUST visually group scan-root input, start action, running/cancel
controls, progress metrics, current path or heartbeat context, and error or
notice content close to the scan command area.

R14. History MUST visually separate completed scans from interrupted runs.
Completed scan rows MUST expose reopen actions, and interrupted-run rows MUST
keep continuity fields visible according to the continuity contract.

R15. Explorer MUST preserve read-only browseable result treatment, including
breadcrumbs or current-location context, sorting affordances, current-level
table content, usage visualization, and Explorer handoff where available.

R16. Duplicates MUST visually separate duplicate analysis controls, analysis
status, verified groups, keep-selection affordances, and delete-candidate
summary.

R17. Duplicate review content MUST render verified groups as review cards or an
equivalent grouped visual treatment that keeps file identity and keep/delete
decision context readable.

R18. Cleanup MUST present a destructive-action funnel with source selection,
preview generation or refresh, candidate review, validation issues, Recycle
Bin execution, and advanced permanent-delete confirmation.

R19. Cleanup MUST keep the Recycle Bin action visually primary over permanent
delete whenever both are visible and eligible.

R20. Safety MUST present durable guidance for unprivileged mode, Recycle
Bin-first behavior, local-only history, protected-path behavior, destructive
action safeguards, and resume actionability source of truth.

### Responsive behavior

R21. The UI MUST be desktop-first and responsive-required for resizable Tauri
windows.

R22. Responsive behavior MUST be based on app-window width, not physical
screen size.

R23. At widths between `640px` and `1050px`, the UI MUST collapse the
two-column shell to one column, reflow the workspace navigation, stack
multi-column content grids when needed, and preserve access to all workspaces.

R24. At widths below `640px`, the UI MUST use a single-column or stacked
layout for navigation, panels, command areas, cards, lists, tables, and footer
actions when needed to prevent incoherent overlap.

R25. Narrow-width layout MUST NOT remove required status content, continuity
fields, cleanup guardrails, or next-safe-action content solely because the app
window is narrow.

R26. Responsive layouts MAY reduce density, wrap controls, stack table-related
content, or hide only secondary metadata when needed, but MUST keep required
workflow actions and safety state available.

R27. The prototype breakpoints at `1050px` and `640px` MUST be treated as the
baseline responsive reference unless a later approved spec or architecture note
records a more suitable app-window threshold.

### Safety and behavior preservation

R28. The UI MUST preserve the workspace-navigation spec's shell-level global
status surface and deterministic next safe action.

R29. The shell-level next safe action MUST remain non-destructive. It MUST NOT
execute cleanup, permanent delete, or resume directly.

R30. The design-fidelity pass MUST NOT change scan, history, explorer,
duplicate-analysis, cleanup, continuity, persistence, Tauri command, or event
behavior unless a later approved spec explicitly changes that behavior.

R31. The UI MUST keep local-only history, preview-first cleanup, Recycle Bin
default execution, protected-path fail-closed behavior, and explicit
permanent-delete friction at least as visible as they were before the design
pass.

R32. Permanent-delete controls MUST remain physically and visually separated
from safe or default cleanup actions.

R33. Resume affordances MUST continue to treat `can_resume` or the equivalent
approved actionability field as the source of truth, not `has_resume` alone.

### Visual verification

R34. Implementation verification MUST include screenshot or human-visible
review evidence at app-window widths above `1050px`, between `640px` and
`1050px`, and below `640px`.

R35. At each responsive width band, verification MUST check that required
status content, continuity fields, cleanup guardrails, and next-safe-action
content remain available.

R36. Screenshot comparison MUST evaluate shell structure, component hierarchy,
panel/card/table treatment, navigation reflow, and destructive-action
separation against the prototype structure, not pixel-perfect equality.

R37. Automated tests MUST continue to cover workspace-navigation accessibility,
global status visibility, non-destructive next safe action, cleanup safety
gates, and degraded or prerequisite states affected by layout changes.

## Inputs and outputs

Inputs:

- app-window width and resizing behavior
- current active workspace
- loaded completed scan state
- live scan and duplicate-analysis state
- interrupted-run summaries and continuity fields
- duplicate-analysis result and keep-selection state
- cleanup source, preview, validation issue, execution, and confirmation state
- approved product copy and status language from governing specs
- the uploaded `docs/ui` prototype and implementation note

Outputs:

- visually aligned workspace shell
- visible workspace navigation for all seven workspaces
- active workspace panel with consistent panel/card/table structure
- shell-level global status and next safe action
- responsive layouts for the three width bands
- visible safety and destructive-action guardrails
- screenshot or human-visible review evidence for visual similarity

## State and invariants

- Only one primary workspace is active at a time.
- Manual workspace navigation does not start backend work by itself.
- The global status surface remains shell-level content, not content hidden
  inside only one workspace panel.
- The next safe action remains deterministic and non-destructive.
- Narrow-window layout changes density and placement, not safety or workflow
  availability.
- The design pass changes presentation only unless a later approved spec
  explicitly changes behavior.
- Approved behavior and safety contracts override prototype sample data and
  copy.

## Error and boundary behavior

- E1. If a metric or summary value shown in the prototype is unavailable in the
  real app state, the UI MUST show a real empty, zero, unavailable, or
  not-yet-run state instead of prototype sample data.
- E2. If a table has too many columns to fit at a narrow width, the UI MUST
  preserve required fields through wrapping, stacking, horizontal scrolling, or
  another accessible treatment rather than dropping required fields.
- E3. If visual similarity conflicts with accessibility semantics, focus
  visibility, keyboard navigation, or screen-reader association, the approved
  accessibility contract MUST win.
- E4. If visual similarity conflicts with safety copy or destructive-action
  friction, the approved safety contract MUST win.
- E5. If a workspace prerequisite is missing, the panel MUST show the approved
  prerequisite, degraded, empty, or error state rather than visually filling
  the panel with stale or sample content.
- E6. If a screenshot cannot be captured in the local environment, final
  verification MUST report the exact blocked command or local limitation and
  identify what manual visual review remains.

## Compatibility and migration

- C1. Existing app data, scan history, continuity records, duplicate hash
  cache, cleanup execution history, and workspace restore context remain
  compatible.
- C2. The design-fidelity pass does not require a data migration.
- C3. Existing Tauri command names, event names, persisted schemas, and
  frontend-backend data contracts remain unchanged by this spec.
- C4. Rollback is a presentation rollback: reverting the design-fidelity UI
  changes should keep the completed workspace-navigation shell and existing
  durable app state intact.
- C5. Older summary-only scans continue to render the approved degraded
  rescan-required behavior even if the prototype shows fully populated
  Explorer content.

## Observability

- O1. The UI MUST expose user-visible status for active work, loaded scan
  context, interrupted-run attention, cleanup preview or execution state, and
  next safe action according to approved workspace-navigation behavior.
- O2. Verification artifacts SHOULD include screenshots or equivalent
  human-visible captures for the three responsive width bands.
- O3. Verification notes MUST state whether visual similarity was checked
  against the prototype structure.
- O4. Verification notes MUST state whether required status content,
  continuity fields, cleanup guardrails, and next-safe-action content remained
  available at each width band.
- O5. Existing local shell notices and workflow status messages MUST remain
  visible according to their approved contracts after the visual changes.

## Security and privacy

- S1. The UI MUST NOT expose more path, history, duplicate, cleanup, or resume
  data than the underlying approved feature contracts already allow.
- S2. The design pass MUST NOT add cloud sync, remote accounts, telemetry, or
  network dependency for UI state.
- S3. The UI MUST NOT add a shell-level destructive action.
- S4. The UI MUST NOT add auto-elevation or weaken protected-path fail-closed
  behavior.
- S5. The UI MUST NOT store or display raw resume tokens, destructive cleanup
  candidate internals, secrets, signing material, or environment secrets.

## Accessibility and UX

- A1. The workspace navigation MUST continue to expose selected state and
  panel association programmatically.
- A2. Keyboard users MUST continue to navigate and activate workspace controls
  through the approved workspace-navigation keyboard model.
- A3. The UI MUST preserve visible focus states for workspace navigation,
  buttons, links, form controls, table actions, and cleanup confirmations.
- A4. The UI MUST NOT rely on color alone for running, interrupted, disabled,
  unsafe, selected, warning, or destructive states.
- A5. Text and controls MUST NOT overlap incoherently at any required width
  band.
- A6. Buttons and form controls MUST keep readable labels or accessible names
  after responsive reflow.
- A7. If content is visually hidden as secondary metadata at narrow widths, it
  MUST NOT be required status content, continuity fields, cleanup guardrails,
  or next-safe-action content.

## Performance expectations

- P1. Manual workspace changes MUST NOT trigger rescans, duplicate
  re-analysis, cleanup preview, cleanup execution, or resume work solely
  because the user viewed a different panel.
- P2. Responsive reflow SHOULD occur without blocking current review work or
  clearing local review state.
- P3. The design pass SHOULD avoid adding new always-running timers, polling
  loops, network calls, or backend commands solely for visual presentation.
- P4. Larger tables and review card lists SHOULD remain usable through layout,
  scrolling, or disclosure behavior already allowed by the underlying feature
  specs.

## Edge cases

- Edge 1: The app opens wider than `1050px`; the left rail and right panel
  shell are visible.
- Edge 2: The app is resized from wider than `1050px` to between `640px` and
  `1050px`; workspace navigation reflows and current workspace state remains
  selected.
- Edge 3: The app is resized below `640px`; buttons, cards, tables, and footer
  actions stack or reflow without hiding required safety or status content.
- Edge 4: Overview has no loaded scan; metric cards render empty or
  not-yet-run states rather than sample prototype values.
- Edge 5: A summary-only historical scan is loaded; Explorer keeps the
  approved degraded rescan-required state instead of rendering prototype sample
  table rows.
- Edge 6: Interrupted runs contain `has_resume = true` and `can_resume =
  false`; the UI keeps resume unavailable and visible as disabled or
  non-actionable.
- Edge 7: Cleanup has no preview yet; permanent delete remains unavailable even
  if the prototype shows a permanent-delete button shape.
- Edge 8: Cleanup preview has validation issues; the issues remain visible at
  all required width bands.
- Edge 9: A destructive warning and the Recycle Bin action are both visible;
  permanent delete remains separated and less primary than the Recycle Bin
  action.
- Edge 10: Prototype sample copy conflicts with approved safety wording; the
  UI uses approved safety wording.
- Edge 11: A visual treatment would remove focus-visible styling; the
  accessibility contract wins.
- Edge 12: Screenshot capture is blocked locally; verification reports the
  blocker and does not claim visual review passed.

## Non-goals

- Changing scan, history, explorer, duplicate-analysis, cleanup, continuity,
  persistence, Tauri command, or event behavior.
- Adding durable duplicate-analysis result persistence.
- Adding durable unexecuted cleanup-preview persistence.
- Adding new cleanup rules or destructive capabilities.
- Adding auto-elevation or a new permissions model.
- Replacing the app with a full routed frontend rewrite.
- Requiring pixel-perfect or byte-identical rendering relative to the
  prototype.
- Treating the prototype sample data as real product state.
- Making the app mobile-first; the contract is desktop-first and
  responsive-required for resizable desktop windows.

## Acceptance criteria

- A reviewer can compare the shipped desktop-width UI against the prototype and
  see the same major structure: persistent header, left workspace rail, right
  workspace panels, cards, tables, duplicate review cards, and cleanup funnel.
- A reviewer can resize the app to above `1050px`, between `640px` and
  `1050px`, and below `640px`, and observe defined responsive behavior at each
  band.
- At every required width band, all seven workspaces remain reachable.
- At every required width band, the global status surface and one next safe
  action or no-action state remain available.
- At every required width band, required continuity fields remain available in
  History.
- At every required width band, Cleanup keeps preview-first behavior, Recycle
  Bin default, permanent-delete friction, and validation issues available when
  relevant.
- The UI uses real product data and approved empty/degraded states instead of
  prototype sample values.
- Prototype sample copy does not override approved product text, status
  language, or safety copy.
- Workspace navigation remains accessible by keyboard and exposes selected
  state and panel association programmatically.
- Existing frontend tests for workspace navigation, scan history, explorer,
  duplicates, and cleanup continue to pass or are updated to assert the same
  approved behavior through the new layout.
- Verification evidence includes screenshots or a reported blocker for all
  three responsive width bands.
- No backend command, event, persistence schema, scan, duplicate, cleanup, or
  resume behavior changes are introduced by this spec.

## Open questions

None.

## Next artifacts

- Spec review for this file.
- Matching test spec: `specs/space-sift-ui-design-fidelity.test.md`.
- Execution plan: likely
  `docs/plans/2026-04-24-space-sift-ui-design-fidelity.md`.

## Follow-on artifacts

None yet.

## Readiness

This spec is approved and ready for execution planning.

It is conditionally ready for `test-spec` after the execution plan confirms
milestone scope and visual verification tooling. It is not ready for
implementation until the matching test spec and reviewed execution plan are in
place.
