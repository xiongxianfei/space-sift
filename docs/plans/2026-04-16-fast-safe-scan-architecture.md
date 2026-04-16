# Design A Fast, Disk-Friendly Scan Architecture

## Metadata

- Status: done
- Created: 2026-04-16
- Updated: 2026-04-16
- Owner: xiongxianfei / Codex
- Related spec(s):
  - `specs/space-sift-scan-history.md`
  - `specs/space-sift-scan-history.test.md`
- Related plan(s):
  - `docs/plans/2026-04-15-space-sift-win11-mvp.md`
  - `docs/plans/2026-04-16-scan-progress-and-active-run-ux.md`
- Supersedes / Superseded by: none
- Branch / PR: none yet
- Last reviewed files:
  - `src-tauri/crates/scan-core/src/lib.rs`
  - `src-tauri/crates/scan-core/examples/measure_scan.rs`
  - `src-tauri/src/commands/scan.rs`
  - `src-tauri/src/state/mod.rs`
  - `src/App.tsx`
  - `docs/scan-performance.md`
  - `README.md`
  - `specs/space-sift-scan-history.md`
  - `specs/space-sift-scan-history.test.md`

## Purpose / Big picture

Make `Space Sift` scans feel faster on real Windows folders without turning the
scanner into a high-impact background job. The scanner should surface space
usage quickly, stay read-only, avoid unnecessary file-content reads, avoid
redundant metadata round-trips, and keep the machine responsive on both small
and large trees.

This initiative is about the scan architecture, not one isolated optimization.
The target outcome is a scan pipeline that is:

- metadata-first for ordinary space discovery
- explicit about when full file reads are forbidden
- conservative about I/O concurrency by default
- clear about which Windows path classes get the optimized backend
- structured so future NTFS-specific fast paths can be added safely

## Context and orientation

- The current scanner in `src-tauri/crates/scan-core/src/lib.rs` is already
  read-only and cancellation-aware. It walks recursively, uses
  `symlink_metadata`, skips reparse points, aggregates directory sizes, and
  stores additive `entries` for the results explorer.
- The current scan contract in `specs/space-sift-scan-history.md` focuses on
  trust, cancellation, local history, and honest progress. It intentionally
  keeps duplicate hashing out of ordinary scan behavior.
- The current scan engine already improved one major bottleneck:
  intermediate progress is now bounded instead of being emitted once per file.
  That helped UI responsiveness, but it does not by itself solve filesystem
  traversal cost on very large trees.
- The current recursive backend still performs extra metadata work per child:
  `read_dir` gathers child paths and `describe_path` follows with another
  metadata call. On large trees, that doubles some of the filesystem round
  trips that a more Windows-aware enumeration path may avoid.
- The duplicate-analysis path is a separate flow and must stay separate.
  Ordinary space scans must not hash file contents, sample file bodies, or
  otherwise convert a directory-size walk into a content-reading job.
- Path-class scope matters for best practice. This plan treats these as
  separate classes that may need different backends or fallbacks:
  - local fixed-volume folders, especially NTFS and ReFS
  - removable storage
  - network shares
  - cloud-backed or sync-provider folders that still project as filesystem
    paths
- The primary optimization target for the first implementation is local fixed
  volumes on Windows 11. Other path classes must remain supported, but they do
  not need the same backend on day one.
- "Do not harm the disk" in this plan means:
  - no writes or mutation
  - no unnecessary file-content reads during ordinary scanning
  - no excessive random I/O or unbounded worker fan-out
  - no aggressive scheduling that makes the machine feel hostile
  - no unsafe shortcuts that weaken correctness, cancellation, or skipped-path
    reporting
- This plan is a focused follow-up to the active scan-progress UX work. That
  earlier plan improves honesty and clarity during long scans; this plan is
  about the backend and scheduling choices that make the scan itself faster and
  lower-impact.

## Constraints

- Keep ordinary space scans read-only and unprivileged.
- Do not read full file contents during a normal space scan. Content reads stay
  opt-in and belong to duplicate analysis, not directory-size discovery.
- Preserve the current completed scan contract unless an explicit spec change
  is approved.
- Keep reparse-point safety, cancellation, skipped-path reporting, and local
  history behavior intact.
- Decide and document a primary enumeration backend for local fixed volumes,
  plus fallback behavior for removable, network, cloud-backed, or otherwise
  unsupported paths.
