# Proposal: implement the advanced Space Sift UI direction from `docs/ui`

## 1. Status

accepted

## 2. Problem

`Space Sift` now ships multiple meaningful workflows: scanning, history reopen, run continuity review, explorer drill-down, duplicate triage, and cleanup preview or execution. The current product UI still concentrates those workflows in a single long page, primarily in `src/App.tsx` and `src/App.css`.

That shape works functionally, but it makes the app feel more like a stacked form than a guided review workflow. It also increases UI coupling, makes feature boundaries harder to read, and limits how clearly the product can present "what is loaded," "what is active," and "what is the next safe action."

The repository already contains a stronger UI direction in:

- `docs/ui/space-sift-tabbed-ui-prototype.html`
- `docs/ui/space-sift-ui-redesign-implementation.md`

The project now needs a decision about whether and how to turn that advanced UI direction into the product, while preserving the existing safety model and feature contracts.

## 3. Goals

- Upgrade the product UI from a single long workflow page to a clearer, task-focused workspace that matches the current feature set.
- Use the existing `docs/ui` prototype work as design input instead of starting a second redesign track.
- Preserve the current safety model: local-only history, preview-first cleanup, Recycle Bin-first default, protected-path fail-closed behavior, and explicit permanent-delete friction.
- Improve orientation so users can move between scan, history, explorer, duplicates, cleanup, and safety context without losing global state awareness.
- Provide a coordinating umbrella for the repo's existing UI-facing work rather than restarting UI direction from scratch.
- Reduce frontend complexity over time by creating clearer UI boundaries and component ownership.

## 4. Non-goals

- No backend scan-engine redesign, no duplicate-verification algorithm changes, and no cleanup-engine behavior change in this proposal.
- No new filesystem permissions model, no UI self-elevation, and no change to protected-path fail-closed behavior.
- No new release, signing, installer, or `winget` workflow work.
- No expansion of cleanup rules or destructive capabilities beyond the currently approved specs.
- No commitment to ship the static `docs/ui` prototype pixel-for-pixel if it conflicts with existing feature contracts, accessibility, or reviewability.

## 5. Context

- `CONSTITUTION.md` requires externally observable UI changes to start from proposal/spec/test-spec before implementation.
- `docs/project-map.md` shows the current frontend is centered in `src/App.tsx`, with the app already spanning scan, history, explorer, duplicates, cleanup, and safety messaging.
- `docs/project-map.md` also identifies three existing risk seams relevant to this proposal:
  - `src/App.tsx` is large and cross-feature.
  - `src-tauri/src/commands/scan.rs` is already a cross-cutting orchestration file.
  - `src-tauri/crates/app-db/src/lib.rs` is already a broad persistence boundary.
- The current product already has feature contracts in:
  - `specs/space-sift-scan-history.md`
  - `specs/space-sift-results-explorer.md`
  - `specs/space-sift-duplicates.md`
  - `specs/space-sift-cleanup.md`
- The `docs/ui` work already proposes a seven-tab workspace: Overview, Scan, History, Explorer, Duplicates, Cleanup, and Safety.
- `docs/plan.md` currently lists active UI-adjacent plans:
  - `docs/plans/2026-04-16-history-and-duplicate-review-clarity.md`
  - `docs/plans/2026-04-16-scan-progress-and-active-run-ux.md`
  - `docs/plans/2026-04-15-space-sift-win11-mvp.md`
- This initiative is a coordinating umbrella for those active UI-facing plans. It does not supersede them by default. Later plan, spec, or architecture artifacts should only supersede an active plan when they explicitly change that plan's governing contract.
- The HTML prototype under `docs/ui/` is now treated as a frozen prototype snapshot, not a living behavior contract.

Observed implication:

- The proposal direction is now settled at the umbrella level, but later spec and planning work still need to reconcile which active plan continues to govern which slice of implementation.

## 6. Options considered

- Option A: keep the current shipped UI and treat `docs/ui` as docs-only reference material.
  - Lowest immediate risk.
  - Preserves the current monolithic UI shape and does not address workflow clarity.
- Option B: keep the existing one-page information architecture and only polish visuals inside `src/App.tsx` and `src/App.css`.
  - Lower implementation risk than a layout shift.
  - Improves look and feel, but does not fix the core workflow separation problem.
