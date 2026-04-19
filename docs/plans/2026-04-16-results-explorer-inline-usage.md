# Unify Results Explorer And Relative Usage View

## Metadata

- Status: done
- Created: 2026-04-16
- Updated: 2026-04-16
- Owner: xiongxianfei / Codex
- Related spec(s):
  - `specs/space-sift-results-explorer.md`
  - `specs/space-sift-results-explorer.test.md`
- Related plan(s):
  - `docs/plans/2026-04-15-space-sift-win11-mvp.md`
- Supersedes / Superseded by: none
- Branch / PR: none yet
- Last reviewed files:
  - `src/App.tsx`
  - `src/App.css`
  - `src/results-explorer.test.tsx`
  - `src/scan-history.test.tsx`

## Purpose / Big picture

Improve the scan-results browsing experience by replacing the current split
layout of `Results explorer` plus a separate `Space map` panel with a single,
clearer explorer table that includes an inline relative-usage visualization for
each visible row.

The user outcome is simpler scanning and comparison: each file or directory
should be understandable in one row without forcing the user to visually match
a table row to a second panel elsewhere on the screen.

## Context and orientation

- The current Milestone 3 results UI in `src/App.tsx` renders two explorer
  panels from the same `visibleEntries` collection:
  - a `Current folder contents` table
  - a separate `Space map` list with relative-usage bars
- That dual-panel approach is contractually reflected in
  `specs/space-sift-results-explorer.md`, which currently requires a separate
  `space map`.
- Recent bug fixes confirmed the current layout problem:
  - file labels had to be restored in the table
  - the space map needed visible labels added so users could align bars to
    rows at all
- The data model does not need to change. The scan result already contains
  every field required for a unified table row:
  - full path
  - kind
  - size
  - current-level relative share computable from `visibleEntries`
- This initiative is a UI contract refinement, not a new backend feature.
  It should remain additive to the MVP plan rather than replacing it.

## Constraints

- Keep the explorer read-only. This change must not introduce delete, move,
  rename, or duplicate-selection behavior into the explorer rows.
- Do not change the stored scan payload or any Rust scan/history backend code.
- Preserve existing explorer capabilities:
  - root-default browsing
  - breadcrumb navigation
  - deterministic sorting
  - Explorer handoff
  - summary-only fallback for older saved scans
- The new inline relative-usage visualization must stay understandable on both
  desktop and mobile layouts.
- Visible row labels must remain unique and readable enough for tests and human
  users to identify the selected item without relying only on `title` or
  `aria-label`.
- The contract should describe the user-facing behavior in terms of a unified
  explorer table, not the current two-panel implementation detail.

## Done when

- The results experience renders one primary table for the current folder
  instead of a separate table plus space-map panel.
- Each visible row includes at least:
  - name
  - kind
  - size
  - relative-usage visualization for the current level
  - row actions
- Sorting by name and size still reorders the unified table deterministically.
- Breadcrumb navigation and Explorer handoff still work as they do today.
- Empty-state handling remains explicit when a browsed folder has no immediate
  children.
- Older summary-only scan history entries still degrade gracefully without
  browseable controls.
- Specs and tests describe the unified explorer behavior rather than the old
  split layout.
- Frontend verification passes with the updated contract.

## Non-goals

- Changing the scan result schema
- Adding treemap geometry, canvas rendering, or a recursive graph view
- Adding new sort modes beyond the current supported options
- Adding search, filter, selection, or bulk actions to explorer rows
- Changing duplicate-review or cleanup flows
- Reworking the whole app visual style outside the current explorer area

## Milestones

### Milestone 1: Realign the explorer contract to a unified table

Scope: update the approved explorer contract and test mapping so the UI is
required to show relative usage inline with each visible row instead of in a
separate space-map panel.

Files or components touched:
- `specs/space-sift-results-explorer.md`
- `specs/space-sift-results-explorer.test.md`
- `docs/roadmap.md`

Dependencies:
- none beyond the existing approved Milestone 3 explorer contract

Risk:
- Weakening the current relative-usage requirement accidentally while removing
  the separate-panel wording

Validation commands:
- `Get-Content specs/space-sift-results-explorer.md`
- `Get-Content specs/space-sift-results-explorer.test.md`
- `Get-Content docs/roadmap.md`

Expected observable result:
- The repository contract now describes one explorer surface with inline usage
  bars or percentages, and the roadmap no longer treats this as an unapproved
  idea.

### Milestone 2: Replace the split explorer layout with a unified table

Scope: implement the inline usage column and remove the separate space-map
panel while preserving current navigation, sorting, and Explorer handoff.

Files or components touched:
- `src/App.tsx`
- `src/App.css`
- `src/results-explorer.test.tsx`
- `src/scan-history.test.tsx`

Dependencies:
- Milestone 1 spec/test-spec updates

Risk:
- The unified table can become too dense or unreadable on narrow screens if
  the usage column is not designed carefully

Validation commands:
- `npm run test -- results`
- `npm run test -- history`
- `npm run lint`
- `npm run build`

Expected observable result:
- The user sees one results table where each row shows both item metadata and
  current-level relative usage, without needing to cross-reference a second
  explorer panel.

### Milestone 3: Full regression pass and UX cleanup

Scope: run the broader frontend suite, resolve any text-query collisions or
responsive-layout regressions caused by the new row content, and record any
 discoveries back into the active plan.

