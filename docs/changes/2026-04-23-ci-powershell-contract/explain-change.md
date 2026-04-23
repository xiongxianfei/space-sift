# PowerShell CI Contract

## Summary

This change closes the final branch-readiness gap for the workspace-navigation
initiative by making PowerShell, not `bash`, the canonical local and GitHub
Actions CI entry point on Windows-hosted environments. The implementation keeps
the same verification steps, preserves `scripts/ci.sh` as a compatibility
wrapper, updates the authoritative docs that define the CI contract, and
normalizes the workspace-navigation plan from "implemented but still active" to
"done and verified."

## Problem

The workspace-navigation feature implementation had already passed its feature
tests, code review, and local verification commands. The blocker was process
and contract drift:

- the repository still treated `bash scripts/ci.sh` as the canonical
  CI-parity gate even though the project target and GitHub Actions runner are
  Windows-first;
- the local `bash` environment on this machine was not a reliable way to reach
  `npm` and `cargo`;
- the active workspace-navigation plan still advertised branch-wide
  verification as pending;
- `docs/plan.md` still listed the workspace-navigation initiative under
  `Active`;
- there was no baseline `docs/changes/<change-id>/` pack explaining the
  CI-contract change; and
- unrelated dirty and untracked local state made the verify diff broader than
  the intended branch-ready change.

The problem was not failing product behavior. It was that the repository's
verification contract no longer matched the environment where the contract is
actually enforced.

## Decision Trail

- Exploration and proposal direction did not change. The accepted advanced UI
  proposal, approved workspace-navigation spec, approved architecture note, and
  reviewed plan still govern the feature work.
- No new feature requirement IDs were introduced by this change. This is a
  verification and artifact-closeout change, not a product-behavior change.
- The direct governing sources for the CI move were:
  - `CONSTITUTION.md` verification rules requiring a canonical CI-parity gate
    for branch-ready claims;
  - `AGENTS.md` verification expectations naming the canonical branch-ready
    command;
  - the workspace-navigation plan's branch-ready validation rule; and
  - the `verify` finding that branch readiness was blocked by stale lifecycle
    artifacts, a missing change pack, and the old `bash`-based CI contract.
- The architecture decision preserved here is the existing Windows-hosted
  delivery model. The change does not alter app runtime boundaries, Tauri
  commands, SQLite storage, or workspace-navigation behavior.
- Plan relationship:
  - feature milestones `M1` through `M4` were already complete;
  - this change performs the post-implementation verification closeout that
    moves the initiative from `active` to `done`.

## Diff Rationale By Area

| File | Change | Reason | Source artifact | Test / evidence |
| --- | --- | --- | --- | --- |
| [scripts/ci.ps1](/D:/Data/20260415-space-sift/scripts/ci.ps1:1) | Added a PowerShell CI script that runs `npm ci`, lint, test, build, and Cargo check. | Make the canonical CI-parity gate run natively in the Windows-hosted environment the repo actually targets. | `CONSTITUTION.md` verification rules, `AGENTS.md` verification expectations, workspace-navigation plan branch-ready rule | `powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1` passed |
| [scripts/ci.sh](/D:/Data/20260415-space-sift/scripts/ci.sh:1) | Replaced the old implementation with a compatibility wrapper that delegates to `ci.ps1` when PowerShell is available. | Preserve compatibility for existing callers and historical references without keeping `bash` as the authoritative CI contract. | CI contract update decision, scope control to avoid silent breakage | `bash -n scripts/ci.sh` passed |
| [.github/workflows/ci.yml](/D:/Data/20260415-space-sift/.github/workflows/ci.yml:1) | Switched the CI job from `bash scripts/ci.sh` to PowerShell running `.\scripts\ci.ps1`. | Keep GitHub Actions aligned with the new canonical local verification path instead of using a different shell contract in CI. | Verification contract in `CONSTITUTION.md`, repo CI workflow ownership in `docs/workflows.md` | Workflow file matches the local command that passed |
| [AGENTS.md](/D:/Data/20260415-space-sift/AGENTS.md:88), [CONSTITUTION.md](/D:/Data/20260415-space-sift/CONSTITUTION.md:37) | Updated the named canonical CI-parity command from `bash scripts/ci.sh` to the PowerShell form. | Prevent the governing instructions from advertising a stale branch-ready command. | Direct repo governance sources | Verified command now matches the passing local CI-parity run |
| [docs/project-map.md](/D:/Data/20260415-space-sift/docs/project-map.md:187), [docs/plans/2026-04-15-space-sift-win11-mvp.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-15-space-sift-win11-mvp.md:61) | Updated the current repository map and still-active MVP plan to point at `scripts/ci.ps1` as the canonical CI path. | These are still authoritative active artifacts and could not keep describing the old CI entry point. | Documentation rules in `CONSTITUTION.md`; active-plan consistency rules | Diff inspection plus successful PowerShell CI run |
| [docs/plans/2026-04-22-space-sift-workspace-navigation-ui.md](/D:/Data/20260415-space-sift/docs/plans/2026-04-22-space-sift-workspace-navigation-ui.md:1), [docs/plan.md](/D:/Data/20260415-space-sift/docs/plan.md:1) | Marked branch-wide verification complete, recorded the successful verification commands, and moved the workspace-navigation initiative from `Active` to `Done`. | Fix the verify blocker that the plan lifecycle and validation notes were stale relative to the implemented and verified feature. | `verify` workflow rules, plan lifecycle policy in `AGENTS.md` and `CONSTITUTION.md` | Plan/index now agree; verification commands recorded in plan |
| [docs/changes/2026-04-23-ci-powershell-contract/change.yaml](/D:/Data/20260415-space-sift/docs/changes/2026-04-23-ci-powershell-contract/change.yaml:1), [docs/changes/2026-04-23-ci-powershell-contract/explain-change.md](/D:/Data/20260415-space-sift/docs/changes/2026-04-23-ci-powershell-contract/explain-change.md:1) | Added the missing change-local pack for this CI-contract update. | The verify skill treats a missing ordinary change pack as blocker-level drift. | `verify` rules, docs/change-pack baseline | Change-local pack now exists and explains the actual diff |