- Option C: adopt the `docs/ui` tabbed workspace as an incremental frontend-only product upgrade over the existing Tauri and Rust contracts.
  - Uses the existing prototype work.
  - Improves workflow clarity without requiring backend redesign.
  - Still requires careful state choreography and accessibility work.
- Option D: rewrite the frontend first into a fully new routed or feature-sliced shell before applying the redesign.
  - Could yield a cleaner long-term frontend architecture.
  - Highest near-term cost and highest delivery risk, because it couples structural refactor and visual redesign.

## 7. Recommended direction

Choose Option C: implement the `docs/ui` tabbed workspace direction as an incremental frontend-driven upgrade, while intentionally preserving the existing backend command, event, and persistence contracts unless a later spec explicitly changes them.

Rationale:

- The repository already has a credible design direction in `docs/ui`, so the fastest path to a reviewable decision is to use that work rather than re-explore the interaction model.
- The product's current workflows naturally map to discrete task areas. Tabs fit the actual user journey of `scan -> inspect -> verify -> preview -> execute` better than one stacked page.
- A frontend-first upgrade can improve clarity without reopening scan, duplicate, cleanup, or release architecture by default.
- The recommended direction also creates a better path for reducing `src/App.tsx` concentration through extraction into feature components after the information architecture is stable.
- This is worth doing now because the product already has the core capabilities; the next user-value gain is cross-workflow discoverability and clearer task context rather than another isolated capability addition.
- The initiative should coordinate the active UI plans rather than replacing them wholesale. Only later artifacts that explicitly change contract should supersede the relevant active plan.

## 8. Expected behavior changes

- The product would present the major workflows as a workspace shell rather than a single stacked page.
- Users would move between Overview, Scan, History, Explorer, Duplicates, Cleanup, and Safety panels through an explicit navigation model.
- Overview is the fallback landing tab, but high-confidence durable task state may choose a more relevant initial tab.
- Non-Overview startup is allowed only for live work, interrupted recovery work, or a valid last task context that is recoverable from local persisted state and tied to a specific run, scan, or analysis identifier.
- The startup priority order should be:
  - a backend-confirmed live running scan opens Scan
  - a backend-confirmed live duplicate analysis tied to the currently loaded scan opens Duplicates
  - startup recovery that finds stale or abandoned scan runs opens History
  - a valid persisted `lastOpenedScanId` plus last workspace Explorer opens Explorer
  - a valid persisted `lastOpenedScanId` plus a durable completed duplicate-analysis result and last workspace Duplicates may open Duplicates
  - a valid persisted `lastOpenedScanId` plus a durable unexecuted cleanup preview and last workspace Cleanup may open Cleanup
  - everything else opens Overview
- Completed history by itself, cancelled or failed prior work, prior cleanup execution, existing cleanup rules, prior Safety viewing, or generic "history exists" signals are not sufficient to choose a non-Overview startup tab.
- The app would still preserve the current safety model and current feature contracts, but it would present them in more task-focused panels.
- High-value workflow transitions would become more explicit, for example:
  - finishing a scan reopens the saved result and emphasizes exploration;
  - reopening from history emphasizes explorer review;
  - starting duplicate analysis emphasizes duplicate-review status;
  - advancing from duplicate selection to cleanup preview becomes clearer.
- Automatic tab switches are contractual only for explicit user actions and these six high-priority live task transitions:
  - `N1_START_SCAN`: starting a scan, or receiving the matching running snapshot for that accepted start action, switches to Scan
  - `N2_SCAN_COMPLETED_AND_OPENED`: a scan that completes, persists successfully, and is opened as the current stored result switches to Explorer
  - `N3_OPEN_HISTORY_SCAN`: explicitly opening a completed scan from History switches to Explorer
  - `N4_START_DUPLICATE_ANALYSIS`: explicitly starting duplicate analysis from a loaded scan switches to Duplicates
  - `N5_REQUEST_CLEANUP_PREVIEW`: explicitly requesting cleanup preview or review switches to Cleanup
  - `N6_REVIEW_INTERRUPTED_RUNS`: explicitly requesting interrupted-run review from Overview or a global notice switches to History