- Prefer metadata-first enumeration and bounded scheduling over "maximum
  parallelism" strategies that can thrash HDDs or cause broad random I/O.
- Preserve the current cross-platform recursive walker as the known-good
  fallback until the optimized Windows path passes manual validation.
- Do not add automatic storage-profile heuristics unless benchmark evidence
  shows they are materially better than a conservative default.
- Design for a future NTFS metadata fast path, but do not make direct MFT or
  USN parsing a prerequisite for this initiative.
- Keep milestones small enough for one reviewable PR each.

## Done when

- A scan architecture is documented and approved that is both faster and safer
  than the current baseline for ordinary Windows scans.
- The repo explicitly names:
  - the primary enumeration backend for local fixed volumes
  - the fallback backend for unsupported or high-risk path classes
  - the scheduling policy used during ordinary scans
- Ordinary space scans remain metadata-first and never hash or fully read
  files.
- The scan backend avoids obvious duplicated metadata work where enumeration can
  safely provide the needed data directly.
- The scanner uses bounded, storage-friendly scheduling rather than
  unconstrained parallel traversal.
- The initiative records a validation matrix covering:
  - cold and warm runs
  - at least one large local fixed-volume folder
  - one slower storage class if available, or an explicit note if unavailable
  - fallback-path correctness on a non-primary path class
- The acceptance criteria include both performance evidence and disk-safety
  evidence, not just passing unit tests.
- The repo has a concrete path for future Windows-specific fast paths without
  rewriting the UI or history contracts.

## Non-goals

- Turning the ordinary space scan into duplicate detection
- Reading full file contents to estimate "real" space usage
- Shipping a direct NTFS MFT or USN parser in the first PR of this initiative
- Background indexing, watcher services, or always-on scanning
- Aggressive max-throughput parallelism that ignores HDD and mixed-storage use
- Registry cleaning, defragmentation, or other maintenance-suite behavior

## Milestones

### Milestone 1: Lock the fast-safe scanning contract and path scope

Scope: document the product and engineering rules for fast, disk-friendly
scanning so later optimization work has a stable target and does not silently
turn scanning into a heavier content-reading workflow.

Files or components touched:
- `specs/space-sift-scan-history.md`
- `specs/space-sift-scan-history.test.md`
- optionally a narrow new spec if scan-performance behavior needs its own
  contract rather than more scan-history scope

Dependencies:
- this approved plan

Risk:
- over-specifying internal implementation details too early, or promising a
  specific Windows backend before a benchmark-backed decision

Validation commands:
- `Get-Content specs/space-sift-scan-history.md`
- `Get-Content specs/space-sift-scan-history.test.md`
- `rg -n "read-only|duplicate|progress|reparse|cancel|fallback|network|removable" specs/space-sift-scan-history.md specs/space-sift-scan-history.test.md`

Expected observable result:
- the repo contract explicitly says ordinary scans are metadata-first,
  read-only, bounded in impact, and separate from duplicate hashing
- the contract defines the primary optimization target and the required support
  level for non-primary path classes

### Milestone 2: Add measurement and baseline fixtures before optimization

Scope: add a repeatable way to measure scan cost so future changes are chosen
from evidence rather than guesswork.

Files or components touched:
- `src-tauri/crates/scan-core/src/lib.rs`
- `tests/fixtures/scan/**` if new synthetic trees are needed
- optionally `benches/**` or test-only instrumentation paths inside `scan-core`
- `specs/space-sift-scan-history.test.md` if benchmark-oriented coverage needs
  explicit mapping

Dependencies:
- Milestone 1 contract

Risk:
- microbenchmarks that measure only CPU work and hide actual filesystem cost

Validation commands:
- `cargo test -p scan-core`
- any added benchmark or instrumentation command documented in the PR

Expected observable result:
- contributors can capture a baseline for:
  - wall-clock elapsed time
  - entries per second
  - explicit follow-up metadata calls or equivalent instrumentation counters
  - progress-event count
  - cancel-to-stop latency
- baseline data exists for cold and warm runs on at least one representative
  local folder tree

### Milestone 3: Choose the enumeration backend and fallback matrix

Scope: make the main architecture choice explicitly instead of letting the
implementation drift into it. Compare candidate approaches, choose the primary
backend for local fixed volumes, and document where the current walker remains
the fallback.

