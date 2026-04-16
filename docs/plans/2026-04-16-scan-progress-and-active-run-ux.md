# Improve Large-Scan Progress, Performance, And Active Result UX

## Metadata

- Status: active
- Created: 2026-04-16
- Updated: 2026-04-16
- Owner: xiongxianfei / Codex
- Related spec(s):
  - `specs/space-sift-scan-history.md`
  - `specs/space-sift-scan-history.test.md`
- Related plan(s):
  - `docs/plans/2026-04-15-space-sift-win11-mvp.md`
- Supersedes / Superseded by: none
- Branch / PR: none yet
- Last reviewed files:
  - `src/App.tsx`
  - `src/App.css`
  - `src/lib/spaceSiftTypes.ts`
  - `src/scan-history.test.tsx`
  - `src-tauri/src/commands/scan.rs`
  - `src-tauri/src/state/mod.rs`
  - `src-tauri/crates/scan-core/src/lib.rs`
  - `specs/space-sift-scan-history.md`
  - `specs/space-sift-scan-history.test.md`

## Purpose / Big picture

Make large scans feel trustworthy and alive. When the user starts scanning a
big folder, the app should immediately switch into a clear active-scan mode,
report useful progress without pretending to know an exact percent-complete,
and stop making the previous completed scan feel like the current result.

This initiative also targets low-risk scan-engine overhead that is likely
amplifying the problem on large trees, especially progress-event spam between
Rust, Tauri, and the React UI.

## Context and orientation

- The Milestone 2 scan contract already requires progress updates, cancellation,
  local history storage, and history reopen support through
  `specs/space-sift-scan-history.md`.
- The current scan status payload in `src/lib/spaceSiftTypes.ts` includes:
  - `scanId`
  - `rootPath`
  - `state`
  - `filesDiscovered`
  - `directoriesDiscovered`
  - `bytesProcessed`
  - `message`
  - `completedScanId`
- The current scan UI in `src/App.tsx` renders those fields as a small status
  strip plus notice text. It does not have a dedicated in-progress results
  surface with elapsed time, current path, or a strong visual distinction
  between an active scan and a previously completed one.
- `handleStartScan()` already clears `currentScan` before requesting a new scan,
  so the user report that the "last scan results" still appear is not only a
  stale-state bug. It is also a product-clarity problem: the page still leaves
  prior history and result-oriented language on screen while the only active
  scan feedback is a weak counter strip.
- The Rust scan engine in `src-tauri/crates/scan-core/src/lib.rs` currently
  emits progress on every discovered directory and every discovered file. On a
  large tree, that means a Tauri event, serialization work, and React state
  churn for every filesystem item.
- The same scan engine currently keeps full `files` and `directories` ranking
  vectors and sorts them later, even though the public result contract only
  needs the top-N largest items plus the full additive `entries` tree. That is
  a plausible source of extra work on large scans.
- The Tauri scan command flow already does one good thing that must be
  preserved: it only auto-opens a completed scan after history persistence has
  succeeded, so a "completed" UI does not race ahead of saved local data.
- This is a focused follow-up initiative to the active MVP plan, not a
  replacement for it.

## Constraints

- Keep scan and history behavior read-only and unprivileged.
- Do not invent a fake 0-100% progress percentage for recursive scans when the
  total tree size is unknown at scan start.
- Preserve the existing completed scan result contract and history reopen
  behavior unless a spec update explicitly changes the observable contract.
- Keep cancellation prompt and reliable; progress throttling must never suppress
  terminal `cancelled`, `failed`, or `completed` snapshots.
- Keep old history entries readable and keep the UI local-only.
- Do not scope-creep this initiative into NTFS MFT scanning, background
  indexing, or duplicate/cleanup behavior.
- Prefer small, reviewable PRs: contract first, backend telemetry/perf second,
  frontend UX third, real large-tree validation last.

## Done when

- Starting a scan immediately puts the app into a clearly labeled active-scan
  mode instead of leaving the user to infer progress from a small status strip.