## Tests Added Or Changed

No new product tests were added in this change.

That is intentional and appropriate for this scope:

- the workspace-navigation feature behavior was already covered by the approved
  test spec and the existing frontend and Rust suites;
- this change does not alter `R1` through `R26a`, Tauri contracts, or app
  runtime behavior; and
- the correct proof surface for a CI-entrypoint change is the canonical CI
  script run itself, not a new product test.

The verification still re-ran meaningful existing proof points:

- the PowerShell CI script executed the full branch-ready local check set;
- `workspace_restore_context` Rust tests were rerun directly to keep the
  persistence seam evidence fresh; and
- `bash -n scripts/ci.sh` proved the compatibility wrapper remains syntactically
  valid.

## Verification Evidence

Working directory for all commands: `D:\Data\20260415-space-sift`

- `powershell -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/ci.ps1`
  - passed
  - important output: `npm ci`, `npm run lint`, `npm run test`, `npm run build`,
    and `cargo check --manifest-path src-tauri/Cargo.toml` all completed
- `cargo test --manifest-path src-tauri/Cargo.toml -p app-db workspace_restore_context`
  - passed
  - important output: `4 passed; 0 failed`
- `cargo test --manifest-path src-tauri/Cargo.toml workspace_restore_context_command_boundary`
  - passed
  - important output: `2 passed; 0 failed`
- `bash -n scripts/ci.sh`
  - passed
- `git diff --check`
  - passed
  - important output: CRLF conversion warnings only

Additional branch-isolation evidence:

- unrelated local state was removed from the working tree before the final
  verify pass with:
  - `git stash push -u -m "verify-isolate-unrelated-2026-04-23"`
- the resulting stash remains available as `stash@{0}` for later recovery

Remote GitHub Actions status was not inspected here. The local CI-parity command
passed, and the workflow file now invokes that same command path.

## Alternatives Rejected

- Keep `bash scripts/ci.sh` as canonical and only harden the shell wrapper.
  Rejected because branch-ready verification for a Windows-targeted repository
  would still depend on fragile `bash` interop.
- Remove `scripts/ci.sh` entirely.
  Rejected because historical references and compatibility callers would fail
  abruptly; a wrapper keeps the transition explicit but not silent.
- Leave the workspace-navigation plan `active` until a later PR step.
  Rejected because `verify` had already completed and the plan/index mismatch
  was itself a blocker.
- Use a top-level `docs/explain/YYYY-MM-DD-*.md` artifact.
  Rejected because this is ordinary non-trivial change rationale and belongs in
  the `docs/changes/<change-id>/` pack.

## Scope Control

The change intentionally did not:

- alter any workspace-navigation product behavior;
- change Tauri commands, events, persistence schema, or frontend state logic;
- modify the release verification command or release workflow contract;
- introduce a new feature spec, test spec, or architecture decision; or
- reapply the unrelated local edits that were isolated into the stash.

This was a closeout and CI-contract alignment change only.

## Risks And Follow-Ups

- `scripts/ci.sh` now depends on PowerShell being available on PATH when used
  as a compatibility path. That is acceptable because it is no longer the
  canonical contract.
- Historical completed plans and older explain artifacts still mention the old
  `bash scripts/ci.sh` path as past-tense evidence. They were left unchanged
  because they describe prior repository state rather than the current
  contract.
- The isolated local stash `stash@{0}` should be restored later only if those
  unrelated changes are still needed.
- GitHub Actions was not observed directly in this explanation pass; the next
  PR stage should rely on an actual workflow run for remote CI evidence.

## PR-Ready Summary

- Canonical CI parity now uses `scripts/ci.ps1` locally and in
  `.github/workflows/ci.yml`.
- `scripts/ci.sh` remains only as an explicit compatibility wrapper.
- Governing docs and active plans now agree on the PowerShell CI contract.
- The workspace-navigation initiative is closed out as `done` with branch-wide
  verification recorded.

Readiness: this explanation is complete, the current local verify evidence is
recorded, and the change set is ready for `pr` preparation. This turn stops at
explanation because the request was isolated to `$explain-change`.
