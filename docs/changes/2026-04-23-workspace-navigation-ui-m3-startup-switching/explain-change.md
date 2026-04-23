# M3 Startup Resolver And Contractual Workspace Switching

## Why this change exists

`M3` exists to make the approved shell navigation rules real. The shell already
had visible tabs and a global status surface after `M2`, but startup routing,
history reopen handoff, scan-completion handoff, duplicate-analysis handoff,
and restore-context failure handling were still manual or implicit.

## What changed

- `App.tsx` now resolves the initial workspace from the approved priority
  order: live scan, live duplicate analysis tied to the loaded scan,
  interrupted runs, then validated Explorer restore context, with `Overview` as
  the safe fallback.
- The shell now persists `lastWorkspace` and `lastOpenedScanId` only at the
  approved write points and shows a shell notice when restore-context reads or
  validation fail.
- Contractual workspace switches now exist for:
  `N1_START_SCAN`, `N2_SCAN_COMPLETED_AND_OPENED`, `N3_OPEN_HISTORY_SCAN`,
  `N4_START_DUPLICATE_ANALYSIS`, `N5_REQUEST_CLEANUP_PREVIEW`, and
  `N6_REVIEW_INTERRUPTED_RUNS`.
- `src/workspaceNavigation.ts` now carries the startup resolver and shell
  navigation-reason helpers so the tab-selection logic is isolated from panel
  rendering.
- `src/workspaceShellLogger.ts` adds local log events for restore-context
  failures, applied auto-switches, skipped duplicate auto-switches, next safe
  action selection, and status-notice rendering.
- The shell now deduplicates replayed terminal events and repeated backend
  snapshots by an operation-aware key so a completed scan or duplicate-analysis
  snapshot does not keep resetting local review state.
- Startup still does not cold-start `Duplicates` or `Cleanup` from prior review
  state alone, but it now hydrates a durably persisted completed
  duplicate-analysis result for the currently loaded scan so cleanup preview
  keeps its duplicate delete candidates.

## Important constraints preserved

- `M3` does not add new backend persistence for duplicate-analysis results or
  cleanup previews.
- Background refreshes still do not steal focus. Only the approved
  contractual-switch reasons move the active workspace automatically.
- The shell action remains navigation-only. It does not execute cleanup,
  permanent delete, or resume directly.

## Verification summary

- `npm run test -- src/workspace-navigation.test.tsx`
- `npm run test -- src/scan-history.test.tsx`
- `npm run lint`
- `npm run test`
- `npm run build`
