# Space Sift Duplicate Detection

## Status

- approved

## Goal and context

`Space Sift` Milestone 4 adds duplicate-file discovery on top of the existing
scan and results-explorer flow. A user who already has a loaded scan result
must be able to run a duplicate analysis, see only fully verified duplicate
groups, apply non-destructive keep-selection helpers, and preview how much
space a later cleanup action could reclaim.

Milestone 4 builds on `specs/space-sift-scan-history.md` and
`specs/space-sift-results-explorer.md`. It does not perform deletion or
Recycle Bin execution yet.

Related plan:
- `docs/plans/2026-04-15-space-sift-win11-mvp.md`
- `docs/plans/2026-04-16-fast-safe-duplicate-analysis.md`
- `docs/plans/2026-04-16-history-and-duplicate-review-clarity.md`

## Examples

### Example 1: find duplicates from a loaded scan

Given the user has reopened a scan result that includes file entries, when they
run duplicate analysis, then the app evaluates the scanned files and shows only
groups whose files were confirmed as duplicates by the staged verification
pipeline.

### Example 2: same size but different content is not shown

Given two files have the same byte size but different contents, when duplicate
analysis runs, then those files are not shown in a duplicate group because full
hash confirmation did not match.

### Example 3: triage many duplicate groups in a stable order

Given duplicate analysis completes with many verified groups, when the review
surface renders, then groups appear in deterministic triage order based on
reclaimable bytes first, then member count, then a stable path tie-breaker
instead of a visually arbitrary insertion order.

### Example 4: distinguish same-name copies from different folders

Given a duplicate group contains files with the same basename in different
folders, when the user reviews the member rows, then each row shows the
filename plus visible location context so the user can distinguish the copies
without relying on hover text alone.

### Example 5: choose which copy to keep

Given a fully verified duplicate group contains three copies of the same file,
when the user applies a keep-selection helper such as `keep newest` or manually
chooses one file to keep, then the preview updates and still leaves at least
one file in the group unselected for deletion.

### Example 6: older history entry without file-entry data

Given a saved scan predates the additive file-entry result model, when the user
opens that history entry, then duplicate analysis stays unavailable and the UI
explains that a fresh scan is required before duplicates can be checked.

### Example 7: file changed after the scan was saved

Given the current loaded scan references a file that was deleted, moved, or
modified after the scan completed, when duplicate analysis reaches that file,
then the app excludes it from verified groups and reports a clean issue instead
of treating it as a duplicate.

### Example 8: cancel a long-running duplicate analysis

Given duplicate analysis is still hashing a large set of candidate files, when
the user requests cancellation, then the active analysis stops promptly,
surfaces a cancelled state, and does not present partial duplicate groups as a
completed result.

### Example 9: avoid hidden network-backed or placeholder-backed reads

Given a loaded scan includes files on a non-primary path class such as a
remote, removable, or on-demand cloud-backed location, when duplicate analysis
cannot safely hash a file without triggering hidden network-backed reads or
placeholder hydration, then the app keeps the workflow local-only by using a
safer fallback or by skipping and reporting the affected file instead of
silently hydrating it.

## Inputs and outputs

Inputs:
- a currently loaded completed scan result
- a user request to run duplicate analysis
- a user keep-selection helper choice for a duplicate group
- an optional manual keep-selection override for a duplicate group member
- an optional user request to expand or collapse duplicate-group details during
  review

Outputs:
- duplicate-analysis lifecycle state and progress stage
- a deterministically ordered list of fully verified duplicate groups
- per-group preview state showing which copy is kept and which copies are later
  cleanup candidates
- an estimated reclaimable-byte total from the current preview selection
- an issue list for files that could not be safely verified
- a cancelled terminal state when the user interrupts a running analysis

## Duplicate analysis model

Milestone 4 duplicate discovery MUST operate on files only. Directories are not
duplicate groups.