- Background refreshes, newly discovered stale or abandoned records, cancellations, failures, duplicate-analysis completion, cleanup-preview completion, cleanup execution completion, keep-selection changes, Safety warnings, and `canResume` updates must use badges, notices, or inline updates rather than stealing focus.
- Automatic navigation must be operation-aware. A stale, duplicated, or replayed backend event must not override fresher UI state for the same operation identifier.
- The redesign would make safety rules more visible as first-class UI content instead of incidental copy.

Behaviors expected to remain unchanged unless a later spec explicitly changes them:

- completed scans still become the current result only after persistence succeeds
- history remains local-only
- older summary-only scans still degrade to a rescan-required flow instead of a broken explorer, duplicate, or cleanup path
- duplicate analysis remains read-only and deterministic
- cleanup remains preview-first with Recycle Bin as the default execution mode
- protected-path cleanup remains fail-closed and does not auto-elevate the normal UI

## 9. Architecture impact

- Primary impact is expected in the React frontend:
  - `src/App.tsx`
  - `src/App.css`
  - likely new component and feature folders under `src/`
- The typed `SpaceSiftClient` boundary should remain the main frontend-backend contract.
- Tauri command names, event names, and persisted data contracts are not expected to change by default.
- A new frontend shell state will likely be introduced for active workspace tab, global status summary, startup resolution, and workflow-driven tab switching.
- The required architecture artifact should be a short navigation-focused note at `docs/architecture/2026-04-22-workspace-navigation-ui.md`.
- That architecture note should resolve ownership and navigation semantics only:
  - app-shell ownership of `activeWorkspace`, startup resolver inputs, and global notices
  - workspace ownership for Scan, History, Explorer, Duplicates, Cleanup, and Safety state
  - the initial workspace resolver and the six contractual auto-switch reasons
  - event-ordering rules that prevent awaited command results or repeated snapshots from overwriting fresher operation state
- That architecture note should explicitly avoid:
  - backend API redesign
  - a new global state-management library
  - a full `App.tsx` rewrite in the first implementation pass
  - new scan, duplicate, cleanup, or resume behavior contracts
- The first implementation pass should keep `App.tsx` as the orchestration boundary while adding a small workspace-navigation layer such as `WorkspaceTab`, `activeWorkspace`, `resolveInitialWorkspace(ctx)`, `navigateWorkspace(target, reason, operationId?)`, `WorkspaceNav`, and `WorkspacePanel`.
- Feature extraction should remain gradual after the workspace behavior is stable.

## 10. Testing and verification strategy

- Add or revise a feature spec for the new workspace UI behavior before implementation.
- Add or revise a matching test spec that maps UI navigation, visibility, safety gates, and workflow transitions to concrete tests.
- Expect most new automated coverage to be frontend tests around:
  - tab rendering and accessible tab semantics
  - panel visibility and context switching
  - workflow-driven tab switching
  - duplicate and cleanup guardrails
  - resume button disabled state when `canResume` is false
- The later spec should define tests that prove a user can tell from any tab what is running, what is loaded, and what the next safe action is.
- Keep backend verification focused on regression confidence rather than new logic unless the spec expands backend behavior.
- Likely verification commands remain:
  - `npm run lint`
  - `npm run test`
  - `npm run build`
  - `cargo check --manifest-path src-tauri/Cargo.toml`

## 11. Rollout and rollback

- Rollout should stay additive and frontend-first, with no schema migration expected for the recommended direction.
- The current one-page shell provides a straightforward rollback target if the new workspace shell creates usability or accessibility regressions.
- If implementation chooses a temporary feature flag or branch-local switch for validation, that should remain a delivery detail and not become part of the permanent product contract unless explicitly specified later.
- Because the recommended direction avoids persistence or command-surface changes by default, rollback risk is primarily UI regression risk rather than data migration risk.

## 12. Risks and mitigations

- Risk: the redesign becomes a combined UX rewrite and frontend architecture rewrite.
  - Mitigation: keep the proposal anchored to the workspace shell and feature presentation first; defer deeper frontend state redesign unless it proves necessary.
