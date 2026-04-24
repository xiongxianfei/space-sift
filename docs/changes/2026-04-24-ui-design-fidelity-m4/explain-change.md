# Responsive Design Fidelity M4

## Summary

This change closes the final implementation milestone for the UI
design-fidelity initiative. It adds automated width-band coverage for required
workflow content, records browser screenshot evidence at the agreed large,
medium, and small app-window widths, and fixes two issues found during visual
review: the plain-browser runtime was calling the Tauri client, and normal shell
copy could break inside short words.

The scope remains frontend-only. No Tauri command, event, persistence, scan,
duplicate, cleanup, or resume contract changed.

## Diff Rationale By Area

| File | Change | Reason | Evidence |
| --- | --- | --- | --- |
| [workspace-navigation.test.tsx](/D:/Data/20260415-space-sift/src/workspace-navigation.test.tsx:1) | Added responsive state-availability coverage at `1280`, `900`, and `560`; added CSS contract checks for focus-visible styles and shell-copy wrapping; added a cleanup validation issue to the preview fixture. | Prove M4 requirements for required content at large/medium/small window states and lock in the visual hardening found during screenshot review. | `npm run test -- src/workspace-navigation.test.tsx` passed |
| [runtimeSpaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/runtimeSpaceSiftClient.ts:1), [runtimeSpaceSiftClient.test.ts](/D:/Data/20260415-space-sift/src/lib/runtimeSpaceSiftClient.test.ts:1), [main.tsx](/D:/Data/20260415-space-sift/src/main.tsx:1) | Added runtime client selection: Tauri uses the Tauri client; plain browser uses the existing unsupported client. | Browser visual review through Vite is not a Tauri runtime and must not call `invoke` without Tauri internals. | Runtime test passed; refreshed screenshots no longer show the invoke error banner |
| [App.css](/D:/Data/20260415-space-sift/src/App.css:1) | Added shared focus-visible outlines and replaced `.current-path` `word-break: break-all` with `overflow-wrap: break-word` plus normal word breaking. | Keep keyboard focus visible across shared controls and avoid mid-word wrapping in normal shell copy while still allowing long paths to wrap. | Focus/wrapping CSS tests passed; visual review at `1280` confirmed normal words no longer split |
| [screenshots/](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/screenshots:1) | Added app and prototype screenshots at `1280`, `900`, and `560` widths. | Preserve the manual/browser-visible evidence required by `R34` and the M4 plan. | App screenshots visually reviewed after fixes |
| [docs/plan.md](/D:/Data/20260415-space-sift/docs/plan.md:1), [2026-04-24-space-sift-ui-design-fidelity.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-24-space-sift-ui-design-fidelity.md:1) | Updated progress, decision log, discoveries, aligned-surface audit, validation notes, and readiness. | Keep active plan state aligned with the implemented milestone and actual validation evidence. | Plan now records M4 complete and CI parity passed |

## Screenshot Evidence

App screenshots:

- [app-1280.png](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/screenshots/app-1280.png:1)
- [app-900.png](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/screenshots/app-900.png:1)
- [app-560.png](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/screenshots/app-560.png:1)

Prototype reference screenshots:

- [prototype-1280.png](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/screenshots/prototype-1280.png:1)
- [prototype-900.png](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/screenshots/prototype-900.png:1)
- [prototype-560.png](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/screenshots/prototype-560.png:1)

## Verification Evidence

Working directory for all commands: `D:\Data\20260415-space-sift`

- `npm run test -- src/workspace-navigation.test.tsx`
  - passed, 43 tests
- `npm run test -- src/lib/runtimeSpaceSiftClient.test.ts`
  - passed, 2 tests
- `npm run test -- src/scan-history.test.tsx`
  - passed, 11 tests
- `npm run test -- src/results-explorer.test.tsx`
  - passed, 5 tests
- `npm run test -- src/duplicates.test.tsx`
  - passed, 12 tests
- `npm run test -- src/cleanup.test.tsx`
  - passed, 6 tests
- `npm run lint`
  - passed
- `npm run test`
  - passed, 8 test files and 87 tests
- `npm run build`
  - passed
- `powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1`
  - passed after stopping the leftover local Vite screenshot server that held
    `node_modules\@esbuild\win32-x64\esbuild.exe` locked on the first attempt
  - covered `npm ci`, lint, tests, build, and
    `cargo check --manifest-path src-tauri/Cargo.toml`

## Scope Control

The change intentionally did not:

- add bridge capability UI;
- change backend commands, persistence, scan, duplicate, cleanup, or resume
  behavior;
- add new cleanup capabilities or auto-elevation;
- make pixel-perfect prototype matching a requirement; or
- close the active plan as done before M4 code review.

Readiness: M4 implementation and validation are complete, and the milestone is
ready for code review.
