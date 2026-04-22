# Improve Scan History Readability And Duplicate Review Clarity

## Metadata

- Status: active
- Created: 2026-04-16
- Updated: 2026-04-16
- Owner: xiongxianfei / Codex
- Related spec(s):
  - `specs/space-sift-scan-history.md`
  - `specs/space-sift-scan-history.test.md`
  - `specs/space-sift-duplicates.md`
  - `specs/space-sift-duplicates.test.md`
- Related plan(s):
  - `docs/plans/2026-04-15-space-sift-win11-mvp.md`
  - `docs/plans/2026-04-16-scan-progress-and-active-run-ux.md`
  - `docs/plans/2026-04-16-fast-safe-duplicate-analysis.md`
- Supersedes / Superseded by: none
- Branch / PR: none yet
- Last reviewed files:
  - `src/App.tsx`
  - `src/App.css`
  - `src/scan-history.test.tsx`
  - `src/duplicates.test.tsx`
  - `specs/space-sift-scan-history.md`
  - `specs/space-sift-scan-history.test.md`
  - `specs/space-sift-duplicates.md`
  - `specs/space-sift-duplicates.test.md`

## Purpose / Big picture

Make the post-scan review experience easier to read and safer to act on when
the user has many saved scans or a large number of duplicate groups.

The current product already has the core capabilities:
- local scan history
- a current completed result
- verified duplicate groups
- keep-selection helpers

But the review surface does not scale well yet:
- the `Recent scans` panel becomes a flat list when many scans exist
- the duplicate panel renders every group as a large expanded card
- duplicate rows visually emphasize only the basename even though the backend
  already has the full path
- when two duplicate copies share the same filename, the user does not have
  enough visible context to choose safely which file to keep

This initiative is about information architecture and decision clarity. It is
not a scan-performance initiative, a duplicate-hashing initiative, or a
cleanup-execution initiative.

## Context and orientation

- The current scan-history contract already requires enough metadata to
  distinguish saved scans: scan identifier, root path, completion time, and
  total bytes.
- The current duplicate contract already requires each member row to expose the
  full path and last-modified timestamp.
- The current React app keeps both experiences on one page in `src/App.tsx`:
  - `Recent scans` is a simple local list with one reopen button per entry
  - `Duplicate analysis` renders all verified groups as expanded cards
- The current duplicate UI uses `getPathLabel(member.path)` as the main visible
  label for a member row and only puts the full path in the `title` attribute.
  That means same-name files can still look identical at a glance.
- The duplicate engine and data model do not appear to be the bottleneck for
  this request. The backend already returns full paths, timestamps, group
  counts, and reclaimable bytes.
- The most likely first-pass implementation is frontend-heavy:
  - contract updates in existing scan-history and duplicate specs
  - React state for filtering, sorting, and progressive disclosure
  - CSS changes for a clearer review layout
- The implementation surface is shared more than the current milestone list
  suggests. `Recent scans`, the completed-result shell, the results explorer,
  duplicate review, and cleanup-preview setup all live in the same `App.tsx`
  page component, and existing coverage is split across:
  - `src/scan-history.test.tsx`
  - `src/duplicates.test.tsx`
  - `src/results-explorer.test.tsx`
  - `src/App.test.tsx`
- This work should stay separate from the fast-safe duplicate backend plan.
  The problem here is not hashing correctness or caching; it is that the
  existing review surface becomes hard to triage when the data volume grows.

## Constraints

- Preserve the existing trust model:
  - scan history remains local-only
  - duplicate analysis remains preview-only and read-only
  - keep-selection rules still guarantee one kept file per group
- Do not change duplicate-verification correctness or duplicate-performance
  architecture in this initiative unless a small additive UI support seam is
  unavoidable.
- Prefer using already available metadata before adding new storage columns or
  new Tauri commands.
- Do not hide identity-critical path information behind hover-only affordances.
  Important path context must be visibly readable in the duplicate-review UI.
- Keep ordering stable while the user is reviewing or changing keep selections.
  Duplicate groups MUST NOT jump around after every preview change unless the
  spec explicitly decides they should.