- Risk: overlap with currently active UI plans causes duplicated or conflicting work.
  - Mitigation: treat this initiative as the umbrella direction and explicitly mark any later supersession only when a downstream artifact changes a plan's governing contract.
- Risk: tabbed navigation hides critical live-state or safety information.
  - Mitigation: keep a global status surface visible across tabs and make Safety a first-class panel, not a footnote.
- Risk: the static prototype overpromises interactions that are awkward with current runtime state.
  - Mitigation: freeze `docs/ui` as prototype input only and keep the approved spec as the real behavior contract.
- Risk: accessibility regresses during the layout shift.
  - Mitigation: require accessible tab semantics, keyboard navigation, focus-visible states, and regression tests.
- Risk: aggressive auto-switching between tabs feels disorienting.
  - Mitigation: constrain contractual auto-switching to explicit user actions and high-priority live task transitions, with the exact list defined in the later spec.

## 13. Open questions

None at the proposal level.

Remaining downstream detail belongs in the feature spec and the short frontend architecture note, especially:

- whether this initiative will actually add durable duplicate-analysis results or durable cleanup-preview storage, or continue treating those as session-scoped and therefore ineligible for cold-start tab restoration
- the exact persisted fields and downgrade behavior for restoring last workspace and last opened scan context
- the final accessible tab interaction details and keyboard model

## 14. Decision log

- 2026-04-22: selected the `docs/ui` tabbed workspace direction as the proposal baseline.
  - Reason: the prototype already matches the product's multi-step workflow better than the current one-page shell.
  - Alternatives rejected: docs-only retention and visual-only polish, because they do not adequately improve workflow separation.
- 2026-04-22: rejected a full frontend rewrite as the recommended first move.
  - Reason: it couples architecture cleanup and UI redesign too tightly for a reviewable near-term upgrade path.
- 2026-04-22: preserved backend contracts as the default recommendation.
  - Reason: the strongest current user value is in product clarity and workflow presentation, not in reopening backend scope by default.
- 2026-04-22: user clarified that the initiative is a coordinating umbrella, not a blanket supersession of active UI plans.
  - Reason: active plan contracts should remain in force unless later artifacts explicitly change them.
- 2026-04-22: user fixed the shell-navigation direction.
  - Decisions:
    - Overview is the fallback landing tab.
    - high-confidence durable task state may choose a more relevant initial tab.
    - automatic tab switches are contractual only for explicit user actions and high-priority live task transitions.
    - the HTML prototype is frozen as a prototype snapshot.
    - a short frontend architecture note is required before implementation.
- 2026-04-22: user finalized the durable startup-tab and auto-switch policy for downstream spec work.
  - Decisions:
    - non-Overview startup is reserved for live work, interrupted recovery work, or a valid persisted last task context
    - startup priority is live scan, live duplicate analysis, interrupted-run recovery, valid last-opened Explorer context, then durable duplicate or cleanup context when those results are actually persisted
    - exactly six auto-switch reasons are contractual: start scan, completed scan opened, open history scan, start duplicate analysis, request cleanup preview, and review interrupted runs
    - background refreshes and repeated backend snapshots must not steal focus
    - navigation decisions must be operation-aware so stale events cannot override fresher state for the same operation id
- 2026-04-22: user narrowed the required frontend architecture artifact.
  - Decisions:
    - the note should live at `docs/architecture/2026-04-22-workspace-navigation-ui.md`
    - it should resolve workspace-navigation ownership and event ordering only
    - it should keep backend contracts unchanged and keep `App.tsx` as the first-pass orchestration boundary

## 15. Next artifacts

- Feature spec: `specs/space-sift-workspace-navigation.md`.
- Short frontend architecture note: `docs/architecture/2026-04-22-workspace-navigation-ui.md`.
- Matching test spec: `specs/space-sift-workspace-navigation.test.md`.
- Execution plan: likely `docs/plans/2026-04-22-space-sift-workspace-navigation-ui.md` after the spec and short frontend architecture note are in place.
- Default next authoring step: `spec`.

## 16. Follow-on artifacts

None yet.

## 17. Readiness

This proposal is accepted and ready for `spec`.

It is not ready for implementation. After the feature spec is stable, the short frontend architecture note is required before execution planning is finalized or implementation begins.
