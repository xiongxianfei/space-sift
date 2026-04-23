# Plan index

This file tracks active, blocked, done, and superseded plans.

## Draft
- none currently

## Active
- [2026-04-16-history-and-duplicate-review-clarity.md](plans/2026-04-16-history-and-duplicate-review-clarity.md)
  - status: active
  - scope: make saved-scan review and duplicate triage clearer by improving
    history readability, active-result context, duplicate-group ordering, and
    visible file-path context for same-name files
- [2026-04-16-scan-progress-and-active-run-ux.md](plans/2026-04-16-scan-progress-and-active-run-ux.md)
  - status: active
  - scope: make large scans feel faster and clearer by improving scan telemetry,
    reducing progress-event overhead, and separating active scans from prior
    completed results in the UI
- [2026-04-15-space-sift-win11-mvp.md](plans/2026-04-15-space-sift-win11-mvp.md)
  - status: active
  - scope: turn the repository template into the first approved `Space Sift`
    MVP using a Rust core engine, Tauri 2 + React UI, SQLite scan history,
    signed Windows releases, and winget distribution
## Blocked
- none currently

## Done
- [2026-04-22-space-sift-workspace-navigation-ui.md](plans/2026-04-22-space-sift-workspace-navigation-ui.md)
  - status: done
  - scope: introduced the workspace shell, startup restoration, contractual auto-switches, and shell-level global status for the advanced UI initiative without silently replacing active UI contracts
- [2026-04-18-scan-run-continuity.md](plans/2026-04-18-scan-run-continuity.md)
  - status: done
  - scope: added additive SQLite-backed scan run continuity, restart recovery,
    non-live run actions, and gated resume without replacing completed scan
    history, with current resume execution explicitly deferred until engine
    cursor support exists
- [2026-04-16-fast-safe-duplicate-analysis.md](plans/2026-04-16-fast-safe-duplicate-analysis.md)
  - status: done
  - scope: defined and implemented a fast, full-hash-correct,
    disk-friendly duplicate-analysis architecture with measurement,
    explicit cache/writeback strategy, cloud/remote safety rules,
    bounded hashing concurrency, and recorded real-folder validation
- [2026-04-16-fast-safe-scan-architecture.md](plans/2026-04-16-fast-safe-scan-architecture.md)
  - status: done
  - scope: defined and implemented a metadata-first, disk-friendly scan
    architecture with a Windows fixed-volume backend, explicit fallback matrix,
    bounded scheduling, and recorded large-folder validation
- [2026-04-16-results-explorer-inline-usage.md](plans/2026-04-16-results-explorer-inline-usage.md)
  - status: done
  - scope: replaced the split results explorer + space-map layout with a
    unified current-folder table that shows inline relative-usage cues

## Superseded
- none yet