A fully verified duplicate group MUST represent files that passed all required
verification stages:
- same byte size
- matching partial hash when partial hashing is applicable
- matching full hash

A duplicate group MUST expose at least:
- a stable group identifier within that analysis result
- the shared file size in bytes
- the duplicate member count
- the estimated reclaimable bytes if exactly one file is kept
- a member list where each file includes:
  - full path
  - file size
  - last-modified timestamp

Milestone 4 may use local SQLite-backed hash caching internally, but cached
data MUST NOT cause a file to be treated as a verified duplicate unless the
cache entry is still valid for the current file on disk.

## Requirements

- R1: The app MUST let the user run duplicate analysis from a currently loaded
  scan result that contains file-entry data for the scanned files.
- R2: If the currently loaded scan result does not contain file-entry data,
  duplicate analysis MUST remain unavailable and the UI MUST explain that a
  fresh scan is required.
- R3: Duplicate analysis in Milestone 4 MUST remain local-only and MUST NOT
  require network access, cloud synchronization, or administrator elevation.
- R3a: Duplicate analysis MUST NOT silently trigger network-backed reads or
  on-demand cloud-file hydration as a side effect of hashing candidate files.
- R3b: Local fixed-volume paths are the primary optimization target for
  duplicate-analysis performance work. Remote, removable, cloud-backed, or
  otherwise non-primary path classes MAY use a slower or more conservative
  verification path, but they MUST preserve the same full-hash, read-only,
  cancellation-aware safety contract.
- R4: Duplicate analysis MUST consider only file entries from the current scan
  result. Directory entries MUST NOT appear as duplicate candidates.
- R5: A file MUST NOT be shown in a duplicate group unless it passed all of
  these verification stages:
  - size match
  - partial hash match when the file is large enough for partial hashing to be
    meaningful
  - full hash match
- R6: Files with the same size but different content MUST NOT appear in a
  duplicate group.
- R7: The duplicate-analysis workflow MUST expose a visible lifecycle state
  that distinguishes at least:
  - `idle`
  - `running`
  - `completed`
  - `cancelled`
  - `failed`
- R8: While duplicate analysis is running, the UI MUST expose visible progress
  that includes at least the current stage and a monotonic count of candidate
  files or groups processed. Progress updates MAY be rate-limited for large
  analyses, but terminal state changes MUST still surface promptly.
- R8a: While duplicate analysis is running, the user MUST be able to request
  cancellation from the duplicate-analysis UI.
- R9: The UI MUST show only fully verified duplicate groups in the completed
  result view.
- R9a: The completed duplicate-group list MUST use this deterministic default
  order:
  - reclaimable bytes descending
  - member count descending
  - first member path ascending as the tie-breaker
- R9b: The completed duplicate review surface MUST support progressive
  disclosure or an equivalent summary-first model so large result sets do not
  require every member row from every group to be simultaneously visible.
- R9c: If duplicate-group details are collapsible, the disclosure controls MUST
  be keyboard-operable and MUST expose explicit expanded or collapsed state.
- R10: Each completed duplicate group MUST show:
  - file size
  - duplicate member count
  - estimated reclaimable bytes when one copy is kept
  - file-member rows with visible file identity and timestamp context
- R10a: Each duplicate member row MUST display:
  - the basename as the primary visible label
  - a visibly rendered secondary location label that disambiguates same-name
    files using scan-root-relative path context or parent-folder context
  - last-modified timestamp
  - file size
- R10b: The full absolute path MAY remain available as supporting text or a
  tooltip, but it MUST NOT be the only way to distinguish same-name copies.
- R11: The duplicate workflow MUST provide non-destructive keep-selection
  helpers for each duplicate group. Milestone 4 MUST support at least:
  - `keep newest`
  - `keep oldest`
  - manual keep selection for a specific file row
- R12: The preview model MUST guarantee that at least one file remains kept in
  each duplicate group. Milestone 4 MUST NOT allow a preview state where every
  copy in a group is marked for deletion.
