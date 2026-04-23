# Proposal: align product UI with the `docs/ui` design direction

## 1. Status

accepted

## 2. Problem

The workspace-navigation initiative is implemented and verified, but the product still needs a dedicated design-fidelity pass so the shipped interface follows the intended UI design direction rather than only satisfying the workspace behavior contract.

The user request is to "totally follow" the UI design in `docs/ui` and optimize the product accordingly. The design source is now present in:

- `docs/ui/space-sift-tabbed-ui-prototype.html`
- `docs/ui/space-sift-ui-redesign-implementation.md`

The proposal should therefore shift from proving the seven-workspace shell is useful to making the shipped product faithfully reflect the uploaded design direction.

## 3. Goals

- Treat the uploaded `docs/ui` design artifacts as the primary visual and interaction reference.
- Optimize the shipped React UI so the product feels like the intended task-focused Windows 11 desktop workspace, not only a functional tabbed shell.
- Preserve the approved workspace-navigation, scan-history, explorer, duplicate-review, cleanup, and continuity contracts unless a later spec explicitly changes them.
- Make visual hierarchy, spacing, density, navigation, status, and workflow panels consistent across the seven workspaces.
- Improve reviewability by separating design-fidelity work from backend, persistence, scan-engine, duplicate-engine, and cleanup-engine changes.

## 4. Non-goals

- No backend scan, duplicate, cleanup, persistence, or release workflow redesign.
- No destructive-action behavior changes, no auto-elevation, and no weakening of protected-path fail-closed behavior.
- No new feature capability beyond UI optimization unless a later approved spec adds it.
- No silent divergence from existing approved behavior specs in order to match a static mockup.
- No broad frontend rewrite or router migration unless a later architecture artifact proves it is required.

## 5. Context

- `CONSTITUTION.md` requires externally visible behavior changes to start from an approved contract before implementation.
- The accepted proposal `docs/proposals/2026-04-22-space-sift-advanced-ui-upgrade.md` selected the `docs/ui` tabbed workspace direction and preserved backend contracts.
- The approved spec `specs/space-sift-workspace-navigation.md` defines the behavior contract for the seven workspace tabs, global status, startup resolution, and automatic navigation.
- The execution plan `docs/plans/2026-04-22-space-sift-workspace-navigation-ui.md` is `done`; it implemented and verified the workspace shell.
- The current request tightens the design direction from "use `docs/ui` as design input" toward a stricter product design-alignment pass.
- `docs/ui/space-sift-ui-redesign-implementation.md` defines the target information architecture, accessible tab pattern, panel responsibilities, interaction rules, styling principles, and expected tests.
- `docs/ui/space-sift-tabbed-ui-prototype.html` provides the concrete visual prototype for the desktop shell, sidebar navigation, cards, tables, workflow panels, and safety-oriented cleanup funnel.

## 6. Options considered

- Option A: Do nothing beyond the completed workspace-navigation work.
  - Lowest implementation cost.
  - Fails the new request to optimize the product according to the UI design.
- Option B: Apply generic visual polish without the `docs/ui` artifacts.
  - Can improve spacing and styling quickly.
  - Risks drifting away from the intended design because it does not use the provided source.
- Option C: Implement a focused design-fidelity pass over the existing workspace shell using the uploaded `docs/ui` artifacts.
  - Best matches the user request.
  - Keeps work scoped to frontend presentation and interaction polish.
  - Requires a short visual contract before implementation so fidelity can be reviewed.
- Option D: Rebuild the frontend from the design as a new app shell.
  - Could maximize visual freedom.
  - Unnecessarily risky because the workspace-navigation shell already exists and is verified.

## 7. Recommended Direction

Choose Option C: run a focused design-fidelity optimization over the existing workspace shell using the uploaded `docs/ui` artifacts.

