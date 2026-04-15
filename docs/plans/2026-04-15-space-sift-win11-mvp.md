# Ship The Space Sift Win11 MVP

## Metadata

- Status: active
- Created: 2026-04-15
- Updated: 2026-04-15
- Owner: xiongxianfei / Codex
- Related spec(s):
  - `specs/space-sift-mvp.md`
  - `specs/space-sift-mvp.test.md`
  - `specs/space-sift-scan-history.md`
  - `specs/space-sift-scan-history.test.md`
  - `specs/space-sift-results-explorer.md`
  - `specs/space-sift-results-explorer.test.md`
- Supersedes / Superseded by: none
- Branch / PR: not started
- Last verified commands:
  - `npm run lint`
  - `npm run test`
  - `npm run test -- history`
  - `npm run test -- results`
  - `npm run build`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p scan-core -p app-db`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo check --manifest-path Cargo.toml`
  - `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; npm run tauri dev`

## Purpose / Big picture

Build `Space Sift`, a Windows 11 desktop tool that helps users reclaim disk
space without blind deletion. The MVP should let a user scan a drive or folder,
persist scan history locally, see where space is going, identify duplicate
files, preview safe cleanup candidates, and delete or recycle selected items
with clear confirmations and auditability.

The product goal is to combine the fast space-discovery feel of WinDirStat and
SquirrelDisk with the preview-first cleanup discipline of BleachBit, Czkawka,
and Storage Sense, while staying intentionally narrower and safer for a first
release. The Windows UI should stay unprivileged by default, and any privileged
operation should be isolated behind a small elevated helper rather than running
the entire app as admin.

## Context and orientation

- The GitHub repository `xiongxianfei/space-sift` exists on `main`; on
  2026-04-15, `git ls-remote` resolved `HEAD` to commit
  `ce455fb3d89e8897bd8364afa98078aa3c1d6edb`.
- The repository baseline is a Codex-oriented project template, not a blank
  workspace. Key existing files now present locally include:
  - `AGENTS.md`
  - `README.md`
  - `docs/plan.md`
  - `docs/roadmap.md`
  - `docs/workflows.md`
  - `docs/plans/0000-00-00-example-plan.md`
  - `specs/feature-template.md`
  - `specs/feature-template.test.md`
  - `scripts/ci.sh`
  - `scripts/release-verify.sh`
  - `.github/workflows/ci.yml`
  - `.github/workflows/release.yml`
- `AGENTS.md` is present and establishes the repo order of precedence:
  approved spec, matching test spec, active plan, `docs/workflows.md`, then the
  agent instructions. Its verification section now lists the current baseline
  commands for lint, test, build, cargo check, and Tauri dev.
- `README.md`, `AGENTS.md`, `scripts/ci.sh`, and `scripts/release-verify.sh`
  have now been replaced with repo-specific baseline content for the Milestone 1
  foundation.
- The local workspace is now a git working tree again after restoring the
  remote repository metadata.
- The chat-supplied local machine notes still apply: Android Studio is at
  `D:\Software\Android\Android Studio`, and the Gradle cache lives at
  `C:\Users\xiongxianfei\.gradle\caches`.
- The user supplied these reference products as inspiration: WinDirStat,
  SquirrelDisk, BleachBit, Czkawka, and Windows Storage Sense. Treat them as UX
  and feature references, not as code to copy.