- R13: The UI MUST show an aggregate preview summary that includes at least:
  - total duplicate groups
  - total files currently marked for later deletion
  - estimated reclaimable bytes from the current preview selection
- R13a: Keep-selection changes MUST update the preview summary and the affected
  group state without reshuffling the visible duplicate-group order during the
  same completed review session.
- R14: If a file referenced by the current scan is missing, unreadable, or no
  longer matches the expected scan metadata when duplicate verification runs,
  the app MUST exclude that file from verified groups and MUST record a clean
  issue for the user instead of crashing.
- R15: If exclusions or verification failures leave fewer than two fully
  verified files in a candidate set, that candidate set MUST NOT be shown as a
  duplicate group.
- R16: Reopening a completed scan from local history MUST allow duplicate
  analysis to run against the current filesystem state for the referenced files
  when file-entry data is present.
- R17: If local hash caching is used, the app MUST recompute or disregard
  cached partial or full hashes when file validity cannot be established for
  the current on-disk file.
- R18: Milestone 4 duplicate analysis and preview MUST remain read-only with
  respect to the filesystem. These actions MUST NOT move, delete, recycle, or
  rename files.
- R19: If the user cancels duplicate analysis, the app MUST stop the active
  analysis promptly, surface a `cancelled` state with a clear message, and
  MUST NOT present partial duplicate groups as if they were a completed
  result.

## Invariants

- Only full-hash-confirmed groups are presented as duplicates.
- Duplicate preview is advisory and read-only in Milestone 4.
- Every duplicate group preview keeps at least one file.
- Duplicate analysis stays scoped to the currently loaded scan root.
- Cancelling duplicate analysis never converts partial verification work into a
  completed duplicate result.
- Duplicate-group ordering is deterministic for a completed analysis result.
- Preview-selection changes do not reshuffle the visible duplicate-group order
  during the same review session.

## Error handling and boundary behavior

- E1: If no loaded scan is available, duplicate analysis MUST not start and the
  UI MUST show a clear prerequisite message.
- E2: If a loaded scan lacks file-entry data, duplicate analysis MUST degrade
  to a rescan prompt rather than failing.
- E3: If no verified duplicate groups are found, the UI MUST show an explicit
  empty state instead of a blank panel.
- E4: If a file cannot be read or no longer exists during verification, the app
  MUST record that issue and continue with the remaining candidates when safe
  to do so.
- E5: If duplicate analysis fails before completion, the UI MUST surface a
  clean error state rather than showing partial groups as fully verified.
- E6: If a file is smaller than the partial-hash window, the staged pipeline
  MAY skip directly from size grouping to full-hash verification, but it still
  MUST require full-hash confirmation before showing a group.
- E7: If duplicate analysis is cancelled while hashing large files, the UI MUST
  surface a clean cancelled state rather than continuing to show the analysis
  as running indefinitely.
- E8: If verifying a file would require hidden network-backed reads,
  placeholder hydration, or another path-class-specific side effect that would
  violate the local-only contract, the app MUST skip and report that file or
  apply a safer fallback policy instead of silently continuing with the risky
  read.

## Compatibility and migration

- C1: Milestone 4 targets Windows 11 only.
- C2: Duplicate analysis depends on file-entry data from the loaded scan
  result, so pre-Milestone-3 summary-only history entries remain readable but
  are not eligible for duplicate analysis.
- C3: SQLite-backed hash caching SHOULD remain additive so existing scan
  history stays readable if duplicate cache tables or columns are introduced.
- C4: Future duplicate-performance work MAY optimize local fixed-volume paths
  more aggressively than remote, removable, or cloud-backed paths, but every
  path class MUST continue to honor the same duplicate-result correctness and
  read-only guarantees.

## Observability expectations

- O1: Rust tests MUST cover staged verification, including the requirement that
  full-hash confirmation is required before a duplicate group is emitted.
