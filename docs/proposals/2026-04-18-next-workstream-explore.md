# Exploration: Next workstream for Space Sift after governance updates (assumption-based)

## 1) Problem (assumption-based restatement)

The repository has active UX/scan plans and a complete release-contract baseline. The practical question is which workstream should be prioritized next to maximize user value without weakening safety: finish active UX improvements, optimize runtime behavior, or invest in platform-level architecture.

## 2) Stakeholders and affected journeys

- End users who run large scans want faster, more predictable progress telemetry and reliable duplicate triage.
- Contributors need a clear plan order and predictable release-safe boundaries.
- Release/maintenance users need stable versioning, signed artifacts, and low-risk changes.
- QA depends on deterministic behavior and observable correctness checks.

## 3) Facts, assumptions, unknowns

Facts:
- Active plans are centered on scan UX clarity and history/duplicate review clarity.
- Release hardening contract exists and is already codified in specs and scripts.
- No `docs/project-map.md` exists, and no `docs/proposals` directory exists yet.
- Active roadmap flags installer/signing hardening as next after MVP core flow.

Assumptions:
- The team wants a visible product gain before deeper architecture experiments.
- No urgent security incident currently blocks normal release or scan flows.
- Safety defaults (`recycle bin` first, preview-first cleanup, protected-path fail-closed) must stay unchanged.

Unknowns:
- Whether duplicate-analysis and scan-telemetry latency are the top user pain points after current MVP milestones.
- Whether maintainers prioritize shipping UX completion or platform acceleration first.
- Available Windows test matrix and automation time for high-risk experiments.

## 4) Option set

### O0: Do nothing / defer

Core idea: stop at current state and ship what is already implemented.
User value: zero new value.
Implementation complexity: none.
Architecture impact: none.
Testing burden: minimal.
Rollout/rollback: none.
Risks: user pain from slow scans, duplicate ambiguity, or unclear active/recent context remains; momentum loss.
What would make this option wrong: any visible user pain or maintainer expectation for progress in current cycle.

### O1: Minimal safe change: finish active plan milestones only

Core idea: complete `2026-04-16-scan-progress-and-active-run-ux` and `2026-04-16-history-and-duplicate-review-clarity` without changing major architecture.
User value: immediate and measurable quality improvements in clarity and trust.
Implementation complexity: low to medium.
Architecture impact: low; largely UI and state wiring.
Testing burden: moderate; existing frontend/contract checks plus manual UX verification.
Rollout/rollback: straightforward and bounded to current screens.
Risks: improves ergonomics but may not address scan throughput concerns.
What would make this option wrong: users care more about raw speed and not only clarity.

### O2: Incremental product improvement: add resumable scan queue + persisted active-run cards

Core idea: add lightweight scan-session persistence and explicit active-run state persistence so interrupted/closed sessions can be resumed safely.
User value: stronger recovery behavior and fewer repeated full rescans.
Implementation complexity: medium.
Architecture impact: moderate, with event/state contracts touching backend/frontend sync.
Testing burden: medium-high for state transition and replay correctness.
Rollout/rollback: possible behind a feature flag or schema-guarded migration.
Risks: stale sessions if scan roots or ignore rules change between launches.
What would make this option wrong: complexity exceeds UX-only improvements with little observed gain.

### O3: Architectural/platform option: metadata+queue rework for scan throughput

Core idea: evolve scan loop into a bounded producer-consumer scheduling model with checkpointed progress and lower event churn.
User value: better responsiveness on large trees and fewer UI stalls.
Architecture impact: medium-high in backend Rust engine and frontend consumption model.
Testing burden: high (load, cancellation, and resume race paths).
Rollout/rollback: medium risk; requires migration around in-flight scan event shape.
Risks: regressions in progress ordering, cancellation determinism, and duplicate-result alignment.
What would make this option wrong: measurable throughput gain is low and changes destabilize current safe scan path.

### O4: High-risk / high-upside: platform-aware index daemon

Core idea: introduce optional pre-index/USN changelog + incremental analysis layer outside the main run.
User value: near-instant duplicate recalc for unchanged trees and much lower repeated-walk cost.
Implementation complexity: high.
Architecture impact: high; new local service-like component and new persistence/migration layer.
Testing burden: high; end-to-end regression matrix required.
Rollout/rollback: staged, hard to revert quickly once enabled.
Risks: high platform dependency risk, permissions exposure, and operational overhead.
What would make this option wrong: security review flags local service risk or maintainers cannot support the operational burden.

## 5) Option comparison and recommendation

Decision criteria:
- User value (primary)
- Safety regression risk
- Time-to-deliver for current cycle
- Operability for maintainers
- Compatibility with existing contracts

Recommendation:
- Recommended sequence: O1 first, O2 second, defer O3 unless O1 data suggests scan runtime pain remains above target.
- O4 is not recommended for this cycle because it changes operating model too much before proving demand.
- Suggested staged gate: require user/review feedback that O1 is sufficient before any O2/O3 expansion.

## 6) Research questions before proposal/spec

- What is the measured pain mix between perceived speed and perceived clarity from existing users?
- How often do users restart interrupted scans due to crashes, reboots, or workspace changes?
- What is the safe baseline for event frequency and memory growth under very large folders?
- Can active-run persistence be introduced without weakening deduplication or duplicate preview guarantees?
- What Windows environments (VM, SMB-mounted folders, symlink-heavy trees) are mandatory for release confidence?

## 7) Readiness statement

This exploration is ready to hand off to `research` or `proposal` once a definitive objective is selected between O1 and O2.