The new work should aim for exact component and layout fidelity with close visual similarity to the uploaded design, while preserving approved product text, status language, and safety copy. It should not commit the product to pixel-for-pixel shipment when that would conflict with approved feature contracts, accessibility, or reviewability.

The prototype should drive the visual structure: left workspace rail, persistent header, panel and card hierarchy, history and explorer tables, duplicate-review cards, and the cleanup funnel. The approved specs remain the behavioral and safety source of truth. If prototype sample copy conflicts with approved product copy or safety wording, the approved repository contracts win.

This proposal should supersede only the earlier assumption that pixel-level design fidelity is not a goal. It should not supersede the approved workspace-navigation behavior, backend boundaries, or safety model.

## 8. Expected Behavior Changes

- The app's visible layout, spacing, visual hierarchy, and panel composition should more closely match the `docs/ui` design.
- The desktop shell should use the design's two-column structure: workspace navigation on the left and task panels on the right, with responsive collapse below tablet width.
- The navigation should retain accessible tab semantics and visual labels/descriptions for Overview, Scan, History, Explorer, Duplicates, Cleanup, and Safety.
- The app should be desktop-first, not desktop-only. Resizable Tauri windows must keep the same workspaces, global status, next safe action, continuity fields, and cleanup guardrails available at narrower widths.
- Responsive behavior should adapt by app window width, not physical screen size. The uploaded prototype's `1050px` and `640px` breakpoints are the baseline reference for collapsing the shell, reflowing tabs, stacking grids, and widening controls.
- Overview should emphasize loaded state, active work, four high-level metrics, and the next safe action.
- Scan, History, Explorer, Duplicates, Cleanup, and Safety panels should follow the uploaded design's required content emphasis and ordering where it does not conflict with approved specs.
- The seven workspaces should remain present: Overview, Scan, History, Explorer, Duplicates, Cleanup, and Safety.
- Global status and next safe action should remain visible and deterministic, but their visual treatment should be aligned with the design.
- Workflow panels should feel intentionally designed as one product surface rather than individually styled sections.
- Existing safety affordances should remain clear or become clearer, especially cleanup preview, Recycle Bin default, permanent-delete friction, protected-path boundaries, and local-only posture.

## 9. Architecture Impact

- Primary impact is expected in the React frontend:
  - `src/App.tsx`
  - `src/App.css`
  - any existing workspace-navigation helper or component files under `src/`
- The `SpaceSiftClient` and Tauri command boundary should remain unchanged by default.
- No SQLite, Rust domain crate, scan-engine, duplicate-engine, cleanup-engine, or release pipeline changes are expected.
- If the design requires component extraction, it should be incremental and scoped to reducing presentation complexity rather than changing app state ownership.

## 10. Testing and Verification Strategy

- Add or revise a focused UI design-fidelity spec before implementation, using the uploaded `docs/ui` artifacts as source input.
- The design-fidelity spec should encode this source precedence when design inputs disagree:
  1. approved behavior and safety sources, including proposals and specs
  2. prototype HTML for layout, hierarchy, and responsive behavior
  3. architecture and implementation notes for component ownership and wiring, not final visuals
  4. screenshots or Figma exports as reference-only unless later checked in and explicitly designated authoritative
- Add or revise a matching test spec for regression-prone behavior that could be affected by layout changes:
  - workspace navigation remains accessible
  - global status remains visible from all workspaces
  - next safe action remains non-destructive
  - cleanup safety gates remain visible and usable
  - explorer, duplicate, and cleanup panels still expose prerequisite and degraded states
- Use existing frontend tests as the main automated safety net.
- Add manual or browser-based visual review notes for desktop and narrow viewport layouts after implementation.
- Verify visual behavior at these app-window width bands:
  - desktop width above `1050px`
  - mid or narrow width between `640px` and `1050px`
  - narrow width below `640px`
