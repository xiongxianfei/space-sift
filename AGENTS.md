# AGENTS.md

This repository uses Codex to help maintain a public open source project.

Optimize for correctness, explicitness, small reviewable diffs, and alignment with the documented contract over speculative improvements.

## Instruction precedence

When instructions conflict, follow this order:

1. Direct user request
2. `CONSTITUTION.md`
3. Approved feature spec in `specs/`
4. Matching test spec in `specs/`
5. Active execution plan file in `docs/plans/`
6. `docs/workflows.md`
7. This file

Do not silently blend conflicting higher-priority instructions. Call out the conflict, explain the impact, and follow the highest-priority source that already implies the answer.

## Repository defaults

- Prefer the smallest change that fully satisfies the request.
- Do not add unrelated refactors while implementing a scoped task.
- Preserve user changes unless explicitly asked to revert them.
- When behavior changes, update the relevant spec, test spec, docs, or examples in the same change when this repository uses them.
- Reuse existing scripts and workflows before inventing new commands or processes.
- Keep `AGENTS.md` practical. Move workflow detail to `docs/workflows.md` and feature-specific detail to `specs/`.

## Planning and workflow

Use a plan first for work that is multi-file, risky, ambiguous, architecture-affecting, migration-heavy, or large enough that it should be split into reviewable milestones.

Use the default workflow for behavior-changing feature work:

`plan -> spec -> test-spec -> implement -> verify -> docs -> review`

Add `plan-review` before spec work when the task is risky, cross-cutting, or hard to sequence cleanly.

Use `bugfix` for bugs, `ci` for GitHub Actions or automation changes, and `pr` only when the branch is already ready for review.

## Plan file policy

- `docs/roadmap.md` stores future ideas and unapproved work.
- `docs/plan.md` is an index of active and closed execution plans. It is not the body of a plan.
- Every approved initiative gets its own living plan file under `docs/plans/YYYY-MM-DD-slug.md`.
- Never overwrite an older plan when starting a new initiative.
- If a new plan replaces an older one, keep the older file and mark it as superseded.
- Execution plans should follow `.codex/PLANS.md`.
- `docs/plan.md` and the plan body metadata must stay aligned on lifecycle state.
- A plan must not remain `draft` once implementation has started; known safe-progress blockers should be recorded as `blocked` promptly.

## Required reading before implementation

Before implementing behavior-changing work, read in this order when the files exist:

1. `docs/plan.md`, then the active plan file in `docs/plans/`
2. the relevant feature spec in `specs/<feature>.md`
3. the matching test spec in `specs/<feature>.test.md`
4. `docs/workflows.md` when the task touches an existing flow or release process
5. the files you expect to modify

If the work changes externally observable behavior and no relevant spec exists, create or request the missing spec before coding the contract into the implementation.

## Spec and test conventions

- `specs/<feature>.md` defines the contract: requirements, examples, edge cases, non-goals, compatibility expectations, and acceptance criteria.
- `specs/<feature>.test.md` maps requirements and edge cases to concrete tests.
- Every `MUST` in a spec should map to at least one test.
- The test spec does not override the feature spec; it operationalizes it.

## Implementation rules

- Keep diffs scoped.
- Write or update tests first when feasible.
- Run the smallest relevant verification scope first, then expand only as needed.
- If validation fails, stop and fix the failure before moving to the next milestone.
- Update the active plan's progress, decisions, discoveries, and validation notes as work proceeds.
- If a spec gap blocks safe implementation, state it explicitly instead of silently guessing.

## Verification expectations

Current repository verification commands from the repo root:

- `npm install`
- `npm run lint`
- `npm run test`
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `npm run tauri dev`
- `bash scripts/ci.sh`
- `bash scripts/release-verify.sh`

`bash scripts/ci.sh` is the canonical CI-parity command when branch-wide readiness matters.

If Rust or other Tauri prerequisites are missing on the local machine, state
that explicitly instead of claiming the desktop validation passed.

If local file locks, path permissions, or other environment issues block a verification command, report the exact command and error instead of implying a test failure.

## Change management

- Do not rewrite plan, spec, or workflow files unless the task requires it.
- Remove or challenge stale instructions when they no longer match reality.
- If a request conflicts with the current spec, ask whether the spec should change or the implementation should intentionally diverge only when the higher-priority sources do not already imply the answer.
- During PR preparation, verify the intended base branch and actual diff instead of assuming `main`, especially when review branches are stacked.
- Keep temporary verification directories and other local-only artifacts out of commits and PRs.

## Definition of done

A task is not done unless all of the following are true:

- the implementation matches the current contract
- relevant verification was run, or any inability to run it is stated clearly
- named edge cases and failure paths are handled or explicitly deferred
- the user-visible scope does not silently exceed what was agreed
- the active plan reflects what actually happened when a plan was used
- meaningful assumptions and open questions are called out in the final response