- Keep the main workflow understandable on both desktop and mobile-width
  layouts.
- New disclosure, filter, or sort controls must stay keyboard reachable and
  expose explicit accessible state such as `aria-expanded`, `aria-controls`, or
  equivalent semantics where the interaction model requires them.
- Avoid turning the review surface into a dense spreadsheet. The goal is
  progressive disclosure and safer decisions, not raw information density.
- Keep milestones small enough for one reviewable PR each.

## Done when

- A user with many saved scans can quickly identify and reopen the right scan
  without reading a long undifferentiated list.
- The history surface clearly distinguishes at least:
  - the currently loaded result
  - the most recent completed scans in default newest-first order
  - a bounded narrowing model that supports at least root-path and scan-ID
    filtering
- The duplicate review surface remains usable when many groups exist.
- Duplicate groups are ordered by a deterministic triage rule:
  - reclaimable bytes descending
  - member count descending
  - first member path ascending as the stable tie-breaker
- Duplicate group order stays stable while the user changes keep selections;
  keep-selection changes update preview totals but do not reshuffle the page.
- Every duplicate member row shows enough visible path context that two files
  with the same basename can still be distinguished without relying on hover
  tooltips.
- The duplicate path model is explicit and consistent:
  - primary label: basename
  - secondary label: scan-root-relative path or parent-folder context that is
    visibly rendered in the row
  - full absolute path may remain available as secondary supporting text or a
    tooltip, but it is not the only way to disambiguate same-name files
- The user can make a keep/delete preview decision with confidence because the
  UI shows both the filename and where that file lives.
- New disclosure or filter controls remain keyboard-operable and expose their
  expanded/collapsed state explicitly.
- Automated tests cover the new history-review and duplicate-triage behavior.
- Manual validation on a seeded large review state confirms the page is easier
  to navigate and does not force the user to cross-reference hidden path data.

## Non-goals

- Changing duplicate hashing, cache reuse, or disk-I/O policy
- Changing cleanup execution or delete semantics
- Adding cloud sync or remote history storage
- Replacing the current results explorer contract
- Introducing backend pagination or history schema migrations unless later
  validation proves the current local payload is insufficient
- Building a full global search experience across all scan entries
- Shipping a new treemap or side-panel visualization

## Milestones

### Milestone 1: Re-spec review clarity for history and duplicate triage

Scope: update the existing scan-history and duplicate contracts so the repo has
an explicit target for readable history review and duplicate decision context.

This milestone must lock these decisions before UI coding starts:
- default history sort: newest completion time first
- minimum history narrowing fields: root path and scan ID
- currently loaded result highlighting in the history list
- duplicate-group default sort:
  - reclaimable bytes descending
  - member count descending
  - first member path ascending
- duplicate group order stability during keep-selection changes
- duplicate member path presentation:
  - basename first
  - visible scan-root-relative path or parent-folder context second
- disclosure-control accessibility expectations for any collapsible duplicate
  group model

Files or components touched:
- `specs/space-sift-scan-history.md`
- `specs/space-sift-scan-history.test.md`
- `specs/space-sift-duplicates.md`
- `specs/space-sift-duplicates.test.md`

Dependencies:
- this approved plan

Risk:
- mixing too many UI opinions into the contract or accidentally promising a new
  backend/search system when the main problem is presentation
- leaving sort, filter, and visible-path rules underspecified enough that M2
  and M3 implement different mental models

Validation commands:
- `Get-Content specs/space-sift-scan-history.md`
- `Get-Content specs/space-sift-scan-history.test.md`
- `Get-Content specs/space-sift-duplicates.md`
- `Get-Content specs/space-sift-duplicates.test.md`
- `rg -n "history|recent scans|scan id|root path|duplicate group|full path|relative path|keep|aria-expanded" specs/space-sift-scan-history.md specs/space-sift-scan-history.test.md specs/space-sift-duplicates.md specs/space-sift-duplicates.test.md`

