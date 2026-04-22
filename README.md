# Space Sift

`Space Sift` is a Windows 11 desktop tool for finding large files, reviewing
duplicate-file cleanup candidates, and reclaiming disk space without blind
deletion.

The checked-in app can start a recursive scan, report progress, store completed results in local SQLite
history, reopen prior scans from the UI, browse stored results with
breadcrumbs, sorting, and Explorer handoff, run duplicate analysis with
keep-selection helpers, and build or execute a safe cleanup preview with a
Recycle-Bin-first default.

## Stack

- Core engine: Rust
- Desktop shell: Tauri 2
- UI: React + TypeScript
- Local data: SQLite
- Cleanup rules: TOML
- Release path: GitHub Actions, GitHub Releases, signed Windows builds, winget

## Safety model

- The normal app UI stays unprivileged by default.
- Recycle Bin first is the default delete strategy.
- Permanent deletion exists as a separate, higher-friction workflow.
- Protected Windows paths stay outside the normal unprivileged cleanup path.

## Current status

Implemented in this repository today:
- a branded Windows desktop shell
- a recursive Rust scan engine with reparse-point avoidance and cancellation
- SQLite-backed scan history with reopen support
- additive per-entry scan results for browseable history reopen
- SQLite-backed duplicate hash caching with metadata validation
- SQLite-backed cleanup execution logging
- Tauri commands for scan progress, cancellation, history loading, duplicate analysis, cleanup preview/execution, protected-path capability reporting, and Explorer handoff
- a results explorer with folder drill-down, sortable current-folder tables, and inline relative-usage cues
- a duplicate-analysis workflow that shows only fully verified groups, excluded-path issues, and keep/delete previews
- a cleanup workflow that combines duplicate delete candidates with a small built-in TOML rule catalog, previews exact file actions, defaults to Recycle Bin execution, and gates permanent delete behind an explicit confirmation toggle
- frontend tests for the scan, history, results-explorer, duplicate-analysis, and cleanup flows
- maintainer scripts for lint, test, build, and release-readiness checks
- a generated release-only Tauri config path so normal local builds do not require updater keys
- an execution plan plus feature/test specs for the foundation, scan/history, results-explorer, duplicate-analysis, and cleanup milestones
- a tag-driven GitHub release workflow with signing gates, updater artifacts, and versioned winget manifests for the current app version

Not implemented yet:
- NTFS fast-path scanning
- a live signed public release and published winget hashes for a real tag

## Quick start

Prerequisites:
- Node.js 22 or newer
- Rust toolchain available on `PATH`
- Tauri Windows prerequisites, including WebView2

Commands:

```bash
npm install
npm run lint
npm run test
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

## Repository workflow

This repository follows:

`plan -> spec -> test-spec -> implement -> verify -> docs -> review`

The active execution plans are indexed in:
- `docs/plan.md`

The current feature contract is:
- `specs/space-sift-mvp.md`
- `specs/space-sift-mvp.test.md`
- `specs/space-sift-scan-history.md`
- `specs/space-sift-scan-history.test.md`
- `specs/space-sift-results-explorer.md`
- `specs/space-sift-results-explorer.test.md`
- `specs/space-sift-duplicates.md`
- `specs/space-sift-duplicates.test.md`
- `specs/space-sift-cleanup.md`
- `specs/space-sift-cleanup.test.md`
- `specs/space-sift-release.md`
- `specs/space-sift-release.test.md`

Maintainer docs:
- `docs/release.md`
- `docs/duplicate-performance.md`
- `docs/scan-performance.md`