Files or components touched:
- `docs/plans/2026-04-16-fast-safe-scan-architecture.md`
- `src-tauri/crates/scan-core/src/lib.rs` if light scaffolding is needed to
  expose instrumentation or backend seams
- any short design note added to a maintainer-facing doc if needed

Dependencies:
- Milestone 2 measurements

Risk:
- choosing a backend only on synthetic speed and missing compatibility,
  reparse-point, or cancellation consequences

Validation commands:
- `cargo test -p scan-core`
- baseline and candidate measurement commands from Milestone 2

Expected observable result:
- the plan records:
  - the chosen primary backend for local fixed-volume scans
  - the fallback backend for removable, network, cloud-backed, or unsupported
    paths
  - why the rejected candidates were not chosen yet
  - whether any background-priority or low-impact scheduling primitive is part
    of the design

### Milestone 4: Implement metadata-path optimization behind a safe fallback

Scope: improve the ordinary recursive scanner so it gets as much information as
possible from the chosen enumeration backend instead of re-querying metadata for
every child path when that can be avoided safely.

Files or components touched:
- `src-tauri/crates/scan-core/src/lib.rs`
- any backend-specific modules extracted from `scan-core`
- `src-tauri/src/commands/scan.rs` only if telemetry fields or backend
  configuration surfaces change

Dependencies:
- Milestone 3 backend decision

Risk:
- a Windows-specific enumeration path can introduce compatibility or
  correctness gaps around reparse points, missing paths, or permission-denied
  entries if rushed

Validation commands:
- `cargo test -p scan-core`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- any added benchmark command from Milestone 2

Expected observable result:
- scan traversal does less redundant metadata work on large trees while keeping
  the same result contract and skipped-path behavior
- the previous recursive backend still exists as a fallback path until manual
  validation signs off on the optimized backend

### Milestone 5: Introduce bounded, low-impact scheduling

Scope: add a deliberate scheduling policy for scan work so `Space Sift` avoids
disk-thrashing patterns. This includes deciding the default worker count,
whether the first implementation should stay single-threaded for some path
classes, and how cancellation interacts with any worker pool or background
priority mode.

Files or components touched:
- `src-tauri/crates/scan-core/src/lib.rs`
- new scheduler or backend modules under `src-tauri/crates/scan-core/**` if
  needed
- `specs/space-sift-scan-history.md` and `.test.md` if the observable behavior
  or configuration surface changes

Dependencies:
- Milestone 4 metadata-path cleanup

Risk:
- parallelism can make NVMe faster but make HDDs or removable storage feel
  worse; a bad default can improve benchmarks and still be the wrong product
  choice

Validation commands:
- `cargo test -p scan-core`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- benchmark and baseline commands from Milestone 2
- manual comparison on at least one large real folder

Expected observable result:
- scans use bounded scheduling that improves throughput without flooding the
  disk with random work or weakening cancellation responsiveness
- the implementation either uses a conservative default for all storage types
  or documents exactly why a storage-specific policy is justified

### Milestone 6: Manual Windows validation and maintainer guidance

Scope: validate the chosen approach on real folders and document the rules
maintainers should follow when changing scan behavior later.

Files or components touched:
- `docs/plans/2026-04-16-fast-safe-scan-architecture.md`
- `README.md` or a maintainer-facing doc if scan-performance guidance needs a
  durable home after implementation
- any small fallout fixes discovered during validation

Dependencies:
- Milestones 4 and 5

Risk:
- synthetic coverage can look good while real folders still feel too slow or
  too invasive on mixed storage

Validation commands:
- `npm run test`
- `npm run lint`
- `npm run build`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p scan-core`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo check --manifest-path src-tauri/Cargo.toml`
- manual Windows 11 scans on:
  - one genuinely large local fixed-volume folder
  - one slower storage class if available
  - one fallback path class, or an explicit note if not available locally

Expected observable result:
- maintainers have a clear record of what "fast and safe" means for this
  scanner, what was verified, what the optimized backend covers, and what still
  stays on the fallback path

## Milestone 3 backend selection

Chosen primary backend for local fixed-volume folders:
- use Win32 directory enumeration with `FindFirstFileExW` and `FindNextFileW`
- request `FindExInfoBasic` so enumeration does not query short names
- consume `WIN32_FIND_DATAW` directly for:
  - file vs directory classification
  - file size
  - reparse-point detection
  - reparse-point tag when present
