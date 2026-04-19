# ADR-20260418-scan-run-persistence-and-resume: Persist scan run continuity in additive SQLite tables

## Status

Proposed

## Context

Space Sift currently persists only completed scan payloads in `scan_history` and keeps live progress only in memory through `ScanManager`. The optimized scan continuity spec now requires:

- durable ordered run snapshots
- stale and abandoned run detection after restart
- optional resumability with explicit gating
- retention and auditability

The repository constitution also requires persistence to remain on the existing SQLite-backed local path unless a migration plan is approved. At the same time, duplicate analysis, cleanup preview, and history reopen flows already rely on the current completed scan payload model and should not be destabilized.

## Decision

Persist scan run continuity in additive tables inside the existing local SQLite database.

The chosen shape is:

- keep `scan_history` as the completed-result store
- add `scan_runs` for current run header state
- add `scan_run_snapshots` for append-only ordered snapshots
- add `scan_run_audit` for recovery and purge events
- reuse the current externally visible `scan_id` UUID as the canonical run identifier
- make the latest persisted snapshot authoritative and treat `scan_runs` as a denormalized mirror for indexed reads
- append synthetic `STALE` and `ABANDONED` snapshots during reconciliation instead of storing those states only in the header row
- own heartbeat generation in the command/runtime layer rather than inside `scan-core`
- finalize `COMPLETED` state and `scan_history` persistence in one SQLite transaction
- make resume opt-in per run and create a new child run when resuming rather than mutating the original run

## Alternatives considered

### Reuse only `scan_history`

Rejected because `scan_history` is terminal-result storage and should remain stable for explorer, duplicate, and cleanup flows. Mixing in-progress and terminal concepts would create unnecessary migration risk.

### Store only one latest snapshot per run

Rejected because the spec requires ordered snapshot previews, deterministic ordering, and better auditability around stale transitions and failures.

### Reconcile stale or abandoned state only in `scan_runs`

Rejected because that would let the header status diverge from the latest snapshot returned by the same API and would make state history ambiguous.

### Let `scan-core` own timer-based heartbeats

Rejected because the current traversal is synchronous and may block inside long filesystem calls, making timer-driven liveness unreliable if it depends on the scanner loop itself.

### Resume in place on the original run

Rejected because it erases the original stale or failed timeline and makes run history harder to reason about. A child-run model preserves lineage and keeps rollback simpler.

## Consequences

Positive:

- additive rollout with low downgrade risk
- current completed-scan consumers remain stable
- run continuity features get a clear repository boundary
- stale recovery and retention become explicit and testable
- run APIs get a single explainable status source
- completed runs cannot be exposed before the completed payload is durably reopenable

Negative:

- scan completion becomes a dual-write path
- reconciliation and heartbeat transitions add more snapshot rows than a header-only design
- `HistoryStore` grows new responsibilities until or unless a narrower repository abstraction is extracted
- the UI will need a separate run-oriented read model in addition to the existing live progress snapshot

## Follow-up

- run `architecture-review` on the companion architecture note
- create an execution plan and test spec before implementation
- keep first-pass UI defaults conservative: advanced resume checkbox, explicit `ABANDONED` badge, and command-response refresh for non-live cancellation
