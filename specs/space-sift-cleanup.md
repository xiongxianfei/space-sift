# Space Sift Safe Cleanup

## Status

- approved

## Goal and context

`Space Sift` Milestone 5 adds the first execution-capable cleanup flow on top
of the existing scan, results-explorer, and duplicate-analysis milestones. A
user who already has a loaded scan result must be able to preview a small,
inspectable set of cleanup candidates, review exactly which files would be
removed, and then execute either a Recycle-Bin-first cleanup or an explicitly
separate permanent-delete path.

Milestone 5 stays intentionally narrow. It does not attempt to replicate the
large cleaner catalogs of older maintenance tools, and it does not run the
whole desktop app as administrator.

Related plan:
- `docs/plans/2026-04-15-space-sift-win11-mvp.md`

## Examples

### Example 1: build a preview from duplicate selections and cleanup rules

Given the user has a loaded scan result with file-entry data, a completed
duplicate analysis, and one or more cleanup rules enabled, when they refresh
the cleanup preview, then the app shows a deduplicated file list, the matched
sources for each file, and the total reclaimable bytes before any files are
touched.

### Example 2: execute the default safe path

Given the user has a cleanup preview with selected file candidates, when they
run cleanup from the default action, then the app attempts to move those files
to the Windows Recycle Bin, records the execution, and reports per-file
successes or failures.

### Example 3: permanent delete stays separate

Given the user has a cleanup preview, when they want irreversible deletion,
then they must opt into a separate advanced control before the app can execute
a permanent delete path.

### Example 4: protected paths stay outside the unprivileged flow

Given a cleanup rule or duplicate selection points at a protected Windows path
that would require elevation, when the user builds the preview, then the app
excludes that path, records a clean issue, and keeps the normal UI
unprivileged.

### Example 5: older history entry without file-entry data

Given a saved scan predates the additive file-entry model, when the user opens
that history entry, then cleanup preview remains unavailable and the UI
explains that a fresh scan is required.

## Inputs and outputs

Inputs:
- a currently loaded completed scan result
- zero or more selected duplicate delete candidates derived from the current
  duplicate-analysis keep selection
- zero or more enabled cleanup rules from the built-in cleanup rule catalog
- an explicit cleanup execution mode:
  - `recycle`
  - `permanent`

Outputs:
- a cleanup rule catalog suitable for rendering in the UI
- a cleanup preview with candidate files, reclaimable bytes, and issue details
- a cleanup execution result with an execution identifier, mode, per-file
  results, and aggregate success/failure counts
- a durable local cleanup log entry for each execution attempt
- a privilege-boundary capability response that makes it explicit that the
  normal desktop UI does not self-elevate

## Cleanup model

Milestone 5 cleanup is file-only. Directories MUST NOT be directly removed by
this milestone.

Milestone 5 supports two cleanup-source types:
- duplicate delete candidates from the current duplicate-analysis selection
- built-in cleanup rules defined in repo-tracked TOML files under
  `src/config/cleanup-rules/`

The built-in rule catalog for this milestone MUST be small and inspectable. The
current supported built-in rules are:
- `temp-folder-files`
- `download-partials`

Cleanup preview candidates are deduplicated by full path. If the same file is
matched by multiple enabled sources, the preview MUST show a single candidate
row that retains all applicable source labels.

Cleanup execution MUST act only on file candidates that were present in a
preview created for the currently loaded scan result.

## Requirements

- R1: The app MUST expose a cleanup rule catalog that includes the built-in
  repo-tracked TOML rule definitions for this milestone.
- R2: The cleanup rule catalog for this milestone MUST include exactly these
  rule identifiers:
  - `temp-folder-files`
  - `download-partials`
- R3: Cleanup preview MUST require a currently loaded scan result with file
  entries. If file-entry data is unavailable, cleanup preview MUST remain
  unavailable and the UI MUST explain that a fresh scan is required.
- R4: Cleanup preview MUST accept both:
  - duplicate delete candidates from the current duplicate selection
  - enabled built-in cleanup rules
- R5: Cleanup preview MUST remain local-only and MUST NOT require cloud sync,
  network access, or whole-app elevation.
- R6: Cleanup preview MUST only include file candidates that are inside the
  current scan root and present in the loaded scan result as file entries.
- R7: If a path is matched by multiple cleanup sources, the preview MUST
  deduplicate that path into a single cleanup candidate row while preserving
  every matching source label.
- R8: Cleanup preview MUST show, at minimum:
  - total candidate files
  - total reclaimable bytes
  - candidate file rows with full path, size, and matched source labels
  - issue rows for excluded or invalid candidates
- R9: Cleanup preview MUST exclude and report a clean issue for any candidate
  file that is:
  - missing
  - no longer a regular file
  - outside the current scan root
  - no longer consistent with the stored scan metadata
  - blocked by the privilege boundary
- R10: The normal cleanup execution path MUST default to `recycle`, meaning the
  app attempts to move selected files to the Windows Recycle Bin rather than
  permanently deleting them.
- R11: Permanent delete MUST remain a separate execution mode and MUST NOT be
  triggered by the default cleanup action.