- use `FIND_FIRST_EX_LARGE_FETCH` on the optimized local fixed-volume path
  because Microsoft documents it as a performance-oriented larger-buffer mode

Why this is the chosen backend:
- it removes the current obvious duplicated metadata round-trip where the scan
  enumerates child paths and then asks for metadata again per child
- it is a stable, documented user-mode Win32 API surface rather than a native
  NT layer dependency
- it returns the metadata this product already needs for ordinary size scans,
  so the first optimization step can stay small and reviewable

Local fixed-volume classifier for the optimized backend:
- root path must resolve to a stable Windows volume root
- `GetDriveTypeW(root)` must report `DRIVE_FIXED`
- the root must still satisfy the existing scan contract, including readable
  directory semantics and existing reparse-point safety rules

Fallback matrix:
- local fixed-volume root:
  - preferred backend: Win32 `FindFirstFileExW` / `FindNextFileW`
- removable root:
  - fallback backend: current recursive walker
  - rationale: keep the safer conservative path until scheduling work proves a
    better default
- remote or UNC root:
  - fallback backend: current recursive walker
  - rationale: the optimized path is intentionally scoped to local fixed
    volumes first
- cloud-backed or sync-provider path on a local fixed volume:
  - start with the optimized local backend when the root classifies as
    `DRIVE_FIXED`
  - keep existing reparse-point skip behavior for entries surfaced as reparse
    points
  - if root-level behavior is unsupported or materially incorrect, fall back to
    the current recursive walker and record the gap in validation notes
- unknown, no-root, or backend-initialization failure:
  - fallback backend: current recursive walker or clean startup failure if the
    root itself is invalid under the existing contract

Rejected or deferred candidates:
- keep `std::fs::read_dir` plus `symlink_metadata` as the primary backend:
  - rejected for Milestone 4 because it preserves the known duplicated metadata
    pattern
- `GetFileInformationByHandleEx` with `FileIdBothDirectoryInfo` or
  `FileIdExtdDirectoryInfo`:
  - deferred because it is richer and potentially interesting later, but it
    adds more handle, buffer, and parsing complexity than the first Windows win
    needs
- `NtQueryDirectoryFile`:
  - deferred because it is a lower-level native API path with more maintenance
    burden than the documented Win32 surface
- direct NTFS MFT or USN parsing:
  - still deferred beyond this initiative's first implementation path

Background-priority decision:
- do not make `THREAD_MODE_BACKGROUND_BEGIN` part of Milestone 4
- keep it as a Milestone 5 scheduling candidate only
- rationale: Microsoft documents background mode as a resource-scheduling tool
  for background work, but also warns it should minimize resource sharing; that
  makes it a scheduling choice to validate separately rather than bundle into
  the first backend swap

## Progress

- [x] 2026-04-16: reviewed the active scan contract, test spec, MVP plan, and
  scan-progress follow-up plan before drafting this architecture plan.
- [x] 2026-04-16: re-read `src-tauri/crates/scan-core/src/lib.rs` to anchor the
  plan in the current implementation rather than generic optimization advice.
- [x] 2026-04-16: revised the plan after `plan-review` to add the backend
  decision gate, path-class scope, explicit fallback requirements, and a
  measurable validation matrix.
- [x] 2026-04-16: completed Milestone 1 by updating
  `specs/space-sift-scan-history.md` and
  `specs/space-sift-scan-history.test.md` to lock metadata-first ordinary
  scans, supported path classes, and fallback-contract requirements.
- [x] 2026-04-16: completed Milestone 2 by adding `scan-core` measurement
  helpers and baseline tests for metadata-call counts, progress-event counts,
  throughput, and cancellation acknowledgement capture without changing the
  user-facing scan contract.
- [x] 2026-04-16: completed Milestone 3 by choosing a Win32 find-file
  enumeration backend for local fixed-volume roots, documenting the fallback
  matrix, and deferring lower-level directory APIs and background-priority
  scheduling to later milestones.
- [x] 2026-04-16: completed Milestone 4 by teaching `scan-core` to consume
  backend-provided child nodes, routing local fixed-volume scans through a
  `FindFirstFileExW` backend, and keeping the recursive walker for non-primary
  root classes.