- Recommended implementation baseline for the MVP:
  - Core engine: Rust crates inside `src-tauri/` for scanning, duplicate
    detection, hashing, cleanup rules, recycle/delete operations, and any
    future privileged helper logic.
  - Desktop shell: Tauri 2
  - UI: React + TypeScript
  - Local data: SQLite for scan history and hash/cache data.
  - Cleanup rule definitions: repo-tracked TOML files under `src/config/`.
  - Release pipeline: GitHub Actions, `tauri-action`, and GitHub Releases.
  - Distribution: signed Windows builds with winget submission as the public
    package distribution path.
  - Future extensibility target: structure the scan backend so an NTFS metadata
    fast path can be added later without rewriting the UI contract.
  - Expected repo layout after milestone 1:
    - `package.json`
    - `src/`
    - `src-tauri/Cargo.toml`
    - `src-tauri/crates/scan-core/`
    - `src-tauri/crates/duplicates-core/`
    - `src-tauri/crates/cleanup-core/`
    - `src-tauri/crates/app-db/`
    - `src-tauri/crates/elevation-helper/`
    - `AGENTS.md` with real verification commands
    - `README.md` describing `Space Sift`
    - `docs/workflows.md` updated for the actual release flow
    - `specs/space-sift-mvp.md`
    - `specs/space-sift-mvp.test.md`
    - `src/config/cleanup-rules/`
    - `winget/`
    - `tests/fixtures/`
- The repository still contains template artifacts that milestone 1 should
  delete or replace once real project files exist, especially the placeholder
  spec files and generic workflow text. Milestone 1 has already removed the
  template example plan and template feature specs.

## Constraints

- Windows 11 only for the MVP. Do not spend milestone budget on macOS/Linux
  support.
- All scan, hash, duplicate, cleanup, recycle-bin, and permanent-delete logic
  should live in Rust. The frontend should orchestrate and render; it should
  not own filesystem mutation logic.
- Keep the normal app unprivileged. Do not run the entire desktop app as admin;
  introduce a small elevated helper only for operations that truly require it.
- Default destructive action must be "move to Recycle Bin" where Windows
  supports it. Permanent delete must be a separate, higher-friction path.
- Scanning and duplicate detection must work offline and without admin
  privileges for normal user locations.
- Persist scan history and cache data locally in SQLite. No cloud sync, account
  dependency, or remote telemetry in MVP.
- Duplicate recommendations must use a staged confirmation pipeline:
  size grouping, then partial hash, then full hash before deletion is offered.
- Cleanup rules must be transparent and inspectable. No opaque "magic cleanup"
  scoring, registry cleaning, or driver/package-store surgery in MVP.
- Cleanup definitions should be checked into the repo as human-readable TOML
  rule files so contributors can review and diff rule changes.
- Handle long paths, symlinks, junctions, reparse points, locked files, and
  permission-denied paths without crashing the scan.
- Design the scan pipeline behind a backend abstraction so an NTFS fast path
  can be added later. Implementing direct NTFS metadata scanning is optional
  for MVP, but blocking it with a rigid design is not acceptable.
- Keep privacy simple: no cloud upload, no account dependency, no telemetry by
  default.
- Sign Windows builds before the first public prerelease or release. Do not
  publish unsigned downloadable artifacts as the public distribution path.
- The release process must support GitHub Releases and winget submission.
- Because several reference projects use strong copyleft licenses, do not copy
  their source code, signatures, or cleaner rule sets unless the project
  license for `Space Sift` is intentionally chosen to be compatible.

## Done when

- A fresh Windows 11 development machine can bootstrap the repo and launch the
  desktop app using documented commands.
- A user can select a drive or folder, run a cancellable scan, and see progress
  plus a clear list of skipped or inaccessible paths.
- Scan history is stored locally in SQLite and can be reopened from the UI.
- The app shows largest directories, largest files, and a space-usage
  visualization that supports drill-down.
- The app finds duplicate groups using staged hashing and lets the user preview
  a keep/delete selection before any files are touched.
- The app offers a limited, explicit cleanup preview for safe-rule categories
  such as temp folders and Recycle Bin contents, with a reclaimed-size estimate.
- Normal app startup and the main UI run without admin rights. If privileged
  cleanup is supported, it happens through a narrow elevated helper path.
- All cleanup actions are confirmed, logged, and reversible through Recycle Bin
  when supported; permanent delete is clearly separate and harder to trigger.