- While a scan is running, the UI shows useful live context such as:
  - scan root
  - lifecycle state
  - elapsed time or last-update heartbeat
  - live item/byte counters
  - the most recent active path or directory being processed
- The active-scan experience does not frame an older completed result as the
  current result while a new scan is running.
- Progress updates for large trees are emitted at a bounded cadence rather than
  once per filesystem item, while still feeling live to the user.
- Any low-risk scan-engine optimization adopted here keeps the existing scan
  result contract intact and does not regress ranking correctness, cancellation,
  skipped-path reporting, or history persistence.
- Completed scans still auto-load after persistence succeeds, and cancelled or
  failed scans do not masquerade as reusable results.
- Automated coverage is added or updated for the richer progress model and the
  active-scan UI state.
- Manual validation on a real large folder confirms the UI no longer feels
  frozen or misleading during a long scan.

## Non-goals

- Shipping an NTFS direct-MFT fast path in this initiative
- Background indexing, watcher services, or scheduled scans
- Multi-scan concurrency
- Partial or incremental explorer results before the scan completes
- Duplicate-analysis or cleanup-preview changes
- A fake exact percent-complete bar for unknown total work

## Milestones

### Milestone 1: Re-spec the active scan contract

Scope: revise the scan/history contract so it explicitly covers honest progress
semantics for long scans, dedicated active-scan UI expectations, and the rule
that a running scan must not be confused with a previously completed result.

Files or components touched:
- `specs/space-sift-scan-history.md`
- `specs/space-sift-scan-history.test.md`

Dependencies:
- none beyond this approved plan

Risk:
- writing a spec that accidentally promises an exact percentage or a complex
  ETA model the scanner cannot know safely

Validation commands:
- `Get-Content specs/space-sift-scan-history.md`
- `Get-Content specs/space-sift-scan-history.test.md`
- `rg -n "progress|running|current result|history" specs/space-sift-scan-history.md specs/space-sift-scan-history.test.md`

Expected observable result:
- the contract now requires a dedicated active-scan experience with honest
  progress telemetry and explicit separation from older completed results

### Milestone 2: Reduce backend progress overhead and enrich telemetry

Scope: change the scan-core and Tauri scan pipeline so large scans emit richer
progress snapshots at a bounded cadence, and implement low-risk scan-engine
optimizations that preserve the existing result contract.

Files or components touched:
- `src-tauri/crates/scan-core/src/lib.rs`
- `src-tauri/src/commands/scan.rs`
- `src-tauri/src/state/mod.rs`
- `src/lib/spaceSiftTypes.ts`
- `src/lib/tauriSpaceSiftClient.ts`

Dependencies:
- Milestone 1 contract updates

Risk:
- progress throttling can make the UI feel dead if the cadence is too sparse,
  and scan-engine optimizations can silently break top-item ranking

Validation commands:
- `cargo test -p scan-core`
- `cargo check --manifest-path src-tauri/Cargo.toml`

Expected observable result:
- long scans no longer emit a progress event per file and directory, terminal
  state transitions stay reliable, and the UI has enough telemetry to show a
  live running scan without a fake percentage

### Milestone 3: Build a dedicated active-scan UI and isolate stale results

Scope: update the React app so starting a scan moves the user into an obvious
running state with richer live feedback, while prior completed results are
hidden or clearly demoted out of the "current result" role until the new scan
completes.

Files or components touched:
- `src/App.tsx`
- `src/App.css`
- `src/scan-history.test.tsx`
- `src/App.test.tsx`

Dependencies:
- Milestone 2 telemetry fields

Risk:
- the UI can become visually noisier or accidentally remove useful access to
  recent history while a scan is in flight

Validation commands:
- `npm run test -- scan`
- `npm run test -- history`
- `npm run lint`
- `npm run build`

Expected observable result:
- after clicking `Start scan`, the page clearly shows an active scan session
  with live context, and the older completed result no longer reads as the
  current output of the running scan

### Milestone 4: Validate against a real large tree and tune the defaults

Scope: run the updated experience against a genuinely large folder, tune the
progress cadence and copy based on real behavior, and capture any final plan
discoveries before closing the initiative.

