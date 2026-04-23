# Space Sift UI Design Fidelity Plan

## Status

active

## Purpose / big picture

Implement a frontend-only design-fidelity pass so Space Sift follows the
uploaded `docs/ui` prototype's component structure, layout hierarchy, and
responsive behavior while preserving the approved workspace-navigation,
workflow, accessibility, and safety contracts.

This plan exists because the workspace shell is already implemented and
verified, but the product surface still needs to align more closely with the
uploaded design direction. The work should improve visual consistency,
responsive behavior, and workflow presentation without changing backend
commands, persistence, scan behavior, duplicate analysis, cleanup execution, or
release automation.

## Source artifacts

- Proposal: [2026-04-24-ui-design-fidelity-optimization.md](/D:/Data/20260415-space-sift/docs/proposals/2026-04-24-ui-design-fidelity-optimization.md:1)
- Spec: [space-sift-ui-design-fidelity.md](/D:/Data/20260415-space-sift/specs/space-sift-ui-design-fidelity.md:1)
- Spec review outcome: approved, with a follow-up to define `R12` metric-card roles in the test spec
- Existing behavior spec: [space-sift-workspace-navigation.md](/D:/Data/20260415-space-sift/specs/space-sift-workspace-navigation.md:1)
- Existing architecture: [2026-04-22-workspace-navigation-ui.md](/D:/Data/20260415-space-sift/docs/architecture/2026-04-22-workspace-navigation-ui.md:1)
- Design source: [space-sift-tabbed-ui-prototype.html](/D:/Data/20260415-space-sift/docs/ui/space-sift-tabbed-ui-prototype.html:1)
- Design implementation note: [space-sift-ui-redesign-implementation.md](/D:/Data/20260415-space-sift/docs/ui/space-sift-ui-redesign-implementation.md:1)
- Project map: [project-map.md](/D:/Data/20260415-space-sift/docs/project-map.md:1)
- Matching test spec: [space-sift-ui-design-fidelity.test.md](/D:/Data/20260415-space-sift/specs/space-sift-ui-design-fidelity.test.md:1)

## Context and orientation

- [App.tsx](/D:/Data/20260415-space-sift/src/App.tsx:1) is the current React orchestration surface for the workspace shell, global status, scan, history, explorer, duplicates, cleanup, and safety views.
- [App.css](/D:/Data/20260415-space-sift/src/App.css:1) is the current styling surface and is the primary expected edit target for shell, panel, card, table, responsive, and safety visual changes.
- [workspaceNavigation.ts](/D:/Data/20260415-space-sift/src/workspaceNavigation.ts:1) owns workspace navigation and shell-status derivation helpers; changes here should be unnecessary unless the visual pass exposes a contract bug.
- [workspace-navigation.test.tsx](/D:/Data/20260415-space-sift/src/workspace-navigation.test.tsx:1) already covers workspace accessibility, global status, next safe action, startup behavior, auto-switching, and shell observability.
- Existing workflow test suites are:
  - [scan-history.test.tsx](/D:/Data/20260415-space-sift/src/scan-history.test.tsx:1)
  - [results-explorer.test.tsx](/D:/Data/20260415-space-sift/src/results-explorer.test.tsx:1)
  - [duplicates.test.tsx](/D:/Data/20260415-space-sift/src/duplicates.test.tsx:1)
  - [cleanup.test.tsx](/D:/Data/20260415-space-sift/src/cleanup.test.tsx:1)
- The prototype uses a desktop two-column shell above `1050px`, collapses to one column below `1050px`, and further stacks navigation and actions below `640px`.
- The source precedence is fixed by the spec: approved behavior and safety sources win over prototype visuals and sample copy.
- Existing dirty local plan files predate this plan. This plan should not rewrite them except where a future explicit coordination update requires it.

## Non-goals

- No backend scan, history, explorer, duplicate-analysis, cleanup, continuity, persistence, Tauri command, or event behavior changes.
- No durable duplicate-analysis result persistence.
- No durable unexecuted cleanup-preview persistence.
- No new cleanup rules, destructive capabilities, auto-elevation, or permissions model.
- No full routed frontend rewrite.
- No pixel-perfect or byte-identical rendering requirement.
- No use of prototype sample data as product state.
- No mobile-first redesign; this is desktop-first and responsive-required for resizable Tauri windows.

