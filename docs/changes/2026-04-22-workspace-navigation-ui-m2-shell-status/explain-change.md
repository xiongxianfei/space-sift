# M2 Workspace Shell And Global Status

## Why this change exists

`M2` exists to introduce the first-pass workspace shell and make the shell-level
status contract visible and testable without changing the backend scan,
duplicate, cleanup, or persistence contracts. The approved spec requires one
active workspace at a time, accessible manual navigation, and a deterministic
global status surface that is visible from every workspace.

## What changed

- `App.tsx` now renders a workspace tablist and exactly one visible tabpanel for
  `Overview`, `Scan`, `History`, `Explorer`, `Duplicates`, `Cleanup`, and
  `Safety`.
- The shell now derives a `GlobalStatusModel` from the currently loaded scan,
  live scan state, live duplicate-analysis state, interrupted runs, cleanup
  preview state, and cleanup execution state.
- `src/workspaceNavigation.ts` centralizes the workspace definitions and
  deterministic next-safe-action priority so the shell logic is isolated from
  the feature panels.
- `App.css` now carries the first-pass shell, tab, panel, and global-status
  presentation.
- `src/workspace-navigation.test.tsx` adds focused shell-contract coverage for
  tab semantics, keyboard behavior, visible global status, next-safe-action
  priority, and non-destructive shell actions.
- Existing workflow suites now activate the relevant workspace explicitly before
  asserting feature-specific UI so they keep testing the product contract after
  the shell split.

## Important constraints preserved

- `M2` does not implement the later contractual auto-switch rules from `M3`.
  Explicit history reopen and scan-completion flows still require manual
  `Explorer` navigation in this milestone.
- The shell-level next safe action only navigates or focuses the relevant
  workspace. It never executes cleanup, permanent delete, or resume directly.
- Existing scan, history, explorer, duplicate-analysis, and cleanup behavior
  stays inside the current app shell instead of introducing a router or a new
  global state library.

## Verification summary

- `npm run test -- src/workspace-navigation.test.tsx`
- `npm run lint`
- `npm run test`
- `npm run build`