Files or components touched:
- `docs/plans/2026-04-16-scan-progress-and-active-run-ux.md`
- any small follow-up files identified by validation fallout

Dependencies:
- Milestones 2 and 3

Risk:
- synthetic tests can pass while the real user experience still feels frozen on
  a deep or wide directory tree

Validation commands:
- `npm run test`
- `npm run lint`
- `npm run build`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; cargo check --manifest-path src-tauri/Cargo.toml`
- `$env:PATH="$env:USERPROFILE\\.cargo\\bin;$env:PATH"; npm run tauri dev`
- manual Windows 11 smoke scan on a large real folder

Expected observable result:
- a reviewer can run a large scan and consistently see active, understandable
  progress from start to completion without mistaking an older result for the
  current run

## Progress

- [x] 2026-04-16: reviewed the existing scan/history spec and test spec before
  drafting this follow-up initiative.
- [x] 2026-04-16: inspected `src/App.tsx`, `src/App.css`,
  `src/lib/spaceSiftTypes.ts`, `src/scan-history.test.tsx`,
  `src-tauri/src/commands/scan.rs`, `src-tauri/src/state/mod.rs`, and
  `src-tauri/crates/scan-core/src/lib.rs` in response to a user report that
  large scans feel too slow, do not show enough progress, and appear to keep
  the previous result on screen.
- [x] 2026-04-16: Milestone 1 completed by revising the scan/history spec and
  test spec to require a dedicated active-scan experience, honest indeterminate
  long-scan progress, additive progress telemetry, and explicit separation
  between a running scan and any older completed result.
- [x] 2026-04-16: Milestone 2 completed by adding additive scan telemetry
  (`startedAt`, `updatedAt`, `currentPath`), bounding intermediate progress
  emission in `scan-core`, preserving the latest counters in cancelled and
  failed terminal snapshots, and limiting largest-item ranking storage to the
  top-N paths instead of every discovered file and directory.
- [x] 2026-04-16: Milestone 3 completed by adding a dedicated `Active scan`
  panel in `src/App.tsx`, showing root/path/heartbeat/counter context from the
  additive telemetry, and making the completed `Current result` section render
  only when the scan is no longer running.

## Surprises & discoveries

- 2026-04-16: the user-visible "last result still showing" problem is partly a
  product-framing issue, not just a stale `currentScan` value.
  `handleStartScan` already clears `currentScan`, but the running-scan
  experience is still too weak and too close to the history/result layout to
  feel unambiguous.
- 2026-04-16: the current scan engine emits progress on every discovered file
  and directory, which is a likely contributor to large-tree slowness because
  every snapshot crosses the Tauri bridge and triggers React state work.
- 2026-04-16: the current scan status payload lacks elapsed-time and active-path
  information, so the UI cannot show much more than counters even when the scan
  is alive and making progress.
- 2026-04-16: `largestFiles` and `largestDirectories` are currently built by
  collecting all ranked paths and sorting later, which duplicates work and
  memory beyond the additive `entries` tree the explorer already needs.
- 2026-04-16: the first regression test for a 200-file fixture captured 202
  intermediate progress snapshots, which confirmed that the existing scanner
  was effectively emitting one progress event per filesystem item.
- 2026-04-16: cancelled and failed terminal snapshots were rebuilding their
  counters from scratch in `scan.rs`, which meant the UI could lose the latest
  discovered-item and byte counts at the moment the scan stopped.
- 2026-04-16: once the dedicated running panel became the primary surface, the
  history tests had to model terminal `cancelled` snapshots explicitly before
  expecting a reopened stored result to retake the page. That clarified the
  intended contract: reopening local history during an active run does not
  replace the active-scan surface until the run actually stops.
- 2026-04-16: a very fast scan can finish and emit its terminal snapshot before
  the awaited `startScan()` call returns to the React handler. Any optimistic
  post-start UI update must preserve newer event-driven state for the same
  `scanId` instead of blindly resetting the page back to `running`.

## Decision log

- 2026-04-16: create a new follow-up plan instead of silently editing the MVP
  plan body.
  Rationale: this work spans the existing scan spec, the Rust progress model,
  and the React scan experience, and it deserves its own reviewable execution
  trail.

- 2026-04-16: prefer honest indeterminate progress plus richer telemetry over a
  fake percentage or speculative ETA.
  Rationale: the scanner does not know the total recursive workload up front,
  and deceptive progress bars are worse than explicit "still scanning" signals.

- 2026-04-16: prioritize low-risk progress-overhead reductions before
  considering bigger scanner rewrites such as parallel traversal or an NTFS
  fast path.
  Rationale: the current per-item progress emission pattern is an obvious
  bottleneck candidate and can be improved without changing the storage or UI
  contracts drastically.

- 2026-04-16: treat "current result" as a strict completed-scan concept.
  Rationale: while a scan is running, the UI should present an active-scan
  session, not a stale completed result with weak counters nearby.

- 2026-04-16: keep partial or incomplete result browsing out of scope for this
  initiative.
  Rationale: mixing incomplete explorer data into the same work would enlarge
  the risk surface substantially and blur what "completed scan history" means.

- 2026-04-16: use deterministic item and byte thresholds for intermediate scan
  progress emission.
  Rationale: bounded per-item telemetry was the immediate bottleneck, and
  threshold-based emission is much simpler to test and reason about than
  wall-clock-only throttling for this milestone.

- 2026-04-16: preserve the latest running counters and path context when
  building cancelled and failed terminal snapshots.
  Rationale: once intermediate progress is rate-limited, terminal snapshots must
  carry forward the last known running values instead of regressing to zeroed
  counters.

- 2026-04-16: keep `largestFiles` and `largestDirectories` bounded to the
  request limit during the scan rather than storing every ranked path.
  Rationale: the explorer already retains the full additive `entries` tree, so
  keeping only top-N ranked summaries reduces scan work and memory without
  changing the result contract.

- 2026-04-16: keep the active-scan panel as the primary content whenever
  `scanStatus.state === "running"`, even if `currentScan` still contains an
  older reopened result.
  Rationale: the stale-result problem is primarily a framing issue, so the
  running scan must win that top-level rendering decision until a terminal
  snapshot arrives.

## Validation and acceptance

Planning validation performed while creating this plan:
- `Get-Content AGENTS.md`
- `Get-Content docs/workflows.md`
- `Get-Content docs/plan.md`
- `Get-Content .codex/PLANS.md`
- `Get-Content specs/space-sift-scan-history.md`
- `Get-Content specs/space-sift-scan-history.test.md`
- `Get-Content src/App.tsx`
- `Get-Content src/App.css`
- `Get-Content src/lib/spaceSiftTypes.ts`
- `Get-Content src/scan-history.test.tsx`
- `Get-Content src-tauri/src/commands/scan.rs`
- `Get-Content src-tauri/src/state/mod.rs`
- `Get-Content src-tauri/crates/scan-core/src/lib.rs`
- `rg -n "progress|scan state|running|history|cancel" src/App.tsx src/App.css src/lib/spaceSiftTypes.ts src/scan-history.test.tsx src-tauri/src/commands/scan.rs src-tauri/src/state/mod.rs src-tauri/crates/scan-core/src/lib.rs`

Acceptance evidence for the implemented change:
- a user starting a new scan immediately sees a dedicated running-scan surface
  rather than a weak status strip next to history-oriented UI
- progress continues to update during a large scan without flooding the app
  with one event per filesystem item
- the UI shows a meaningful heartbeat and live context even when total work is
  unknown
- an older completed scan is not framed as the current result while the new
  scan is running
- completed, cancelled, and failed transitions stay correct, and history reopen
  still behaves as before

## Validation notes

- 2026-04-16: planning-only turn. No implementation, lint, build, or cargo
  commands were run for this plan creation itself.
- 2026-04-16: the existing scan/history contract already required progress, but
  it did not define a dedicated active-scan UX or an explicit stale-result
  policy for long scans.
- 2026-04-16: the current scan engine implementation confirmed that progress is
  emitted on every directory and file, making backend-to-frontend telemetry
  overhead a first-class suspect for large-scan slowness.
- 2026-04-16: `specs/space-sift-scan-history.md` was updated to define the
  long-scan progress model, active-scan UI contract, indeterminate progress
  requirement, and stale-result separation rule.
- 2026-04-16: `specs/space-sift-scan-history.test.md` was updated to add
  concrete coverage for bounded progress emission, active-scan mode, and the
  transition from running scan to persisted completed result.
- 2026-04-16: this Milestone 1 step was doc-only. No runtime validation
  commands were run yet.
- 2026-04-16: `cargo test -p scan-core long_scan_progress_is_bounded` failed
  first with `expected bounded progress snapshots, got 202`, confirming the
  existing scanner still emitted one intermediate snapshot per filesystem item.
- 2026-04-16: after the Milestone 2 implementation landed, `cargo test -p
  scan-core long_scan_progress_is_bounded`,
  `cargo test -p scan-core scan_reports_cancellation_without_completed_result`,
  and `cargo test -p scan-core` all passed.
- 2026-04-16: targeted command-layer tests
  `cargo test --manifest-path src-tauri/Cargo.toml terminal_snapshot_preserves_latest_progress_context`
  and
  `cargo test --manifest-path src-tauri/Cargo.toml completed_snapshot_uses_completed_scan_totals`
  passed after the Tauri scan command started preserving the last running
  counters and timestamps in terminal snapshots.
- 2026-04-16: after wiring the additive telemetry through the Tauri command
  layer, `cargo check --manifest-path src-tauri/Cargo.toml` passed from the
  repo root.
- 2026-04-16: because shared TypeScript types changed, `npm run test -- history`,
  `npm run test`, `npm run lint`, and `npm run build` were also run and passed.
- 2026-04-16: the first Milestone 3 frontend run failed because the new
  `scan-history` regressions could not find an `Active scan` heading; the app
  still rendered only the old status strip and empty-state/current-result
  split.
- 2026-04-16: after the dedicated active-scan panel and stale-result
  suppression landed in `src/App.tsx`/`src/App.css`, `npm run test -- scan`,
  `npm run test -- history`, `npm run test`, `npm run lint`, and
  `npm run build` all passed.
- 2026-04-16: a later regression in the active-scan start flow let
  `handleStartScan()` overwrite a just-completed scan back to `running` if the
  backend finished before `startScan()` resolved. The targeted regression test
  `does not overwrite a fast completed scan back to running after start
  resolves` was added to `src/scan-history.test.tsx`, and `npm run test --
  src/scan-history.test.tsx`, `npm run test -- src/App.test.tsx`,
  `npm run lint`, and `npm run build` passed after preserving fresher
  same-`scanId` snapshots in `src/App.tsx`.

## Idempotence and recovery

- Land this as small PRs. If the backend telemetry change proves riskier than
  expected, ship the active-scan UI separation first with the current counters,
  then iterate on throttling in a follow-up PR.
- Keep any new progress fields additive where possible so the frontend can roll
  forward without breaking older snapshots during development.
- Ensure throttling logic always emits start, completion, cancellation, and
  failure snapshots even when intermediate progress is suppressed.
- If bounded top-item ranking turns out to be error-prone, revert that part and
  keep the cadence and throttling work; the user-facing scan-clarity
  improvement is still valuable on its own.
- If the active-scan UI becomes too disruptive, keep prior scan history
  available in `Recent scans` and only defer the completed-result section until
  the running scan finishes.

## Outcomes & retrospective

Expected outcome:
- large scans feel visibly active, the UI stops implying that yesterday's or
  the last completed result is the output of the current run, and the scan
  engine wastes less time shuttling per-item progress events across the desktop
  bridge

Retrospective focus:
- whether the chosen progress cadence feels alive without becoming noisy
- whether a recent-path display materially improves user trust on big folders
- whether low-risk scan-core optimizations are enough for v1 or whether the
  post-MVP NTFS fast-path work should be pulled forward