## Requirements covered

| Requirement area | Planned milestone |
| --- | --- |
| `R1`-`R3` source precedence, approved copy, and non-pixel-perfect fidelity | `M1`-`M4`, test spec |
| `R4`-`R9` shell, header, navigation, selected state, and active panel shape | `M1` |
| `R10`-`R15` panel/card hierarchy for Overview, Scan, History, and Explorer | `M2` |
| `R16`-`R20` Duplicates, Cleanup, and Safety panel fidelity | `M3` |
| `R21`-`R27` responsive width-band behavior | `M1`, `M4` |
| `R28`-`R33` safety and behavior preservation | `M1`-`M3` |
| `R34`-`R37`, observability, acceptance visual evidence | `M4` |
| Error and boundary behavior `E1`-`E6`, edge cases, compatibility | `M2`-`M4` |

## Milestones

### M1. Shell layout, rail, and responsive foundation

- Goal: Align the app shell with the prototype's persistent header, left workspace rail, right content panel layout, and baseline responsive breakpoints without changing workflow behavior.
- Requirements: `R1`-`R9`, `R21`-`R29`, `A1`-`A7`, `P1`-`P3`, Edge 1-3.
- Files/components likely touched:
  - [App.tsx](/D:/Data/20260415-space-sift/src/App.tsx:1)
  - [App.css](/D:/Data/20260415-space-sift/src/App.css:1)
  - [workspace-navigation.test.tsx](/D:/Data/20260415-space-sift/src/workspace-navigation.test.tsx:1)
  - new focused UI design-fidelity test file if created by the test spec
- Dependencies:
  - approved spec
  - reviewed plan
  - matching test spec with width-band and `R12` metric-role coverage defined before implementation
- Tests to add/update:
  - desktop shell exposes left workspace rail and active panel
  - all seven workspaces remain reachable after responsive reflow
  - global status and next safe action remain visible after shell restyling
  - selected workspace remains programmatically exposed
- Implementation steps:
  1. Map current shell sections against the prototype's header, workspace rail, status, and panel areas.
  2. Adjust shell markup only where needed to support the left-rail and right-panel visual structure.
  3. Update CSS for desktop layout above `1050px`.
  4. Add responsive CSS for `640px`-`1050px` and below `640px`.
  5. Preserve existing ARIA tab semantics, selected state, keyboard behavior, and shell-level global status.
- Validation commands:
  - `npm run test -- src/workspace-navigation.test.tsx`
  - `npm run lint`
  - `npm run test`
  - `npm run build`
- Expected observable result: the app opens with a prototype-aligned shell and rail at desktop width, collapses predictably at narrower widths, and retains the existing workspace-navigation behavior.
- Commit message: `M1: align workspace shell layout with design prototype`
- Milestone closeout:
  - validation passed
  - progress updated
  - decision log updated if needed
  - validation notes updated
  - milestone committed
- Risks:
  - shell restyling breaks accessibility or focus behavior
  - global status becomes visually hidden in the new layout
- Rollback/recovery: revert shell markup/CSS changes while keeping the existing workspace shell and tests intact.

### M2. Overview, Scan, History, and Explorer panel fidelity

- Goal: Align the core scan-review panels with the prototype's panel header, metric-card, command, table, and read-only explorer composition while preserving approved product copy and workflow behavior.
- Requirements: `R1`-`R3`, `R10`-`R15`, `R25`-`R33`, `E1`-`E5`, Edge 4-6.
- Files/components likely touched:
  - [App.tsx](/D:/Data/20260415-space-sift/src/App.tsx:1)
  - [App.css](/D:/Data/20260415-space-sift/src/App.css:1)
  - [workspace-navigation.test.tsx](/D:/Data/20260415-space-sift/src/workspace-navigation.test.tsx:1)
  - [scan-history.test.tsx](/D:/Data/20260415-space-sift/src/scan-history.test.tsx:1)
  - [results-explorer.test.tsx](/D:/Data/20260415-space-sift/src/results-explorer.test.tsx:1)
- Dependencies:
  - `M1` shell foundation
  - test spec definitions for Overview metric-card roles and acceptable empty states
