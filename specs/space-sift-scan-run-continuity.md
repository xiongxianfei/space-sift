# Scan Run Continuity Specification (Optimized)

## Purpose

Define a deterministic, testable contract for scan-run continuity, visibility, and optional resumability in Space Sift. This spec keeps behavior changes minimal while making telemetry, retention, migration, and safety expectations explicit.

## Scope

- In scope: scan-run state tracking, snapshot persistence, staleness detection, progress telemetry, optional resume, and backward-compatible migration from existing scan-run records.
- Out of scope: changing core file discovery/parsing logic, changing scan algorithms, and introducing remote/cloud synchronization.

## Definitions

- **Scan run**: a single invocation of the scan feature, from `START` to terminal state.
- **Run snapshot**: a persisted progress record emitted during a scan run.
- **Checkpoint sequence (`seq`)**: strict total order identifier for snapshots in one run.
- **Heartbeat**: periodic run update emitted while a run is active.
- **Resume token**: opaque artifact that allows rehydrating a run from a point-in-progress.
- **Terminal states**: `COMPLETED`, `FAILED`, `CANCELLED`.

## Examples

### E1: `ABANDONED` run keeps resume metadata visible while engine resume is unsupported

Given a run with `resume_enabled = true`, valid persisted resume metadata, and no heartbeat for more than `abandon_after_hours`  
When startup recovery evaluates the run  
Then the run status is surfaced as `ABANDONED`  
And the read model still exposes `has_resume = true` and `can_resume = false`  
And the system does not auto-resume the run

### E2: Old `ABANDONED` run becomes purge-eligible when resume is no longer valid

Given an `ABANDONED` run older than `run_retention_days` with expired or otherwise invalid resume metadata  
When retention purge evaluates continuity rows  
Then the run is purge-eligible  
And purge may delete the continuity rows after deletion verification succeeds

## Functional Requirements

### R1: Scan run lifecycle continuity is always recorded

1. A scan run MUST persist a header record before processing begins with:
   - `run_id` (stable UUID v4),
   - `started_at` (UTC ISO8601),
   - initial `status = RUNNING`,
   - `target_id` (scan target discriminator),
   - optional `resume_enabled` flag.
2. The system MUST append snapshots during execution until a terminal state is reached.
3. The system MUST persist at least one terminal snapshot with `status` in `COMPLETED | FAILED | CANCELLED`.
4. A scan run MUST have exactly one current status in `RUNNING | STALE | ABANDONED | COMPLETED | FAILED | CANCELLED`.

### R2: Snapshot ordering is deterministic

1. Every snapshot MUST include:
   - `seq` as a strict, gapless, monotonically increasing integer starting at 1 for each run,
   - `snapshot_at` UTC ISO8601 timestamp.
2. Reads that expose progress MUST sort snapshots by (`seq`, then `snapshot_at`) and ignore any out-of-order inserts.
3. The latest state for a run MUST be the snapshot with highest `seq`.
4. If `seq` is missing or zero in legacy data, consumers MUST fall back to `(snapshot_at, created_at)` ordering only for display and migration compatibility.

### R3: Progress metrics are bounded and meaningful

1. Every snapshot MUST include monotonic counters:
   - `items_discovered`,
   - `items_scanned`,
   - `errors_count`.
2. For each adjacent snapshot pair, counters MUST be non-decreasing.
3. The derived value `scan_rate_items_per_sec` MUST be computed as:
   - `delta(items_scanned)/delta(snapshot_at_seconds)`,
   - and clamped to `0..1_000_000` for exported telemetry.
4. The scan completion percentage SHOULD remain in `[0, 100]`; values outside this range MUST be normalized before UI rendering.

### R4: Heartbeat cadence and staleness handling

1. The scanner MUST attempt to emit heartbeat snapshots on a fixed cadence:
   - `heartbeat_interval_seconds = 30` (default).
2. A heartbeat MUST be considered missing if not observed for:
   - `heartbeat_stale_after_seconds = 120` (default; 4 × interval).
3. A `RUNNING` run with no heartbeat in `heartbeat_stale_after_seconds`:
   - MUST transition to `STALE` exactly once,
   - MUST continue accepting manual resume/cancel actions while stale,
   - MUST not auto-advance to terminal status.
4. `STALE` duration must be tracked by `stale_since`.

### R5: Continuation after app crash/shutdown

1. On startup, unfinished runs with:
   - `status = RUNNING` and `last_snapshot_at` older than 24 hours MUST be surfaced as `ABANDONED`.
2. `ABANDONED` is a recovery status, not a terminal outcome.
3. `ABANDONED` runs:
   - MUST be excluded from auto-resume,
   - MAY still surface as having resume metadata while executable resume remains disabled under R6.
4. An unfinished run with `status = STALE` and `last_snapshot_at` younger than 24h MAY remain `STALE` and be shown as recoverable.
5. An unfinished run with `status = STALE` and `last_snapshot_at` older than 24h MAY transition to `ABANDONED`.
6. Default stale-run recovery grace:
   - `abandon_after_hours = 24`.
7. The system MUST record a recovery reason in an audit log.

### R6: Optional resumability is explicit and gated

1. Resumability is **feature-flagged** per run:
   - `resume_enabled` default is `false`.
2. When `resume_enabled = false`, the system MUST:
   - persist continuity snapshots,
   - omit `resume_token`,
   - treat resume UI/CLI actions as disabled.
3. Current release posture:
   - resume execution is not supported by the current scan engine,
   - read/list endpoints MUST expose only `has_resume` and `can_resume` for resume availability,
   - while engine resume capability is disabled, `can_resume` MUST be `false` even if persisted resume metadata exists,
   - `resume_scan_run(run_id)` MUST reject with `UNSUPPORTED_ENGINE` without creating a child run, mutating the original run, or starting traversal.