- [x] 2026-04-16: completed Milestone 5 by replacing implicit recursive
  traversal with an explicit depth-first scheduler, locking the default policy
  to one active directory at a time, and keeping background mode disabled for
  now.
- [x] 2026-04-16: completed Milestone 6 by adding a maintainer-facing
  `scan-core` measurement example, validating the optimized local fixed-volume
  path on a large real folder, validating UNC fallback correctness on a stable
  tree, and recording the actual machine constraints in durable docs.

## Decision log

- 2026-04-16: ordinary space scans must stay metadata-first.
  Rationale: the fastest and safest scan is one that measures directory space
  without reading file bodies; full-content reads belong to duplicate analysis.

- 2026-04-16: "do no harm" means bounded impact, not zero I/O.
  Rationale: scanning necessarily reads filesystem metadata, but the product
  should avoid unnecessary content reads, excessive random I/O, or unbounded
  concurrency that makes the machine feel hostile.

- 2026-04-16: local fixed-volume folders are the primary optimization target.
  Rationale: that is where Windows-specific enumeration choices are most likely
  to deliver a large win without forcing risky assumptions onto network,
  removable, or cloud-backed paths.

- 2026-04-16: keep a known-good fallback backend during rollout.
  Rationale: performance changes in filesystem traversal can regress correctness
  in subtle ways; the current walker should remain available until manual
  validation confirms parity on real folders.

- 2026-04-16: add measurement as an additive `scan-core` helper instead of
  changing the app-facing scan flow first.
  Rationale: Milestone 2 needs baseline evidence, not a new UI surface. A
  counted backend wrapper and measured scan helper expose the needed counters
  while keeping optimization decisions deferred to Milestone 3.

- 2026-04-16: use `FindFirstFileExW` plus `FindNextFileW` as the primary
  optimized backend for local fixed-volume roots.
  Rationale: it is the smallest documented Windows enumeration path that
  removes the current duplicated metadata round-trip while still exposing the
  size, attribute, and reparse-point information ordinary scans need.

- 2026-04-16: extend the backend seam with `read_dir_nodes` instead of
  rewriting the whole scan contract around a new result model.
  Rationale: this keeps the recursive walker and the app-facing scan API stable
  while letting optimized backends hand pre-described child nodes directly to
  the traversal loop.

- 2026-04-16: keep `GetFileInformationByHandleEx` directory-info classes and
  `NtQueryDirectoryFile` deferred.
  Rationale: both are viable deeper options, but they add complexity without
  being necessary for the first backend win or the Milestone 4 scope.

- 2026-04-16: treat background thread mode as a scheduling experiment, not part
  of the first backend swap.
  Rationale: backend selection and resource-priority tuning are separate
  product decisions and should not be coupled before measurement on real trees.

- 2026-04-16: use an explicit depth-first scheduler with one active directory
  instead of introducing a worker pool.
  Rationale: the plan requires a bounded, low-impact scheduling policy and the
  current evidence still favors a conservative default over unproven storage
  heuristics or concurrent enumeration.

- 2026-04-16: expose scan measurement through a `scan-core` example instead of
  inventing a new app-facing diagnostics surface.
  Rationale: maintainers need a repeatable measurement path, but the product
  does not need a user-visible benchmark mode to validate the backend choice.

- 2026-04-16: prioritize removing duplicated metadata work before jumping to a
  direct NTFS parser.
  Rationale: there is a clear intermediate best-practice step between the
  current recursive walker and a full MFT fast path, and it is far safer to
  land in small reviewable PRs.

- 2026-04-16: performance work must be benchmarked and manually validated on
  real Windows folders.
  Rationale: scan performance is highly sensitive to storage type, tree shape,
  filesystem class, and Windows behavior; synthetic tests alone are not enough.

## Surprises and discoveries

- 2026-04-16: the current scan engine is already safer than many
  maintenance-suite patterns because it does not read file contents during
  ordinary scans. The biggest remaining issue is traversal cost and redundant
  metadata work, not an obviously dangerous content-reading bug.

- 2026-04-16: the current recursive backend gets child paths from `read_dir`
  and then asks for metadata again through `describe_path`, which is a clean
  abstraction but likely leaves Windows-specific performance on the table.

- 2026-04-16: earlier scan work already proved that per-item progress emission
  was a real bottleneck. That makes it more important that future performance
  changes be measured independently from UI event cadence improvements.