- Tests to add/update:
  - Overview metric cards render real or explicit unavailable states, not sample values
  - Scan command area keeps start/cancel/progress/error content grouped
  - History visually separates completed scans from interrupted runs and keeps continuity fields available
  - Explorer keeps breadcrumbs/current location, sorting, usage visualization, and degraded summary-only state
- Implementation steps:
  1. Replace any remaining generic stacked panel treatment in Overview with metric and status composition aligned to the prototype.
  2. Restyle Scan command and live progress areas while preserving active-scan behavior.
  3. Restyle History completed-scan and interrupted-run sections as distinct review surfaces.
  4. Restyle Explorer as a read-only current-level table with existing breadcrumbs, sort controls, usage cues, and Explorer handoff.
  5. Confirm approved product text and safety/status language wins over prototype sample copy.
- Validation commands:
  - `npm run test -- src/workspace-navigation.test.tsx`
  - `npm run test -- src/scan-history.test.tsx`
  - `npm run test -- src/results-explorer.test.tsx`
  - `npm run lint`
  - `npm run test`
  - `npm run build`
- Expected observable result: Overview, Scan, History, and Explorer match the prototype's component hierarchy closely while preserving existing behavior and degraded states.
- Commit message: `M2: align scan review panels with design prototype`
- Milestone closeout:
  - validation passed
  - progress updated
  - decision log updated if needed
  - validation notes updated
  - milestone committed
- Risks:
  - tables become less readable at narrow widths
  - continuity fields are accidentally treated as secondary metadata
- Rollback/recovery: revert the affected panel markup/CSS and keep the shell foundation from `M1` if it remains correct.

### M3. Duplicates, Cleanup, and Safety fidelity

- Goal: Align the duplicate-review cards, cleanup funnel, and safety panel with the prototype while preserving explicit cleanup guardrails and duplicate-selection behavior.
- Requirements: `R1`-`R3`, `R16`-`R20`, `R25`-`R33`, `S1`-`S5`, Edge 7-11.
- Files/components likely touched:
  - [App.tsx](/D:/Data/20260415-space-sift/src/App.tsx:1)
  - [App.css](/D:/Data/20260415-space-sift/src/App.css:1)
  - [duplicates.test.tsx](/D:/Data/20260415-space-sift/src/duplicates.test.tsx:1)
  - [cleanup.test.tsx](/D:/Data/20260415-space-sift/src/cleanup.test.tsx:1)
  - [workspace-navigation.test.tsx](/D:/Data/20260415-space-sift/src/workspace-navigation.test.tsx:1)
- Dependencies:
  - `M1` shell foundation
  - `M2` shared panel/card styling
- Tests to add/update:
  - verified duplicate groups render as grouped review cards with keep/delete decision context
  - cleanup source selection, preview, issues, Recycle Bin action, and permanent-delete confirmation remain visibly distinct
  - permanent delete remains unavailable without preview and explicit confirmation
  - Safety panel keeps unprivileged, local-only, Recycle Bin, protected-path, and resume-actionability guidance
- Implementation steps:
  1. Restyle duplicate analysis summary and verified groups into prototype-aligned review cards.
  2. Restyle cleanup as a staged funnel: sources, preview, issues, execution, advanced permanent-delete confirmation.
  3. Make Recycle Bin action visually primary over permanent delete while preserving existing gating.
  4. Restyle Safety as first-class guidance cards without changing copy meaning.
  5. Confirm no shell-level destructive actions are introduced.
- Validation commands:
  - `npm run test -- src/duplicates.test.tsx`
  - `npm run test -- src/cleanup.test.tsx`
  - `npm run test -- src/workspace-navigation.test.tsx`
  - `npm run lint`
  - `npm run test`
  - `npm run build`
- Expected observable result: duplicate, cleanup, and safety workflows visually match the prototype's card/funnel model while preserving all destructive-action safeguards.
- Commit message: `M3: align duplicate cleanup and safety panels with design`
- Milestone closeout:
  - validation passed
  - progress updated
  - decision log updated if needed
  - validation notes updated
  - milestone committed
- Risks:
  - visual polish makes destructive affordances too quiet
  - duplicate-review cards become denser and reduce path readability
- Rollback/recovery: revert the duplicate/cleanup/safety panel changes independently of `M1` and `M2`.

### M4. Responsive visual verification and final hardening

