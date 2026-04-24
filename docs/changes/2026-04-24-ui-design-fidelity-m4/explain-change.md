# Responsive Design Fidelity M4

## Summary

This change closes the final implementation milestone for the UI
design-fidelity initiative. It adds automated width-band coverage for required
workflow content, records browser screenshot evidence at the agreed large,
medium, and small app-window widths, and fixes two issues found during visual
review: the plain-browser runtime was calling the Tauri client, and normal shell
copy could break inside short words.

The follow-up lifecycle edit records that M4 code review completed
clean-with-notes and that final verification is the active gate. Unrelated dirty
plan edits were isolated outside this change.

The scope remains frontend-only. No Tauri command, event, persistence, scan,
duplicate, cleanup, or resume contract changed.

## Problem

M4 existed to prove the finished workspace UI still satisfied the responsive
contract after the M1-M3 design-fidelity work. The contract required meaningful
large, medium, and small app-window checks at `1280`, `900`, and `560` widths,
with required status, continuity, cleanup, and next-safe-action content still
available.

During browser-visible review, the app also exposed two concrete defects:

- plain-browser screenshot review showed a Tauri `invoke` failure because
  `main.tsx` always selected the Tauri client;
- `.current-path` used `word-break: break-all`, which made ordinary shell copy
  split inside words.

## Decision Trail

| Source | Decision |
| --- | --- |
| Proposal | Preserve truthful product copy and align to the uploaded design structurally, not pixel-perfect. |
| Spec | `R21`-`R27` require app-window-width responsive behavior using `1050px` and `640px` as baseline breakpoints. |
| Spec | `R28`-`R33` require preserving global status, non-destructive next safe action, cleanup safety, and resume actionability. |
| Spec | `R34`-`R36` require screenshot or human-visible evidence at the three width bands. |
| Spec | `R37` requires existing behavioral coverage to continue. |
| Test spec | `T12`, `T13`, `T16`, and `T20` operationalize responsive and screenshot evidence. |
| Architecture | The workspace shell remains frontend-only; Tauri command, event, persistence, and backend contracts are unchanged. |
| Plan | M4 owns responsive visual verification, final hardening, screenshot evidence, and CI parity before branch-ready claims. |
| Review | Code review completed clean-with-notes; no required code changes were found. |
| Verify | Verify found lifecycle text drift and unrelated dirty files; lifecycle text was updated and unrelated changes were isolated. |

## Diff Rationale By Area

| File | Change | Reason | Source artifact | Test/evidence |
| --- | --- | --- | --- | --- |
| [workspace-navigation.test.tsx](/D:/Data/20260415-space-sift/src/workspace-navigation.test.tsx:1) | Added responsive state-availability coverage at `1280`, `900`, and `560`; added CSS contract checks for focus-visible styles and shell-copy wrapping; added a cleanup validation issue to the preview fixture. | Prove required content remains available across large, medium, and small app-window states. | `R21`-`R27`, `R34`-`R37`, `T12`, `T13`, `T16`, `T20` | `npm run test -- src/workspace-navigation.test.tsx` passed |
| [runtimeSpaceSiftClient.ts](/D:/Data/20260415-space-sift/src/lib/runtimeSpaceSiftClient.ts:1), [runtimeSpaceSiftClient.test.ts](/D:/Data/20260415-space-sift/src/lib/runtimeSpaceSiftClient.test.ts:1), [main.tsx](/D:/Data/20260415-space-sift/src/main.tsx:1) | Added runtime client selection: Tauri uses the Tauri client; plain browser uses the existing unsupported client. | Browser visual review through Vite is not a Tauri runtime and must not call `invoke` without Tauri internals. | `R30`, `C1`-`C4`, `T19`, M4 visual review | Runtime test passed; refreshed screenshots no longer show the invoke error banner |
| [App.css](/D:/Data/20260415-space-sift/src/App.css:1) | Added shared focus-visible outlines and replaced `.current-path` `word-break: break-all` with `overflow-wrap: break-word` plus normal word breaking. | Keep keyboard focus visible and avoid mid-word wrapping in normal shell copy while still allowing long paths to wrap. | `A3`, Edge 11, `T20` | Focus/wrapping CSS tests passed; visual review confirmed normal words no longer split |
| [screenshots/](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/screenshots:1) | Added app and prototype screenshots at `1280`, `900`, and `560` widths. | Preserve the manual/browser-visible evidence required before branch-ready claims. | `R34`-`R36`, `O2`-`O4`, `T20` | App screenshots visually reviewed after fixes |
| [docs/plan.md](/D:/Data/20260415-space-sift/docs/plan.md:1), [2026-04-24-space-sift-ui-design-fidelity.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-24-space-sift-ui-design-fidelity.md:1) | Recorded M4 implementation, validation, CI parity, code-review completion, and final verification as the active gate. | Keep lifecycle-managed artifacts aligned with the actual workflow state. | Plan policy, verify lifecycle checks | Plan/index no longer advertise M4 as waiting for code review |
| [change.yaml](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/change.yaml:1), [explain-change.md](/D:/Data/20260415-space-sift/docs/changes/2026-04-24-ui-design-fidelity-m4/explain-change.md:1) | Added durable change metadata and rationale. | The workflow requires a baseline change-local pack for ordinary non-trivial work. | `verify` change-pack requirement | Artifact is tracked with the implementation |

