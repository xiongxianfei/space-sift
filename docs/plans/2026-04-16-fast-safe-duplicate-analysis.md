# Design A Fast, Correct, Disk-Friendly Duplicate Analysis Architecture

## Metadata

- Status: done
- Created: 2026-04-16
- Updated: 2026-04-16
- Owner: xiongxianfei / Codex
- Related spec(s):
  - `specs/space-sift-duplicates.md`
  - `specs/space-sift-duplicates.test.md`
  - `specs/space-sift-scan-history.md`
- Related plan(s):
  - `docs/plans/2026-04-15-space-sift-win11-mvp.md`
  - `docs/plans/2026-04-16-fast-safe-scan-architecture.md`
- Supersedes / Superseded by: none
- Branch / PR: none yet
- Last reviewed files:
  - `src-tauri/crates/duplicates-core/src/lib.rs`
  - `src-tauri/crates/app-db/src/lib.rs`
  - `src-tauri/src/commands/duplicates.rs`
  - `src/App.tsx`
  - `src/duplicates.test.tsx`
  - `specs/space-sift-duplicates.md`
  - `specs/space-sift-duplicates.test.md`

## Purpose / Big picture

Make duplicate analysis feel materially faster on large real folders without
weakening the trust model that makes the feature safe to use. The end state is
not "hash everything faster at any cost." The end state is a duplicate-analysis
pipeline that:

- stays full-hash-correct
- reduces avoidable work before content hashing starts
- reuses valid local cache data efficiently
- uses bounded, disk-friendly hashing concurrency
- remains cancellation-aware and read-only

This initiative is specifically about duplicate analysis. It is not a scan
initiative, a cleanup initiative, or a UI redesign initiative.

## Context and orientation

- The current duplicate flow already has the correct high-level product shape:
  a loaded scan result drives staged duplicate verification through
  `duplicates-core`, the Tauri command layer emits progress, and the React UI
  shows only fully verified groups.
- The current duplicate contract in `specs/space-sift-duplicates.md` is
  intentionally strict: size match alone is not enough, partial hash alone is
  not enough, and the app must not show a group unless full-hash confirmation
  matched.
- The current backend already includes two valuable safety/performance wins:
  - `duplicate_size_candidates(...)` filters out unique-size files before
    metadata validation
  - progress emission is bounded instead of emitting once per processed item
- The remaining hot path in `src-tauri/crates/duplicates-core/src/lib.rs` is
  still expensive on large candidate sets:
  - candidate validation is single-threaded
  - cache lookup and cache save happen per file and per stage
  - full hashing is single-threaded
  - the full-hash loop uses a small `8192` byte buffer
  - partial hashing allocates a new buffer per file
- The current `HistoryStore` cache API in `src-tauri/crates/app-db/src/lib.rs`
  is correctness-safe but not performance-oriented. Each lookup or save still
  goes through the SQLite path individually, which is likely a meaningful cost
  on warm reruns with large candidate sets.
- Duplicate analysis is different from ordinary scan work:
  - ordinary scans should avoid file-content reads
  - duplicate analysis must read file contents by design
  - because of that, "do no harm" here means bounded, sequential-friendly
    content reads rather than pretending hashing can be free
- The duplicate contract in `specs/space-sift-duplicates.md` already sets a
  strict external boundary for this work:
  - analysis must remain local-only
  - analysis must not require network access or cloud synchronization
  - that means hashing policy cannot silently hydrate on-demand placeholder
    files or turn remote-backed reads into an invisible side effect
- Path class still matters. A bounded hashing policy that is reasonable on a
  local fixed-volume SSD may be the wrong default for:
  - removable storage
  - remote or UNC paths
  - cloud-backed folders with on-demand hydration behavior
- The repo currently has no maintainer-facing duplicate-performance benchmark or
  measurement entrypoint comparable to the new scan measurement example.

## Constraints