- Goal: Capture and record design-fidelity evidence at the required width bands, then fix any remaining overlap, hidden required content, or visual divergence that affects the approved contract.
- Requirements: `R21`-`R37`, `O1`-`O5`, `E2`, `E6`, Edge 1-3, Edge 8-12, acceptance criteria.
- Files/components likely touched:
  - [App.css](/D:/Data/20260415-space-sift/src/App.css:1)
  - [workspace-navigation.test.tsx](/D:/Data/20260415-space-sift/src/workspace-navigation.test.tsx:1)
  - focused UI design-fidelity test file from the test spec
  - plan validation notes
- Dependencies:
  - `M1` through `M3`
  - local ability to run the app in a browser or Tauri shell for screenshots
- Tests to add/update:
  - responsive shell contract tests feasible in jsdom
  - assertions that required status, continuity fields, cleanup guardrails, and next safe action remain in the DOM at responsive states where feasible
  - manual or browser screenshot checklist for `>1050px`, `640px`-`1050px`, and `<640px`
- Implementation steps:
  1. Run targeted tests and full frontend verification after `M1`-`M3`.
  2. Start a local Vite or Tauri preview as appropriate for screenshot review.
  3. Capture or manually inspect screenshots at representative widths above `1050px`, between `640px` and `1050px`, and below `640px`.
  4. Compare screenshots against the prototype structure, not pixel-perfect output.
  5. Fix remaining overlap, hidden required content, inaccessible controls, or safety-visibility issues.
  6. Record visual review evidence and any blocked screenshot commands in validation notes.
- Validation commands:
  - `npm run test -- src/workspace-navigation.test.tsx`
  - `npm run test -- src/scan-history.test.tsx`
  - `npm run test -- src/results-explorer.test.tsx`
  - `npm run test -- src/duplicates.test.tsx`
  - `npm run test -- src/cleanup.test.tsx`
  - `npm run lint`
  - `npm run test`
  - `npm run build`
  - `powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1` before branch-ready claims
  - visual review at widths above `1050px`, between `640px` and `1050px`, and below `640px`
- Expected observable result: the completed UI has recorded responsive visual evidence and no known hidden required status, continuity, cleanup guardrail, or next-safe-action content.
- Commit message: `M4: verify responsive design fidelity`
- Milestone closeout:
  - validation passed
  - progress updated
  - decision log updated if needed
  - validation notes updated
  - milestone committed
- Risks:
  - local environment cannot capture Tauri screenshots
  - responsive fixes regress desktop structure
- Rollback/recovery: keep verified milestone slices and revert only the final responsive hardening that caused regression; if screenshots cannot be captured, record the blocker and stop before branch-ready claims.

## Validation plan

- Pre-implementation gate:
  - plan review must approve or request revisions
  - matching test spec must map `R1`-`R37`, edge cases, and acceptance criteria to concrete tests or manual verification
- `M1`: `npm run test -- src/workspace-navigation.test.tsx`, `npm run lint`, `npm run test`, `npm run build`
- `M2`: `npm run test -- src/workspace-navigation.test.tsx`, `npm run test -- src/scan-history.test.tsx`, `npm run test -- src/results-explorer.test.tsx`, `npm run lint`, `npm run test`, `npm run build`
- `M3`: `npm run test -- src/duplicates.test.tsx`, `npm run test -- src/cleanup.test.tsx`, `npm run test -- src/workspace-navigation.test.tsx`, `npm run lint`, `npm run test`, `npm run build`
- `M4`: run all targeted workflow suites, `npm run lint`, `npm run test`, `npm run build`, visual review at the three width bands, and `powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1` before branch-ready claims
- Rust validation is not expected because this plan is frontend-only. If any `src-tauri/` file changes unexpectedly, add `cargo check --manifest-path src-tauri/Cargo.toml`.
- If local browser or Tauri screenshot capture is blocked, record the exact command and error in validation notes and do not claim visual review passed.

## Risks and recovery

- Risk: `App.tsx` grows more complex during visual work.
  - Recovery: extract only presentation helpers or components that reduce local complexity; do not move business logic without a plan update.
- Risk: visual similarity conflicts with approved safety copy or behavior.
  - Recovery: approved specs win; record the divergence and keep product copy/behavior.
- Risk: narrow layouts hide continuity fields or cleanup guardrails.
  - Recovery: stack or scroll required content; do not drop required fields as secondary metadata.