## Tests Added Or Changed

| Test | What it proves | Why this level is appropriate |
| --- | --- | --- |
| `responsive_width_bands_keep_continuity_cleanup_and_review_state_available` | Required shell status, next safe action, interrupted-run continuity, cleanup preview, cleanup issues, Recycle Bin action, permanent-delete separation, and duplicate-review state remain available at `1280`, `900`, and `560`. | jsdom can prove DOM availability and no accidental backend invocation across width contracts. |
| Focus-visible stylesheet assertion | Shared controls expose visible keyboard focus treatment. | Static CSS assertion is enough for the explicit selector contract. |
| Current-path wrapping stylesheet assertion | Long paths can wrap without forcing all normal words to break inside words. | Static CSS assertion protects the regression found during screenshot review. |
| Runtime client tests | Plain browser selects `unsupportedClient`; Tauri selects `tauriSpaceSiftClient`. | Unit-level coverage isolates runtime selection without launching Tauri. |

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
- `npm run test -- src/workspace-navigation.test.tsx`
  - rerun during verify, passed, 43 tests
- `git diff --check -- docs/plan.md docs/plans/2026-04-24-space-sift-ui-design-fidelity.md`
  - passed after lifecycle text update

## Alternatives Rejected

- Adding a shell `bridge: connected` pill was rejected because bridge capability
  is not modeled truthfully yet.
- Modeling a new bridge capability state was deferred as a larger follow-up;
  M4 only needed truthful default shell copy and visual verification.
- Pixel-perfect prototype matching was rejected by the proposal and spec in
  favor of structural fidelity and approved product copy.
- Tauri screenshot automation was not added because browser-visible screenshots
  plus CI parity satisfied the M4 evidence contract in this environment.

## Scope Control

The change intentionally did not:

- add bridge capability UI;
- change backend commands, persistence, scan, duplicate, cleanup, or resume
  behavior;
- add new cleanup capabilities or auto-elevation;
- make pixel-perfect prototype matching a requirement;
- close the active plan as done before final verification; or
- include unrelated older plan lifecycle edits in the M4 change.

Unrelated dirty edits to older plan files were isolated in
`stash@{0}: On main: isolate unrelated pre-existing plan and cargo line-ending changes`.
The temporary `src-tauri/Cargo.toml` dirty entry was line-ending-only and had no
content diff.

## Risks And Follow-Ups

- The screenshot set shows the default shell state. Stateful cleanup and
  interrupted-run availability are covered by DOM tests at the same width bands,
  not by separate stateful screenshots.
- The plan remains active until final verification confirms branch-ready state
  or records any remaining blocker.
- The isolated stash should be reapplied or dropped intentionally after the M4
  branch-ready decision.

## PR Handoff Summary

- M4 implementation, visual evidence, focused tests, full frontend checks, and
  CI parity are recorded.
- Code review completed clean-with-notes.
- Verify blockers found in lifecycle text and unrelated dirty worktree state
  were addressed by updating lifecycle text and isolating unrelated changes.

Readiness: final lifecycle blockers were resolved, the plan is closed as
`done`, and the change is ready for PR preparation.