- Full-hash confirmation remains non-negotiable for presented duplicate groups.
- Duplicate analysis stays read-only and preview-only. No delete, recycle,
  rename, or move behavior belongs in this initiative.
- The normal app stays unprivileged. No admin requirement is allowed as a
  shortcut for performance.
- Keep cancellation prompt. Performance work must not turn cancellation into a
  long "finish hashing first" wait.
- Preserve the current history and UI contract unless a spec update explicitly
  changes it.
- Prefer eliminating redundant work before adding wider concurrency.
- Duplicate analysis MUST NOT silently hydrate on-demand cloud placeholder
  files or trigger network-backed content fetches as a side effect of hashing.
  If that guarantee cannot be made for a path class, the plan must keep a
  conservative fallback or skip/report that case instead of weakening the
  local-only contract.
- Do not use unbounded worker counts or "hash everything at once" scheduling.
- Do not treat remote, removable, or cloud-backed paths as equivalent to local
  fixed-volume SSD paths unless measurement proves the same policy is safe.
- Any cache strategy chosen for warm reruns must avoid pathological concurrent
  SQLite write contention. Worker-pool design is downstream of that decision,
  not a substitute for it.
- Keep milestones small enough for one reviewable PR each.

## Done when

- The repo documents a duplicate-analysis architecture that is faster and safer
  than the current baseline on real large duplicate candidate sets.
- The repo explicitly names:
  - the primary performance strategy for local fixed-volume duplicate analysis
  - the file-open and sequential-read strategy used while hashing
  - the fallback or reduced-concurrency behavior for non-primary path classes,
    including cloud-placeholder handling
  - the duplicate-cache access strategy used during warm reruns, including
    lookup behavior and writeback behavior
- The duplicate verifier still requires full-hash confirmation before emitting a
  group.
- Measurement exists for duplicate analysis, including at least:
  - wall-clock elapsed time
  - candidate counts by stage
  - cache lookup and write counts, plus cache hit / miss or equivalent stage
    reuse evidence
  - bytes hashed for partial and full hashing
  - progress-event count
  - cancel-to-stop latency
- Same-dataset cold and warm reruns exist for the chosen local fixed-volume
  benchmark set so cache reuse claims are backed by evidence instead of
  inference.
- The implementation removes obvious duplicated work in the cache or hashing
  path before or alongside any worker-pool change.
- Bounded concurrency, if introduced, improves local fixed-volume performance
  without turning the machine or disk behavior hostile.
- The implementation keeps the current conservative duplicate-analysis path
  available until the new strategy has passed Milestone 6 validation.
- Manual validation records:
  - one large local fixed-volume duplicate run
  - one warm rerun on the same candidate set
  - one non-primary fallback path class if locally available, or an explicit
    note that it was unavailable
  - cloud-placeholder or network-trigger behavior if locally available, or an
    explicit note that it could not be validated on the machine used

## Non-goals

- Similar-image, media, or fuzzy-content matching
- Removing the full-hash confirmation requirement
- Converting duplicate analysis into a background service
- Running the whole desktop app as administrator
- Deletion or Recycle Bin execution behavior
- NTFS MFT- or USN-based duplicate discovery in this initiative
- Broad UI redesign beyond additive progress or diagnostic details that are
  strictly required by the performance work

## Milestones

### Milestone 1: Lock the fast-safe duplicate-performance contract

Scope: update the duplicate-analysis contract so future optimization work has a
stable target and cannot silently trade correctness for benchmark wins.

Files or components touched:
- `specs/space-sift-duplicates.md`
- `specs/space-sift-duplicates.test.md`

Dependencies:
- this approved plan

Risk:
- over-specifying internal implementation details too early, or promising an
  aggressive worker strategy before the repo has baseline measurements

Validation commands:
- `Get-Content specs/space-sift-duplicates.md`
- `Get-Content specs/space-sift-duplicates.test.md`
- `rg -n "full-hash|cancel|running|cache|read-only|path class|progress" specs/space-sift-duplicates.md specs/space-sift-duplicates.test.md`