- 2026-04-16: "fast and safe" is not one metric. The plan needs to track wall
  time, metadata-call reduction, cancellation latency, and machine impact
  together, or it will optimize the wrong thing.

- 2026-04-16: the current scan-core API already had a clean seam for
  measurement. Wrapping the backend and progress callback was enough to capture
  metadata-call and progress-event baselines without rewriting the recursive
  walker yet.

- 2026-04-16: the best first Windows optimization is not the most exotic API.
  `FindFirstFileExW` with `FindExInfoBasic` already gives enough metadata to
  remove the current extra per-child metadata call while staying on a widely
  used Win32 surface.

- 2026-04-16: the cleanest implementation path was to keep `describe_path` for
  root validation and let only child enumeration switch to embedded node data.
  That avoids special-casing the root path while still removing the dominant
  duplicated metadata work inside directory traversal.

- 2026-04-16: converting traversal to an explicit stack was the cleanest way to
  make scheduling deliberate without changing the scan contract. It keeps
  cancellation checks between directory work units and avoids coupling the
  scheduling milestone to a concurrency milestone.

- 2026-04-16: loopback UNC validation was useful for fallback correctness, but
  it also confirmed that even the safe fallback path is dramatically slower
  than the optimized local fixed-volume backend on the same stable tree.

- 2026-04-16: live mutable cache trees are poor backend-parity fixtures.
  A large cache directory is still valuable for throughput measurement, but it
  can drift during long fallback scans and should not be treated as the source
  of truth for correctness parity.

- 2026-04-16: this machine did not expose a slower storage class for validation.
  Both detected physical disks were NVMe SSDs, so the milestone had to record
  that limitation explicitly instead of manufacturing an HDD or removable-media
  result.

## Validation and acceptance

Planning validation performed while revising this plan:
- `Get-Content AGENTS.md`
- `Get-Content docs/workflows.md`
- `Get-Content docs/plan.md`
- `Get-Content .codex/PLANS.md`
- `Get-Content specs/space-sift-scan-history.md`
- `Get-Content specs/space-sift-scan-history.test.md`
- `Get-Content docs/plans/2026-04-15-space-sift-win11-mvp.md`
- `Get-Content docs/plans/2026-04-16-scan-progress-and-active-run-ux.md`
- `Get-Content docs/plans/2026-04-16-fast-safe-scan-architecture.md`
- `rg -n "read_dir|metadata|reparse|progress|top_items_limit" src-tauri/crates/scan-core/src/lib.rs`
- `Get-Content src-tauri/crates/scan-core/src/lib.rs | Select-Object -First 260`

Acceptance evidence for the implemented initiative:
- ordinary scans remain read-only and do not hash or fully read files
- the chosen primary backend and fallback matrix are documented in the plan or
  a maintainer-facing follow-up doc
- optimized local fixed-volume scans show a documented reduction in explicit
  metadata follow-up work versus baseline, or the plan records why that was not
  achieved
- wall-clock results are documented for both cold and warm runs on at least one
  large local folder
- cancellation, skipped-path handling, and history persistence still behave as
  required by the current contract
- one fallback-path validation run confirms correctness, or the plan records
  why that path class could not be tested locally
- if a scheduling change improves throughput but makes the machine feel more
  hostile, the plan records that as a failed product outcome even if the raw
  benchmark looks better

## Validation notes

- 2026-04-16: planning-only turn. No build, lint, test, or manual runtime
  commands were run for the plan revision itself.
- 2026-04-16: Milestone 1 validation was doc-only:
  `Get-Content specs/space-sift-scan-history.md`,
  `Get-Content specs/space-sift-scan-history.test.md`, and
  `rg -n "metadata-first|fallback|local fixed-volume|network shares|duplicate-confirmation hashing" specs/space-sift-scan-history.md specs/space-sift-scan-history.test.md`.
- 2026-04-16: Milestone 2 validation passed with:
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p scan-core measured_scan_reports`,
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p scan-core`,
  and `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo check --manifest-path Cargo.toml`.
- 2026-04-16: Milestone 3 validation was architecture review only. No build or
  test commands were run because this milestone records the backend decision
  and fallback matrix rather than landing code. The decision was checked
  against official Microsoft API documentation for `FindFirstFileExW`,
  `FindNextFileW`, `WIN32_FIND_DATAW`, `GetDriveTypeW`,
  `GetFileInformationByHandleEx`, and background thread mode.