Expected observable result:
- the scan-history contract explicitly covers how many-entry history should stay
  readable, including newest-first ordering, current-result highlighting, and
  bounded filter fields
- the duplicate contract explicitly covers visible path context, group ordering,
  ordering stability, and disclosure-state accessibility instead of leaving
  them implicit

### Milestone 2: Improve the scan-history review surface

Scope: make `Recent scans` readable and navigable when many entries exist using
the metadata already stored locally.

The intended first-pass history UX is:
- default newest-first ordering by completion time
- visible current-result highlight for the loaded scan, if any
- bounded narrowing controls for:
  - root-path text matching
  - scan-ID text matching
- no backend search, pagination, or schema migration in this milestone

Files or components touched:
- `src/App.tsx`
- `src/App.css`
- `src/scan-history.test.tsx`
- `src/App.test.tsx` if the page-level review state needs additive coverage

Dependencies:
- Milestone 1 contract updates

Risk:
- making the history surface more featureful but also more visually noisy
- introducing local filtering or highlighting that conflicts with the active
  scan state or with the current-result shell on the same page

Validation commands:
- `npm run test -- history`
- `npm run test -- results`
- `npm run test -- src/App.test.tsx`
- `npm run test`
- `npm run lint`
- `npm run build`

Expected observable result:
- saved scans are easier to distinguish and reopen when the list is long
- the currently loaded scan is visually obvious
- the user can narrow the visible history list by root path and scan ID instead
  of reading every entry sequentially
- shared-page results and current-result behavior remain intact after the
  history-surface changes

### Milestone 3: Redesign duplicate review for safe keep/delete decisions

Scope: make the duplicate panel usable when many groups exist and same-name
files appear in different folders.

The intended first-pass duplicate UX is:
- groups sorted by:
  - reclaimable bytes descending
  - member count descending
  - first member path ascending
- summary-first progressive disclosure so the page does not open every group as
  a fully expanded wall by default
- visible member identity model:
  - basename as the primary label
  - scan-root-relative path or parent-folder context as visible secondary text
- stable group ordering while keep-selection changes update preview totals

Files or components touched:
- `src/App.tsx`
- `src/App.css`
- `src/duplicates.test.tsx`

Dependencies:
- Milestone 1 contract updates

Risk:
- collapsing or filtering groups too aggressively and making it harder to
  review the full result when needed
- resorting groups dynamically after keep-selection changes and making the page
  jump under the user
- adding disclosure controls that look correct visually but are not explicit to
  keyboard or assistive-technology users

Validation commands:
- `npm run test -- duplicates`
- `npm run test -- results`
- `npm run test`
- `npm run lint`
- `npm run build`

Expected observable result:
- duplicate groups are ordered by the documented reclaimable-bytes/member-count
  rule instead of appearing as a flat wall of expanded cards
- the user can progressively disclose group details instead of scanning every
  member row at once
- each duplicate member shows visible filename plus clear path context such as
  parent folder or scan-root-relative path
- same-name files can be distinguished without relying on browser tooltips
- keep-selection changes update the preview summary without causing confusing
  resorting of the visible group list

### Milestone 4: Validate large review states and record the UX decisions

Scope: validate the new review surface against seeded large history and
duplicate fixtures, then record any final fallout or maintainer notes.

Files or components touched:
- `docs/plans/2026-04-16-history-and-duplicate-review-clarity.md`
- any small follow-up files identified by validation fallout

Dependencies:
- Milestones 2 and 3

Risk:
- synthetic tests can pass while the real page still feels cluttered on a
  larger saved-history or duplicate-review state

Validation commands:
- `npm run test`
- `npm run lint`
- `npm run build`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo check --manifest-path src-tauri/Cargo.toml`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; npm run tauri dev`
- manual Windows 11 review pass with:
  - a seeded history list with at least 20 completed scans
  - a duplicate result with at least 20 groups
  - at least 3 duplicate groups containing repeated basenames in different folders
  - at least one 2-member group and one 3+-member group
  - keyboard-only checks for the new history controls and duplicate disclosure
    controls

Expected observable result:
- the review page remains readable and decision-oriented under larger real
  result sets