Expected observable result:
- the contract explicitly says duplicate analysis remains full-hash-correct,
  read-only, cancellation-aware, and bounded in disk impact
- the contract names local fixed-volume paths as the primary optimization
  target and requires safe fallback behavior for non-primary path classes

### Milestone 2: Add measurement and baseline fixtures before deeper optimization

Scope: add a repeatable duplicate-analysis measurement seam so later decisions
about batching, caching, and concurrency are made from evidence.

Files or components touched:
- `src-tauri/crates/duplicates-core/src/lib.rs`
- `src-tauri/crates/duplicates-core/examples/measure_duplicates.rs` or an
  equivalent maintainer-facing example command
- `specs/space-sift-duplicates.test.md` if measurement-oriented coverage needs
  explicit mapping

Dependencies:
- Milestone 1 contract

Risk:
- measuring only end-to-end elapsed time and missing where the time is actually
  spent by stage

Validation commands:
- `cargo test -p duplicates-core`
- the new measurement example command documented in the PR

Expected observable result:
- maintainers can capture baseline duplicate-analysis metrics for:
  - total elapsed time
  - metadata-validated candidates
  - partial-hash candidates
  - full-hash candidates
  - cache lookup and write counts
  - cache hits and misses
  - bytes read for partial and full hashing
  - progress-event count
  - cancel-to-stop latency

### Milestone 3: Choose the cache and scheduling architecture explicitly

Scope: turn "best practice" into explicit repo decisions instead of letting the
implementation drift into them. This milestone should decide the first
performance architecture before landing worker-pool code. It must explicitly
choose:
- the cache lookup model for warm reruns
- the cache writeback model under hashing concurrency
- the file-open and sequential-read policy used while hashing
- the path-class routing policy for local fixed, removable, remote/UNC, and
  cloud-placeholder-heavy roots

Files or components touched:
- `docs/plans/2026-04-16-fast-safe-duplicate-analysis.md`
- `src-tauri/crates/duplicates-core/src/lib.rs` if small seams are required for
  measurement or path-class routing
- `src-tauri/crates/app-db/src/lib.rs` if small cache-batch seams are needed

Dependencies:
- Milestone 2 measurements

Risk:
- choosing a concurrency model based only on SSD throughput and missing warm
  rerun, remote path, cancellation, or SQLite contention consequences

Validation commands:
- `cargo test -p duplicates-core`
- baseline and candidate measurement commands from Milestone 2

Expected observable result:
- the plan records:
  - the chosen cache strategy for repeated hash lookups
  - the chosen cache writeback strategy, including how concurrent writes are
    avoided or serialized
  - the chosen file-open and sequential-read policy for hashing
  - the chosen hashing scheduling policy for local fixed-volume paths
  - the fallback or reduced-concurrency policy for removable, remote, or
    otherwise risky paths
  - the cloud-placeholder policy: conservative fallback, skip/report, or other
    explicitly justified behavior that preserves the local-only contract
  - why rejected candidates were not chosen yet

### Milestone 4: Remove avoidable overhead before wider parallelism

Scope: land the no-regret backend improvements that should help even if the
final worker-count decision stays conservative. This milestone is intentionally
limited to low-risk work such as buffer reuse, larger sequential hashing
buffers, cache prefetch or preload, batched writeback, and removal of
per-candidate SQLite overhead that would otherwise distort later concurrency
measurements.

Files or components touched:
- `src-tauri/crates/duplicates-core/src/lib.rs`
- `src-tauri/crates/app-db/src/lib.rs`
- `src-tauri/src/commands/duplicates.rs` only if additive telemetry fields or
  measurement surfaces change

Dependencies:
- Milestone 3 architecture decisions

Risk:
- cache batching or metadata reuse can accidentally weaken cache invalidation
  rules if rushed

Validation commands:
- `cargo test -p duplicates-core`
- `cargo test -p app-db`
- `cargo check --manifest-path src-tauri/Cargo.toml`

