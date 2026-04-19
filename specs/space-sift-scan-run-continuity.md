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
4. A scan run MUST include one of:
   - `status = RUNNING` until explicitly terminal,
   - `status = STALE` when heartbeat cadence is violated (see R4),
   - terminal state after explicit completion/failure/cancellation.

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
   - `status = RUNNING` and `last_snapshot_at` older than 24 hours MUST be surfaced as `ABANDONED`,
   - and must be excluded from auto-resume unless resume is explicitly enabled (R6).
2. An unfinished run with `status = STALE` and `last_snapshot_at` younger than 24h MAY remain `STALE` and be shown as recoverable.
3. Default stale-run recovery grace:
   - `abandon_after_hours = 24`.
4. The system MUST record a recovery reason in an audit log.

### R6: Optional resumability is explicit and gated

1. Resumability is **feature-flagged** per run:
   - `resume_enabled` default is `false`.
2. When `resume_enabled = false`, the system MUST:
   - persist continuity snapshots,
   - omit `resume_token`,
   - treat resume UI/CLI actions as disabled.
3. When `resume_enabled = true`, the system MUST:
   - persist a stable `resume_token`,
   - persist `resume_payload` sufficient to continue from last snapshot `seq`,
   - verify token freshness against current target fingerprint before resume.
4. If restore conditions fail, resume MUST:
   - reject with explicit reason (`TARGET_CHANGED`, `PRIVACY_SCOPE_MISMATCH`, `TOKEN_EXPIRED`),
   - leave the original run immutable.
5. Any resume operation MUST create a new child run (new `run_id`) and link it to `resumed_from_run_id`.

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
   - stale/abandoned run snapshots: purge after `30` days.
5. Purge process MUST include verification of deletion and emit a metric event.

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

### R9: Failure handling

1. Any snapshot write failure MUST:
   - fail the current scan run as `FAILED` if unrecoverable,
   - include one terminal snapshot and error code,
   - preserve previous snapshots.
2. A scan cancellation request MUST:
   - stop processing at next safe checkpoint,
   - emit terminal snapshot with `status = CANCELLED`.

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
3. Given a run with no heartbeat in 24h, startup recovery marks run `ABANDONED` unless explicit resume was enabled and valid.
4. Given `resume_enabled = false`, resume action is unavailable and no token is persisted.
5. Given resume enabled and target unchanged, resume creates a new run and continues from latest valid `seq`.
6. Given a snapshot with decreasing counters, run rejects write and raises validation error.
7. Given old legacy data without `seq`, list/read remains usable with fallback ordering and does not error.
8. Given retention policy run, completed snapshots older than 30 days are removed and deletion is logged.

## Open Defaults (explicitly fixed by this spec)

- `heartbeat_interval_seconds = 30`
- `heartbeat_stale_after_seconds = 120`
- `abandon_after_hours = 24`
- `run_retention_days = 30`
- `scan_rate_max_items_per_sec = 1_000_000`