- the final plan notes explain what ordering, filtering, and visible path model
  were chosen and why

## Progress

- [x] 2026-04-16: reviewed `AGENTS.md`, `docs/workflows.md`, `docs/plan.md`,
  `.codex/PLANS.md`, the active MVP and duplicate-performance plans, the
  scan-history and duplicate specs/test specs, and the current `App.tsx` /
  `App.css` review surfaces before drafting this initiative.
- [x] 2026-04-16: revised this plan after `plan-review` to make the UX defaults
  explicit: bounded history filters, deterministic duplicate ordering, stable
  post-selection group order, explicit visible path context, accessibility
  requirements for disclosure controls, and broader shared-page validation.
- [x] 2026-04-16: completed Milestone 1 by updating
  `specs/space-sift-scan-history.md`,
  `specs/space-sift-scan-history.test.md`,
  `specs/space-sift-duplicates.md`, and
  `specs/space-sift-duplicates.test.md` to lock the bounded history-review
  model, deterministic duplicate ordering, visible same-name path context, and
  disclosure-state accessibility requirements before UI implementation.
- [x] 2026-04-16: completed Milestone 2 in `src/App.tsx`,
  `src/App.css`, and `src/scan-history.test.tsx` by adding newest-first local
  history ordering, bounded root-path and scan-ID filters, an explicit
  no-match state, and a visible `Loaded result` marker for the current scan.
- [x] 2026-04-16: completed Milestone 3 in `src/App.tsx`,
  `src/App.css`, and `src/duplicates.test.tsx` by sorting duplicate groups
  deterministically, collapsing member rows behind explicit disclosure
  controls, and rendering basename-first member rows with visible root-relative
  or `Scan root` location labels.
- [ ] 2026-04-16: advanced Milestone 4 by adding seeded large-state regression
  coverage in `src/scan-history.test.tsx` and `src/duplicates.test.tsx` for a
  24-entry history list, a 22-group duplicate review set, repeated basenames,
  and focusable history/disclosure controls; broader frontend and desktop
  startup verification passed, while true manual Windows 11 visual review is
  still pending outside automation.

## Surprises & discoveries

- 2026-04-16: the duplicate backend is already returning full paths, so the
  most urgent same-name-file problem is a presentation bug and information
  architecture gap, not a missing data-model field.
- 2026-04-16: the current history list already has enough metadata to start
  improving readability without a schema migration.
- 2026-04-16: the current duplicate member rows visually emphasize only the
  basename, even though the spec already requires full path in the result
  model. That means part of this work is tightening the contract around visible
  context, not just adding a new widget.
- 2026-04-16: the hardest usability problem is not finding one duplicate group;
  it is staying oriented when many groups and repeated basenames exist at once.
  That points to ordering, filtering, and progressive disclosure as the first
  best-practice moves.
- 2026-04-16: because history, results, duplicates, and cleanup-preview setup
  all share the same `App.tsx` page shell, narrowly targeted UI verification is
  not enough on its own; this initiative needs broader page-level regression
  checks even if the code change looks localized.
- 2026-04-16: the many-scan history review problem was solvable entirely with
  existing local metadata. No new SQLite columns, Tauri commands, or backend
  sort/filter seams were needed for the first-pass history UX.
- 2026-04-16: the duplicate-review usability problem was also solvable without
  widening the backend contract. The current duplicate payload already had
  enough metadata for impact ordering and root-relative member context once the
  UI stopped rendering every group fully expanded.
- 2026-04-16: repeated `completed` duplicate snapshots can arrive after the
  user has already started reviewing a loaded result. Reopening the same
  analysis payload on every replay wipes local disclosure state and makes
  `Show details` appear broken even though the toggle handler itself works.
- 2026-04-16: seeded large-state fixtures did not reveal any additional layout
  or ordering fallout. The bounded history filters and collapsed duplicate
  groups scaled to 20+ review items without requiring new UI state, backend
  metadata, or schema changes.

## Decision log