Expected observable result:
- warm reruns perform fewer redundant SQLite cache round-trips
- cache writes avoid per-file contention patterns that would fight later
  bounded concurrency
- hashing paths reuse buffers or read more efficiently without changing the
  correctness contract
- cancellation and issue reporting still behave exactly as required

### Milestone 5: Introduce bounded, stage-aware hashing concurrency

Scope: add a deliberate hashing scheduler only after the cache and overhead
work above is in place. The target is bounded throughput gains, not unbounded
parallel hashing. Any concurrency added here must be local-fixed-volume-first
and paired with the cache-write strategy chosen in Milestone 3 so the repo does
not trade hashing throughput for SQLite lock thrash.

Files or components touched:
- `src-tauri/crates/duplicates-core/src/lib.rs`
- any new scheduler helpers under `src-tauri/crates/duplicates-core/**`
- `specs/space-sift-duplicates.md` and `.test.md` if the observable telemetry
  or path-class policy changes

Dependencies:
- Milestone 4 backend cleanup

Risk:
- an aggressive worker pool can improve SSD benchmarks while making remote,
  removable, or cloud-backed paths far worse

Validation commands:
- `cargo test -p duplicates-core`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- baseline and candidate measurement commands from Milestone 2
- manual comparison on at least one large real candidate set

Expected observable result:
- local fixed-volume duplicate analysis is materially faster than baseline
- cancellation remains prompt during full hashing
- non-primary path classes either use a safer reduced-concurrency policy or are
  explicitly documented as such
- cache writes remain serialized, deferred, or otherwise bounded enough that
  warm-rerun performance does not regress under concurrency

### Milestone 6: Manual validation and maintainer guidance

Scope: validate the chosen approach on real folders and document the rules
maintainers should follow when touching duplicate-analysis performance later.

Files or components touched:
- `docs/plans/2026-04-16-fast-safe-duplicate-analysis.md`
- `README.md` or a maintainer-facing doc if duplicate-performance guidance
  needs a durable home
- any small fallout fixes identified during validation

Dependencies:
- Milestones 4 and 5

Risk:
- synthetic duplicate fixtures can look good while real large groups still feel
  too slow or too disk-heavy