4. Future engine-supported posture:
   - when `resume_enabled = true`, the system persists a stable `resume_token` and `resume_payload` sufficient to continue from the last valid snapshot `seq`,
   - a valid resume MUST validate token freshness, target fingerprint, and privacy scope,
   - a valid resume MUST create a new child run (new `run_id`) linked by `resumed_from_run_id`,
   - the original run remains immutable except for audit evidence.
5. Manual resume metadata visibility MUST be computed independently from `STALE` versus `ABANDONED` status:
   - `has_resume = true` when persisted resume metadata exists for the run,
   - `ABANDONED` status alone MUST NOT clear `has_resume`.

### R7: Privacy, security, and retention

1. Snapshots MUST NOT store file contents or in-memory buffers.
2. Snapshots SHOULD store only:
   - normalized counters,
   - safe paths needed for diagnostics (path redaction permitted by config),
   - error categories.
3. Sensitive values (tokens, secrets, credentials) MUST be excluded from logs and snapshot payloads.
4. Default retention:
   - active metadata for in-progress runs: unlimited until terminal,
   - completed run snapshots: `30` days default (`run_retention_days`),
   - failed and cancelled run snapshots: the same terminal retention window as completed runs unless a stricter product rule is defined,
   - stale/abandoned run snapshots: purge after `30` days unless the run still surfaces as `can_resume = true`.
5. Purge eligibility MUST match the read-model resume contract; a run that still surfaces as resumable MUST NOT be deleted by retention.
6. Resume-related externally visible fields MUST be limited to `has_resume`, `can_resume`, and explicit rejection codes. Read/list APIs, UI models, logs, and audit payloads MUST NOT expose raw `resume_token`.
7. Purge process MUST include verification of deletion and emit a metric event.

### R8: API and UX consistency

1. All read/list endpoints must return:
   - run header,
   - latest ordered snapshot,
   - ordered snapshot preview (configured page size),
   - `has_resume` and `can_resume` booleans.
2. Endpoints must reject stale invalid `run_id` values with `404`.
3. UI/CLI summaries MUST:
   - expose `run_id`, `seq`, `status`, `created_at`, `items_scanned`, `errors_count`,
   - show progress as percent and rate.
4. UI/CLI summaries for `ABANDONED` runs MUST keep the recovery status visible even when `has_resume = true`.
5. `cancel_scan_run(runId)` and `resume_scan_run(runId)` MUST return machine-readable `NOT_FOUND` for unknown runs.
6. `cancel_scan_run(runId)` MUST return machine-readable `CONFLICT` when the latest run state is already terminal.
7. Resume rejection MUST return the explicit rejection code as the primary machine-readable failure code, including `UNSUPPORTED_ENGINE` when engine resume capability is disabled.

### R9: Failure handling

1. Any snapshot write failure MUST:
   - fail the current scan run as `FAILED` if unrecoverable,
   - include one terminal snapshot and error code,
   - preserve previous snapshots.
2. A scan cancellation request MUST:
   - stop processing at next safe checkpoint,
   - emit terminal snapshot with `status = CANCELLED`.
3. `cancel_scan_run(runId)` for a run whose latest state is `STALE` or `ABANDONED` MUST append a synthetic terminal snapshot with `status = CANCELLED`.

### R10: Migration compatibility

1. Existing pre-spec scans without `seq`/`status`/`resume` fields must not break.
2. Missing `seq`:
   - infer ordering by `(snapshot_at, created_at)` and backfill virtual `seq` for display only.
3. Missing `resume_token`:
   - treated as non-resumable run.
4. Schema additions SHOULD be optional so older consumers can parse unknown fields safely.

## Non-Goals

- Defining cloud/offline sync behavior.
- Redesigning scan task semantics.
- Any guaranteed deterministic re-scan output ordering across different storage backends.

## Acceptance Criteria (testable)

1. Given a running scan, snapshots are stored with `seq = 1, 2, 3, ...` in commit order.
2. Given a run whose latest snapshot is older than `heartbeat_stale_after_seconds`, system marks status `STALE` and records `stale_since`.
3. Given a run with no heartbeat in 24h, startup recovery marks run `ABANDONED` regardless of later manual resume eligibility.
4. Given `resume_enabled = false`, resume action is unavailable and no token is persisted.
5. Given engine resume support is disabled, list/read returns `can_resume = false` even when `has_resume = true`.
6. Given engine resume support is disabled, `resume_scan_run` returns `UNSUPPORTED_ENGINE` and does not create a child run.
7. Given a snapshot with decreasing counters, run rejects write and raises validation error.
8. Given old legacy data without `seq`, list/read remains usable with fallback ordering and does not error.
9. Given retention policy run, completed snapshots older than 30 days are removed and deletion is logged.
10. Given an `ABANDONED` run with valid resume metadata, the read model still exposes `has_resume = true` and `can_resume = false` while keeping status `ABANDONED`.
11. Given an unknown `run_id`, `cancel_scan_run` and `resume_scan_run` return machine-readable `NOT_FOUND`, and no raw `resume_token` is exposed through read/list APIs or audit payloads.
12. Future engine-supported acceptance:
    - Given engine resume support is enabled and target unchanged, resume creates a new child run and continues from latest valid `seq`.

## Open Defaults (explicitly fixed by this spec)

- `heartbeat_interval_seconds = 30`
- `heartbeat_stale_after_seconds = 120`
- `abandon_after_hours = 24`
- `run_retention_days = 30`
- `scan_rate_max_items_per_sec = 1_000_000`