- 2026-04-16: create a new follow-up plan instead of stretching the active MVP
  plan body.
  Rationale: this is a cross-feature UX clarification initiative spanning scan
  history and duplicate review, and it deserves a separate reviewable trail.

- 2026-04-16: treat this as a review-surface clarity initiative, not a backend
  performance initiative.
  Rationale: the core user complaint is that the data is hard to read and act
  on, not that scan or duplicate verification is incorrect.

- 2026-04-16: prefer frontend-first improvements using existing metadata.
  Rationale: history metadata and duplicate full paths already exist, so the
  first pass should avoid unnecessary DB or command churn.

- 2026-04-16: path context in duplicate review must be visibly readable, not
  hover-only.
  Rationale: deletion-adjacent decisions should not depend on `title`
  tooltips, especially when two files share the same basename.

- 2026-04-16: large duplicate results should use progressive disclosure.
  Rationale: when many groups exist, forcing every group open at once creates a
  review wall instead of a triage workflow.

- 2026-04-16: default history review should stay bounded and simple.
  Rationale: the first-pass history surface should sort newest first and narrow
  only by root path and scan ID, because that solves the concrete readability
  problem without widening this initiative into backend search or pagination.

- 2026-04-16: duplicate groups should sort by reclaimable bytes descending,
  then member count descending, then first member path ascending.
  Rationale: that gives a deterministic triage order grounded in cleanup impact
  while still staying stable for tests and user orientation.

- 2026-04-16: duplicate group order should stay stable while keep selections
  change.
  Rationale: preview totals may change during review, but the page should not
  jump underneath the user after every `keep newest`, `keep oldest`, or manual
  keep-selection action.

- 2026-04-16: duplicate member rows should show basename first and visible
  scan-root-relative path or parent-folder context second.
  Rationale: same-name files are easiest to parse when the row preserves the
  familiar filename-first scan but still renders enough visible context to make
  safe decisions without relying on hover tooltips.

- 2026-04-16: use a visible `Loaded result` badge plus row styling for the
  current scan in `Recent scans`.
  Rationale: a style-only distinction would be easier to miss in a long list,
  while explicit text remains testable and clearer for the user.

- 2026-04-16: default duplicate review should keep all groups collapsed until
  the reviewer explicitly opens one.
  Rationale: the summary-first triage goal is better served by a short impact
  list with explicit `Show details` state than by rendering every duplicate
  member row at once.

- 2026-04-16: duplicate member context should prefer scan-root-relative parent
  folders and fall back to `Scan root` for files directly under the scan root.
  Rationale: that keeps same-name files visually distinct without forcing the
  user to parse long absolute Windows paths on every row.

- 2026-04-16: validate large review states with seeded frontend fixtures before
  treating manual desktop review as the only remaining gate.
  Rationale: the shared `App.tsx` shell makes it cheap and durable to keep
  20+ history-entry and 20+ duplicate-group states under regression coverage,
  while a true Windows UI review still depends on a human-visible desktop.

## Validation and acceptance

Planning validation performed while creating this plan:
- `Get-Content AGENTS.md`
- `Get-Content docs/workflows.md`
- `Get-Content docs/plan.md`
- `Get-Content .codex/PLANS.md`
- `Get-Content specs/space-sift-scan-history.md`
- `Get-Content specs/space-sift-scan-history.test.md`
- `Get-Content specs/space-sift-duplicates.md`
- `Get-Content specs/space-sift-duplicates.test.md`
- `Get-Content docs/plans/2026-04-15-space-sift-win11-mvp.md`
- `Get-Content docs/plans/2026-04-16-fast-safe-duplicate-analysis.md`
- `Get-Content src/App.tsx | Select-Object -Skip 820 -First 520`
- `Get-Content src/App.css | Select-Object -Skip 300 -First 320`
- `rg -n "Recent scans|Duplicate analysis|duplicate groups|keep newest|Delete candidate|Kept copy" src/App.tsx src/App.css src/scan-history.test.tsx src/duplicates.test.tsx`

Acceptance evidence for the implemented initiative:
- a reviewer with many saved scans can find the intended history entry faster
  than reading a flat list top to bottom