- Risk: test churn from layout changes obscures behavior regressions.
  - Recovery: keep workflow assertions behavior-focused and add design-fidelity tests only for visual contract points.
- Risk: visual screenshot review cannot run locally.
  - Recovery: state the blocked command/environment limitation and leave manual visual review as an explicit blocker before branch-ready claims.

## Dependencies

- Approved proposal and approved spec.
- Plan review before implementation.
- Matching test spec before implementation.
- Existing workspace-navigation architecture remains sufficient unless plan review identifies an ownership or visual-verification architecture gap.
- `M1` should land before panel-specific milestones because later panels depend on shared shell and responsive styling.
- `M2` and `M3` can be reviewed independently after `M1`; they should not edit overlapping panel areas in parallel unless write scopes are split carefully.
- `M4` depends on `M1` through `M3`.

## Progress

- [x] Proposal accepted
- [x] Spec approved
- [x] Plan reviewed
- [x] Matching test spec created
- [x] M1 complete
- [x] M2 complete
- [x] M3 complete
- [ ] M4 complete
- [ ] Branch-wide verification complete

## Decision log

- 2026-04-24: Plan status starts as `draft`.
  - Reason: implementation must wait for plan review and matching test spec.
- 2026-04-24: Keep the initiative frontend-only.
  - Reason: the approved design-fidelity spec prohibits backend command, event, persistence, scan, duplicate, cleanup, and resume behavior changes.
- 2026-04-24: Use the existing workspace-navigation architecture.
  - Reason: this design pass changes visual fidelity and responsive layout, not shell state ownership or backend boundaries.
- 2026-04-24: Put responsive visual verification in a final hardening milestone.
  - Reason: meaningful screenshots require the shell and panel surfaces to be mostly complete.
- 2026-04-24: Plan review approved this plan for the test-spec stage.
  - Reason: review found the sequencing, source precedence, and visual-verification milestones sufficient before implementation.
- 2026-04-24: Created the matching UI design-fidelity test spec.
  - Reason: implementation needs traceable coverage for `R1`-`R37`, edge cases, responsive width bands, and manual visual evidence before code changes.
- 2026-04-24: Treat the explicit `$implement` request as activation of the draft test spec for implementation.
  - Reason: the test spec allowed implementation after acceptance or explicit activation; the user explicitly requested implementation after plan and test-spec creation.
- 2026-04-24: Implemented M1 by reshaping the shell into a persistent topbar, left workspace rail, right active workspace area, and `1050px` / `640px` responsive foundation.
  - Reason: this is the smallest scope-complete shell slice before panel-specific fidelity work in M2 and M3.
- 2026-04-24: Replaced the M1 stylesheet static test's `import.meta.url`
  file lookup with a repo-root path.
  - Reason: Vitest under jsdom did not expose `import.meta.url` as a `file:`
    URL for that assertion, while the test only needed to inspect the tracked
    stylesheet source.
- 2026-04-24: Removed the prototype-only `Desktop bridge connected` topbar
  pill.
  - Reason: bridge capability is not modeled as truthful UI state yet, so the
    shell keeps only stable safety/locality utility copy by default.
- 2026-04-24: Added direct M1 DOM availability proof at `1280`, `900`, and
  `560` CSS pixels.
  - Reason: the first code-review pass found that M1 had breakpoint CSS proof
    but lacked direct `T12` proof for required shell content across the
    accepted large, medium, and small app-window widths.
- 2026-04-24: Implemented M2 with explicit Overview metric cards and
  accessible review regions for Scan, History, and Explorer.
  - Reason: the core scan-review panels needed prototype-aligned panel/card,
    command, table, and degraded-state composition without changing scan,
    history, Explorer handoff, or resume behavior.
- 2026-04-24: Implemented M3 with explicit Duplicates, Cleanup, and Safety
  review regions.
  - Reason: the duplicate-review cards, cleanup funnel, and durable safety
    guidance needed prototype-aligned structure while preserving Recycle Bin
    priority, permanent-delete gating, protected-path exclusion, and
    `can_resume` actionability.

## Surprises and discoveries

- The earlier local esbuild `Error: spawn EPERM` blocker did not reproduce
  during the final M1 validation pass; Vite-backed tests and build completed.