- 2026-04-16: Milestone 4 validation passed with:
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p scan-core routed_scan`,
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p scan-core measured_scan_path_uses_windows_find_backend_for_fixed_roots`,
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p scan-core`,
  and `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo check --manifest-path Cargo.toml`.
- 2026-04-16: Milestone 5 validation passed with:
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p scan-core`,
  and `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo check --manifest-path Cargo.toml`.
- 2026-04-16: Milestone 6 validation added a maintainer measurement entrypoint
  at `src-tauri/crates/scan-core/examples/measure_scan.rs`, then recorded real
  Windows runs with:
  `Get-PhysicalDisk | Select-Object FriendlyName,MediaType,BusType,Size,HealthStatus | Format-Table -AutoSize`,
  `Get-Volume | Select-Object DriveLetter,FileSystem,DriveType,HealthStatus,SizeRemaining,Size | Format-Table -AutoSize`,
  `Get-SmbShare | Select-Object Name,Path,Description | Format-Table -AutoSize`,
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo run --release --manifest-path src-tauri/Cargo.toml -p scan-core --example measure_scan -- C:\\Users\\xiongxianfei\\.gradle\\caches --repeat 2`,
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo run --release --manifest-path src-tauri/Cargo.toml -p scan-core --example measure_scan -- D:\\Data\\20260415-space-sift`,
  and `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo run --release --manifest-path src-tauri/Cargo.toml -p scan-core --example measure_scan -- \\\\localhost\\D$\\Data\\20260415-space-sift`.
- 2026-04-16: Milestone 6 real-machine results:
  both detected physical disks were NVMe SSDs, so no slower storage class was
  available locally; the large local fixed-volume run on
  `C:\\Users\\xiongxianfei\\.gradle\\caches` measured `6323 ms` on the first
  run and `7456 ms` on the second run with only `1` `describe_path` call in
  both cases; the stable-tree UNC fallback run on
  `\\\\localhost\\D$\\Data\\20260415-space-sift` matched the local fixed
  counts and bytes exactly but took `10034 ms` versus `75 ms` locally and
  required `28798` `describe_path` calls versus `1` locally.
- 2026-04-16: repo-wide post-doc verification passed with:
  `npm run lint`,
  `npm run test`,
  `npm run build`,
  `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p scan-core`,
  and `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo check --manifest-path src-tauri/Cargo.toml`.
- 2026-04-16: the current scan contract already protects against one major
  safety mistake by keeping duplicate hashing out of ordinary scanning.
- 2026-04-16: the current implementation review confirmed that the next
  best-practice backend target is reducing duplicated metadata work in the
  recursive walker, not adding risky content reads or broad concurrency first.
- 2026-04-16: the original draft plan needed a stronger backend decision gate
  and a clearer acceptance matrix before it could reasonably claim to represent
  best practice.

## Idempotence and recovery

- Land this as small PRs so the scanner can fall back to the current recursive
  backend if a Windows-specific optimization proves incorrect.
- Keep backend abstractions additive: introduce optimized enumeration paths
  behind the existing scan contract rather than rewriting the UI or history
  model.
- If a concurrency or scheduling change improves benchmarks but worsens manual
  HDD or removable-media behavior, revert that scheduling change and keep the
  metadata-path improvements.
- If a Windows-specific fast path introduces edge-case correctness issues,
  retain it behind a clearly bounded backend switch until skipped-path,
  cancellation, and history behavior match the baseline.
- If the backend decision remains ambiguous after Milestone 3, stop and record
  the measurement gap instead of silently blending backends in an ad hoc way.

## Outcomes & retrospective

Expected outcome:
- `Space Sift` gets a scan architecture that is faster on real folders,
  remains read-only and metadata-first, and has a clean path toward future
  Windows-specific acceleration without compromising disk safety or user trust

Retrospective focus:
- whether Windows-friendly metadata enumeration is enough for the current
  product level, or whether a later NTFS MFT fast path should move higher in
  priority
- whether the chosen default scheduling policy feels safe on HDDs as well as
  SSD and NVMe systems
- whether the fallback matrix stayed simple and honest, or whether one more
  explicit backend split is warranted for network or cloud-backed paths
