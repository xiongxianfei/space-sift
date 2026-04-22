# Space Sift constitution

## 1) Project purpose

- The repository produces `Space Sift`, a Windows 11 desktop tool for safe local disk-space review and cleanup.
- Agents and contributors MUST optimize for correctness, explicit behavior contracts, and reviewable diffs before convenience-only changes.
- Scope drift into unrelated refactors is prohibited unless explicitly approved in the active plan.

## 2) Source-of-truth order

- 1. Direct user request.
- 2. `CONSTITUTION.md`.
- 3. Approved feature specs in `specs/*.md`.
- 4. Active architecture decisions in approved plans or spec-owned architecture notes where they define boundaries.
- 5. Matching test specs in `specs/*.test.md`.
- 6. Active plans in `docs/plan.md` and `docs/plans/*.md`.
- 7. `docs/workflows.md`.
- 8. `AGENTS.md`.
- 9. Reviewed code, generated outputs, and final chat context as tiebreakers only.
- When conflicts exist, the first matching higher-priority source MUST win.

## 3) Spec-driven rules

- Behavior-changing work MUST start from an existing approved feature spec.
- If no applicable spec exists for externally visible behavior, the agent MUST propose/spec the contract before implementation.
- Feature work MUST include edge cases, compatibility statements, and acceptance criteria before code changes.
- The required reading sequence MUST be:
  - `docs/plan.md`
  - the active plan file in `docs/plans/`
  - the matching feature spec
  - the matching test spec
  - `docs/workflows.md` when workflow changes are involved
  - files planned for edit
- Missing spec evidence MUST be called out in the final response as a follow-up dependency.

## 4) Test-driven rules

- Tests are required when behavior can be exercised by a test harness.
- Regressions MUST be added before or alongside bug fixes that change behavior.
- `bash scripts/ci.sh` MUST be treated as the canonical CI-parity gate when claiming a branch is ready for review or merge.
- Before claiming a scoped task is complete, agents MUST run the smallest relevant subset of:
  - `npm run lint`
  - `npm run test`
  - `npm run build`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Dependency or lockfile validation SHOULD prefer `npm ci`; `npm install` MAY be used for local bootstrap when CI-parity validation is not yet required.
- For release or publish-bound work, `bash scripts/release-verify.sh` MUST run before merge.
- Agents MUST NOT make unverifiable claims about verification; any blocked or skipped command MUST be explicitly reported.
- `MAY` reduce scope to the smallest relevant subset for tiny changes, but not below the minimum for confidence.

## 5) Architecture rules

- The Rust backend MUST own scan execution, cleanup execution, history persistence, and duplicate analysis.
- The React frontend MUST consume backend commands/events and SHOULD treat backend data as the source of truth.
- Persistence logic MUST remain in SQLite-backed paths already used by the project unless a migration plan is approved.
- File deletion and recycle-bin operations MUST remain behind explicit user action boundaries and confirmation flows.
- New cross-layer data contracts MUST be schema-reviewed by backend and frontend owners before merge.
- Data ownership, migration approach, and API shape MUST be documented in the active plan or spec.

## 6) Security and privacy rules

- Secrets, signing keys, and updater private keys MUST NEVER be checked into source control.
- Any command path that deletes files MUST remain explicit and auditable, preserving `Recycle Bin` as the default execution mode.
- Temporary logs MUST NOT print certificate material, private keys, or environment secrets.
- Any new dependency MUST be added with intent and justification; dependency risk MUST be considered in review.
- Agents MUST preserve existing safe defaults for protected paths and elevated privileges.

## 7) Compatibility rules

- Public behavior MUST preserve previous external behavior unless a spec permits breakage.
- API/event contracts and command names MUST remain backward-compatible unless deprecation is documented.
- Release versioning MUST stay synchronized across:
  - `package.json`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
- Config and storage migrations MUST be additive-first and reversible when feasible.
- Breaking migration steps MUST be documented in plan/spec and acceptance criteria.

## 8) Verification rules

- `.github/workflows/ci.yml` and `bash scripts/ci.sh` define the minimum CI contract for this project.
- `.github/workflows/release.yml` and `bash scripts/release-verify.sh` define the release-readiness contract.
- Agents MUST attach command-level verification status to final responses for completed work.
- If required commands cannot run due missing prerequisites, file locks, path-permission failures, or other local-environment constraints, the exact blocking command and error MUST be clearly stated.
- Temporary verification workarounds such as alternate target directories MAY be used for diagnosis, but they MUST NOT be committed or silently left out of the final report.
- Desktop/build verification MAY be skipped only when not requested, but the reason and impact MUST be documented.

## 9) Review rules

- Use the plan/spec/test-spec/verify lifecycle for cross-cutting or risky work by default.
- `plan-review`, `spec-review`, or `architecture-review` tasks MUST be requested when risk, sequence, or boundaries are unclear.
- PR preparation MUST inspect the working tree, current branch, intended base branch, and actual diff before opening or updating a PR.
- When stacked PRs are in use, agents MUST target the smallest correct base branch and MUST NOT assume `main`.
- Dirty worktrees and uncommitted generated or temporary verification artifacts MUST be called out before PR creation.
- Tiny one-file docs updates MAY skip formal proposals.
- Any conflict with existing instructions MUST be surfaced before implementation proceeds.

## 10) Documentation rules

- Behavior changes MUST update at least one of:
  - the relevant feature spec
  - the matching test spec
  - user-facing docs (`README.md`, `docs/release.md`, or equivalent)
- If release behavior changes, `docs/release.md` or equivalent MUST be updated.
- Plan progress, decision logs, and validation notes MUST remain current during active execution.
- `docs/plan.md` and the affected plan body MUST stay aligned when initiative status changes from draft, active, blocked, done, or superseded.
- Once implementation work has started, the initiative MUST NOT remain marked `draft`.
- If a known contract or artifact gap blocks safe progress, the initiative MUST be marked `blocked` as soon as that blocker is accepted.
- `AGENTS.md` should stay practical and point to deeper workflows/contracts.

## 11) Agent behavior rules

- Agents MUST call out assumptions and open questions instead of inferring unstated requirements.
- Agents MUST keep diffs scoped to user intent and avoid unrelated refactors.
- Agents MUST NOT silently revert user edits made outside the current scope.
- Agents MUST preserve user and local changes unless explicitly requested.
- Agents MUST update the relevant active plan when milestone progress or decisions change.
- Agents MUST keep temporary build outputs, verification scratch directories, and local-only artifacts out of commits and PRs.

## 12) Fast-lane exceptions

- One-file typo/docs fixes and narrowly scoped cleanup edits MAY skip plan/spec/test-spec if no behavior contract changes.
- Even in fast-lane mode, security and release-related boundaries MUST still be respected.
- Fast-lane work MUST include a concise risk note and a minimal verification command set.
