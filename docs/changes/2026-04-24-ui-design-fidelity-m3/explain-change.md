# UI Design Fidelity M3

## Summary

M3 aligns the Duplicates, Cleanup, and Safety panels with the approved
design-fidelity contract. The change keeps existing duplicate-selection and
cleanup execution behavior intact while making the review stages explicit,
separated, and testable.

## Scope

- In scope: `R1`-`R3`, `R16`-`R20`, `R25`-`R33`, `S1`-`S5`, and Edge 7-11 for
  Duplicates, Cleanup, and Safety.
- Out of scope: backend commands, persistence, new cleanup rules, bridge
  capability modeling, auto-elevation, and final screenshot evidence.

## Diff rationale

| File | Change | Reason |
| --- | --- | --- |
| `src/App.tsx` | Added named Duplicates regions for analysis controls, status, delete summary, and verified duplicate review. | Cover the prototype-aligned card/review structure while preserving existing analysis, cancellation, keep/delete, and empty-state behavior. |
| `src/App.tsx` | Split Cleanup into named source, preview, issue, Recycle Bin, and advanced permanent-delete stages. | Keep safe cleanup flow visually distinct and make Recycle Bin priority stronger than the gated permanent-delete path. |
| `src/App.tsx` | Expanded Safety guidance with durable local-only, protected-path, permanent-delete, and `can_resume` actionability copy. | Replace prototype-style copy with truthful utility guidance that matches approved safety behavior. |
| `src/App.css` | Added light styling hooks for duplicate status and cleanup funnel stages. | Support the staged visual hierarchy without broad restyling or behavior changes. |
| `src/duplicates.test.tsx` | Added a grouped duplicate-card decision-context test. | Cover `T8`, including summary context and keep/delete review affordances. |
| `src/cleanup.test.tsx` | Added cleanup funnel assertions and protected-path validation issue fixture coverage. | Cover `T9`, `T10`, Edge 7, Edge 8, and Edge 9 around safe and destructive cleanup separation. |
| `src/workspace-navigation.test.tsx` | Added durable Safety guidance assertions. | Cover `T11` and Edge 10 without accepting prototype-only or unstable copy. |
| `docs/plan.md`, `docs/plans/2026-04-24-space-sift-ui-design-fidelity.md` | Recorded M3 completion, decisions, unaffected surfaces, and validation evidence. | Keep the active plan aligned with implementation progress. |

## Validation

- `npm run test -- src/duplicates.test.tsx`: passed, 12 tests.
- `npm run test -- src/cleanup.test.tsx`: passed, 6 tests.
- `npm run test -- src/workspace-navigation.test.tsx`: passed, 40 tests.
- `npm run lint`: passed.
- `npm run test`: passed, 7 test files and 82 tests.
- `npm run build`: passed.
- `git diff --check -- src\App.tsx src\App.css src\duplicates.test.tsx src\cleanup.test.tsx src\workspace-navigation.test.tsx docs\plan.md docs\plans\2026-04-24-space-sift-ui-design-fidelity.md docs\changes\2026-04-24-ui-design-fidelity-m3\change.yaml docs\changes\2026-04-24-ui-design-fidelity-m3\explain-change.md`: passed with CRLF conversion warnings only.

## Unaffected Surfaces

- `src-tauri/`: unaffected. M3 is frontend presentation and test coverage only.
- Overview, Scan, History, and Explorer panel fidelity: unaffected. M2 owns
  those panels.
- Final visual screenshot evidence: unaffected. M4 owns browser or Tauri visual
  review across the required width bands.

## Follow-ups

- Hand M3 to code review before starting M4.