Validation commands:
- `npm run test`
- `npm run lint`
- `npm run build`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p duplicates-core`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo test -p app-db`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo check --manifest-path src-tauri/Cargo.toml`
- manual Windows 11 duplicate-analysis runs on:
  - one large local fixed-volume candidate set
  - one warm rerun of the same set
  - one non-primary path class if available, or an explicit note if unavailable
  - one cloud-placeholder or network-trigger-sensitive path if available, or an
    explicit note if unavailable

Expected observable result:
- maintainers have a clear record of what "fast and safe" means for duplicate
  analysis, which path classes get the optimized policy, what was validated,
  and what stays intentionally conservative

## Progress

- [x] 2026-04-16: reviewed `AGENTS.md`, `docs/workflows.md`, `docs/plan.md`,
  `.codex/PLANS.md`, the duplicate spec/test spec, `duplicates-core`,
  `app-db`, the Tauri duplicate command layer, and the React duplicate panel
  before drafting this plan.
- [x] 2026-04-16: completed Milestone 1 by updating
  `specs/space-sift-duplicates.md` and
  `specs/space-sift-duplicates.test.md` to lock the fast-safe duplicate
  contract around local-only hashing, safe non-primary fallback behavior, and
  manual validation expectations.
- [x] 2026-04-16: completed Milestone 2 by adding
  `measure_duplicate_analysis(...)` and
  `src-tauri/crates/duplicates-core/examples/measure_duplicates.rs`, plus
  metric-focused Rust tests for cold runs, warm cache reruns, and
  cancellation-latency reporting.
- [x] 2026-04-16: completed Milestone 3 by choosing a candidate-scoped
  in-memory cache overlay, single-writer staged cache flushes, a sequential
  file-read policy, and explicit path-class routing for local fixed,
  removable, remote/UNC, and cloud-placeholder-sensitive paths.
- [x] 2026-04-16: completed Milestone 4 by implementing a preloaded
  duplicate-cache session in `duplicates-core`, staged batch cache flushes,
  reusable partial/full hashing buffers, and transactional batch cache load/save
  paths in `app-db`.
- [x] 2026-04-16: completed Milestone 5 by adding bounded stage-aware hashing
  workers for local fixed-volume roots, conservative serial policies for
  removable/other roots, and clean skip/report handling for remote and
  placeholder-sensitive paths.
- [x] 2026-04-16: completed Milestone 6 by recording real-folder validation in
  `docs/duplicate-performance.md`, linking the maintainer guidance from the
  README, and closing the plan after the full verification pass.

## Decision log

- 2026-04-16: full-hash confirmation remains the trust boundary.
  Rationale: duplicate analysis exists to recommend later deletion candidates,
  so speed improvements that weaken full-hash correctness are not acceptable.

- 2026-04-16: optimize avoidable cache and hashing overhead before wider
  concurrency.
  Rationale: per-file cache round-trips and small-buffer hashing are obvious
  no-regret targets; a worker pool layered on top of avoidable waste is the
  wrong first move.

- 2026-04-16: treat local fixed-volume paths as the primary performance target.
  Rationale: duplicate analysis reads file contents, so path class matters even
  more than it does for ordinary metadata scans.

- 2026-04-16: any concurrency added here must be bounded and path-class-aware.
  Rationale: max-throughput hashing can be fine on SSDs and still be the wrong
  product choice for network, removable, or hydrated cloud-backed paths.

- 2026-04-16: cache lookup/writeback architecture must be chosen before adding
  worker-pool hashing.
  Rationale: per-file SQLite calls are already likely to dominate some warm
  reruns, and adding parallel hashing before the cache path is explicit risks
  creating lock contention instead of end-to-end speed gains.

- 2026-04-16: duplicate analysis must preserve the local-only contract even on
  cloud-backed or remote-backed folders.
  Rationale: content hashing is inherently more dangerous than metadata scans;
  if a path class risks placeholder hydration or network-backed reads, the safe
  answer is a conservative fallback or skip/report policy, not silent
  background fetches.

- 2026-04-16: measurement for Milestone 2 stays inside `duplicates-core`
  instead of changing the Tauri/UI contract.
  Rationale: this milestone needs backend evidence first; exposing additive
  metrics through a library API and example command keeps later architecture
  decisions informed without committing the desktop contract too early.

- 2026-04-16: warm-rerun cache lookup should use a candidate-scoped preload and
  in-memory overlay, not inline SQLite lookups during hashing.
  Rationale: `HistoryStore` currently does per-key reads and read-before-write
  updates; Milestone 2 metrics confirm the engine already makes one lookup per
  candidate and per stage, so leaving those calls on the SQLite path would
  amplify contention as soon as bounded concurrency is introduced.

- 2026-04-16: cache writes should use a single-writer staged flush model.
  Rationale: computed partial/full hashes are valid additive artifacts, but
  writing them per file from hashing workers would trade throughput for SQLite
  lock contention. The chosen model is: update the in-memory overlay
  immediately, batch staged writes, and flush them from one writer on the
  analysis thread at stage boundaries or terminal completion.

- 2026-04-16: the first hashing I/O policy stays sequential and conservative.
  Rationale: each worker should open one file at a time, read it from offset 0
  with reusable buffers, and avoid memory mapping or chunk-striping a single
  file across multiple workers. That keeps cancellation predictable and avoids
  turning duplicate analysis into random-read pressure.

- 2026-04-16: path-class routing is explicit before concurrency lands.
  Rationale:
  - local fixed-volume roots are the only primary optimized path
  - removable paths keep the same correctness contract but stay on a more
    conservative scheduler
  - remote/UNC paths are not content-hashed because doing so would require
    network-backed reads and violate the local-only contract
  - cloud-placeholder-heavy local paths use a conservative fallback and skip or
    report files when safe local hashing cannot be guaranteed without
    placeholder hydration

- 2026-04-16: Milestone 4 uses a preloaded cache session plus staged batch
  flushes instead of a direct write-through cache wrapper.
  Rationale: the analysis thread still needs instant visibility of hashes it
  just computed, but SQLite does not. An in-memory overlay gives immediate
  reuse while deferring the persistence work to fewer batched writes.

- 2026-04-16: the first batch cache implementation in `HistoryStore` uses one
  connection plus prepared statements for batched loads and a transaction with
  `ON CONFLICT ... DO UPDATE` for staged writes.
  Rationale: this removes the current read-before-write hot path and most of
  the connection churn without introducing a risky SQL shape such as temporary
  tables or custom virtual-table dependencies.

- 2026-04-16: Milestone 5 uses bounded per-stage workers, not a general worker
  pool.
  Rationale: local fixed-volume hashing now runs with at most `2` partial-hash
  workers and at most `4` full-hash workers, while each worker still hashes one
  file at a time sequentially. That captures the first obvious throughput win
  without turning the stage into open-ended parallel I/O.

- 2026-04-16: remote roots are skipped and reported rather than hashed.
  Rationale: content-hashing a UNC/remote root would require network-backed
  reads and directly conflict with the local-only contract; the safe answer is a
  clean issue list, not a slower remote fallback that silently violates the
  contract.

- 2026-04-16: placeholder-style Windows file attributes are treated as unsafe
  for duplicate hashing on local fixed roots.
  Rationale: `FILE_ATTRIBUTE_OFFLINE`, `RECALL_ON_OPEN`, and
  `RECALL_ON_DATA_ACCESS` are enough to justify a conservative skip/report path
  before the app attempts a content read that could hydrate data on demand.

- 2026-04-16: maintainer duplicate-performance guidance lives in
  `docs/duplicate-performance.md`.
  Rationale: the duplicate path now has stable measurement commands,
  path-class policy, and real validation results that should be documented in a
  durable maintainer-facing location rather than left only in terminal output.

## Surprises and discoveries

- 2026-04-16: the repo already landed one important prefilter:
  unique-size candidates are skipped before metadata validation, so the next
  wins are deeper in the cache and hashing path rather than the initial size
  grouping.

- 2026-04-16: `HistoryStore` is already correctness-safe as a hash cache, but
  it still does per-key lookup/save work that is likely expensive on repeated
  large analyses.

- 2026-04-16: duplicate progress is bounded already, but the current status
  model still does not expose measurement-oriented data such as cache reuse,
  bytes hashed, or cancel-to-stop timing.

- 2026-04-16: the current full-hash loop reads with a small `8192` byte
  buffer, which is safe and simple but unlikely to be the best steady-state
  throughput choice for large local files.

- 2026-04-16: the current cache API shape is correctness-safe but likely a bad
  fit for future concurrency because per-file SQLite lookups and writes would
  compound contention unless the write path is made explicit first.

- 2026-04-16: duplicate performance has a risk ordinary scans do not:
  opening placeholder-backed files may have side effects beyond local disk I/O,
  so path-class policy needs to be part of the performance architecture rather
  than an afterthought.

- 2026-04-16: `HistoryStore::save_hash_cache_entry(...)` currently reloads the
  existing cache row before every write and then performs `INSERT OR REPLACE`,
  which confirms that inline write-through caching would multiply SQLite work
  even before concurrency is added.

- 2026-04-16: the duplicate engine can reduce SQLite overhead without changing
  the existing UI/status contract because cache preloading and staged flushes
  are fully internal to `duplicates-core`.

- 2026-04-16: a warm rerun over the disposable duplicate fixture still reports
  the same candidate-stage counts as Milestone 2, which is good; Milestone 4
  changed the storage path overhead, not the duplicate-verification contract.

- 2026-04-16: the repeated-size prefilter means a measurement run over a folder
  with no repeated file sizes will legitimately report zero validated/hash
  candidates even when candidate enumeration succeeded.

- 2026-04-16: a real duplicate-analysis run over
  `C:\Users\xiongxianfei\.gradle\caches` is large enough to exercise the new
  scheduler meaningfully: `254065` candidates, `248308` validated, `248170`
  full-hash candidates, and `23167` verified groups.

- 2026-04-16: on that `.gradle\caches` run, the warm rerun dropped from
  `45320 ms` to `7818 ms` with identical group and issue counts and zero
  content bytes read on the warm pass. That primarily validates the cache path,
  but it also confirms the bounded worker scheduler did not destabilize the
  duplicate result contract.

- 2026-04-16: the local `OneDrive` root existed during Milestone 6 validation,
  but it only contained `desktop.ini`, so no usable placeholder-backed
  duplicate fixture was available on this machine.

## Validation and acceptance

Planning validation performed while drafting this plan:
- `Get-Content AGENTS.md`
- `Get-Content docs/workflows.md`
- `Get-Content docs/plan.md`
- `Get-Content .codex/PLANS.md`
- `Get-Content specs/space-sift-duplicates.md`
- `Get-Content specs/space-sift-duplicates.test.md`
- `Get-Content src-tauri/crates/duplicates-core/src/lib.rs | Select-Object -First 360`
- `Get-Content src-tauri/crates/duplicates-core/src/lib.rs | Select-Object -Skip 360 -First 420`
- `Get-Content src-tauri/crates/app-db/src/lib.rs | Select-Object -Skip 180 -First 220`
- `Get-Content src-tauri/src/commands/duplicates.rs`
- `Get-Content src/App.tsx | Select-String -Pattern "duplicate|Analyze duplicates|Cancel analysis" -Context 4,8`
- `rg -n "duplicate_size_candidates|group_by_partial_hash|group_by_full_hash|save_partial_hash|get_cached_hashes|cancel|progress" src-tauri/crates/duplicates-core/src/lib.rs src-tauri/crates/app-db/src/lib.rs src-tauri/src/commands/duplicates.rs`

Acceptance evidence for the implemented initiative:
- duplicate analysis is measurably faster on a large real local fixed-volume
  candidate set than the current baseline
- warm reruns on the same candidate set show cache-reuse evidence instead of
  repeating equivalent work
- cache lookup/write counts and bytes-read totals show where the speedup came
  from rather than relying only on elapsed time
- cancellation remains prompt during long full-hash stages
- only fully verified groups are presented after optimization
- non-primary path classes either honor a safer fallback policy or are
  explicitly documented as unavailable or unvalidated locally
- cloud-placeholder or network-trigger behavior is either explicitly validated
  or explicitly called out as unavailable on the validation machine

## Validation notes

- 2026-04-16: planning-only turn. No `cargo`, `npm`, or Tauri runtime commands
  were run because this step created a plan only.
- 2026-04-16: Milestone 1 was doc-only. Validation was limited to spec/test
  readback and `rg` checks against the updated contract terms; no Rust,
  frontend, or Tauri runtime commands were needed for this step.
- 2026-04-16: Milestone 2 validation ran:
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p duplicates-core`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo check --manifest-path Cargo.toml`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo run -p duplicates-core --example measure_duplicates -- crates/duplicates-core --repeat 2`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo run -p duplicates-core --example measure_duplicates -- <temporary duplicate fixture> --repeat 2`
- 2026-04-16: Milestone 3 was an architecture-decision step. Validation used:
  - `Get-Content src-tauri/crates/app-db/src/lib.rs | Select-Object -Skip 180 -First 220`
  - `Get-Content src-tauri/src/commands/duplicates.rs`
  - `Get-Content src-tauri/crates/duplicates-core/examples/measure_duplicates.rs`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p duplicates-core`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo run -p duplicates-core --example measure_duplicates -- <temporary duplicate fixture> --repeat 2`
  The measurement runs confirmed the current engine-level stage counts and warm
  cache-hit behavior, but they do not yet count the extra SQLite round-trips
  inside `HistoryStore`; that is why the chosen cache architecture moves lookup
  and writeback out of the per-file hot path before concurrency work.
- 2026-04-16: Milestone 4 validation ran:
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p duplicates-core`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p app-db`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo check --manifest-path Cargo.toml`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo run -p duplicates-core --example measure_duplicates -- <temporary duplicate fixture> --repeat 2`
  Additional focused regression checks confirmed that:
  - `duplicates-core` now uses one batched cache preload and staged batch
    flushes instead of single-key cache calls in the hot path
  - `app-db` batch saves merge partial and full hashes correctly through
    transactional upserts
- 2026-04-16: Milestone 5 validation ran:
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p duplicates-core`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo check --manifest-path Cargo.toml`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo run -p duplicates-core --example measure_duplicates -- C:\Users\xiongxianfei\.gradle\caches --repeat 2`
  Focused regression checks covered:
  - bounded local-fixed worker selection
  - remote-root skip/report behavior for the local-only contract
  - placeholder-attribute detection for conservative local fixed-volume skips