- O2: Rust tests MUST cover exclusion behavior for files that are missing,
  unreadable, or changed after the scan result was created.
- O3: Rust tests MUST cover hash-cache reuse and invalidation rules when local
  duplicate caching is implemented.
- O4: Frontend tests MUST cover running duplicate analysis from a loaded scan,
  showing verified groups, applying keep-selection helpers, updating the
  preview summary, and cancelling a running analysis.
- O4a: Frontend tests MUST cover deterministic duplicate-group ordering,
  visible location context for same-name files, and progressive disclosure
  behavior for large result sets.
- O4b: Frontend tests MUST cover keyboard-operable disclosure state when the
  duplicate review surface uses collapsible group details.
- O5: Frontend tests MUST cover the rescan-required fallback for older
  history entries that lack file-entry data.
- O6: Milestone verification MUST include focused duplicate tests in both Rust
  and the frontend.
- O7: Rust tests MUST cover bounded progress emission for large duplicate
  analyses and prompt cancellation during hashing.
- O8: Duplicate-performance validation MUST record at least one large local
  fixed-volume run, one warm rerun on the same candidate set, and one
  non-primary path class if available, or an explicit note that such a path
  class was unavailable on the validation machine.
- O9: Rust or integration coverage MUST verify that files which cannot be read
  without violating the local-only contract are skipped or reported cleanly
  rather than treated as verified duplicates.

## Edge cases

- Edge 1: Two zero-byte files can still form a verified duplicate group after
  full-hash confirmation.
- Edge 2: Same-size files with different content are excluded from the final
  duplicate result.
- Edge 3: A candidate file missing after the scan completes is excluded and
  reported cleanly.
- Edge 4: A scan reopened from history can still drive duplicate analysis when
  file-entry data is present.
- Edge 5: An older summary-only history entry shows a rescan prompt instead of
  a broken duplicate workflow.
- Edge 6: A duplicate group with only two members still supports helper-based
  keep selection and a correct reclaimable-byte preview.
- Edge 7: Cancelling duplicate analysis during full hashing leaves the workflow
  in a cancelled state without any partial duplicate result.
- Edge 8: A file that would require placeholder hydration or hidden
  network-backed reads to hash is skipped or reported instead of being silently
  hydrated.
- Edge 9: A non-primary path class may run a slower or more conservative
  duplicate-verification path, but it still emits only fully verified groups
  and preserves prompt cancellation.
- Edge 10: Two or more duplicate copies can share the same basename while
  living in different folders and still remain visually distinguishable during
  review.
- Edge 11: Keep-selection changes update preview totals without changing the
  visible order of groups in the same completed result.

## Non-goals

- Moving files to the Recycle Bin
- Permanently deleting files
- Duplicate-directory detection
- Similar-photo, media, or fuzzy-content matching
- Background scheduling or cloud-backed dedupe state
- Running the entire app as administrator

## Acceptance criteria

- A reviewer can load a scan, run duplicate analysis, and see only fully
  verified duplicate groups rather than same-size guesses.
- A reviewer can apply keep-selection helpers or manual keep selection and see
  the preview summary update while still leaving one copy kept per group.
- A reviewer can distinguish same-name files from different folders using
  visible path context rather than hover-only hints.
- A reviewer facing many duplicate groups can triage them in deterministic
  order without parsing every group fully expanded at once.
- A reviewer can reopen a scan from local history and run duplicate analysis
  again when file-entry data is available.
- A reviewer opening an older summary-only history entry sees a rescan prompt
  instead of a broken duplicate workflow.
- A reviewer can cancel a long-running duplicate analysis and see the workflow
  stop without partial duplicate groups being shown as completed output.
- A reviewer or maintainer validating duplicate-performance changes can show
  that local fixed-volume runs remain full-hash-correct while non-primary path
  classes preserve the same local-only safety contract through fallback or
  clean issue reporting.