- Signed Windows build artifacts are produced through GitHub Actions and
  published through GitHub Releases.
- The winget distribution path is working, evidenced by a validated submission
  or a merged public manifest for the shipped version.
- The repo contains a feature spec, a test spec, automated tests for core scan
  and duplicate logic, and release instructions for the MVP build.

## Milestones

### Milestone 1: Foundation, contracts, and scaffold

Scope: initialize the repo for real work, lock the MVP contract, and create a
bootable Tauri shell before any heavy filesystem logic is added.

Files or components touched:
- `AGENTS.md`
- `README.md`
- `.gitignore`
- `package.json`
- `src/`
- `src-tauri/`
- `src-tauri/Cargo.toml`
- `docs/workflows.md`
- `docs/roadmap.md`
- `specs/space-sift-mvp.md`
- `specs/space-sift-mvp.test.md`
- `specs/feature-template.md`
- `specs/feature-template.test.md`
- `scripts/ci.sh`
- `scripts/release-verify.sh`
- `.github/workflows/ci.yml`
- `.github/workflows/release.yml`
- `tests/fixtures/`

Dependencies:
- Git working tree initialization or a fresh clone of the real repo
- Node.js LTS on Windows
- Rust stable toolchain
- Tauri 2 prerequisites and WebView2 runtime

Risk:
- Toolchain choice churn will create rework if the desktop stack changes after
  scaffolding.