- 2026-04-16: Milestone 6 validation ran:
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo run --release --manifest-path src-tauri/Cargo.toml -p duplicates-core --example measure_duplicates -- C:\Users\xiongxianfei\.gradle\caches --repeat 2`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo run --release --manifest-path src-tauri/Cargo.toml -p duplicates-core --example measure_duplicates -- \\localhost\D$\Data\20260415-space-sift`
  - `Get-ChildItem -LiteralPath "$env:USERPROFILE\OneDrive" -Force -Recurse -ErrorAction SilentlyContinue | Select-Object -First 50 FullName, Attributes`
  - `npm run test`
  - `npm run lint`
  - `npm run build`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p duplicates-core`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p app-db`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo check --manifest-path src-tauri/Cargo.toml`
  Recorded results:
  - local fixed-volume release run on `C:\Users\xiongxianfei\.gradle\caches`:
    cold `24800 ms`, warm `7304 ms`, `23167` groups, `3410` issues, and zero
    content bytes read on the warm rerun
  - remote UNC fallback run on `\\localhost\D$\Data\20260415-space-sift`:
    `13 ms`, zero validated/hash candidates, zero content bytes read, `21723`
    issues reported
  - placeholder-sensitive validation remained unavailable because the local
    `OneDrive` root did not contain a usable duplicate candidate set

## Idempotence and recovery

- Land this as small PRs so cache, scheduler, and measurement changes can be
  reviewed independently.
- Keep performance telemetry additive wherever possible so the current UI and
  persistence contract do not need disruptive rewrites.
- Keep the current conservative duplicate-analysis path or an equivalent
  no-concurrency mode available until Milestone 6 validation shows the new
  strategy is correct and well-bounded on real folders.
- If a batching or concurrency change improves throughput but weakens cache
  validity, cancellation responsiveness, or correctness, revert that change and
  keep the no-regret overhead reductions.
- If path-class-specific scheduling proves ambiguous after measurement, keep the
  conservative policy and record the gap rather than silently applying the SSD
  policy everywhere.
- If cloud-placeholder-safe behavior cannot be guaranteed for a path class,
  keep that class on the conservative path or skip/report it rather than
  treating hydration as an acceptable hidden side effect.

## Outcomes & retrospective

Expected outcome:
- `Space Sift` duplicate analysis becomes fast enough on large real folders to
  feel practical, while still preserving full-hash trust, bounded disk impact,
  and predictable cancellation behavior

Retrospective focus:
- whether cache-path improvements delivered more value than parallel hashing
- whether the chosen worker policy is still appropriate for non-local paths
- whether a later Windows-specific file-open hint or file-ID optimization
  should move higher in priority after the first round of measurement