- The first rerun of `npm run test -- src/workspace-navigation.test.tsx`
  exposed a test-only file URL assumption in the new stylesheet assertion. The
  production shell implementation did not need changes for that failure.
- The first code-review pass found two M1 issues: an unconditional prototype
  bridge-status pill that could misrepresent the unsupported bridge state, and
  missing direct `T12` responsive DOM proof. Both were fixed in the M1
  review-resolution commit.
- M2 did not require backend, state-management, or component extraction. The
  existing panel functions already preserved the approved workflows; the
  implementation added truthful metric-card state, accessible region names, and
  light styling hooks around the existing behavior.
- M3 did not require backend, persistence, cleanup capability, or duplicate
  selection changes. The existing workflow state already provided the needed
  guardrails; the implementation made the review stages explicit and testable.

## Aligned-surface audit

- `src-tauri/`: unaffected with rationale. M1 is frontend shell/layout only
  and the approved spec prohibits backend command, event, persistence, scan,
  duplicate, cleanup, and resume behavior changes.
- Feature panel behavior: unaffected with rationale. M1 only changes the
  shell wrapper, workspace rail, responsive foundation, and tab keyboard
  support; M2 and M3 own panel-specific visual fidelity.
- Visual screenshot evidence: unaffected with rationale for M1 closeout.
  M4 owns final screenshot or human-visible comparison after panel fidelity is
  implemented.
- `src-tauri/`: unaffected with rationale for M2. The milestone changed only
  React markup, CSS, and frontend tests; no Tauri command, event, persistence,
  scan, duplicate, cleanup, or resume contract changed.
- Duplicates, Cleanup, and Safety panel fidelity: unaffected with rationale.
  M3 owns these panels; M2 intentionally touched only Overview, Scan, History,
  and Explorer.
- Visual screenshot evidence: still unaffected with rationale for M2 closeout.
  M4 owns final screenshot or human-visible comparison after M3 panel fidelity
  is implemented.
- `src-tauri/`: unaffected with rationale for M3. The milestone changed only
  React markup, CSS, and frontend tests; no Tauri command, event, persistence,
  scan, duplicate, cleanup, or resume contract changed.
- Overview, Scan, History, and Explorer panel fidelity: unaffected with
  rationale. M2 owns those panels; M3 intentionally touched only Duplicates,
  Cleanup, and Safety plus shared safety guidance copy.
- Visual screenshot evidence: still unaffected with rationale for M3 closeout.
  M4 owns final screenshot or human-visible comparison across the required
  width bands.

## Validation notes

- 2026-04-24: `npm run lint`
  - passed
- 2026-04-24: `npm run build`
  - passed
- 2026-04-24: `npm run test -- src/workspace-navigation.test.tsx`
  - failed once after the earlier environment blocker cleared
  - exact failure: `workspace_shell_styles_define_design_breakpoints` used
    `readFileSync(new URL("./App.css", import.meta.url), "utf8")`, but Vitest
    reported `TypeError: The URL must be of scheme file`
  - fixed by reading `src/App.css` from the repo root
- 2026-04-24: `npm run test -- src/workspace-navigation.test.tsx`
  - passed after the test-file lookup fix
- 2026-04-24: `npm run test`
  - passed
  - result: 7 test files passed, 74 tests passed
- 2026-04-24: M1 code review
  - result: changes requested
  - findings: remove untruthful `Desktop bridge connected` prototype copy; add
    direct responsive DOM availability proof at `1280`, `900`, and `560`
- 2026-04-24: `npm run test -- src/workspace-navigation.test.tsx`
  - passed after review-resolution
  - result: 36 tests passed
- 2026-04-24: `npm run test`
  - passed after review-resolution
  - result: 7 test files passed, 75 tests passed
- 2026-04-24: `git diff --check -- src\App.tsx src\App.css src\workspace-navigation.test.tsx`
  - passed
  - output contained CRLF conversion warnings only
- 2026-04-24: `npm run test -- src/workspace-navigation.test.tsx`
  - failed after M2 tests were added, before production changes
  - expected failure: missing Overview metric region and Scan command/progress
    region
- 2026-04-24: `npm run test -- src/scan-history.test.tsx`
  - failed after M2 tests were added, before production changes
  - expected failure: missing completed-history and interrupted-run continuity
    regions
