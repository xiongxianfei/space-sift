# UI Design Fidelity M2

## Summary

M2 aligns the Overview, Scan, History, and Explorer panels with the approved
design-fidelity contract. The change keeps existing behavior intact and adds
truthful metric cards plus accessible review regions that match the prototype's
panel, card, command, and table hierarchy.

## Scope

- In scope: `R1`-`R3`, `R10`-`R15`, `R25`-`R33`, `E1`-`E5`, and Edge 4-6 for
  Overview, Scan, History, and Explorer.
- Out of scope: Duplicates, Cleanup, Safety panel fidelity, backend commands,
  persistence, event contracts, migration, and final screenshot evidence.

## Diff rationale

| File | Change | Reason |
| --- | --- | --- |
| `src/App.tsx` | Added Overview metric cards for total bytes, total files, duplicate reclaimable bytes, and cleanup candidates. | Prove Overview uses real product state or explicit unavailable states instead of prototype sample metrics. |
| `src/App.tsx` | Added named regions for Scan command/progress, active scan details, completed scan history, interrupted-run continuity, read-only Explorer results, and summary-only compatibility. | Make the M2 panel hierarchy explicit while preserving existing workflow actions and degraded states. |
| `src/App.css` | Added light styling hooks for Overview metrics, metric cards, Scan command grouping, and compatibility note content. | Keep the new M2 surfaces visually consistent with the existing card rhythm without broad restyling. |
| `src/workspace-navigation.test.tsx` | Added M2 tests for Overview metric empty/loaded states and Scan command/progress grouping. | Cover `T4`, `T5`, Edge 4, and behavior-preserving Scan structure. |
| `src/scan-history.test.tsx` | Added a History separation test for completed scans and non-actionable interrupted-run continuity. | Cover `T6` and Edge 6, including `canResume` as actionability source. |
| `src/results-explorer.test.tsx` | Added Explorer region assertions for browseable scans and summary-only compatibility. | Cover `T7` and Edge 5 without replacing approved degraded behavior with sample rows. |
| `docs/plan.md`, `docs/plans/2026-04-24-space-sift-ui-design-fidelity.md` | Recorded M2 completion, decisions, unaffected surfaces, and validation evidence. | Keep the active plan aligned with implementation progress. |

## Validation

- `npm run test -- src/workspace-navigation.test.tsx`: passed, 39 tests.
- `npm run test -- src/scan-history.test.tsx`: passed, 11 tests.
- `npm run test -- src/results-explorer.test.tsx`: passed, 5 tests.
- `npm run lint`: passed.
- `npm run test`: passed, 7 test files and 79 tests.
- `npm run build`: passed.
- `git diff --check -- src\App.tsx src\App.css src\workspace-navigation.test.tsx src\scan-history.test.tsx src\results-explorer.test.tsx`: passed with CRLF conversion warnings only.

## Unaffected Surfaces

- `src-tauri/`: unaffected. M2 is frontend presentation and test coverage only.
- Duplicates, Cleanup, and Safety panel fidelity: unaffected. M3 owns those
  panels.
- Final visual screenshot evidence: unaffected. M4 owns browser or Tauri visual
  review across the required width bands.

## Follow-ups

- Hand M2 to code review before starting M3.
