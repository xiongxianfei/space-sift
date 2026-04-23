# UI Design Fidelity M1

## Summary

M1 implements the shell foundation for the UI design-fidelity initiative. The
app now follows the uploaded prototype's first-level structure: persistent
topbar, left workspace rail, right active workspace content area, and a
responsive foundation at `1050px` and `640px`.

The change is intentionally limited to shell layout and navigation foundation.
Panel-specific fidelity for Overview, Scan, History, Explorer, Duplicates,
Cleanup, and Safety remains in later milestones.

## Scope

- In scope: `R1`-`R9`, `R21`-`R29`, `A1`-`A7`, `P1`-`P3`, and Edge 1-3 for
  the shell foundation.
- Out of scope: backend commands, persistence, scan behavior, duplicate
  analysis behavior, cleanup execution behavior, and panel-specific visual
  redesign.

## Diff rationale

| File | Change | Reason |
| --- | --- | --- |
| `src/App.tsx` | Replaced the hero/workspace-card shell with a topbar, workspace navigation rail, active content region, and vertical tab orientation. Added ArrowUp/ArrowDown tab focus movement. | Match the prototype shell structure while preserving the approved workspace-navigation behavior and accessibility contract. |
| `src/App.css` | Added topbar, left rail, workspace layout, active workspace region, and responsive rules at `1050px` and `640px`. | Establish the desktop-first responsive foundation required before panel-specific fidelity work. |
| `src/workspace-navigation.test.tsx` | Added tests for persistent header, left rail, active content containment, and breakpoint stylesheet contract. | Prove the M1 shell structure and responsive baseline are intentional. |
| `specs/space-sift-ui-design-fidelity.test.md` | Moved the test spec to `active` after the explicit `$implement` request. | The test spec allowed implementation after explicit activation. |
| `docs/plan.md`, `docs/plans/2026-04-24-space-sift-ui-design-fidelity.md` | Moved the initiative to active, recorded M1 completion, validation results, and unaffected surfaces. | Keep the active plan aligned with implementation progress. |

## Validation

- `npm run test -- src/workspace-navigation.test.tsx`: passed.
- `npm run lint`: passed.
- `npm run test`: passed.
- `npm run build`: passed.
- `git diff --check -- src\App.tsx src\App.css src\workspace-navigation.test.tsx`: passed with CRLF conversion warnings only.

## Follow-ups

- Continue with M2 only after M1 validation, commit, and code review.