- a reviewer can immediately identify which scan is currently loaded
- a reviewer can narrow history by root path and scan ID without losing the
  default newest-first sort model
- a reviewer opening duplicate groups with repeated basenames can distinguish
  copies by visible path context and choose which one to keep
- a reviewer faced with many duplicate groups sees the documented
  reclaimable-bytes/member-count ordering and does not have to parse every
  group fully expanded at once
- a reviewer changing keep selections does not see the duplicate list reshuffle
  unexpectedly during review
- keyboard-only navigation can reach and operate the new history controls and
  duplicate disclosure controls with explicit state cues

## Validation notes

- 2026-04-16: planning-only turn. No `npm`, `cargo`, or Tauri runtime commands
  were run because this step only created a new execution plan and updated the
  plan index.
- 2026-04-16: Milestone 1 was doc-only. Validation used spec/test-spec
  readback plus targeted `rg` checks against the new history-review and
  duplicate-review terms; no `npm`, `cargo`, or Tauri runtime commands were
  needed yet because this step only changed the contract.
- 2026-04-16: Milestone 2 validation ran:
  - `npm run test -- src/scan-history.test.tsx`
  - `npm run test -- src/results-explorer.test.tsx`
  - `npm run test -- src/App.test.tsx`
  - `npm run test`
  - `npm run lint`
  - `npm run build`
  All passed after the frontend-only history-review update landed.
- 2026-04-16: Milestone 3 validation ran:
  - `npm run test -- src/duplicates.test.tsx`
  - `npm run test -- src/results-explorer.test.tsx`
  - `npm run test`
  - `npm run lint`
  - `npm run build`
  All passed after the duplicate-review redesign landed in the shared
  `App.tsx` surface.
- 2026-04-16: Milestone 4 automation-backed validation ran:
  - `npm run test -- src/scan-history.test.tsx`
  - `npm run test -- src/duplicates.test.tsx`
  - `npm run test`
  - `npm run lint`
  - `npm run build`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo check --manifest-path src-tauri/Cargo.toml`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; npm run tauri dev`
  The seeded large-state regressions, full frontend suite, lint, build, and
  desktop compile check all passed. `npm run tauri dev` timed out while the
  dev session stayed running, which is consistent with a live desktop session
  rather than a startup crash. A true manual Windows 11 visual review pass is
  still pending because this automation environment cannot inspect the launched
  window directly.
- 2026-04-16: a post-Milestone-3 duplicate-review regression let repeated
  terminal `completed` snapshots reopen the same analysis and reset local
  disclosure state. The new frontend regression
  `keeps duplicate details open when the same completed snapshot is replayed`
  failed first, then passed after `src/App.tsx` stopped reopening an already
  loaded analysis on redundant same-ID `completed` replays.

## Idempotence and recovery

- Land this as small PRs so history-surface changes and duplicate-review
  surface changes can be reviewed independently if needed.
- Prefer additive UI state and styling changes before introducing any new local
  storage or backend query behavior.
- If a filtering or disclosure pattern makes the review page harder to scan in
  real validation, keep the visible path-context improvements and revert the
  more aggressive disclosure behavior.
- If progressive disclosure helps but dynamic resorting disorients users, keep
  the disclosure model and fall back to a fixed deterministic duplicate order.
- If frontend-only filtering proves insufficient for very large saved-history
  sets, record that as a follow-up instead of silently widening this initiative
  into a schema or backend-search redesign.
- If the new duplicate review ordering helps triage but confuses deterministic
  test fixtures, codify the ordering rule in the spec rather than leaving it as
  an implicit implementation detail.

## Outcomes & retrospective

Expected outcome:
- `Space Sift` becomes easier to trust in the review stage because the user can
  find the right saved scan quickly and can clearly see which duplicate copy
  lives where before building a cleanup preview

Retrospective focus:
- whether history filtering and active-result highlighting were enough without
  any backend changes
- whether root-relative path context is clearer than full absolute paths in the
  duplicate review surface
- whether progressive disclosure improved triage or just added extra clicks