- R12: Cleanup execution MUST act only on candidates that were present in the
  current preview. Arbitrary path deletion from the frontend MUST NOT be
  allowed.
- R13: Before executing a candidate, the backend MUST revalidate that the file
  still exists, is still a regular file, and is still consistent with the
  preview metadata. Files that fail revalidation MUST be skipped and reported as
  failed execution items.
- R14: Cleanup execution MUST return a result that includes:
  - execution identifier
  - preview identifier
  - execution mode
  - aggregate success count
  - aggregate failure count
  - per-file execution results
- R15: Cleanup execution MUST record a durable local log entry for each
  execution attempt, including the chosen mode and the per-file outcomes.
- R16: The app MUST expose an explicit privilege-boundary response that makes
  it clear the main desktop UI stays unprivileged and does not auto-elevate for
  protected-path cleanup.
- R17: If a cleanup candidate would require elevation, the app MUST exclude it
  from the unprivileged preview/execution path and surface a clean issue rather
  than elevating the full app.
- R18: After a cleanup execution completes, the UI MUST tell the user that a
  fresh scan is recommended because the stored scan result may now be stale.

## Invariants

- Cleanup preview is review-first; files are not touched until an explicit
  execution command is issued.
- The default cleanup execution mode is `recycle`.
- The normal app UI stays unprivileged.
- Cleanup candidates stay scoped to the currently loaded scan root.
- Cleanup preview and execution are file-only in this milestone.

## Error handling and boundary behavior

- E1: If no scan is loaded, cleanup preview MUST not start and the UI MUST show
  a prerequisite message.
- E2: If the loaded scan lacks file-entry data, cleanup preview MUST degrade to
  a rescan-required message instead of failing.
- E3: If the user requests a cleanup preview without any enabled source, the UI
  MUST show a clear prerequisite message instead of creating an empty preview
  silently.
- E4: If no valid cleanup candidates remain after filtering and validation, the
  UI MUST show an explicit empty state rather than a blank panel.
- E5: If execution of one candidate fails, the backend MUST continue attempting
  the remaining selected candidates when safe to do so and MUST report the
  mixed result.
- E6: If a protected path would require elevation, the app MUST fail closed for
  that candidate and MUST NOT auto-elevate the whole desktop UI.
- E7: If permanent delete is requested without the explicit advanced user path,
  the UI MUST block that action.

## Compatibility and migration

- C1: Milestone 5 targets Windows 11 only.
- C2: Cleanup preview depends on file-entry data from the loaded scan result, so
  pre-Milestone-3 summary-only history entries remain readable but are not
  eligible for cleanup preview or execution.
- C3: Cleanup execution logging in SQLite SHOULD remain additive so existing
  scan history and duplicate hash cache data remain readable.
- C4: Repo-tracked TOML cleanup rules are part of the reviewed product
  contract; adding or changing a rule file is a behavior change and SHOULD be
  reviewed like any other spec-visible change.

## Observability expectations

- O1: Rust tests MUST cover cleanup preview generation from duplicate
  selections and enabled built-in rules.
- O2: Rust tests MUST cover candidate deduplication and issue generation for
  invalid or blocked paths.
- O3: Rust tests MUST cover cleanup execution for both `recycle` and
  `permanent` modes using test-safe execution boundaries.
- O4: Rust tests MUST cover durable logging of cleanup execution results.
- O5: Rust tests MUST cover the privilege boundary for protected paths.
- O6: Frontend tests MUST cover cleanup preview rendering, recycle-bin-first
  execution, the separate permanent-delete path, and the rescan-required
  fallback for older saved scans.

## Edge cases

- Edge 1: A file matched by both a duplicate selection and a cleanup rule
  appears once in the preview with both source labels.
- Edge 2: A file that disappeared after the scan completed is excluded from the
  preview and reported cleanly.
- Edge 3: A file that changed after preview generation fails revalidation at
  execution time and is reported as a failed execution item.
- Edge 4: A preview with zero valid candidates still renders a clear empty
  state.
- Edge 5: A protected path under a Windows system directory is blocked by the
  privilege boundary and does not trigger whole-app elevation.
- Edge 6: A summary-only saved scan shows a rescan prompt instead of a broken
  cleanup workflow.

## Non-goals

- Directory deletion
- Registry cleaning
- Driver-store cleanup
- Automatic or scheduled background cleanup
- Self-elevating the whole app UI
- Secure erase or shred behavior
- Recycle Bin inventory outside the current scan root

## Acceptance criteria

- A reviewer can load a scan with file-entry data, enable cleanup sources, and
  see a preview that lists exact candidate files, bytes, and issue details.
- A reviewer can include duplicate delete candidates in the cleanup preview
  without the frontend sending arbitrary delete paths directly to the backend.
- A reviewer can run the default cleanup action and get a Recycle-Bin-first
  execution result plus a rescan recommendation.
- A reviewer can reach a separate permanent-delete path only through an
  explicit advanced action.
- A reviewer targeting a protected path sees a fail-closed issue and a clear
  note that the main UI remains unprivileged.
