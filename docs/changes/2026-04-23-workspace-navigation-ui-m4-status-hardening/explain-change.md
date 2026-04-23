# M4 Global Status Hardening And Observability

## Why this change exists

`M4` exists to harden the shell contract that became visible in `M2`. The app
already had a global status surface and one next safe action, but cleanup,
continuity, and cross-workflow edges still needed proof that the shell stayed
scan-aware, non-destructive, and observable when users moved across workflows.

## What changed

- `App.tsx` now keeps cleanup execution shell state together with the
  originating `scanId` instead of storing only the execution result.
- `src/workspaceNavigation.ts` now ignores cleanup-completion status unless the
  stored cleanup execution state still belongs to the currently loaded scan.
- The shell now derives explicit notice entries for:
  - active scan presence
  - active duplicate-analysis presence for the loaded scan
  - interrupted runs that need review
- The shell now logs:
  - `workspace_next_safe_action_selected`
  - `workspace_status_notice_rendered`
- `src/workspace-navigation.test.tsx` adds shell-level regressions for:
  - ignoring cleanup state for a different loaded scan
  - interrupted-run notice visibility without focus steal
  - live-task notice visibility from non-Overview workspaces
  - shell logging for next safe action and notice rendering
- `src/cleanup.test.tsx` now proves that reopening a different stored scan
  clears cleanup-specific shell status and removes the stale `Start a fresh
  scan` action.
- `src/duplicates.test.tsx` now proves that duplicate-analysis live-task
  notices stay visible while another workspace remains selected and focused.

## Important constraints preserved

- The shell still never executes cleanup, permanent delete, or resume directly.
- Cleanup preview and cleanup execution remain session-scoped frontend state in
  this slice; `M4` only hardens how that state is surfaced in the shell.
- No backend API, persistence, or `src-tauri/` contract changed in `M4`.

## Verification summary

- `npm run test -- src/workspace-navigation.test.tsx`
- `npm run test -- src/cleanup.test.tsx`
- `npm run test -- src/duplicates.test.tsx`
- `npm run lint`
- `npm run test`
- `npm run build`