Validation commands:
- `npm install`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run lint`
- `npm run tauri dev`
- `npm run test`

Expected observable result:
- The template repository is converted into a real `Space Sift` baseline: the
  README names the product, AGENTS/workflows/scripts name real commands, the
  placeholder specs are replaced, and a blank but bootable Tauri desktop shell
  opens.

### Milestone 2: Scan engine, backend abstraction, and local data

Scope: implement the recursive scanner, directory size aggregation, progress
events, cancellation, exclusion handling, SQLite-backed scan history, and a
stable result model the UI can consume. Introduce a scan backend abstraction so
the MVP recursive walker can later gain an NTFS fast path without contract
churn.

Files or components touched:
- `src-tauri/crates/scan-core/**`
- `src-tauri/crates/app-db/**`
- `src-tauri/src/commands/scan.rs`
- `src-tauri/src/commands/history.rs`
- `src-tauri/src/state/**`
- `tests/fixtures/scan/**`
- `tests/fixtures/history/**`

Dependencies:
- Milestone 1 scaffold and specs

Risk:
- Large-tree performance, memory growth, reparse-point loops, and early SQLite
  schema churn can break the first real user scans.

Validation commands:
- `cargo test -p scan-core`
- `cargo test -p app-db`
- `cargo test scan_handles_reparse_points`
- `npm run test -- scan`
- `npm run test -- history`

Expected observable result:
- Scanning a sample directory returns stable totals, progress updates, and
  explicit skip reasons for denied or excluded paths, and completed scans are
  stored locally so the UI can reload them later.

### Milestone 3: Explorer UI and scan history experience

Scope: build the results experience for understanding disk usage quickly:
location picker, scan dashboard, scan history list, sortable tables,
breadcrumbs, and a space map.

Files or components touched:
- `src/routes/**`
- `src/components/history/**`
- `src/components/results/**`
- `src/components/treemap/**`
- `src/styles/**`
- `src-tauri/src/commands/shell.rs`

Dependencies:
- Milestone 2 scan result model

Risk:
- Rendering very large result sets can make the UI sluggish or confusing.

Validation commands:
- `npm run lint`
- `npm run test -- results`
- `npm run test -- history`
- `npm run tauri dev`

Expected observable result:
- A user can find the biggest folders and files, drill into a subtree, and open
  the selected path in Windows Explorer, and can revisit recent scans without
  rescanning immediately.

### Milestone 4: Duplicate detection workflow

Scope: add duplicate discovery using staged hashing, result grouping, selection
helpers, SQLite-backed hash caching where helpful, and a preview-first deletion
workflow for duplicate files.

Files or components touched:
- `src-tauri/crates/duplicates-core/**`
- `src-tauri/crates/app-db/**`
- `src-tauri/src/commands/duplicates.rs`
- `src/components/duplicates/**`
- `tests/fixtures/duplicates/**`

Dependencies:
- Milestone 2 scan pipeline
- Milestone 3 UI shell

Risk:
- False positives or long-running hash passes would destroy user trust.

Validation commands:
- `cargo test -p duplicates-core`
- `cargo test -p app-db`
- `cargo test duplicate_requires_full_hash_confirmation`
- `npm run test -- duplicates`

Expected observable result:
- Only fully verified duplicate groups are shown, and the user can preview
  which copies to keep before any deletion flow starts; repeat scans can reuse
  cached metadata safely where the contract allows.

### Milestone 5: Safe cleanup, recycle-bin-first execution, and privilege boundary

Scope: implement a small, inspectable set of cleanup rules and an execution
path with reclaimed-size estimates, confirmation UX, action logging, and a
minimal elevated helper for any privileged operations that cannot run in the
normal unprivileged UI process.

Files or components touched:
- `src-tauri/crates/cleanup-core/**`
- `src-tauri/crates/elevation-helper/**`
- `src-tauri/src/commands/cleanup.rs`
- `src-tauri/src/commands/privileged.rs`
- `src/components/cleanup/**`
- `src/config/cleanup-rules/**`
- `docs/safety.md`

Dependencies:
- Milestone 1 specs
- Milestone 2 scan primitives
- Milestone 3 results UI

Risk:
- Unsafe deletions, UAC/elevation mistakes, and vague rule behavior are the
  biggest product risks in this entire MVP.

Validation commands:
- `cargo test -p cleanup-core`
- `cargo test -p elevation-helper`
- `cargo test cleanup_preview_matches_fixture`
- `npm run test -- cleanup`
- Manual smoke test on a disposable Windows profile or VM snapshot
- Manual protected-path smoke test that verifies the UI stays unprivileged until
  a privileged helper path is explicitly requested

Expected observable result:
- The user sees exactly which files each rule would remove, the estimated space
  reclaimed, and a confirmation flow that defaults to Recycle Bin where
  possible. The main app stays unprivileged, and privileged operations use a
  narrow helper flow instead of elevating the whole UI.

### Milestone 6: Signed release automation, GitHub distribution, and winget ship

Scope: finish packaging, settings persistence, diagnostics, release docs, and
the final acceptance pass needed for the first public MVP build using signed
Windows artifacts, GitHub Releases, and winget distribution.

Files or components touched:
- `src/config/**`
- `src-tauri/tauri.conf.json`
- `src-tauri/icons/**`
- `.github/workflows/**`
- `scripts/ci.sh`
- `scripts/release-verify.sh`
- `docs/release.md`
- `winget/**`
- `README.md`

Dependencies:
- Milestones 1 through 5
- Signing certificate and secret-management setup
- GitHub repository permissions for releases
- Publisher metadata needed for winget submission

Risk:
- Packaging, signing, secret handling, and external winget submission timing can
  make an otherwise functional MVP feel unsafe or incomplete.

Validation commands:
- `bash scripts/ci.sh`
- `bash scripts/release-verify.sh`
- `npm run build`
- `npm run tauri build`
- End-to-end smoke checklist from `specs/space-sift-mvp.test.md`
- Installer smoke test on a clean Windows 11 VM
- Public release candidate smoke test with signed artifacts
- Winget submission evidence: validated submission output or accepted manifest
  PR for the release version

Expected observable result:
- A repeatable signed installer exists, core flows pass from a clean
  environment, GitHub Releases is the publication path, and the winget
  distribution process is proven for the shipped version.

## Progress

- [x] 2026-04-15: inspected the repo and confirmed the local workspace was
  initially missing the real GitHub project contents.
- [x] 2026-04-15: fetched the GitHub repository metadata and verified `main`
  exists remotely.
- [x] 2026-04-15: copied the current GitHub project template files into the
  workspace, excluding `.git`.
- [x] 2026-04-15: created and then revised this concrete execution plan against
  the actual repository template structure.
- [x] 2026-04-15: revised the plan again to lock in the user-selected Rust +
  Tauri 2 + React + SQLite stack, signed release pipeline, winget path, and
  unprivileged-by-default safety model.
- [x] 2026-04-15: restored the workspace as a real git working tree by copying
  the remote repository `.git` metadata into place.
- [x] 2026-04-15: replaced the template specs with `specs/space-sift-mvp.md`
  and `specs/space-sift-mvp.test.md`.
- [x] 2026-04-15: scaffolded the Tauri 2 + React foundation, replaced the demo
  UI with the branded `Space Sift` landing shell, and added frontend tests plus
  lint configuration.
- [x] 2026-04-15: replaced placeholder README, verification commands, CI script,
  and release-readiness script with repo-specific baseline content.
- [x] 2026-04-15: after installing Rust, WebView2, and the Windows SDK/MSVC
  components, `cargo check --manifest-path src-tauri/Cargo.toml` succeeded and
  `npm run tauri -- info` reported a healthy Windows Tauri environment.
- [x] Milestone 1 completed.
- [x] 2026-04-15: added dedicated Milestone 2 scan/history specs and test spec
  before implementation.
- [x] 2026-04-15: implemented the `scan-core` crate with recursive aggregation,
  progress snapshots, cancellation, skipped-path reporting, and reparse-point
  avoidance behind a backend abstraction.
- [x] 2026-04-15: implemented the `app-db` crate with local SQLite history
  persistence and reopen support using summary columns plus JSON payload
  storage.
- [x] 2026-04-15: added Tauri scan/history commands, app state management, and
  a Milestone 2 React UI for starting scans, viewing progress, cancelling
  scans, and reopening local history.
- [x] Milestone 2 completed.
- [x] 2026-04-15: wrote the Milestone 3 results-explorer spec and test spec
  before changing the stored scan result model or UI.
- [x] 2026-04-15: extended the completed scan payload with additive per-entry
  tree data and kept older history JSON reopenable by defaulting missing
  browseable entries to an empty list.
- [x] 2026-04-15: added a Windows Explorer handoff command plus a richer React
  results explorer with breadcrumbs, sortable current-folder contents, a
  current-level space map, and a summary-only fallback for older saved scans.
- [x] Milestone 3 completed.
- [ ] Milestones 4 through 6 not started.

## Surprises & Discoveries

- The remote repository is real, but the checked workspace still lacks `.git`
  after copying files locally; milestone work should not rely on local git
  commands until that is corrected.
- The repository began as a Codex-oriented template, so Milestone 1 had to
  replace placeholder docs, example specs, and generic automation before the
  app scaffold could become trustworthy.
- Because the repo is template-first today, milestone 1 must upgrade docs,
  specs, scripts, and scaffolding together instead of only adding app code.
- The user has now made the architecture decision explicit, so the plan should
  optimize for execution rather than revisit shell/framework selection.
- The default shell session did not pick up the freshly installed Rust toolchain
  automatically, so direct `cargo` commands failed until the environment was
  refreshed.
- Visual Studio Community and the MSVC tools were present, but the Windows SDK
  libraries were missing until the Windows 11 SDK installation completed.
- The shell session still required an explicit `%USERPROFILE%\.cargo\bin`
  prefix for later milestone verification commands even after Rust was
  installed successfully.
- `useEffectEvent` matched the intended React subscription pattern for the
  Tauri progress bridge, but the repository lint rule needed a narrow local
  override because `eslint-plugin-react-hooks` still treated those handlers as
  missing dependencies.
- The additive `entries` field was enough to make Milestone 3 backward
  compatible; older history rows reopened cleanly without a schema migration as
  long as the deserialize path defaulted missing browseable data.
- This shell can run Rust and Tauri successfully once `%USERPROFILE%\.cargo\bin`
  is prefixed into `PATH`, but plain `npm run tauri dev` still fails here when
  that prefix is missing.

## Decision Log

- Decision: treat this as a Windows 11-only MVP.
  Rationale: Windows integration and deletion safety already carry enough risk
  without adding cross-platform abstraction early.
  Date/Author: 2026-04-15 / Codex

- Decision: use Tauri 2 with a React + TypeScript UI and Rust backend crates.
  Rationale: the UI stays fast to iterate, while scan and hash operations live
  in a performant language that can own Windows-specific filesystem behavior.
  Date/Author: 2026-04-15 / Codex

- Decision: store scan history and reusable cache data in local SQLite.
  Rationale: the app needs durable local history and cache reuse without adding
  network dependencies or inventing a custom persistence layer first.
  Date/Author: 2026-04-15 / User + Codex

- Decision: keep cleanup rule definitions as repo-tracked TOML files.
  Rationale: contributors need readable, diffable, human-reviewed rule changes
  for a safety-sensitive cleanup product.
  Date/Author: 2026-04-15 / User + Codex

- Decision: keep the template repository structure and replace placeholders in
  place instead of starting from a separate blank repo.
  Rationale: the GitHub project already has planning, spec, CI, and community
  scaffolding that should be preserved and customized rather than recreated.
  Date/Author: 2026-04-15 / Codex

- Decision: ship signed Windows artifacts through GitHub Actions, Tauri
  release tooling, GitHub Releases, and winget distribution.
  Rationale: desktop trust and installability matter for this product category,
  and unsigned downloads would create avoidable SmartScreen friction.
  Date/Author: 2026-04-15 / User + Codex

- Decision: keep the app unprivileged by default and isolate privileged work in
  a small elevated helper.
  Rationale: this reduces the blast radius of UI bugs and avoids treating every
  scan or cleanup preview as an administrator operation.
  Date/Author: 2026-04-15 / User + Codex

- Decision: design for a future NTFS fast path without making it MVP-critical.
  Rationale: serious Windows disk analyzers benefit from faster filesystem
  metadata access, but forcing that implementation into v1 would slow safer
  user-facing milestones.
  Date/Author: 2026-04-15 / User + Codex

- Decision: keep cleanup scope intentionally narrow for MVP.
  Rationale: a transparent temp-file and Recycle-Bin cleaner is much safer than
  trying to replicate the giant cleaner catalogs of older maintenance suites.
  Date/Author: 2026-04-15 / Codex

- Decision: require full-hash confirmation before duplicate deletion is
  offered.
  Rationale: reclaiming disk space is valuable, but a single false positive
  would permanently damage product trust.
  Date/Author: 2026-04-15 / Codex

- Decision: store the full completed scan payload as JSON in SQLite while also
  indexing summary columns for history views.
  Rationale: Milestone 2 needs exact-result reopen behavior now, and this keeps
  the schema simple while preserving room for additive migrations later.
  Date/Author: 2026-04-15 / Codex

- Decision: use a Tauri progress-event bridge plus a local status command
  instead of polling-only progress updates.
  Rationale: scans should feel live in the desktop shell, while `get_scan_status`
  still gives the UI a clean recovery path on startup or reload.
  Date/Author: 2026-04-15 / Codex

- Decision: extend the completed scan payload with additive per-entry tree data
  instead of replacing the Milestone 2 summary model.
  Rationale: Milestone 3 needs browseable drill-down, but older saved scan
  JSON must remain readable without a destructive migration.
  Date/Author: 2026-04-15 / Codex

- Decision: degrade older saved scans to a summary-only experience when the
  browseable tree data is missing.
  Rationale: this preserves reopen reliability and gives the user a clear
  rescan path instead of breaking history or inventing partial navigation.
  Date/Author: 2026-04-15 / Codex

## Validation and Acceptance

Current document validation from repo root:
- `git ls-remote https://github.com/xiongxianfei/space-sift`
- `Get-Content AGENTS.md`
- `Get-Content docs\\plan.md`
- `Get-Content docs\\workflows.md`
- `Get-ChildItem docs, docs\\plans`
- `Get-Content docs\\plans\\2026-04-15-space-sift-win11-mvp.md`

Planned implementation validation after milestones land:
- `npm install`
- `cargo test -p scan-core`
- `cargo test -p app-db`
- `cargo test -p duplicates-core`
- `cargo test -p cleanup-core`
- `cargo test -p elevation-helper`
- `npm run test`
- `npm run lint`
- `npm run tauri dev`
- `npm run tauri build`
- `bash scripts/ci.sh`
- `bash scripts/release-verify.sh`

Acceptance evidence for the shipped MVP:
- A clean Windows 11 machine can install dependencies, launch the app, run a
  scan, reopen prior scan history, inspect large files, inspect duplicate
  groups, preview safe cleanup, and complete a recycle-bin-first deletion flow
  without undocumented steps.
- Public downloadable artifacts are signed, and the main app does not require
  administrator privileges for normal startup and preview flows.
- The release version has a working GitHub Release artifact set and a validated
  winget distribution path.

## Validation Notes

- 2026-04-15: `git ls-remote https://github.com/xiongxianfei/space-sift`
  confirmed the remote `main` branch exists at
  `ce455fb3d89e8897bd8364afa98078aa3c1d6edb`.
- 2026-04-15: `Get-Content AGENTS.md`, `Get-Content docs\plan.md`, and
  `Get-Content docs\workflows.md` confirmed the repo is a Codex-friendly
  template with placeholder verification steps and no active plan yet.
- 2026-04-15: `Get-ChildItem docs, docs\plans` confirmed the active plan file
  exists in the expected location before the template example plan was removed.
- 2026-04-15: the active plan was revised to use the user-selected stack:
  Rust core engine, Tauri 2 shell, React + TypeScript UI, SQLite local data,
  signed GitHub-based releases, and winget distribution.
- 2026-04-15: `Get-Content docs\plans\2026-04-15-space-sift-win11-mvp.md`
  confirmed the revised milestone sequencing and constraints were written as
  intended.
- 2026-04-15: `npm install`, `npm run lint`, `npm run test`, and `npm run
  build` all passed after the Milestone 1 scaffold landed.
- 2026-04-15: `cargo check --manifest-path src-tauri/Cargo.toml` failed because
  `cargo` is not installed or not on `PATH` on this machine.
- 2026-04-15: `npm run tauri dev` failed for the same reason while trying to
  run `cargo metadata`.
- 2026-04-15: Rust was later confirmed installed at
  `C:\Users\xiongxianfei\.cargo\bin`, but the current shell had not picked up
  that path automatically.
- 2026-04-15: `npm run tauri -- info` reported WebView2, Rust, cargo, and
  rustup as present, but could not detect a Visual Studio or Build Tools
  instance with both MSVC and SDK components.
- 2026-04-15: `cargo check --manifest-path src-tauri/Cargo.toml` still failed
  under the Visual Studio developer environment with `LINK : fatal error
  LNK1181: cannot open input file 'kernel32.lib'`, confirming the Windows SDK
  libraries are still missing.
- 2026-04-15: after installing the Windows 11 SDK, `kernel32.lib` was present
  under `C:\Program Files (x86)\Windows Kits\10\Lib\10.0.22621.0\um\x64`.
- 2026-04-15: `npm run tauri -- info` then reported a healthy environment with
  WebView2, MSVC, rustc, cargo, and rustup all detected.
- 2026-04-15: `cargo check --manifest-path src-tauri/Cargo.toml` completed
  successfully under the Visual Studio developer environment.
- 2026-04-15: `npm run tauri dev` stayed running until the command timeout,
  which is consistent with a dev server continuing normally rather than
  crashing on startup.
- 2026-04-15: `cargo test -p scan-core` passed after the recursive scanner and
  fake-backend tests landed, including the named `scan_handles_reparse_points`
  coverage.
- 2026-04-15: `cargo test -p app-db` passed after the SQLite-backed history
  store was implemented.
- 2026-04-15: `npm install`, `npm run lint`, `npm run test -- scan`,
  `npm run test -- history`, `npm run test`, and `npm run build` all passed
  after the Milestone 2 React UI and Tauri bridge landed.
- 2026-04-15: `cargo check --manifest-path Cargo.toml` passed for the root
  Tauri workspace after the app state, commands, and local crate dependencies
  were wired together.
- 2026-04-15: `npm run test -- results` failed first against the Milestone 2
  summary-only UI, confirming the new browse/sort/Explorer-handoff tests were
  exercising missing Milestone 3 behavior.
- 2026-04-15: after the Milestone 3 implementation landed, `npm run lint`,
  `npm run test -- history`, `npm run test -- results`, `npm run test`, and
  `npm run build` all passed from the repo root.
- 2026-04-15: `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo test -p scan-core -p app-db`
  passed after the additive result-tree model and legacy-history reopen test
  landed.
- 2026-04-15: `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo check --manifest-path Cargo.toml`
  passed for the full Tauri workspace after wiring the Explorer command and
  updated shared scan model.
- 2026-04-15: plain `npm run tauri dev` still failed in this shell because
  `cargo` was not on `PATH`, but `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; npm run tauri dev`
  stayed running until the command timeout, which is consistent with a healthy
  dev server.

## Idempotence and Recovery

- Re-running milestone 1 scaffold commands should be safe only after scripts
  and package manifests are added; until then, update this plan before
  regenerating framework files on top of hand edits.
- Turn this workspace into a real git working tree before milestone 1
  implementation so each milestone can be reviewed and reverted independently.
- Keep SQLite schema changes additive and migration-tested so existing local
  scan history survives iterative development.
- Keep all destructive filesystem actions behind a dry-run preview and explicit
  confirmation path. Do not enable permanent delete until logging and recovery
  are implemented.
- Prefer Recycle Bin for user-triggered deletions so recovery remains possible
  outside the app.
- If signing secrets or certificate setup are unavailable, block public release
  work instead of silently shipping unsigned builds.
- Keep the elevated helper versioned with the main app and fail closed if the
  helper is missing, mismatched, or denied by UAC.
- If the Tauri stack decision changes before coding begins, revise this plan,
  `docs/plan.md`, and the future spec files before generating the new scaffold.

## Outcomes & Retrospective

Milestone 3 results-explorer code now exists. The repo has a recursive Rust
scan engine, SQLite-backed result persistence, additive browseable scan-tree
payloads, a Windows Explorer handoff command, and a React shell that can start
scans, report progress, cancel work, reopen saved history, browse the stored
tree from the root with breadcrumbs and sort controls, and degrade older saved
scans to a readable summary-only mode.

Immediate next step:
- Start Milestone 4 by adding staged duplicate detection, grouping, and a
  preview-first duplicate cleanup workflow on top of the existing scan/history
  model and browseable results shell.

Explicit non-goals for this MVP:
- Registry cleaning
- Driver-store cleanup
- Scheduled/background auto-cleaning
- Network-share scanning
- Cloud-account integration
- Secure erase / shred workflows
- Photo similarity or media transcoding features
- Shipping the NTFS fast path in v1, as opposed to designing clean extension
  points for it