- At each width band, verify that required status content, continuity fields, cleanup guardrails, and next-safe-action content remain available.
- Verify close visual similarity by comparing implementation screenshots against the prototype structure, not by byte-level or pixel-perfect matching.
- Likely validation commands:
  - `npm run lint`
  - `npm run test`
  - `npm run build`
  - `powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1` before branch-ready claims

## 11. Rollout and Rollback

- Rollout should be frontend-only and incremental.
- No data migration or command-surface migration is expected.
- Rollback should be possible by reverting the design-fidelity UI changes while keeping the completed workspace-navigation shell and restore-context work intact.
- If a specific design treatment creates accessibility or safety ambiguity, rollback only that treatment rather than reopening the whole workspace shell.

## 12. Risks and Mitigations

- Risk: the implementation follows only the prototype's sample data or decorative details instead of the product's real state.
  - Mitigation: use the implementation markdown for contract details, use the prototype for composition, and keep the approved specs as behavior boundaries.
- Risk: visual fidelity conflicts with approved behavior contracts.
  - Mitigation: treat approved specs as behavioral source of truth and document any required contract change explicitly.
- Risk: a design polish pass becomes a broad frontend rewrite.
  - Mitigation: keep the first pass inside the existing workspace shell and extract components only when it reduces local complexity.
- Risk: safety affordances become visually quieter while making the app look more polished.
  - Mitigation: require explicit review of cleanup, permanent delete, protected-path, and local-only messaging states.
- Risk: accessibility regresses during visual changes.
  - Mitigation: preserve the approved workspace-navigation accessibility tests and add focused checks for any changed controls.

## 13. Open Questions

None at the proposal level.

The fidelity target, responsive contract, and source precedence are settled for the next spec.

## 14. Decision Log

- 2026-04-24: proposed a focused design-fidelity optimization pass over the completed workspace shell.
  - Reason: the user requested that the product totally follow the UI design in `docs/ui`.
  - Alternatives rejected: generic polish without the design source and a full frontend rebuild.
- 2026-04-24: marked downstream readiness as blocked on restoring or providing `docs/ui`.
  - Reason: exact design alignment cannot be specified or verified against absent design artifacts.
- 2026-04-24: preserved approved behavior and safety contracts as the implementation boundary.
  - Reason: design fidelity should improve the product surface without silently changing scan, duplicate, cleanup, persistence, or destructive-action behavior.
- 2026-04-24: restored the `docs/ui` source input to the proposal context.
  - Reason: the uploaded prototype and implementation markdown now provide enough design direction for a focused design-fidelity spec.
  - Alternatives rejected: continuing to treat the work as blocked by missing design files.
- 2026-04-24: settled the fidelity target as exact component and layout fidelity with close visual similarity, not unconditional pixel-for-pixel shipment.
  - Reason: the design should guide product structure and visual quality, while approved text, status language, safety copy, accessibility, and behavior contracts remain authoritative.
- 2026-04-24: settled the responsive contract as desktop-first and responsive-required for resizable Tauri windows.
  - Reason: narrow-window behavior must preserve required workspaces, global status, next safe action, continuity fields, and cleanup guardrails while adapting density and layout by app window width.
- 2026-04-24: established source precedence for conflicting design inputs.
  - Reason: the next spec needs a deterministic rule for resolving conflicts among approved contracts, prototype HTML, implementation notes, and any future screenshots or Figma exports.

## 15. Next Artifacts

- Feature/design spec: likely `specs/space-sift-ui-design-fidelity.md`.
- Matching test spec: likely `specs/space-sift-ui-design-fidelity.test.md`.
- Execution plan: likely `docs/plans/2026-04-24-space-sift-ui-design-fidelity.md` after the design source and spec are stable.

## 16. Follow-on Artifacts

None yet.

## 17. Readiness

This proposal is accepted and ready for spec.

It is not ready for implementation yet. The next step is a focused UI design-fidelity spec that turns the uploaded `docs/ui` artifacts, fidelity target, responsive contract, and source precedence into a reviewable implementation contract.