Files or components touched:
- `src/results-explorer.test.tsx`
- `src/scan-history.test.tsx`
- `src/duplicates.test.tsx`
- `docs/plans/2026-04-16-results-explorer-inline-usage.md`

Dependencies:
- Milestone 2 implementation

Risk:
- Shared `App.tsx` render changes can break unrelated tests that currently rely
  on global text matches or older DOM structure assumptions

Validation commands:
- `npm run test`
- `npm run lint`
- `npm run build`

Expected observable result:
- The explorer change is stable across the existing frontend surface and the
  plan reflects any real implementation surprises rather than the initial
  design assumptions.

## Progress

- [x] 2026-04-16: user approved moving from a split `Results explorer` +
  `Space map` layout toward a unified row-based explorer with inline usage.
- [x] 2026-04-16: reviewed the active MVP plan, current explorer spec/test
  spec, and the present `App.tsx` / `App.css` implementation before drafting
  this follow-up plan.
- [x] 2026-04-16: Milestone 1 completed by updating the explorer contract and
  test spec from a split `Space map` panel to inline per-row relative usage.
- [x] 2026-04-16: Milestone 2 completed by replacing the dual-panel explorer
  UI with a unified current-folder table that includes a `Usage` column with
  visible percentages and inline bars.
- [x] 2026-04-16: Milestone 3 completed by running the full frontend suite,
  lint, and build checks after the shared `App.tsx` markup change.

## Surprises & discoveries

- 2026-04-16: the current space map already uses the same `visibleEntries`
  ordering as the table, which means this change is a presentation refactor,
  not a data-model or command-layer change.
- 2026-04-16: recent UI bug fixes already pushed the current design toward
  visible per-item labels in both places, which is a strong signal that the
  split layout is creating unnecessary alignment work for both users and tests.
- 2026-04-16: once the usage cue moved into the table, one scan-history test
  needed to scope its filename assertion to the explorer table because visible
  labels were no longer isolated to one region of the page.

## Decision log

- 2026-04-16: treat this as a new approved follow-up initiative, not a silent
  edit to the MVP plan body.
  Rationale: the MVP plan already spans the whole product; this UX refinement
  deserves its own reviewable execution trail and can proceed independently.

- 2026-04-16: prefer a unified explorer table with inline relative-usage bars
  over preserving a second side-panel visualization.
  Rationale: users compare one row at a time when deciding what consumes space;
  the usage cue should live on that row rather than in a separate panel.

- 2026-04-16: keep this initiative frontend-only unless implementation proves
  otherwise.
  Rationale: the current scan payload and view-model logic already expose the
  required data, so backend churn would add risk without clear value.

- 2026-04-16: show both a visible percentage label and an inline usage bar in
  the unified table instead of a bar-only presentation.
  Rationale: the visible percentage keeps row comparison understandable in
  tests, accessibility tooling, and narrow layouts without hovering.

## Validation and acceptance

Planning validation performed while creating this plan:
- `Get-Content AGENTS.md`
- `Get-Content docs/workflows.md`
- `Get-Content docs/plan.md`
- `Get-Content docs/plans/2026-04-15-space-sift-win11-mvp.md`
- `Get-Content specs/space-sift-results-explorer.md`
- `Get-Content specs/space-sift-results-explorer.test.md`
- `Get-Content src/results-explorer.test.tsx`
- `Get-Content src/scan-history.test.tsx`
- `rg -n "Space map|space map|results-table|current folder contents" src/App.tsx src/App.css specs/space-sift-results-explorer.md specs/space-sift-results-explorer.test.md`

Acceptance evidence for the implemented change:
- The results explorer shows one primary table for current-folder browsing.
- Reviewers can still sort, navigate with breadcrumbs, and open entries in
  Explorer without learning a new interaction model.
- The current-level usage cue is visible on the same row as the item it
  describes.
- No separate space-map panel remains in the current-result view.
- Frontend tests pass with the new structure.

## Validation notes

- 2026-04-16: planning-only turn. No code, lint, or build commands were run
  for this plan creation itself.
- 2026-04-16: the current explorer contract and implementation were read and
  confirmed to still reference a separate `Space map` panel, which is why a
  spec-first follow-up plan is needed before implementation.
- 2026-04-16: `npm run test -- results` failed first with `Unable to find
  role="columnheader" and name /usage/i`, confirming the new unified-table
  test was exercising missing behavior in the old split layout.
- 2026-04-16: `npm run test -- results` passed after the `Usage` column and
  inline usage cell landed in `src/App.tsx`.
- 2026-04-16: `npm run test -- history` passed after preserving the existing
  reopen/history behavior against the unified table.
- 2026-04-16: `npm run test`, `npm run lint`, and `npm run build` all passed
  after the broader frontend regression pass.

## Idempotence and recovery

- This initiative should land behind normal small PRs. If implementation turns
  out to be confusing on mobile or too dense for long labels, revert the UI
  change and keep the contract/spec updates out of the same merge until the
  replacement design is proven.
- Because no data model or backend schema changes are planned, rollback should
  be limited to frontend files and spec/test updates.
- If the unified table introduces unstable tests because of repeated text, fix
  the tests by scoping them to the correct region rather than hiding visible
  labels again.

## Outcomes & retrospective

Outcome:
- The results explorer becomes easier to scan because users compare one row at
  a time instead of matching content across two panels.

Retrospective focus:
- whether the inline usage column is enough on its own
- whether mobile layout needs a dedicated stacked row treatment
- whether the current sort options remain sufficient once usage is inline