- 2026-04-24: `npm run test -- src/results-explorer.test.tsx`
  - failed after M2 tests were added, before production changes
  - expected failure: missing read-only Explorer and summary-only compatibility
    regions
- 2026-04-24: `npm run test -- src/workspace-navigation.test.tsx`
  - passed after M2 implementation
  - result: 39 tests passed
- 2026-04-24: `npm run test -- src/scan-history.test.tsx`
  - passed after M2 implementation
  - result: 11 tests passed
- 2026-04-24: `npm run test -- src/results-explorer.test.tsx`
  - passed after M2 implementation
  - result: 5 tests passed
- 2026-04-24: `npm run lint`
  - passed after M2 implementation
- 2026-04-24: `npm run test`
  - passed after M2 implementation
  - result: 7 test files passed, 79 tests passed
- 2026-04-24: `npm run build`
  - passed after M2 implementation
- 2026-04-24: `git diff --check -- src\App.tsx src\App.css src\workspace-navigation.test.tsx src\scan-history.test.tsx src\results-explorer.test.tsx`
  - passed
  - output contained CRLF conversion warnings only
- 2026-04-24: `npm run test -- src/duplicates.test.tsx`
  - failed after M3 tests were added, before production changes
  - expected failure: missing duplicate analysis controls/status, duplicate
    delete summary, and verified duplicate review regions
- 2026-04-24: `npm run test -- src/cleanup.test.tsx`
  - failed after M3 tests were added, before production changes
  - expected failure: missing cleanup source selection, preview review,
    validation issues, Recycle Bin action, and advanced permanent-delete
    regions
- 2026-04-24: `npm run test -- src/workspace-navigation.test.tsx`
  - failed after M3 tests were added, before production changes
  - expected failure: missing Safety guidance region
- 2026-04-24: `npm run test -- src/duplicates.test.tsx`
  - passed after M3 implementation
  - result: 12 tests passed
- 2026-04-24: `npm run test -- src/cleanup.test.tsx`
  - passed after M3 implementation
  - result: 6 tests passed
- 2026-04-24: `npm run test -- src/workspace-navigation.test.tsx`
  - passed after M3 implementation
  - result: 40 tests passed
- 2026-04-24: `npm run lint`
  - passed after M3 implementation
- 2026-04-24: `npm run test`
  - failed once after M3 implementation
  - exact failures: `App.test.tsx` still expected the durable phrase `local
    SQLite storage`; `tsc` rejected test fixture code `protected_path` because
    `CleanupIssueCode` already models this as `requires_elevation`
  - fixed by preserving the old stable SQLite wording and using the existing
    typed cleanup issue code
- 2026-04-24: `npm run test`
  - passed after M3 validation fixes
  - result: 7 test files passed, 82 tests passed
- 2026-04-24: `npm run build`
  - failed once after M3 implementation
  - exact failure: `src/cleanup.test.tsx` used issue code `protected_path`,
    which is not assignable to `CleanupIssueCode`
  - fixed by using the existing `requires_elevation` code while preserving the
    protected-path visible issue summary
- 2026-04-24: `npm run build`
  - passed after M3 validation fixes

## Outcome and retrospective

- M1 is implemented and validation-complete. The shell now uses a persistent
  header, left workspace rail, right active workspace content area, shell-level
  global status within that content area, vertical tab orientation semantics,
  arrow-up/down keyboard support, and responsive CSS at the prototype baseline
  breakpoints.
- M2 is implemented and validation-complete. Overview now renders four truthful
  metric cards with real or explicit unavailable states; Scan exposes a grouped
  command/progress region and active-scan detail region; History separates
  completed scan history from interrupted-run continuity; Explorer exposes a
  read-only browseable result region and a summary-only compatibility region.
- M3 is implemented and validation-complete. Duplicates now expose distinct
  analysis controls, status, delete summary, and verified group review regions;
  Cleanup now exposes source, preview, validation issue, Recycle Bin execution,
  and advanced permanent-delete stages; Safety now presents durable local,
  unprivileged, Recycle Bin, protected-path, permanent-delete, and resume
  guidance.

## Readiness

This plan is active. M3 is implemented and ready for code review.

M4 remains unstarted. Branch-wide readiness is blocked until the remaining
milestone is implemented and final visual verification is recorded.
