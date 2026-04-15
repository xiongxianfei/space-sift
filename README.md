# Space Sift

`Space Sift` is a Windows 11 desktop tool for finding large files, reviewing
duplicate-file cleanup candidates, and reclaiming disk space without blind
deletion.

The repository is currently in Milestone 3. The checked-in app can start a
recursive scan, report progress, store completed results in local SQLite
history, reopen prior scans from the UI, and browse stored results with
breadcrumbs, sorting, and Explorer handoff.

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
- Permanent deletion is a later, higher-friction workflow.

## Current status

Implemented in this repository today:
- a branded Windows desktop shell
- a recursive Rust scan engine with reparse-point avoidance and cancellation
- SQLite-backed scan history with reopen support
- additive per-entry scan results for browseable history reopen
- Tauri commands for scan progress, cancellation, history loading, and Explorer handoff
- a results explorer with folder drill-down, sortable current-folder tables, and a current-level space map
- frontend tests for the scan, history, and results-explorer flows
- maintainer scripts for lint, test, build, and release-readiness checks
- an execution plan plus feature/test specs for the foundation, scan/history, and results-explorer milestones

Not implemented yet:
- duplicate detection
- cleanup execution
- NTFS fast-path scanning

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

The active execution plan is:
- `docs/plans/2026-04-15-space-sift-win11-mvp.md`

The current feature contract is:
- `specs/space-sift-mvp.md`
- `specs/space-sift-mvp.test.md`
- `specs/space-sift-scan-history.md`
- `specs/space-sift-scan-history.test.md`
- `specs/space-sift-results-explorer.md`
- `specs/space-sift-results-explorer.test.md`
