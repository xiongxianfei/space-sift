use chrono::{DateTime, Utc};
use cleanup_core::CleanupExecutionResult;
use duplicates_core::{
    CachedHashes, DuplicateAnalysisFailure, HashCache, HashCacheKey, HashCacheWrite,
};
use scan_core::{
    CompletedScan, ScanHistoryEntry, ScanRunDetail, ScanRunHeader, ScanRunSnapshot,
    ScanRunStatus, ScanRunSummary, DEFAULT_SCAN_RUN_PRIVACY_SCOPE_ID,
    SCAN_RESUME_ENGINE_SUPPORTED,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone)]
pub struct HistoryStore {
    db_path: PathBuf,
    now: Arc<dyn Fn() -> String + Send + Sync>,
    before_completed_scan_history_write:
        Option<Arc<dyn Fn() -> Result<(), HistoryStoreError> + Send + Sync>>,
}

const DEFAULT_SCAN_RUN_PREVIEW_PAGE_SIZE: u32 = 20;
const HEARTBEAT_STALE_AFTER_SECONDS: i64 = 120;
const ABANDON_AFTER_SECONDS: i64 = 24 * 60 * 60;
const RUN_RETENTION_DAYS: i64 = 30;
const RECONCILE_EVENT_TYPE: &str = "reconciled";
const CANCEL_EVENT_TYPE: &str = "cancelled";
const RESUME_REJECTED_EVENT_TYPE: &str = "resume_rejected";
const PURGE_EVENT_TYPE: &str = "purged";
const REASON_HEARTBEAT_STALE: &str = "HEARTBEAT_STALE";
const REASON_RUN_ABANDONED: &str = "RUN_ABANDONED";
const REASON_USER_CANCELLED: &str = "USER_CANCELLED";
const REASON_RETENTION_EXPIRED: &str = "RETENTION_EXPIRED";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurgedScanRuns {
    pub purged_count: usize,
    pub deleted_run_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ScanRunStartOptions<'a> {
    pub current_path: Option<&'a str>,
    pub target_id: Option<&'a str>,
    pub resumed_from_run_id: Option<&'a str>,
    pub resume_enabled: bool,
    pub resume_token: Option<&'a str>,
    pub resume_expires_at: Option<&'a str>,
    pub resume_payload_json: Option<&'a str>,
    pub resume_target_fingerprint_json: Option<&'a str>,
    pub privacy_scope_id: Option<&'a str>,
}

impl HistoryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_now(path, scan_core::current_timestamp)
    }

    pub fn with_now(
        path: impl Into<PathBuf>,
        now: impl Fn() -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            db_path: path.into(),
            now: Arc::new(now),
            before_completed_scan_history_write: None,
        }
    }

    #[cfg(test)]
    pub fn with_test_finalize_hook(
        path: impl Into<PathBuf>,
        now: impl Fn() -> String + Send + Sync + 'static,
        hook: impl Fn() -> Result<(), HistoryStoreError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            db_path: path.into(),
            now: Arc::new(now),
            before_completed_scan_history_write: Some(Arc::new(hook)),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn initialize(&self) -> Result<(), HistoryStoreError> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        }

        let connection = self.open_connection()?;
        connection
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS scan_history (
                    scan_id TEXT PRIMARY KEY,
                    root_path TEXT NOT NULL,
                    completed_at TEXT NOT NULL,
                    total_bytes INTEGER NOT NULL,
                    scan_json TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS scan_runs (
                    run_id TEXT PRIMARY KEY,
                    target_id TEXT NOT NULL,
                    root_path TEXT NOT NULL,
                    status TEXT NOT NULL,
                    started_at TEXT NOT NULL,
                    last_snapshot_at TEXT NOT NULL,
                    last_progress_at TEXT NOT NULL,
                    stale_since TEXT,
                    terminal_at TEXT,
                    completed_scan_id TEXT,
                    resumed_from_run_id TEXT,
                    resume_enabled INTEGER NOT NULL DEFAULT 0,
                    resume_token TEXT,
                    resume_expires_at TEXT,
                    resume_payload_json TEXT,
                    resume_target_fingerprint_json TEXT,
                    privacy_scope_id TEXT,
                    error_code TEXT,
                    error_message TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    latest_seq INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS scan_run_snapshots (
                    run_id TEXT NOT NULL,
                    seq INTEGER NOT NULL,
                    snapshot_at TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    status TEXT NOT NULL,
                    files_discovered INTEGER NOT NULL,
                    directories_discovered INTEGER NOT NULL,
                    items_discovered INTEGER NOT NULL,
                    items_scanned INTEGER NOT NULL,
                    errors_count INTEGER NOT NULL,
                    bytes_processed INTEGER NOT NULL,
                    scan_rate_items_per_sec REAL NOT NULL,
                    progress_percent REAL,
                    current_path TEXT,
                    message TEXT,
                    PRIMARY KEY (run_id, seq),
                    FOREIGN KEY (run_id) REFERENCES scan_runs(run_id) ON DELETE CASCADE
                );

                CREATE INDEX IF NOT EXISTS idx_scan_runs_status_last_snapshot
                ON scan_runs(status, last_snapshot_at);

                CREATE INDEX IF NOT EXISTS idx_scan_runs_started_at
                ON scan_runs(started_at DESC);

                CREATE INDEX IF NOT EXISTS idx_scan_runs_resumed_from
                ON scan_runs(resumed_from_run_id);

                CREATE INDEX IF NOT EXISTS idx_scan_run_snapshots_latest
                ON scan_run_snapshots(run_id, seq DESC);

                CREATE TABLE IF NOT EXISTS scan_run_audit (
                    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    run_id TEXT,
                    event_type TEXT NOT NULL,
                    reason_code TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    event_json TEXT NOT NULL,
                    FOREIGN KEY (run_id) REFERENCES scan_runs(run_id) ON DELETE CASCADE
                );

                CREATE TABLE IF NOT EXISTS duplicate_hash_cache (
                    path TEXT NOT NULL,
                    size_bytes INTEGER NOT NULL,
                    modified_at_millis INTEGER NOT NULL,
                    partial_hash TEXT,
                    full_hash TEXT,
                    PRIMARY KEY (path, size_bytes, modified_at_millis)
                );

                CREATE TABLE IF NOT EXISTS cleanup_execution_history (
                    execution_id TEXT PRIMARY KEY,
                    preview_id TEXT NOT NULL,
                    mode TEXT NOT NULL,
                    completed_at TEXT NOT NULL,
                    completed_count INTEGER NOT NULL,
                    failed_count INTEGER NOT NULL,
                    execution_json TEXT NOT NULL
                );
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        Ok(())
    }

    pub fn record_scan_run_started(
        &self,
        run_id: &str,
        root_path: &str,
        started_at: &str,
        current_path: Option<&str>,
    ) -> Result<ScanRunDetail, HistoryStoreError> {
        self.record_scan_run_started_with_options(
            run_id,
            root_path,
            started_at,
            ScanRunStartOptions {
                current_path,
                ..ScanRunStartOptions::default()
            },
        )
    }

    pub fn record_scan_run_started_with_options(
        &self,
        run_id: &str,
        root_path: &str,
        started_at: &str,
        options: ScanRunStartOptions<'_>,
    ) -> Result<ScanRunDetail, HistoryStoreError> {
        self.initialize()?;

        let created_at = self.now_timestamp();
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        transaction
            .execute(
                r#"
                INSERT INTO scan_runs (
                    run_id,
                    target_id,
                    root_path,
                    status,
                    started_at,
                    last_snapshot_at,
                    last_progress_at,
                    stale_since,
                    terminal_at,
                    completed_scan_id,
                    resumed_from_run_id,
                    resume_enabled,
                    resume_token,
                    resume_expires_at,
                    resume_payload_json,
                    resume_target_fingerprint_json,
                    privacy_scope_id,
                    error_code,
                    error_message,
                    created_at,
                    updated_at,
                    latest_seq
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL, NULL, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, NULL, NULL, ?15, ?16, 1
                );
                "#,
                rusqlite::params![
                    run_id,
                    options.target_id.unwrap_or(root_path),
                    root_path,
                    scan_run_status_label(&ScanRunStatus::Running),
                    started_at,
                    started_at,
                    started_at,
                    options.resumed_from_run_id,
                    options.resume_enabled,
                    options.resume_token,
                    options.resume_expires_at,
                    options.resume_payload_json,
                    options.resume_target_fingerprint_json,
                    options
                        .privacy_scope_id
                        .unwrap_or(DEFAULT_SCAN_RUN_PRIVACY_SCOPE_ID),
                    created_at,
                    created_at
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        transaction
            .execute(
                r#"
                INSERT INTO scan_run_snapshots (
                    run_id,
                    seq,
                    snapshot_at,
                    created_at,
                    status,
                    files_discovered,
                    directories_discovered,
                    items_discovered,
                    items_scanned,
                    errors_count,
                    bytes_processed,
                    scan_rate_items_per_sec,
                    progress_percent,
                    current_path,
                    message
                ) VALUES (?1, 1, ?2, ?3, ?4, 0, 0, 0, 0, 0, 0, 0.0, NULL, ?5, NULL);
                "#,
                rusqlite::params![
                    run_id,
                    started_at,
                    created_at,
                    scan_run_status_label(&ScanRunStatus::Running),
                    options.current_path
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        self.open_scan_run(run_id)
    }

    pub fn append_scan_run_snapshot(
        &self,
        snapshot: &ScanRunSnapshot,
    ) -> Result<ScanRunDetail, HistoryStoreError> {
        self.append_scan_run_snapshot_with_error_code(snapshot, None)
    }

    pub fn append_scan_run_snapshot_with_error_code(
        &self,
        snapshot: &ScanRunSnapshot,
        error_code: Option<&str>,
    ) -> Result<ScanRunDetail, HistoryStoreError> {
        self.initialize()?;

        let created_at = self.now_timestamp();
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let previous = load_latest_scan_run_snapshot_in_transaction(&transaction, &snapshot.run_id)?;
        let expected = previous.seq.saturating_add(1);
        if snapshot.seq != expected {
            return Err(HistoryStoreError::InvalidScanRunSequence {
                run_id: snapshot.run_id.clone(),
                expected,
                actual: snapshot.seq,
            });
        }
        ensure_counter_not_regressive(
            &snapshot.run_id,
            "files_discovered",
            previous.files_discovered,
            snapshot.files_discovered,
        )?;
        ensure_counter_not_regressive(
            &snapshot.run_id,
            "directories_discovered",
            previous.directories_discovered,
            snapshot.directories_discovered,
        )?;
        ensure_counter_not_regressive(
            &snapshot.run_id,
            "items_discovered",
            previous.items_discovered,
            snapshot.items_discovered,
        )?;
        ensure_counter_not_regressive(
            &snapshot.run_id,
            "items_scanned",
            previous.items_scanned,
            snapshot.items_scanned,
        )?;
        ensure_counter_not_regressive(
            &snapshot.run_id,
            "errors_count",
            previous.errors_count,
            snapshot.errors_count,
        )?;
        ensure_counter_not_regressive(
            &snapshot.run_id,
            "bytes_processed",
            previous.bytes_processed,
            snapshot.bytes_processed,
        )?;
        let current_last_progress_at =
            load_scan_run_last_progress_at_in_transaction(&transaction, &snapshot.run_id)?;

        transaction
            .execute(
                r#"
                INSERT INTO scan_run_snapshots (
                    run_id,
                    seq,
                    snapshot_at,
                    created_at,
                    status,
                    files_discovered,
                    directories_discovered,
                    items_discovered,
                    items_scanned,
                    errors_count,
                    bytes_processed,
                    scan_rate_items_per_sec,
                    progress_percent,
                    current_path,
                    message
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15);
                "#,
                rusqlite::params![
                    snapshot.run_id,
                    i64::try_from(snapshot.seq)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    snapshot.snapshot_at,
                    created_at,
                    scan_run_status_label(&snapshot.status),
                    i64::try_from(snapshot.files_discovered)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(snapshot.directories_discovered)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(snapshot.items_discovered)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(snapshot.items_scanned)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(snapshot.errors_count)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(snapshot.bytes_processed)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    snapshot.scan_rate_items_per_sec,
                    snapshot.progress_percent,
                    snapshot.current_path,
                    snapshot.message
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        let finished_at = terminal_finished_at(snapshot);
        let last_progress_at =
            next_last_progress_at(&current_last_progress_at, &previous, snapshot);

        transaction
            .execute(
                r#"
                UPDATE scan_runs
                SET status = ?2,
                    last_snapshot_at = ?3,
                    last_progress_at = ?4,
                    stale_since = ?5,
                    terminal_at = ?6,
                    error_code = ?7,
                    error_message = ?8,
                    updated_at = ?9,
                    latest_seq = ?10,
                    resume_payload_json = CASE
                        WHEN resume_enabled = 1 THEN ?11
                        ELSE resume_payload_json
                    END
                WHERE run_id = ?1;
                "#,
                rusqlite::params![
                    snapshot.run_id,
                    scan_run_status_label(&snapshot.status),
                    snapshot.snapshot_at,
                    last_progress_at,
                    next_stale_since(&transaction, snapshot)?,
                    finished_at,
                    error_code,
                    snapshot.message,
                    created_at,
                    i64::try_from(snapshot.seq)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    build_resume_payload_json(snapshot.current_path.as_deref(), snapshot.seq)
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        self.open_scan_run(&snapshot.run_id)
    }

    pub fn finalize_completed_scan_run(
        &self,
        completed_scan: &CompletedScan,
        current_path: Option<&str>,
        message: Option<&str>,
    ) -> Result<ScanRunDetail, HistoryStoreError> {
        self.initialize()?;

        let created_at = self.now_timestamp();
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let previous = load_latest_scan_run_snapshot_in_transaction(&transaction, &completed_scan.scan_id)?;
        let next_seq = previous.seq.saturating_add(1);
        let items_scanned = completed_scan.total_files.saturating_add(completed_scan.total_directories);
        let current_last_progress_at =
            load_scan_run_last_progress_at_in_transaction(&transaction, &completed_scan.scan_id)?;
        let last_progress_at = if completed_scan.total_files > previous.files_discovered
            || completed_scan.total_directories > previous.directories_discovered
            || items_scanned > previous.items_scanned
            || completed_scan.total_bytes > previous.bytes_processed
        {
            completed_scan.completed_at.clone()
        } else {
            current_last_progress_at
        };

        transaction
            .execute(
                r#"
                INSERT INTO scan_run_snapshots (
                    run_id,
                    seq,
                    snapshot_at,
                    created_at,
                    status,
                    files_discovered,
                    directories_discovered,
                    items_discovered,
                    items_scanned,
                    errors_count,
                    bytes_processed,
                    scan_rate_items_per_sec,
                    progress_percent,
                    current_path,
                    message
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15);
                "#,
                rusqlite::params![
                    completed_scan.scan_id,
                    i64::try_from(next_seq)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    completed_scan.completed_at,
                    created_at,
                    scan_run_status_label(&ScanRunStatus::Completed),
                    i64::try_from(completed_scan.total_files)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(completed_scan.total_directories)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(items_scanned)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(items_scanned)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(previous.errors_count)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    i64::try_from(completed_scan.total_bytes)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    0.0_f64,
                    100.0_f64,
                    current_path.or(previous.current_path.as_deref()),
                    message.or(Some("Scan complete."))
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        transaction
            .execute(
                r#"
                UPDATE scan_runs
                SET status = ?2,
                    last_snapshot_at = ?3,
                    last_progress_at = ?4,
                    terminal_at = ?5,
                    completed_scan_id = ?6,
                    error_code = NULL,
                    error_message = NULL,
                    updated_at = ?7,
                    latest_seq = ?8
                WHERE run_id = ?1;
                "#,
                rusqlite::params![
                    completed_scan.scan_id,
                    scan_run_status_label(&ScanRunStatus::Completed),
                    completed_scan.completed_at,
                    last_progress_at,
                    completed_scan.completed_at,
                    completed_scan.scan_id,
                    created_at,
                    i64::try_from(next_seq)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        if let Some(hook) = &self.before_completed_scan_history_write {
            hook()?;
        }

        persist_completed_scan_in_transaction(&transaction, completed_scan)?;
        transaction
            .commit()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        self.open_scan_run(&completed_scan.scan_id)
    }

    pub fn save_completed_scan(&self, scan: &CompletedScan) -> Result<(), HistoryStoreError> {
        self.initialize()?;

        let connection = self.open_connection()?;
        persist_completed_scan_in_connection(&connection, scan)?;

        Ok(())
    }

    pub fn list_history(&self) -> Result<Vec<ScanHistoryEntry>, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT scan_id, root_path, completed_at, total_bytes
                FROM scan_history
                ORDER BY completed_at DESC;
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                Ok(ScanHistoryEntry {
                    scan_id: row.get(0)?,
                    root_path: row.get(1)?,
                    completed_at: row.get(2)?,
                    total_bytes: u64::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
                })
            })
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))
    }

    pub fn open_history_entry(&self, scan_id: &str) -> Result<CompletedScan, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let payload = connection.query_row(
            "SELECT scan_json FROM scan_history WHERE scan_id = ?1;",
            rusqlite::params![scan_id],
            |row| row.get::<_, String>(0),
        );

        match payload {
            Ok(payload) => serde_json::from_str(&payload)
                .map_err(|error| HistoryStoreError::Persistence(error.to_string())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(HistoryStoreError::NotFound {
                scan_id: scan_id.to_string(),
            }),
            Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
        }
    }

    pub fn list_scan_runs(&self) -> Result<Vec<ScanRunSummary>, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let now = self.now_timestamp();
        let mut statement = connection
            .prepare(
                r#"
                SELECT run_id
                FROM scan_runs
                ORDER BY last_snapshot_at DESC, started_at DESC;
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?
            .into_iter()
            .map(|run_id| load_scan_run_summary(&connection, &run_id, &now))
            .collect()
    }

    pub fn open_scan_run(&self, run_id: &str) -> Result<ScanRunDetail, HistoryStoreError> {
        self.open_scan_run_paged(run_id, 1, DEFAULT_SCAN_RUN_PREVIEW_PAGE_SIZE)
    }

    pub fn record_scan_run_resume_rejection(
        &self,
        run_id: &str,
        reason_code: &str,
        timestamp: &str,
    ) -> Result<(), HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let event_json = serde_json::to_string(&serde_json::json!({
            "runId": run_id,
            "reasonCode": reason_code,
            "timestamp": timestamp,
        }))
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        connection
            .execute(
                r#"
                INSERT INTO scan_run_audit (
                    run_id,
                    event_type,
                    reason_code,
                    created_at,
                    event_json
                ) VALUES (?1, ?2, ?3, ?4, ?5);
                "#,
                rusqlite::params![
                    run_id,
                    RESUME_REJECTED_EVENT_TYPE,
                    reason_code,
                    timestamp,
                    event_json
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        Ok(())
    }

    pub fn open_scan_run_paged(
        &self,
        run_id: &str,
        page: u32,
        page_size: u32,
    ) -> Result<ScanRunDetail, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let now = self.now_timestamp();
        load_scan_run_detail(&connection, run_id, page, page_size, &now)
    }

    pub fn reconcile_scan_runs(&self) -> Result<(), HistoryStoreError> {
        self.initialize()?;
        let now = self.now_timestamp();
        let mut connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT run_id, status, last_snapshot_at
                FROM scan_runs
                WHERE status IN ('running', 'stale')
                ORDER BY last_snapshot_at ASC;
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let candidates = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        drop(statement);

        for (run_id, status, last_snapshot_at) in candidates {
            let Some((next_status, reason_code, message)) =
                reconciliation_transition(&status, &last_snapshot_at, &now)
            else {
                continue;
            };

            let transaction = connection
                .transaction()
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
            append_reconciliation_snapshot(
                &transaction,
                &run_id,
                next_status,
                &now,
                reason_code,
                message,
            )?;
            transaction
                .commit()
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        }

        Ok(())
    }

    pub fn cancel_non_live_scan_run(
        &self,
        run_id: &str,
    ) -> Result<ScanRunDetail, HistoryStoreError> {
        self.initialize()?;
        let now = self.now_timestamp();
        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let previous = load_latest_scan_run_snapshot_in_transaction(&transaction, run_id)?;

        match previous.status {
            ScanRunStatus::Stale | ScanRunStatus::Abandoned => {}
            status => {
                return Err(HistoryStoreError::Conflict {
                    run_id: run_id.to_string(),
                    status: scan_run_status_label(&status).to_string(),
                })
            }
        }

        append_cancelled_snapshot(&transaction, run_id, &now, &previous)?;
        transaction
            .commit()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        self.open_scan_run(run_id)
    }

    pub fn purge_expired_scan_runs(&self) -> Result<PurgedScanRuns, HistoryStoreError> {
        self.initialize()?;
        let now = self.now_timestamp();
        let cutoff = retention_cutoff_timestamp(&now, RUN_RETENTION_DAYS)?;
        let mut connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT run_id, terminal_at
                FROM scan_runs
                WHERE status IN ('completed', 'cancelled', 'failed', 'abandoned')
                  AND terminal_at IS NOT NULL
                  AND terminal_at <= ?1
                ORDER BY terminal_at ASC;
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let candidates = statement
            .query_map(rusqlite::params![cutoff], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        drop(statement);

        let mut purgeable = Vec::with_capacity(candidates.len());
        for (run_id, terminal_at) in candidates {
            let header = load_scan_run_header(&connection, &run_id)?;
            let (_, can_resume) = scan_run_resume_flags(&header, &now);
            if can_resume {
                continue;
            }

            purgeable.push((run_id, terminal_at));
        }

        if purgeable.is_empty() {
            return Ok(PurgedScanRuns {
                purged_count: 0,
                deleted_run_ids: Vec::new(),
            });
        }

        let transaction = connection
            .transaction()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let mut deleted_run_ids = Vec::with_capacity(purgeable.len());

        for (run_id, terminal_at) in &purgeable {
            preserve_purged_run_audit_rows(&transaction, run_id)?;
            let deleted = transaction
                .execute(
                    "DELETE FROM scan_runs WHERE run_id = ?1;",
                    rusqlite::params![run_id],
                )
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
            if deleted != 1 {
                return Err(HistoryStoreError::Persistence(format!(
                    "purge deleted {deleted} rows for {run_id}"
                )));
            }

            let remaining = transaction
                .query_row(
                    "SELECT COUNT(*) FROM scan_runs WHERE run_id = ?1;",
                    rusqlite::params![run_id],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
            if remaining != 0 {
                return Err(HistoryStoreError::Persistence(format!(
                    "purge verification failed for {run_id}"
                )));
            }

            let event_json = serde_json::to_string(&serde_json::json!({
                "runId": run_id,
                "terminalAt": terminal_at,
                "purgedAt": now,
            }))
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
            transaction
                .execute(
                    r#"
                    INSERT INTO scan_run_audit (
                        run_id,
                        event_type,
                        reason_code,
                        created_at,
                        event_json
                    ) VALUES (NULL, ?1, ?2, ?3, ?4);
                    "#,
                    rusqlite::params![
                        PURGE_EVENT_TYPE,
                        REASON_RETENTION_EXPIRED,
                        now,
                        event_json
                    ],
                )
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
            deleted_run_ids.push(run_id.clone());
        }

        transaction
            .commit()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        Ok(PurgedScanRuns {
            purged_count: deleted_run_ids.len(),
            deleted_run_ids,
        })
    }

    fn open_connection(&self) -> Result<rusqlite::Connection, HistoryStoreError> {
        let connection = rusqlite::Connection::open(&self.db_path)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        Ok(connection)
    }

    fn now_timestamp(&self) -> String {
        (self.now)()
    }

    fn load_hash_cache_entry(&self, key: &HashCacheKey) -> Result<Option<CachedHashes>, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let row = connection.query_row(
            r#"
            SELECT partial_hash, full_hash
            FROM duplicate_hash_cache
            WHERE path = ?1 AND size_bytes = ?2 AND modified_at_millis = ?3;
            "#,
            rusqlite::params![
                key.path,
                i64::try_from(key.size_bytes)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                key.modified_at_millis
            ],
            |row| {
                Ok(CachedHashes {
                    partial_hash: row.get(0)?,
                    full_hash: row.get(1)?,
                })
            },
        );

        match row {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
        }
    }

    fn load_hash_cache_entries(
        &self,
        keys: &[HashCacheKey],
    ) -> Result<HashMap<HashCacheKey, CachedHashes>, HistoryStoreError> {
        self.initialize()?;
        if keys.is_empty() {
            return Ok(HashMap::new());
        }

        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT partial_hash, full_hash
                FROM duplicate_hash_cache
                WHERE path = ?1 AND size_bytes = ?2 AND modified_at_millis = ?3;
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let mut entries = HashMap::with_capacity(keys.len());

        for key in keys {
            let row = statement.query_row(
                rusqlite::params![
                    key.path,
                    i64::try_from(key.size_bytes)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    key.modified_at_millis
                ],
                |row| {
                    Ok(CachedHashes {
                        partial_hash: row.get(0)?,
                        full_hash: row.get(1)?,
                    })
                },
            );

            match row {
                Ok(entry) => {
                    entries.insert(key.clone(), entry);
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {}
                Err(error) => return Err(HistoryStoreError::Persistence(error.to_string())),
            }
        }

        Ok(entries)
    }

    pub fn save_cleanup_execution(
        &self,
        result: &CleanupExecutionResult,
    ) -> Result<(), HistoryStoreError> {
        self.initialize()?;

        let payload = serde_json::to_string(result)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let completed_count = i64::try_from(result.completed_count)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let failed_count = i64::try_from(result.failed_count)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let connection = self.open_connection()?;
        connection
            .execute(
                r#"
                INSERT OR REPLACE INTO cleanup_execution_history (
                    execution_id,
                    preview_id,
                    mode,
                    completed_at,
                    completed_count,
                    failed_count,
                    execution_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
                "#,
                rusqlite::params![
                    result.execution_id,
                    result.preview_id,
                    serde_json::to_string(&result.mode)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    result.completed_at,
                    completed_count,
                    failed_count,
                    payload
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        Ok(())
    }

    pub fn open_cleanup_execution(
        &self,
        execution_id: &str,
    ) -> Result<CleanupExecutionResult, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let payload = connection.query_row(
            "SELECT execution_json FROM cleanup_execution_history WHERE execution_id = ?1;",
            rusqlite::params![execution_id],
            |row| row.get::<_, String>(0),
        );

        match payload {
            Ok(payload) => serde_json::from_str(&payload)
                .map_err(|error| HistoryStoreError::Persistence(error.to_string())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(HistoryStoreError::NotFound {
                scan_id: execution_id.to_string(),
            }),
            Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
        }
    }

    fn save_hash_cache_entry(
        &self,
        key: &HashCacheKey,
        partial_hash: Option<&str>,
        full_hash: Option<&str>,
    ) -> Result<(), HistoryStoreError> {
        self.save_hash_cache_entries(&[HashCacheWrite {
            key: key.clone(),
            partial_hash: partial_hash.map(str::to_string),
            full_hash: full_hash.map(str::to_string),
        }])
    }

    fn save_hash_cache_entries(
        &self,
        writes: &[HashCacheWrite],
    ) -> Result<(), HistoryStoreError> {
        self.initialize()?;
        if writes.is_empty() {
            return Ok(());
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let mut statement = transaction
            .prepare(
                r#"
                INSERT INTO duplicate_hash_cache (
                    path,
                    size_bytes,
                    modified_at_millis,
                    partial_hash,
                    full_hash
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(path, size_bytes, modified_at_millis) DO UPDATE SET
                    partial_hash = COALESCE(excluded.partial_hash, duplicate_hash_cache.partial_hash),
                    full_hash = COALESCE(excluded.full_hash, duplicate_hash_cache.full_hash);
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        for write in writes {
            statement
                .execute(rusqlite::params![
                    write.key.path,
                    i64::try_from(write.key.size_bytes)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    write.key.modified_at_millis,
                    write.partial_hash,
                    write.full_hash
                ])
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        }

        drop(statement);
        transaction
            .commit()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        Ok(())
    }
}

impl HashCache for HistoryStore {
    fn get_cached_hashes(&self, key: &HashCacheKey) -> Result<Option<CachedHashes>, DuplicateAnalysisFailure> {
        self.load_hash_cache_entry(key).map_err(|error| DuplicateAnalysisFailure::Internal {
            message: error.to_string(),
        })
    }

    fn save_partial_hash(&self, key: &HashCacheKey, partial_hash: &str) -> Result<(), DuplicateAnalysisFailure> {
        self.save_hash_cache_entry(key, Some(partial_hash), None)
            .map_err(|error| DuplicateAnalysisFailure::Internal {
                message: error.to_string(),
            })
    }

    fn save_full_hash(&self, key: &HashCacheKey, full_hash: &str) -> Result<(), DuplicateAnalysisFailure> {
        self.save_hash_cache_entry(key, None, Some(full_hash))
            .map_err(|error| DuplicateAnalysisFailure::Internal {
                message: error.to_string(),
            })
    }

    fn get_cached_hashes_batch(
        &self,
        keys: &[HashCacheKey],
    ) -> Result<HashMap<HashCacheKey, CachedHashes>, DuplicateAnalysisFailure> {
        self.load_hash_cache_entries(keys)
            .map_err(|error| DuplicateAnalysisFailure::Internal {
                message: error.to_string(),
            })
    }

    fn save_hashes_batch(
        &self,
        writes: &[HashCacheWrite],
    ) -> Result<(), DuplicateAnalysisFailure> {
        self.save_hash_cache_entries(writes)
            .map_err(|error| DuplicateAnalysisFailure::Internal {
                message: error.to_string(),
            })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HistoryStoreError {
    #[error("history entry not found: {scan_id}")]
    NotFound { scan_id: String },
    #[error("scan run conflict for {run_id}: status {status}")]
    Conflict { run_id: String, status: String },
    #[error(
        "scan run snapshot sequence mismatch for {run_id}: expected {expected}, got {actual}"
    )]
    InvalidScanRunSequence {
        run_id: String,
        expected: u64,
        actual: u64,
    },
    #[error(
        "scan run counters regressed for {run_id} on {field}: previous {previous}, actual {actual}"
    )]
    InvalidScanRunCounters {
        run_id: String,
        field: String,
        previous: u64,
        actual: u64,
    },
    #[error("history persistence failed: {0}")]
    Persistence(String),
}

fn load_scan_run_detail(
    connection: &rusqlite::Connection,
    run_id: &str,
    page: u32,
    page_size: u32,
    now: &str,
) -> Result<ScanRunDetail, HistoryStoreError> {
    let header = load_scan_run_header(connection, run_id)?;
    let latest_snapshot = load_latest_scan_run_snapshot(connection, run_id)?;
    let (snapshot_preview_page, snapshot_preview_page_size) =
        normalize_scan_run_preview_paging(page, page_size);
    let snapshot_preview = load_scan_run_snapshot_preview(
        connection,
        run_id,
        snapshot_preview_page,
        snapshot_preview_page_size,
    )?;
    let snapshot_preview_total = count_scan_run_snapshots(connection, run_id)?;
    let (has_resume, can_resume) = scan_run_resume_flags(&header, now);
    let seq = latest_snapshot.seq;
    let created_at = header.created_at.clone();
    let items_scanned = latest_snapshot.items_scanned;
    let errors_count = latest_snapshot.errors_count;
    let progress_percent = normalize_scan_run_progress_percent(latest_snapshot.progress_percent);
    let scan_rate_items_per_sec = latest_snapshot.scan_rate_items_per_sec;

    Ok(ScanRunDetail {
        header,
        latest_snapshot,
        snapshot_preview,
        snapshot_preview_page,
        snapshot_preview_page_size,
        snapshot_preview_total,
        seq,
        created_at,
        items_scanned,
        errors_count,
        progress_percent,
        scan_rate_items_per_sec,
        has_resume,
        can_resume,
    })
}

fn load_scan_run_header(
    connection: &rusqlite::Connection,
    run_id: &str,
) -> Result<ScanRunHeader, HistoryStoreError> {
    let header = connection.query_row(
        r#"
        SELECT run_id,
               target_id,
               root_path,
               status,
               started_at,
               last_snapshot_at,
               last_progress_at,
               stale_since,
               terminal_at,
               completed_scan_id,
               resumed_from_run_id,
               resume_enabled,
               resume_token,
               resume_expires_at,
               resume_payload_json,
               resume_target_fingerprint_json,
               privacy_scope_id,
               error_code,
               error_message,
               created_at,
               updated_at,
               latest_seq
        FROM scan_runs
        WHERE run_id = ?1;
        "#,
        rusqlite::params![run_id],
        |row| {
            Ok(ScanRunHeader {
                run_id: row.get(0)?,
                target_id: row.get(1)?,
                root_path: row.get(2)?,
                status: parse_scan_run_status(&row.get::<_, String>(3)?)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                started_at: row.get(4)?,
                last_snapshot_at: row.get(5)?,
                last_progress_at: row.get(6)?,
                stale_since: row.get(7)?,
                terminal_at: row.get(8)?,
                completed_scan_id: row.get(9)?,
                resumed_from_run_id: row.get(10)?,
                resume_enabled: row.get::<_, i64>(11)? != 0,
                resume_token: row.get(12)?,
                resume_expires_at: row.get(13)?,
                resume_payload_json: row.get(14)?,
                resume_target_fingerprint_json: row.get(15)?,
                privacy_scope_id: row.get(16)?,
                error_code: row.get(17)?,
                error_message: row.get(18)?,
                created_at: row.get(19)?,
                updated_at: row.get(20)?,
                latest_seq: u64::try_from(row.get::<_, i64>(21)?).unwrap_or_default(),
            })
        },
    );

    match header {
        Ok(header) => Ok(header),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(HistoryStoreError::NotFound {
            scan_id: run_id.to_string(),
        }),
        Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
    }
}

fn load_latest_scan_run_snapshot(
    connection: &rusqlite::Connection,
    run_id: &str,
) -> Result<ScanRunSnapshot, HistoryStoreError> {
    let latest_snapshot = connection.query_row(
        r#"
        SELECT run_id,
               seq,
               snapshot_at,
               created_at,
               status,
               files_discovered,
               directories_discovered,
               items_discovered,
               items_scanned,
               errors_count,
               bytes_processed,
               scan_rate_items_per_sec,
               progress_percent,
               current_path,
               message
        FROM scan_run_snapshots
        WHERE run_id = ?1
        ORDER BY seq DESC
        LIMIT 1;
        "#,
        rusqlite::params![run_id],
        |row| {
            Ok(ScanRunSnapshot {
                run_id: row.get(0)?,
                seq: u64::try_from(row.get::<_, i64>(1)?).unwrap_or_default(),
                snapshot_at: row.get(2)?,
                created_at: row.get(3)?,
                status: parse_scan_run_status(&row.get::<_, String>(4)?)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                files_discovered: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                directories_discovered: u64::try_from(row.get::<_, i64>(6)?)
                    .unwrap_or_default(),
                items_discovered: u64::try_from(row.get::<_, i64>(7)?).unwrap_or_default(),
                items_scanned: u64::try_from(row.get::<_, i64>(8)?).unwrap_or_default(),
                errors_count: u64::try_from(row.get::<_, i64>(9)?).unwrap_or_default(),
                bytes_processed: u64::try_from(row.get::<_, i64>(10)?).unwrap_or_default(),
                scan_rate_items_per_sec: row.get(11)?,
                progress_percent: row.get(12)?,
                current_path: row.get(13)?,
                message: row.get(14)?,
            })
        },
    );

    match latest_snapshot {
        Ok(latest_snapshot) => Ok(latest_snapshot),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(HistoryStoreError::Persistence(format!(
            "scan run missing latest snapshot: {run_id}"
        ))),
        Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
    }
}

fn load_scan_run_summary(
    connection: &rusqlite::Connection,
    run_id: &str,
    now: &str,
) -> Result<ScanRunSummary, HistoryStoreError> {
    let header = load_scan_run_header(connection, run_id)?;
    let latest_snapshot = load_latest_scan_run_snapshot(connection, run_id)?;
    let snapshot_preview = load_scan_run_snapshot_preview(
        connection,
        run_id,
        1,
        DEFAULT_SCAN_RUN_PREVIEW_PAGE_SIZE,
    )?;
    let (has_resume, can_resume) = scan_run_resume_flags(&header, now);
    let seq = latest_snapshot.seq;
    let created_at = header.created_at.clone();
    let items_scanned = latest_snapshot.items_scanned;
    let errors_count = latest_snapshot.errors_count;
    let progress_percent = normalize_scan_run_progress_percent(latest_snapshot.progress_percent);
    let scan_rate_items_per_sec = latest_snapshot.scan_rate_items_per_sec;

    Ok(ScanRunSummary {
        header,
        latest_snapshot,
        snapshot_preview,
        seq,
        created_at,
        items_scanned,
        errors_count,
        progress_percent,
        scan_rate_items_per_sec,
        has_resume,
        can_resume,
    })
}

fn load_scan_run_snapshot_preview(
    connection: &rusqlite::Connection,
    run_id: &str,
    page: u32,
    page_size: u32,
) -> Result<Vec<ScanRunSnapshot>, HistoryStoreError> {
    let offset = i64::from(page.saturating_sub(1).saturating_mul(page_size));
    let limit = i64::from(page_size);
    let mut statement = connection
        .prepare(
            r#"
            SELECT run_id,
                   seq,
                   snapshot_at,
                   created_at,
                   status,
                   files_discovered,
                   directories_discovered,
                   items_discovered,
                   items_scanned,
                   errors_count,
                   bytes_processed,
                   scan_rate_items_per_sec,
                   progress_percent,
                   current_path,
                   message
            FROM scan_run_snapshots
            WHERE run_id = ?1
            ORDER BY seq DESC
            LIMIT ?2 OFFSET ?3;
            "#,
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    let rows = statement
        .query_map(rusqlite::params![run_id, limit, offset], |row| {
            Ok(ScanRunSnapshot {
                run_id: row.get(0)?,
                seq: u64::try_from(row.get::<_, i64>(1)?).unwrap_or_default(),
                snapshot_at: row.get(2)?,
                created_at: row.get(3)?,
                status: parse_scan_run_status(&row.get::<_, String>(4)?)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                files_discovered: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                directories_discovered: u64::try_from(row.get::<_, i64>(6)?)
                    .unwrap_or_default(),
                items_discovered: u64::try_from(row.get::<_, i64>(7)?).unwrap_or_default(),
                items_scanned: u64::try_from(row.get::<_, i64>(8)?).unwrap_or_default(),
                errors_count: u64::try_from(row.get::<_, i64>(9)?).unwrap_or_default(),
                bytes_processed: u64::try_from(row.get::<_, i64>(10)?).unwrap_or_default(),
                scan_rate_items_per_sec: row.get(11)?,
                progress_percent: row.get(12)?,
                current_path: row.get(13)?,
                message: row.get(14)?,
            })
        })
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))
}

fn count_scan_run_snapshots(
    connection: &rusqlite::Connection,
    run_id: &str,
) -> Result<u64, HistoryStoreError> {
    connection
        .query_row(
            "SELECT COUNT(*) FROM scan_run_snapshots WHERE run_id = ?1;",
            rusqlite::params![run_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| u64::try_from(count).unwrap_or_default())
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => HistoryStoreError::NotFound {
                scan_id: run_id.to_string(),
            },
            other => HistoryStoreError::Persistence(other.to_string()),
        })
}

fn load_latest_scan_run_snapshot_in_transaction(
    connection: &rusqlite::Transaction<'_>,
    run_id: &str,
) -> Result<ScanRunSnapshot, HistoryStoreError> {
    let latest_snapshot = connection.query_row(
        r#"
        SELECT run_id,
               seq,
               snapshot_at,
               created_at,
               status,
               files_discovered,
               directories_discovered,
               items_discovered,
               items_scanned,
               errors_count,
               bytes_processed,
               scan_rate_items_per_sec,
               progress_percent,
               current_path,
               message
        FROM scan_run_snapshots
        WHERE run_id = ?1
        ORDER BY seq DESC
        LIMIT 1;
        "#,
        rusqlite::params![run_id],
        |row| {
            Ok(ScanRunSnapshot {
                run_id: row.get(0)?,
                seq: u64::try_from(row.get::<_, i64>(1)?).unwrap_or_default(),
                snapshot_at: row.get(2)?,
                created_at: row.get(3)?,
                status: parse_scan_run_status(&row.get::<_, String>(4)?)
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?,
                files_discovered: u64::try_from(row.get::<_, i64>(5)?).unwrap_or_default(),
                directories_discovered: u64::try_from(row.get::<_, i64>(6)?)
                    .unwrap_or_default(),
                items_discovered: u64::try_from(row.get::<_, i64>(7)?).unwrap_or_default(),
                items_scanned: u64::try_from(row.get::<_, i64>(8)?).unwrap_or_default(),
                errors_count: u64::try_from(row.get::<_, i64>(9)?).unwrap_or_default(),
                bytes_processed: u64::try_from(row.get::<_, i64>(10)?).unwrap_or_default(),
                scan_rate_items_per_sec: row.get(11)?,
                progress_percent: row.get(12)?,
                current_path: row.get(13)?,
                message: row.get(14)?,
            })
        },
    );

    match latest_snapshot {
        Ok(latest_snapshot) => Ok(latest_snapshot),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(HistoryStoreError::NotFound {
            scan_id: run_id.to_string(),
        }),
        Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
    }
}

fn persist_completed_scan_in_connection(
    connection: &rusqlite::Connection,
    scan: &CompletedScan,
) -> Result<(), HistoryStoreError> {
    let payload = serde_json::to_string(scan)
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    let total_bytes = i64::try_from(scan.total_bytes)
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    connection
        .execute(
            r#"
            INSERT OR REPLACE INTO scan_history (
                scan_id,
                root_path,
                completed_at,
                total_bytes,
                scan_json
            ) VALUES (?1, ?2, ?3, ?4, ?5);
            "#,
            rusqlite::params![
                scan.scan_id,
                scan.root_path,
                scan.completed_at,
                total_bytes,
                payload
            ],
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    Ok(())
}

fn persist_completed_scan_in_transaction(
    transaction: &rusqlite::Transaction<'_>,
    scan: &CompletedScan,
) -> Result<(), HistoryStoreError> {
    let payload = serde_json::to_string(scan)
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    let total_bytes = i64::try_from(scan.total_bytes)
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    transaction
        .execute(
            r#"
            INSERT OR REPLACE INTO scan_history (
                scan_id,
                root_path,
                completed_at,
                total_bytes,
                scan_json
            ) VALUES (?1, ?2, ?3, ?4, ?5);
            "#,
            rusqlite::params![
                scan.scan_id,
                scan.root_path,
                scan.completed_at,
                total_bytes,
                payload
            ],
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    Ok(())
}

fn scan_run_status_label(status: &ScanRunStatus) -> &'static str {
    match status {
        ScanRunStatus::Running => "running",
        ScanRunStatus::Completed => "completed",
        ScanRunStatus::Cancelled => "cancelled",
        ScanRunStatus::Failed => "failed",
        ScanRunStatus::Stale => "stale",
        ScanRunStatus::Abandoned => "abandoned",
    }
}

fn parse_scan_run_status(value: &str) -> Result<ScanRunStatus, HistoryStoreError> {
    match value {
        "running" => Ok(ScanRunStatus::Running),
        "completed" => Ok(ScanRunStatus::Completed),
        "cancelled" => Ok(ScanRunStatus::Cancelled),
        "failed" => Ok(ScanRunStatus::Failed),
        "stale" => Ok(ScanRunStatus::Stale),
        "abandoned" => Ok(ScanRunStatus::Abandoned),
        other => Err(HistoryStoreError::Persistence(format!(
            "unknown scan run status: {other}"
        ))),
    }
}

fn terminal_finished_at(snapshot: &ScanRunSnapshot) -> Option<&str> {
    match snapshot.status {
        ScanRunStatus::Completed
        | ScanRunStatus::Cancelled
        | ScanRunStatus::Failed
        | ScanRunStatus::Abandoned => Some(snapshot.snapshot_at.as_str()),
        ScanRunStatus::Running | ScanRunStatus::Stale => None,
    }
}

fn ensure_counter_not_regressive(
    run_id: &str,
    field: &str,
    previous: u64,
    actual: u64,
) -> Result<(), HistoryStoreError> {
    if actual < previous {
        return Err(HistoryStoreError::InvalidScanRunCounters {
            run_id: run_id.to_string(),
            field: field.to_string(),
            previous,
            actual,
        });
    }

    Ok(())
}

fn load_scan_run_last_progress_at_in_transaction(
    connection: &rusqlite::Transaction<'_>,
    run_id: &str,
) -> Result<String, HistoryStoreError> {
    connection
        .query_row(
            "SELECT last_progress_at FROM scan_runs WHERE run_id = ?1;",
            rusqlite::params![run_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => HistoryStoreError::NotFound {
                scan_id: run_id.to_string(),
            },
            other => HistoryStoreError::Persistence(other.to_string()),
        })
}

fn next_last_progress_at(
    current_last_progress_at: &str,
    previous: &ScanRunSnapshot,
    next: &ScanRunSnapshot,
) -> String {
    if next.files_discovered > previous.files_discovered
        || next.directories_discovered > previous.directories_discovered
        || next.items_discovered > previous.items_discovered
        || next.items_scanned > previous.items_scanned
        || next.errors_count > previous.errors_count
        || next.bytes_processed > previous.bytes_processed
    {
        next.snapshot_at.clone()
    } else {
        current_last_progress_at.to_string()
    }
}

fn next_stale_since(
    connection: &rusqlite::Transaction<'_>,
    snapshot: &ScanRunSnapshot,
) -> Result<Option<String>, HistoryStoreError> {
    let current_stale_since = connection.query_row(
        "SELECT stale_since FROM scan_runs WHERE run_id = ?1;",
        rusqlite::params![snapshot.run_id],
        |row| row.get::<_, Option<String>>(0),
    );

    let current_stale_since = match current_stale_since {
        Ok(value) => value,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return Err(HistoryStoreError::NotFound {
                scan_id: snapshot.run_id.clone(),
            });
        }
        Err(error) => return Err(HistoryStoreError::Persistence(error.to_string())),
    };

    Ok(match snapshot.status {
        ScanRunStatus::Stale => current_stale_since.or_else(|| Some(snapshot.snapshot_at.clone())),
        ScanRunStatus::Abandoned => current_stale_since,
        ScanRunStatus::Running
        | ScanRunStatus::Completed
        | ScanRunStatus::Cancelled
        | ScanRunStatus::Failed => current_stale_since,
    })
}

fn normalize_scan_run_preview_paging(page: u32, page_size: u32) -> (u32, u32) {
    let normalized_page = if page == 0 { 1 } else { page };
    let normalized_page_size = if page_size == 0 {
        DEFAULT_SCAN_RUN_PREVIEW_PAGE_SIZE
    } else {
        page_size
    };
    (normalized_page, normalized_page_size)
}

fn scan_run_resume_flags(header: &ScanRunHeader, now: &str) -> (bool, bool) {
    let has_resume = header.resume_enabled
        && header
            .resume_token
            .as_deref()
            .is_some_and(|value| !value.is_empty())
        && header
            .resume_payload_json
            .as_deref()
            .is_some_and(|value| !value.is_empty());
    let resume_metadata_eligible = has_resume
        && matches!(header.status, ScanRunStatus::Stale | ScanRunStatus::Abandoned)
        && resume_is_not_expired(header.resume_expires_at.as_deref(), now)
        && resume_privacy_scope_matches(header)
        && resume_target_fingerprint_matches(header);
    let can_resume = SCAN_RESUME_ENGINE_SUPPORTED && resume_metadata_eligible;

    (has_resume, can_resume)
}

fn normalize_scan_run_progress_percent(progress_percent: Option<f64>) -> Option<f64> {
    progress_percent.map(|value| value.clamp(0.0, 100.0))
}

fn resume_is_not_expired(expires_at: Option<&str>, now: &str) -> bool {
    match expires_at {
        Some(expires_at) => match (parse_timestamp(now), parse_timestamp(expires_at)) {
            (Some(now), Some(expires_at)) => expires_at >= now,
            _ => false,
        },
        None => true,
    }
}

fn build_resume_payload_json(current_path: Option<&str>, latest_seq: u64) -> String {
    serde_json::json!({
        "currentPath": current_path,
        "latestSeq": latest_seq,
    })
    .to_string()
}

fn resume_privacy_scope_matches(header: &ScanRunHeader) -> bool {
    header.privacy_scope_id.as_deref() == Some(DEFAULT_SCAN_RUN_PRIVACY_SCOPE_ID)
}

fn resume_target_fingerprint_matches(header: &ScanRunHeader) -> bool {
    let Some(stored) = header.resume_target_fingerprint_json.as_deref() else {
        return false;
    };
    let Ok(stored_value) = serde_json::from_str::<serde_json::Value>(stored) else {
        return false;
    };

    stored_value
        == serde_json::json!({
            "rootPath": header.root_path,
            "targetId": header.target_id,
        })
}

fn reconciliation_transition(
    status_label: &str,
    last_snapshot_at: &str,
    now: &str,
) -> Option<(ScanRunStatus, &'static str, &'static str)> {
    let elapsed = elapsed_seconds(last_snapshot_at, now)?;

    match status_label {
        "running" if elapsed >= ABANDON_AFTER_SECONDS => Some((
            ScanRunStatus::Abandoned,
            REASON_RUN_ABANDONED,
            "Run marked abandoned during startup reconciliation.",
        )),
        "running" if elapsed >= HEARTBEAT_STALE_AFTER_SECONDS => Some((
            ScanRunStatus::Stale,
            REASON_HEARTBEAT_STALE,
            "Run marked stale during startup reconciliation.",
        )),
        "stale" if elapsed >= ABANDON_AFTER_SECONDS => Some((
            ScanRunStatus::Abandoned,
            REASON_RUN_ABANDONED,
            "Run marked abandoned during startup reconciliation.",
        )),
        _ => None,
    }
}

fn append_reconciliation_snapshot(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &str,
    status: ScanRunStatus,
    snapshot_at: &str,
    reason_code: &str,
    message: &str,
) -> Result<(), HistoryStoreError> {
    let previous = load_latest_scan_run_snapshot_in_transaction(transaction, run_id)?;
    let current_last_progress_at = load_scan_run_last_progress_at_in_transaction(transaction, run_id)?;
    let snapshot = ScanRunSnapshot {
        run_id: run_id.to_string(),
        seq: previous.seq.saturating_add(1),
        snapshot_at: snapshot_at.to_string(),
        created_at: snapshot_at.to_string(),
        status,
        files_discovered: previous.files_discovered,
        directories_discovered: previous.directories_discovered,
        items_discovered: previous.items_discovered,
        items_scanned: previous.items_scanned,
        errors_count: previous.errors_count,
        bytes_processed: previous.bytes_processed,
        scan_rate_items_per_sec: 0.0,
        progress_percent: previous.progress_percent,
        current_path: previous.current_path.clone(),
        message: Some(message.to_string()),
    };

    transaction
        .execute(
            r#"
            INSERT INTO scan_run_snapshots (
                run_id,
                seq,
                snapshot_at,
                created_at,
                status,
                files_discovered,
                directories_discovered,
                items_discovered,
                items_scanned,
                errors_count,
                bytes_processed,
                scan_rate_items_per_sec,
                progress_percent,
                current_path,
                message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15);
            "#,
            rusqlite::params![
                snapshot.run_id,
                i64::try_from(snapshot.seq)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                snapshot.snapshot_at,
                snapshot.created_at,
                scan_run_status_label(&snapshot.status),
                i64::try_from(snapshot.files_discovered)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.directories_discovered)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.items_discovered)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.items_scanned)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.errors_count)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.bytes_processed)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                snapshot.scan_rate_items_per_sec,
                snapshot.progress_percent,
                snapshot.current_path,
                snapshot.message
            ],
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    transaction
        .execute(
            r#"
            UPDATE scan_runs
            SET status = ?2,
                last_snapshot_at = ?3,
                last_progress_at = ?4,
                stale_since = ?5,
                terminal_at = ?6,
                error_code = NULL,
                error_message = NULL,
                updated_at = ?7,
                latest_seq = ?8
            WHERE run_id = ?1;
            "#,
            rusqlite::params![
                run_id,
                scan_run_status_label(&snapshot.status),
                snapshot.snapshot_at,
                current_last_progress_at,
                next_stale_since(transaction, &snapshot)?,
                terminal_finished_at(&snapshot),
                snapshot.created_at,
                i64::try_from(snapshot.seq)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?
            ],
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    let event_json = serde_json::to_string(&serde_json::json!({
        "fromStatus": scan_run_status_label(&previous.status),
        "toStatus": scan_run_status_label(&snapshot.status),
        "snapshotAt": snapshot.snapshot_at,
    }))
    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    transaction
        .execute(
            r#"
            INSERT INTO scan_run_audit (
                run_id,
                event_type,
                reason_code,
                created_at,
                event_json
            ) VALUES (?1, ?2, ?3, ?4, ?5);
            "#,
            rusqlite::params![
                run_id,
                RECONCILE_EVENT_TYPE,
                reason_code,
                snapshot.created_at,
                event_json
            ],
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    Ok(())
}

fn append_cancelled_snapshot(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &str,
    snapshot_at: &str,
    previous: &ScanRunSnapshot,
) -> Result<(), HistoryStoreError> {
    let current_last_progress_at =
        load_scan_run_last_progress_at_in_transaction(transaction, run_id)?;
    let snapshot = ScanRunSnapshot {
        run_id: run_id.to_string(),
        seq: previous.seq.saturating_add(1),
        snapshot_at: snapshot_at.to_string(),
        created_at: snapshot_at.to_string(),
        status: ScanRunStatus::Cancelled,
        files_discovered: previous.files_discovered,
        directories_discovered: previous.directories_discovered,
        items_discovered: previous.items_discovered,
        items_scanned: previous.items_scanned,
        errors_count: previous.errors_count,
        bytes_processed: previous.bytes_processed,
        scan_rate_items_per_sec: 0.0,
        progress_percent: previous.progress_percent,
        current_path: previous.current_path.clone(),
        message: Some("Run cancelled by user.".to_string()),
    };

    transaction
        .execute(
            r#"
            INSERT INTO scan_run_snapshots (
                run_id,
                seq,
                snapshot_at,
                created_at,
                status,
                files_discovered,
                directories_discovered,
                items_discovered,
                items_scanned,
                errors_count,
                bytes_processed,
                scan_rate_items_per_sec,
                progress_percent,
                current_path,
                message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15);
            "#,
            rusqlite::params![
                snapshot.run_id,
                i64::try_from(snapshot.seq)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                snapshot.snapshot_at,
                snapshot.created_at,
                scan_run_status_label(&snapshot.status),
                i64::try_from(snapshot.files_discovered)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.directories_discovered)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.items_discovered)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.items_scanned)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.errors_count)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                i64::try_from(snapshot.bytes_processed)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                snapshot.scan_rate_items_per_sec,
                snapshot.progress_percent,
                snapshot.current_path,
                snapshot.message
            ],
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    transaction
        .execute(
            r#"
            UPDATE scan_runs
            SET status = ?2,
                last_snapshot_at = ?3,
                last_progress_at = ?4,
                stale_since = ?5,
                terminal_at = ?6,
                error_code = NULL,
                error_message = NULL,
                updated_at = ?7,
                latest_seq = ?8
            WHERE run_id = ?1;
            "#,
            rusqlite::params![
                run_id,
                scan_run_status_label(&snapshot.status),
                snapshot.snapshot_at,
                current_last_progress_at,
                next_stale_since(transaction, &snapshot)?,
                terminal_finished_at(&snapshot),
                snapshot.created_at,
                i64::try_from(snapshot.seq)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?
            ],
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    let event_json = serde_json::to_string(&serde_json::json!({
        "fromStatus": scan_run_status_label(&previous.status),
        "toStatus": scan_run_status_label(&snapshot.status),
        "snapshotAt": snapshot.snapshot_at,
    }))
    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    transaction
        .execute(
            r#"
            INSERT INTO scan_run_audit (
                run_id,
                event_type,
                reason_code,
                created_at,
                event_json
            ) VALUES (?1, ?2, ?3, ?4, ?5);
            "#,
            rusqlite::params![
                run_id,
                CANCEL_EVENT_TYPE,
                REASON_USER_CANCELLED,
                snapshot.created_at,
                event_json
            ],
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

    Ok(())
}

fn preserve_purged_run_audit_rows(
    transaction: &rusqlite::Transaction<'_>,
    run_id: &str,
) -> Result<(), HistoryStoreError> {
    let mut statement = transaction
        .prepare(
            r#"
            SELECT event_type, reason_code, created_at, event_json
            FROM scan_run_audit
            WHERE run_id = ?1
            ORDER BY event_id ASC;
            "#,
        )
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    let audit_rows = statement
        .query_map(rusqlite::params![run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    drop(statement);

    for (event_type, reason_code, created_at, event_json) in audit_rows {
        transaction
            .execute(
                r#"
                INSERT INTO scan_run_audit (
                    run_id,
                    event_type,
                    reason_code,
                    created_at,
                    event_json
                ) VALUES (NULL, ?1, ?2, ?3, ?4);
                "#,
                rusqlite::params![
                    event_type,
                    reason_code,
                    created_at,
                    preserved_purge_audit_event_json(run_id, &event_json)?
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    }

    Ok(())
}

fn preserved_purge_audit_event_json(
    run_id: &str,
    event_json: &str,
) -> Result<String, HistoryStoreError> {
    let parsed = serde_json::from_str::<serde_json::Value>(event_json)
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
    let preserved = match parsed {
        serde_json::Value::Object(mut object) => {
            object.insert(
                "runId".to_string(),
                serde_json::Value::String(run_id.to_string()),
            );
            object.insert(
                "preservedAfterPurge".to_string(),
                serde_json::Value::Bool(true),
            );
            serde_json::Value::Object(object)
        }
        other => serde_json::json!({
            "runId": run_id,
            "preservedAfterPurge": true,
            "originalEvent": other,
        }),
    };

    serde_json::to_string(&preserved)
        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))
}

fn elapsed_seconds(from: &str, to: &str) -> Option<i64> {
    let from = parse_timestamp(from)?;
    let to = parse_timestamp(to)?;
    Some((to - from).num_seconds())
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn retention_cutoff_timestamp(
    now: &str,
    retention_days: i64,
) -> Result<String, HistoryStoreError> {
    let now = parse_timestamp(now).ok_or_else(|| {
        HistoryStoreError::Persistence(format!("invalid retention clock timestamp: {now}"))
    })?;
    Ok((now - chrono::Duration::days(retention_days))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleanup_core::{CleanupExecutionEntry, CleanupExecutionItemStatus, CleanupExecutionMode};
    use scan_core::{
        ScanEntry, ScanEntryKind, ScanRunDetail, ScanRunSnapshot, ScanRunStatus, SizedPath,
        SkipReasonCode, SkippedPath, DEFAULT_SCAN_RUN_PRIVACY_SCOPE_ID,
    };
    use tempfile::tempdir;

    fn sample_completed_scan() -> CompletedScan {
        CompletedScan {
            scan_id: "scan-1".to_string(),
            root_path: "C:\\scan-root".to_string(),
            started_at: "2026-04-15T10:00:00Z".to_string(),
            completed_at: "2026-04-15T10:00:05Z".to_string(),
            total_bytes: 42,
            total_files: 3,
            total_directories: 2,
            largest_files: vec![SizedPath {
                path: "C:\\scan-root\\large.bin".to_string(),
                size_bytes: 42,
            }],
            largest_directories: vec![SizedPath {
                path: "C:\\scan-root".to_string(),
                size_bytes: 42,
            }],
            skipped_paths: vec![SkippedPath {
                path: "C:\\scan-root\\blocked".to_string(),
                reason_code: SkipReasonCode::PermissionDenied,
                summary: "access denied".to_string(),
            }],
            entries: vec![
                ScanEntry {
                    path: "C:\\scan-root".to_string(),
                    parent_path: None,
                    kind: ScanEntryKind::Directory,
                    size_bytes: 42,
                },
                ScanEntry {
                    path: "C:\\scan-root\\large.bin".to_string(),
                    parent_path: Some("C:\\scan-root".to_string()),
                    kind: ScanEntryKind::File,
                    size_bytes: 42,
                },
            ],
        }
    }

    fn sample_running_snapshot(seq: u64) -> ScanRunSnapshot {
        ScanRunSnapshot {
            run_id: "run-1".to_string(),
            seq,
            snapshot_at: "2026-04-18T10:00:00Z".to_string(),
            created_at: "2026-04-18T10:00:01Z".to_string(),
            status: ScanRunStatus::Running,
            files_discovered: 4,
            directories_discovered: 2,
            items_discovered: 6,
            items_scanned: 4,
            errors_count: 0,
            bytes_processed: 64,
            scan_rate_items_per_sec: 12.0,
            progress_percent: Some(25.0),
            current_path: Some("C:\\scan-root".to_string()),
            message: None,
        }
    }

    fn scripted_clock(values: &[&str]) -> impl Fn() -> String + Send + Sync + 'static {
        let values = std::sync::Arc::new(std::sync::Mutex::new(
            values.iter().map(|value| value.to_string()).collect::<Vec<_>>(),
        ));
        move || {
            let mut values = values.lock().expect("clock lock");
            if values.len() > 1 {
                values.remove(0)
            } else {
                values
                    .first()
                    .cloned()
                    .expect("scripted clock should contain at least one value")
            }
        }
    }

    fn load_audit_reason_codes(store: &HistoryStore, run_id: &str) -> Vec<String> {
        let connection =
            rusqlite::Connection::open(store.db_path()).expect("history database should open");
        let mut statement = connection
            .prepare(
                r#"
                SELECT reason_code
                FROM scan_run_audit
                WHERE run_id = ?1
                ORDER BY event_id ASC;
                "#,
            )
            .expect("audit query");
        statement
            .query_map(rusqlite::params![run_id], |row| row.get::<_, String>(0))
            .expect("audit rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("audit values")
    }

    fn load_audit_events_without_run_id(store: &HistoryStore, event_type: &str) -> Vec<String> {
        let connection =
            rusqlite::Connection::open(store.db_path()).expect("history database should open");
        let mut statement = connection
            .prepare(
                r#"
                SELECT reason_code
                FROM scan_run_audit
                WHERE run_id IS NULL AND event_type = ?1
                ORDER BY event_id ASC;
                "#,
            )
            .expect("audit query");
        statement
            .query_map(rusqlite::params![event_type], |row| row.get::<_, String>(0))
            .expect("audit rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("audit values")
    }

    fn assert_running_detail(detail: &ScanRunDetail, expected_seq: u64) {
        assert_eq!(detail.header.run_id, "run-1");
        assert_eq!(detail.header.target_id, "C:\\scan-root");
        assert_eq!(detail.header.root_path, "C:\\scan-root");
        assert_eq!(detail.header.status, ScanRunStatus::Running);
        assert_eq!(
            detail.header.privacy_scope_id,
            Some(DEFAULT_SCAN_RUN_PRIVACY_SCOPE_ID.to_string())
        );
        assert!(!detail.header.resume_enabled);
        assert_eq!(detail.header.resume_token, None);
        assert_eq!(detail.header.latest_seq, expected_seq);
        assert_eq!(detail.latest_snapshot.run_id, "run-1");
        assert_eq!(detail.latest_snapshot.seq, expected_seq);
        assert_eq!(detail.latest_snapshot.status, ScanRunStatus::Running);
    }

    #[test]
    fn persists_and_reopens_completed_scans() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let expected = sample_completed_scan();

        store.initialize().expect("schema initialization");
        store
            .save_completed_scan(&expected)
            .expect("scan should persist successfully");

        let entries = store.list_history().expect("history list");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].scan_id, expected.scan_id);
        assert_eq!(entries[0].total_bytes, expected.total_bytes);

        let reopened = store
            .open_history_entry(&expected.scan_id)
            .expect("stored entry should reopen");
        assert_eq!(reopened, expected);
    }

    #[test]
    fn records_started_scan_runs_with_initial_snapshot() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-18T10:00:01Z".to_string(),
        );

        let detail = store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        assert_running_detail(&detail, 1);
        assert_eq!(detail.header.last_snapshot_at, "2026-04-18T10:00:00Z");
        assert_eq!(detail.header.last_progress_at, "2026-04-18T10:00:00Z");
        assert_eq!(detail.header.created_at, "2026-04-18T10:00:01Z");
        assert_eq!(detail.header.updated_at, "2026-04-18T10:00:01Z");
        assert_eq!(detail.header.terminal_at, None);
        assert_eq!(detail.latest_snapshot.snapshot_at, "2026-04-18T10:00:00Z");
        assert_eq!(detail.latest_snapshot.created_at, "2026-04-18T10:00:01Z");
        assert_eq!(detail.latest_snapshot.items_discovered, 0);
        assert_eq!(detail.latest_snapshot.items_scanned, 0);
        assert_eq!(detail.latest_snapshot.errors_count, 0);
        assert_eq!(
            detail.latest_snapshot.current_path,
            Some("C:\\scan-root".to_string())
        );

        let reopened = store.open_scan_run("run-1").expect("scan run should reopen");
        assert_eq!(reopened, detail);
    }

    #[test]
    fn appends_scan_run_snapshots_in_seq_order_and_mirrors_latest_status() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-18T10:00:02Z".to_string(),
        );
        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        let progress = ScanRunSnapshot {
            seq: 2,
            snapshot_at: "2026-04-18T10:00:30Z".to_string(),
            created_at: "2026-04-18T10:00:02Z".to_string(),
            files_discovered: 9,
            directories_discovered: 3,
            items_discovered: 12,
            items_scanned: 9,
            errors_count: 1,
            bytes_processed: 512,
            scan_rate_items_per_sec: 128.0,
            progress_percent: Some(75.0),
            current_path: Some("C:\\scan-root\\nested".to_string()),
            ..sample_running_snapshot(2)
        };
        let terminal = ScanRunSnapshot {
            seq: 3,
            snapshot_at: "2026-04-18T10:00:35Z".to_string(),
            created_at: "2026-04-18T10:00:02Z".to_string(),
            status: ScanRunStatus::Failed,
            files_discovered: 9,
            directories_discovered: 3,
            items_discovered: 12,
            items_scanned: 9,
            errors_count: 2,
            bytes_processed: 512,
            scan_rate_items_per_sec: 0.0,
            progress_percent: Some(75.0),
            current_path: Some("C:\\scan-root\\nested".to_string()),
            message: Some("disk error".to_string()),
            ..sample_running_snapshot(3)
        };

        let progress_detail = store
            .append_scan_run_snapshot(&progress)
            .expect("progress snapshot should append");
        assert_running_detail(&progress_detail, 2);
        assert_eq!(progress_detail.header.last_snapshot_at, "2026-04-18T10:00:30Z");
        assert_eq!(progress_detail.header.last_progress_at, "2026-04-18T10:00:30Z");
        assert_eq!(progress_detail.latest_snapshot.bytes_processed, 512);
        assert_eq!(progress_detail.latest_snapshot.items_scanned, 9);

        let terminal_detail = store
            .append_scan_run_snapshot(&terminal)
            .expect("terminal snapshot should append");
        assert_eq!(terminal_detail.header.status, ScanRunStatus::Failed);
        assert_eq!(terminal_detail.header.latest_seq, 3);
        assert_eq!(
            terminal_detail.header.terminal_at,
            Some("2026-04-18T10:00:35Z".to_string())
        );
        assert_eq!(terminal_detail.header.error_code, None);
        assert_eq!(terminal_detail.latest_snapshot, terminal);
    }

    #[test]
    fn preserves_last_progress_at_for_liveness_only_snapshots() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-18T10:00:02Z".to_string(),
        );
        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                seq: 2,
                snapshot_at: "2026-04-18T10:00:30Z".to_string(),
                created_at: "2026-04-18T10:00:02Z".to_string(),
                files_discovered: 9,
                directories_discovered: 3,
                items_discovered: 12,
                items_scanned: 9,
                errors_count: 1,
                bytes_processed: 512,
                scan_rate_items_per_sec: 128.0,
                progress_percent: Some(75.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                ..sample_running_snapshot(2)
            })
            .expect("progress snapshot should append");

        let detail = store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                seq: 3,
                snapshot_at: "2026-04-18T10:01:00Z".to_string(),
                created_at: "2026-04-18T10:00:02Z".to_string(),
                files_discovered: 9,
                directories_discovered: 3,
                items_discovered: 12,
                items_scanned: 9,
                errors_count: 1,
                bytes_processed: 512,
                scan_rate_items_per_sec: 0.0,
                progress_percent: Some(75.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                message: Some("still scanning".to_string()),
                ..sample_running_snapshot(3)
            })
            .expect("liveness snapshot should append");

        assert_eq!(detail.header.last_snapshot_at, "2026-04-18T10:01:00Z");
        assert_eq!(detail.header.last_progress_at, "2026-04-18T10:00:30Z");
        assert_eq!(detail.header.latest_seq, 3);
        assert_eq!(detail.latest_snapshot.message, Some("still scanning".to_string()));
    }

    #[test]
    fn persists_terminal_error_code_for_failed_snapshots() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-18T10:00:02Z".to_string(),
        );
        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        let detail = store
            .append_scan_run_snapshot_with_error_code(
                &ScanRunSnapshot {
                    seq: 2,
                    snapshot_at: "2026-04-18T10:00:35Z".to_string(),
                    created_at: "2026-04-18T10:00:02Z".to_string(),
                    status: ScanRunStatus::Failed,
                    files_discovered: 9,
                    directories_discovered: 3,
                    items_discovered: 12,
                    items_scanned: 9,
                    errors_count: 2,
                    bytes_processed: 512,
                    scan_rate_items_per_sec: 0.0,
                    progress_percent: Some(75.0),
                    current_path: Some("C:\\scan-root\\nested".to_string()),
                    message: Some("snapshot insert failed".to_string()),
                    ..sample_running_snapshot(2)
                },
                Some("SNAPSHOT_WRITE_FAILED"),
            )
            .expect("failed snapshot should append");

        assert_eq!(detail.header.status, ScanRunStatus::Failed);
        assert_eq!(
            detail.header.error_code,
            Some("SNAPSHOT_WRITE_FAILED".to_string())
        );
        assert_eq!(
            detail.header.error_message,
            Some("snapshot insert failed".to_string())
        );
    }

    #[test]
    fn rejects_out_of_order_scan_run_snapshot_sequences() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-18T10:00:02Z".to_string(),
        );
        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        let error = store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                seq: 3,
                snapshot_at: "2026-04-18T10:00:30Z".to_string(),
                created_at: "2026-04-18T10:00:02Z".to_string(),
                ..sample_running_snapshot(3)
            })
            .expect_err("out of order snapshot should fail");

        assert_eq!(
            error,
            HistoryStoreError::InvalidScanRunSequence {
                run_id: "run-1".to_string(),
                expected: 2,
                actual: 3,
            }
        );
    }

    #[test]
    fn rejects_regressive_scan_run_counters() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-18T10:00:02Z".to_string(),
        );
        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                seq: 2,
                snapshot_at: "2026-04-18T10:00:30Z".to_string(),
                created_at: "2026-04-18T10:00:02Z".to_string(),
                files_discovered: 9,
                directories_discovered: 3,
                items_discovered: 12,
                items_scanned: 9,
                errors_count: 1,
                bytes_processed: 512,
                scan_rate_items_per_sec: 128.0,
                progress_percent: Some(75.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                ..sample_running_snapshot(2)
            })
            .expect("progress snapshot should append");

        let error = store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                seq: 3,
                snapshot_at: "2026-04-18T10:00:35Z".to_string(),
                created_at: "2026-04-18T10:00:02Z".to_string(),
                files_discovered: 9,
                directories_discovered: 3,
                items_discovered: 12,
                items_scanned: 8,
                errors_count: 1,
                bytes_processed: 512,
                scan_rate_items_per_sec: 64.0,
                progress_percent: Some(75.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                ..sample_running_snapshot(3)
            })
            .expect_err("regressive counters should fail");

        assert_eq!(
            error,
            HistoryStoreError::InvalidScanRunCounters {
                run_id: "run-1".to_string(),
                field: "items_scanned".to_string(),
                previous: 9,
                actual: 8,
            }
        );
    }

    #[test]
    fn finalizes_completed_scan_runs_atomically_with_history_payload() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-19T10:00:01Z".to_string(),
        );
        let completed = sample_completed_scan();

        store
            .record_scan_run_started(
                &completed.scan_id,
                &completed.root_path,
                &completed.started_at,
                Some(&completed.root_path),
            )
            .expect("scan run should persist");

        let detail = store
            .finalize_completed_scan_run(
                &completed,
                Some("C:\\scan-root\\large.bin"),
                Some("Scan complete."),
            )
            .expect("finalization should succeed");

        assert_eq!(detail.header.status, ScanRunStatus::Completed);
        assert_eq!(detail.header.latest_seq, 2);
        assert_eq!(
            detail.header.completed_scan_id,
            Some(completed.scan_id.clone())
        );
        assert_eq!(
            detail.header.terminal_at,
            Some(completed.completed_at.clone())
        );
        assert_eq!(detail.latest_snapshot.status, ScanRunStatus::Completed);
        assert_eq!(detail.latest_snapshot.seq, 2);
        assert_eq!(detail.latest_snapshot.files_discovered, completed.total_files);
        assert_eq!(
            detail.latest_snapshot.items_scanned,
            completed.total_files + completed.total_directories
        );
        assert_eq!(detail.latest_snapshot.progress_percent, Some(100.0));

        let reopened = store
            .open_history_entry(&completed.scan_id)
            .expect("completed payload should reopen");
        assert_eq!(reopened, completed);
    }

    #[test]
    fn finalization_rolls_back_continuity_when_completed_history_write_fails() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_test_finalize_hook(
            fixture.path().join("history.db"),
            || "2026-04-19T10:00:01Z".to_string(),
            || Err(HistoryStoreError::Persistence("injected finalize failure".to_string())),
        );
        let completed = sample_completed_scan();

        store
            .record_scan_run_started(
                &completed.scan_id,
                &completed.root_path,
                &completed.started_at,
                Some(&completed.root_path),
            )
            .expect("scan run should persist");

        let error = store
            .finalize_completed_scan_run(
                &completed,
                Some("C:\\scan-root\\large.bin"),
                Some("Scan complete."),
            )
            .expect_err("finalization should fail");

        assert_eq!(
            error,
            HistoryStoreError::Persistence("injected finalize failure".to_string())
        );

        let run = store
            .open_scan_run(&completed.scan_id)
            .expect("running detail should remain");
        assert_eq!(run.header.status, ScanRunStatus::Running);
        assert_eq!(run.header.latest_seq, 1);

        let missing_history = store
            .open_history_entry(&completed.scan_id)
            .expect_err("completed payload should not exist");
        assert_eq!(
            missing_history,
            HistoryStoreError::NotFound {
                scan_id: completed.scan_id.clone(),
            }
        );
    }

    #[test]
    fn scan_run_snapshot_rows_capture_created_at_from_the_store_clock() {
        let fixture = tempdir().expect("db fixture");
        let times = std::sync::Arc::new(std::sync::Mutex::new(vec![
            "2026-04-18T10:00:01Z".to_string(),
            "2026-04-18T10:00:01Z".to_string(),
            "2026-04-18T10:00:31Z".to_string(),
            "2026-04-18T10:00:31Z".to_string(),
        ]));
        let clock = std::sync::Arc::clone(&times);
        let store = HistoryStore::with_now(fixture.path().join("history.db"), move || {
            clock
                .lock()
                .expect("clock lock")
                .remove(0)
        });

        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                seq: 2,
                snapshot_at: "2026-04-18T10:00:30Z".to_string(),
                created_at: "2026-04-18T10:00:31Z".to_string(),
                ..sample_running_snapshot(2)
            })
            .expect("progress snapshot should append");

        let connection =
            rusqlite::Connection::open(store.db_path()).expect("history database should open");
        let created_at = connection
            .prepare(
                r#"
                SELECT created_at
                FROM scan_run_snapshots
                WHERE run_id = ?1
                ORDER BY seq ASC;
                "#,
            )
            .expect("created_at query")
            .query_map(rusqlite::params!["run-1"], |row| row.get::<_, String>(0))
            .expect("created_at rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("created_at values");

        assert_eq!(
            created_at,
            vec![
                "2026-04-18T10:00:01Z".to_string(),
                "2026-04-18T10:00:31Z".to_string(),
            ]
        );
    }

    #[test]
    fn scan_run_reconcile_marks_running_run_stale_once() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-04-19T10:00:01Z",
                "2026-04-19T10:03:00Z",
                "2026-04-19T10:03:00Z",
                "2026-04-19T10:04:00Z",
                "2026-04-19T10:04:00Z",
            ]),
        );

        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-19T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        store
            .reconcile_scan_runs()
            .expect("reconciliation should succeed");

        let detail = store
            .open_scan_run("run-1")
            .expect("stale run should reopen");
        assert_eq!(detail.header.status, ScanRunStatus::Stale);
        assert_eq!(detail.latest_snapshot.status, ScanRunStatus::Stale);
        assert_eq!(detail.header.latest_seq, 2);
        assert_eq!(
            detail.header.stale_since,
            Some("2026-04-19T10:03:00Z".to_string())
        );
        assert_eq!(
            load_audit_reason_codes(&store, "run-1"),
            vec![REASON_HEARTBEAT_STALE.to_string()]
        );

        store
            .reconcile_scan_runs()
            .expect("repeat reconciliation should stay idempotent");

        let detail = store
            .open_scan_run("run-1")
            .expect("stale run should still reopen");
        assert_eq!(detail.header.status, ScanRunStatus::Stale);
        assert_eq!(detail.header.latest_seq, 2);
        assert_eq!(
            load_audit_reason_codes(&store, "run-1"),
            vec![REASON_HEARTBEAT_STALE.to_string()]
        );
    }

    #[test]
    fn scan_run_reconcile_prefers_abandoned_over_stale_in_single_pass() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-04-18T09:00:01Z",
                "2026-04-19T10:00:00Z",
                "2026-04-19T10:00:00Z",
            ]),
        );

        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T09:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        store
            .reconcile_scan_runs()
            .expect("reconciliation should succeed");

        let detail = store
            .open_scan_run("run-1")
            .expect("abandoned run should reopen");
        assert_eq!(detail.header.status, ScanRunStatus::Abandoned);
        assert_eq!(detail.latest_snapshot.status, ScanRunStatus::Abandoned);
        assert_eq!(detail.header.latest_seq, 2);
        assert_eq!(
            detail.header.terminal_at,
            Some("2026-04-19T10:00:00Z".to_string())
        );
        assert_eq!(
            load_audit_reason_codes(&store, "run-1"),
            vec![REASON_RUN_ABANDONED.to_string()]
        );
    }

    #[test]
    fn scan_run_reconcile_abandons_old_stale_runs() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-04-18T09:00:01Z",
                "2026-04-18T09:02:01Z",
                "2026-04-19T10:00:00Z",
                "2026-04-19T10:00:00Z",
            ]),
        );

        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-18T09:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-18T09:02:00Z".to_string(),
                created_at: "2026-04-18T09:02:01Z".to_string(),
                status: ScanRunStatus::Stale,
                files_discovered: 0,
                directories_discovered: 0,
                items_discovered: 0,
                items_scanned: 0,
                errors_count: 0,
                bytes_processed: 0,
                scan_rate_items_per_sec: 0.0,
                progress_percent: None,
                current_path: Some("C:\\scan-root".to_string()),
                message: Some("Run marked stale during startup reconciliation.".to_string()),
            })
            .expect("stale snapshot should append");

        store
            .reconcile_scan_runs()
            .expect("reconciliation should succeed");

        let detail = store
            .open_scan_run("run-1")
            .expect("abandoned run should reopen");
        assert_eq!(detail.header.status, ScanRunStatus::Abandoned);
        assert_eq!(detail.latest_snapshot.status, ScanRunStatus::Abandoned);
        assert_eq!(detail.header.latest_seq, 3);
        assert_eq!(
            detail.header.stale_since,
            Some("2026-04-18T09:02:00Z".to_string())
        );
        assert_eq!(
            load_audit_reason_codes(&store, "run-1"),
            vec![REASON_RUN_ABANDONED.to_string()]
        );
    }

    #[test]
    fn scan_run_purge_removes_old_terminal_runs_and_verifies_deletion() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-03-01T10:00:01Z",
                "2026-03-01T10:00:02Z",
                "2026-04-19T10:00:00Z",
            ]),
        );

        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-03-01T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-03-01T10:00:01Z".to_string(),
                created_at: "2026-03-01T10:00:02Z".to_string(),
                status: ScanRunStatus::Failed,
                files_discovered: 0,
                directories_discovered: 0,
                items_discovered: 0,
                items_scanned: 0,
                errors_count: 1,
                bytes_processed: 0,
                scan_rate_items_per_sec: 0.0,
                progress_percent: None,
                current_path: Some("C:\\scan-root".to_string()),
                message: Some("walk failed".to_string()),
            })
            .expect("terminal snapshot should append");

        let purged = store
            .purge_expired_scan_runs()
            .expect("purge should succeed");

        assert_eq!(purged.purged_count, 1);
        assert_eq!(purged.deleted_run_ids, vec!["run-1".to_string()]);
        assert_eq!(
            store.open_scan_run("run-1").expect_err("run should be deleted"),
            HistoryStoreError::NotFound {
                scan_id: "run-1".to_string(),
            }
        );
        assert_eq!(
            load_audit_events_without_run_id(&store, PURGE_EVENT_TYPE),
            vec![REASON_RETENTION_EXPIRED.to_string()]
        );
    }

    #[test]
    fn scan_run_purge_preserves_recoverable_and_recent_runs() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-04-01T10:00:01Z",
                "2026-04-01T10:00:02Z",
                "2026-04-19T10:00:01Z",
                "2026-04-19T10:00:02Z",
                "2026-04-19T10:00:03Z",
            ]),
        );

        store
            .record_scan_run_started(
                "stale-run",
                "C:\\scan-root",
                "2026-04-01T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("stale run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "stale-run".to_string(),
                seq: 2,
                snapshot_at: "2026-04-01T10:00:01Z".to_string(),
                created_at: "2026-04-01T10:00:02Z".to_string(),
                status: ScanRunStatus::Stale,
                files_discovered: 0,
                directories_discovered: 0,
                items_discovered: 0,
                items_scanned: 0,
                errors_count: 0,
                bytes_processed: 0,
                scan_rate_items_per_sec: 0.0,
                progress_percent: None,
                current_path: Some("C:\\scan-root".to_string()),
                message: Some("Run marked stale during startup reconciliation.".to_string()),
            })
            .expect("stale snapshot should append");
        store
            .record_scan_run_started(
                "recent-terminal",
                "C:\\recent-root",
                "2026-04-19T10:00:00Z",
                Some("C:\\recent-root"),
            )
            .expect("recent run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "recent-terminal".to_string(),
                seq: 2,
                snapshot_at: "2026-04-19T10:00:01Z".to_string(),
                created_at: "2026-04-19T10:00:02Z".to_string(),
                status: ScanRunStatus::Failed,
                files_discovered: 0,
                directories_discovered: 0,
                items_discovered: 0,
                items_scanned: 0,
                errors_count: 1,
                bytes_processed: 0,
                scan_rate_items_per_sec: 0.0,
                progress_percent: None,
                current_path: Some("C:\\recent-root".to_string()),
                message: Some("walk failed".to_string()),
            })
            .expect("recent terminal snapshot should append");

        let purged = store
            .purge_expired_scan_runs()
            .expect("purge should succeed");

        assert_eq!(purged.purged_count, 0);
        assert!(store.open_scan_run("stale-run").is_ok());
        assert!(store.open_scan_run("recent-terminal").is_ok());
    }

    #[test]
    fn scan_run_purge_preserves_completed_history_payloads() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-03-01T10:00:01Z",
                "2026-04-19T10:00:00Z",
            ]),
        );
        let completed = sample_completed_scan();

        store
            .record_scan_run_started(
                &completed.scan_id,
                &completed.root_path,
                "2026-03-01T09:59:00Z",
                Some(&completed.root_path),
            )
            .expect("scan run should persist");
        store
            .finalize_completed_scan_run(
                &CompletedScan {
                    started_at: "2026-03-01T09:59:00Z".to_string(),
                    completed_at: "2026-03-01T10:00:00Z".to_string(),
                    ..completed.clone()
                },
                Some("C:\\scan-root"),
                Some("Scan complete."),
            )
            .expect("completion should persist");

        let purged = store
            .purge_expired_scan_runs()
            .expect("purge should succeed");

        assert_eq!(purged.purged_count, 1);
        let reopened = store
            .open_history_entry(&completed.scan_id)
            .expect("completed history payload should remain");
        assert_eq!(reopened.scan_id, completed.scan_id);
    }

    #[test]
    fn scan_run_purge_removes_abandoned_runs_when_engine_resume_is_unsupported() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-03-01T10:00:01Z",
                "2026-04-19T10:00:00Z",
            ]),
        );

        store
            .record_scan_run_started(
                "resume-run",
                "C:\\scan-root",
                "2026-03-01T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "resume-run".to_string(),
                seq: 2,
                snapshot_at: "2026-03-01T10:00:00Z".to_string(),
                created_at: "2026-03-01T10:00:01Z".to_string(),
                status: ScanRunStatus::Abandoned,
                files_discovered: 0,
                directories_discovered: 0,
                items_discovered: 0,
                items_scanned: 0,
                errors_count: 0,
                bytes_processed: 0,
                scan_rate_items_per_sec: 0.0,
                progress_percent: None,
                current_path: Some("C:\\scan-root".to_string()),
                message: Some("Run marked abandoned during startup reconciliation.".to_string()),
            })
            .expect("abandoned snapshot should append");

        let connection =
            rusqlite::Connection::open(store.db_path()).expect("history database should open");
        connection
            .execute(
                r#"
                UPDATE scan_runs
                SET resume_enabled = 1,
                    resume_token = 'resume-token',
                    resume_expires_at = '2026-05-01T00:00:00Z',
                    resume_payload_json = '{"currentPath":"C:\\scan-root","latestSeq":2}',
                    resume_target_fingerprint_json = '{"rootPath":"C:\\scan-root","targetId":"C:\\scan-root"}'
                WHERE run_id = 'resume-run';
                "#,
                [],
            )
            .expect("resume metadata should persist");

        let before = store
            .open_scan_run("resume-run")
            .expect("resume-metadata run should reopen");
        assert!(before.has_resume);
        assert!(!before.can_resume);

        let purged = store
            .purge_expired_scan_runs()
            .expect("purge should succeed");

        assert_eq!(purged.purged_count, 1);
        assert_eq!(purged.deleted_run_ids, vec!["resume-run".to_string()]);
        assert_eq!(
            store
                .open_scan_run("resume-run")
                .expect_err("unsupported-engine abandoned run should purge"),
            HistoryStoreError::NotFound {
                scan_id: "resume-run".to_string(),
            }
        );
    }

    #[test]
    fn scan_run_purge_preserves_prior_audit_evidence() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-03-01T10:00:01Z",
                "2026-04-19T10:00:00Z",
            ]),
        );

        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-03-01T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-03-01T10:00:00Z".to_string(),
                created_at: "2026-03-01T10:00:01Z".to_string(),
                status: ScanRunStatus::Failed,
                files_discovered: 0,
                directories_discovered: 0,
                items_discovered: 0,
                items_scanned: 0,
                errors_count: 1,
                bytes_processed: 64,
                scan_rate_items_per_sec: 0.0,
                progress_percent: None,
                current_path: Some("C:\\scan-root".to_string()),
                message: Some("walk failed".to_string()),
            })
            .expect("terminal snapshot should append");
        let connection =
            rusqlite::Connection::open(store.db_path()).expect("history database should open");
        connection
            .execute(
                r#"
                INSERT INTO scan_run_audit (
                    run_id,
                    event_type,
                    reason_code,
                    created_at,
                    event_json
                ) VALUES (?1, ?2, ?3, ?4, ?5);
                "#,
                rusqlite::params![
                    "run-1",
                    CANCEL_EVENT_TYPE,
                    REASON_USER_CANCELLED,
                    "2026-03-01T10:00:01Z",
                    r#"{"fromStatus":"stale","toStatus":"cancelled","snapshotAt":"2026-03-01T10:00:01Z"}"#
                ],
            )
            .expect("audit row should persist");

        let purged = store
            .purge_expired_scan_runs()
            .expect("purge should succeed");

        assert_eq!(purged.purged_count, 1);
        assert_eq!(
            load_audit_events_without_run_id(&store, CANCEL_EVENT_TYPE),
            vec![REASON_USER_CANCELLED.to_string()]
        );
        assert_eq!(
            load_audit_events_without_run_id(&store, PURGE_EVENT_TYPE),
            vec![REASON_RETENTION_EXPIRED.to_string()]
        );
    }

    #[test]
    fn missing_scan_id_returns_not_found() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        store.initialize().expect("schema initialization");

        let error = store
            .open_history_entry("missing-scan-id")
            .expect_err("missing history entry should fail");

        assert_eq!(
            error,
            HistoryStoreError::NotFound {
                scan_id: "missing-scan-id".to_string(),
            }
        );
    }

    #[test]
    fn reopens_legacy_history_payloads_without_browseable_entries() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        store.initialize().expect("schema initialization");

        let connection =
            rusqlite::Connection::open(store.db_path()).expect("history database should open");
        connection
            .execute(
                r#"
                INSERT INTO scan_history (
                    scan_id,
                    root_path,
                    completed_at,
                    total_bytes,
                    scan_json
                ) VALUES (?1, ?2, ?3, ?4, ?5);
                "#,
                rusqlite::params![
                    "legacy-scan",
                    "C:\\scan-root",
                    "2026-04-15T10:00:05Z",
                    42_i64,
                    r#"{
                        "scanId":"legacy-scan",
                        "rootPath":"C:\\scan-root",
                        "startedAt":"2026-04-15T10:00:00Z",
                        "completedAt":"2026-04-15T10:00:05Z",
                        "totalBytes":42,
                        "totalFiles":3,
                        "totalDirectories":2,
                        "largestFiles":[{"path":"C:\\scan-root\\large.bin","sizeBytes":42}],
                        "largestDirectories":[{"path":"C:\\scan-root","sizeBytes":42}],
                        "skippedPaths":[]
                    }"#
                ],
            )
            .expect("legacy payload insert");

        let reopened = store
            .open_history_entry("legacy-scan")
            .expect("legacy payload should reopen");

        assert!(reopened.entries.is_empty());
    }

    #[test]
    fn stores_and_reuses_duplicate_hash_cache_entries() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let key = HashCacheKey {
            path: "C:\\scan-root\\dup.bin".to_string(),
            size_bytes: 42,
            modified_at_millis: 1234,
        };

        store
            .save_partial_hash(&key, "partial-1")
            .expect("partial hash should persist");
        store
            .save_full_hash(&key, "full-1")
            .expect("full hash should persist");

        let cached = store
            .get_cached_hashes(&key)
            .expect("cache lookup should succeed")
            .expect("cache entry should exist");

        assert_eq!(cached.partial_hash.as_deref(), Some("partial-1"));
        assert_eq!(cached.full_hash.as_deref(), Some("full-1"));
    }

    #[test]
    fn duplicate_hash_cache_key_changes_invalidate_lookup() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let key = HashCacheKey {
            path: "C:\\scan-root\\dup.bin".to_string(),
            size_bytes: 42,
            modified_at_millis: 1234,
        };
        let changed_key = HashCacheKey {
            modified_at_millis: 5678,
            ..key.clone()
        };

        store
            .save_full_hash(&key, "full-1")
            .expect("full hash should persist");

        assert!(store
            .get_cached_hashes(&changed_key)
            .expect("changed lookup should succeed")
            .is_none());
    }

    #[test]
    fn batch_duplicate_hash_cache_save_merges_partial_and_full_hashes() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let key = HashCacheKey {
            path: "C:\\scan-root\\dup.bin".to_string(),
            size_bytes: 42,
            modified_at_millis: 1234,
        };

        store
            .save_hashes_batch(&[
                HashCacheWrite {
                    key: key.clone(),
                    partial_hash: Some("partial-1".to_string()),
                    full_hash: None,
                },
                HashCacheWrite {
                    key: key.clone(),
                    partial_hash: None,
                    full_hash: Some("full-1".to_string()),
                },
            ])
            .expect("batch hash save should persist");

        let cached = store
            .get_cached_hashes(&key)
            .expect("cache lookup should succeed")
            .expect("cache entry should exist");

        assert_eq!(cached.partial_hash.as_deref(), Some("partial-1"));
        assert_eq!(cached.full_hash.as_deref(), Some("full-1"));
    }

    #[test]
    fn batch_duplicate_hash_cache_lookup_returns_only_requested_entries() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let existing = HashCacheKey {
            path: "C:\\scan-root\\dup.bin".to_string(),
            size_bytes: 42,
            modified_at_millis: 1234,
        };
        let missing = HashCacheKey {
            path: "C:\\scan-root\\missing.bin".to_string(),
            size_bytes: 84,
            modified_at_millis: 5678,
        };

        store
            .save_full_hash(&existing, "full-1")
            .expect("full hash should persist");

        let entries = store
            .get_cached_hashes_batch(&[existing.clone(), missing.clone()])
            .expect("batch lookup should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries.get(&existing).and_then(|entry| entry.full_hash.as_deref()),
            Some("full-1")
        );
        assert!(!entries.contains_key(&missing));
    }

    #[test]
    fn persists_cleanup_execution_logs() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let expected = CleanupExecutionResult {
            execution_id: "execution-1".to_string(),
            preview_id: "preview-1".to_string(),
            mode: CleanupExecutionMode::Recycle,
            completed_at: "2026-04-15T11:03:00Z".to_string(),
            completed_count: 1,
            failed_count: 1,
            entries: vec![
                CleanupExecutionEntry {
                    action_id: "action-1".to_string(),
                    path: "C:\\scan-root\\left.bin".to_string(),
                    status: CleanupExecutionItemStatus::Completed,
                    summary: "Moved to the Recycle Bin.".to_string(),
                },
                CleanupExecutionEntry {
                    action_id: "action-2".to_string(),
                    path: "C:\\scan-root\\right.bin".to_string(),
                    status: CleanupExecutionItemStatus::Failed,
                    summary: "File metadata changed after cleanup preview was generated."
                        .to_string(),
                },
            ],
        };

        store
            .save_cleanup_execution(&expected)
            .expect("cleanup execution should persist");

        let reopened = store
            .open_cleanup_execution(&expected.execution_id)
            .expect("cleanup execution should reopen");

        assert_eq!(reopened, expected);
    }

    #[test]
    fn scan_run_detail_keeps_abandoned_status_while_still_resumable() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-19T10:00:00Z".to_string(),
        );

        store
            .record_scan_run_started_with_options(
                "run-1",
                "C:\\scan-root",
                "2026-04-19T09:00:00Z",
                ScanRunStartOptions {
                    current_path: Some("C:\\scan-root\\nested"),
                    target_id: Some("C:\\scan-root"),
                    resume_enabled: true,
                    resume_token: Some("resume-run-1"),
                    resume_expires_at: Some("2099-04-20T09:00:00Z"),
                    resume_payload_json: Some(
                        "{\"currentPath\":\"C:\\\\scan-root\\\\nested\",\"latestSeq\":2}",
                    ),
                    resume_target_fingerprint_json: Some(
                        "{\"rootPath\":\"C:\\\\scan-root\",\"targetId\":\"C:\\\\scan-root\"}",
                    ),
                    privacy_scope_id: Some(DEFAULT_SCAN_RUN_PRIVACY_SCOPE_ID),
                    ..ScanRunStartOptions::default()
                },
            )
            .expect("resume-enabled run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-19T09:05:00Z".to_string(),
                created_at: "2026-04-19T09:05:00Z".to_string(),
                status: ScanRunStatus::Abandoned,
                files_discovered: 3,
                directories_discovered: 1,
                items_discovered: 4,
                items_scanned: 4,
                errors_count: 0,
                bytes_processed: 256,
                scan_rate_items_per_sec: 0.0,
                progress_percent: Some(50.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                message: Some("Run marked abandoned during startup reconciliation.".to_string()),
            })
            .expect("abandoned snapshot should append");

        let detail = store.open_scan_run("run-1").expect("run should reopen");

        assert_eq!(detail.header.status, ScanRunStatus::Abandoned);
        assert!(detail.has_resume);
        assert!(!detail.can_resume);
        assert_eq!(detail.seq, 2);
        assert_eq!(detail.created_at, detail.header.created_at);
        assert_eq!(detail.items_scanned, detail.latest_snapshot.items_scanned);
        assert_eq!(detail.errors_count, detail.latest_snapshot.errors_count);
        assert_eq!(detail.progress_percent, detail.latest_snapshot.progress_percent);
        assert_eq!(
            detail.scan_rate_items_per_sec,
            detail.latest_snapshot.scan_rate_items_per_sec
        );
    }

    #[test]
    fn serialized_run_detail_omits_raw_resume_token_fields() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-19T10:00:00Z".to_string(),
        );

        store
            .record_scan_run_started_with_options(
                "run-1",
                "C:\\scan-root",
                "2026-04-19T09:00:00Z",
                ScanRunStartOptions {
                    current_path: Some("C:\\scan-root"),
                    target_id: Some("C:\\scan-root"),
                    resume_enabled: true,
                    resume_token: Some("resume-run-1"),
                    resume_expires_at: Some("2099-04-20T09:00:00Z"),
                    resume_payload_json: Some(
                        "{\"currentPath\":\"C:\\\\scan-root\",\"latestSeq\":1}",
                    ),
                    resume_target_fingerprint_json: Some(
                        "{\"rootPath\":\"C:\\\\scan-root\",\"targetId\":\"C:\\\\scan-root\"}",
                    ),
                    privacy_scope_id: Some(DEFAULT_SCAN_RUN_PRIVACY_SCOPE_ID),
                    ..ScanRunStartOptions::default()
                },
            )
            .expect("resume-enabled run should persist");

        let detail = store.open_scan_run("run-1").expect("run should reopen");
        let serialized = serde_json::to_value(&detail).expect("run detail should serialize");
        let header = serialized
            .get("header")
            .expect("serialized run detail should include header");

        assert!(header.get("resumeToken").is_none());
        assert!(header.get("resumeEnabled").is_none());
        assert!(header.get("resumeExpiresAt").is_none());
        assert!(header.get("resumePayloadJson").is_none());
        assert!(header.get("resumeTargetFingerprintJson").is_none());
        assert!(header.get("privacyScopeId").is_none());
        assert_eq!(serialized["hasResume"], true);
        assert_eq!(serialized["canResume"], false);
    }

    #[test]
    fn scan_run_summary_exposes_ui_card_metrics_and_normalized_progress() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-04-21T10:00:00Z",
                "2026-04-21T10:00:01Z",
                "2026-04-21T10:00:02Z",
            ]),
        );

        store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-21T09:55:00Z",
                Some("C:\\scan-root"),
            )
            .expect("run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-21T10:00:01Z".to_string(),
                created_at: "2026-04-21T10:00:01Z".to_string(),
                status: ScanRunStatus::Stale,
                files_discovered: 10,
                directories_discovered: 2,
                items_discovered: 12,
                items_scanned: 12,
                errors_count: 2,
                bytes_processed: 4096,
                scan_rate_items_per_sec: 3.5,
                progress_percent: Some(140.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                message: None,
            })
            .expect("snapshot should append");

        let summary = store
            .list_scan_runs()
            .expect("summary list should load")
            .into_iter()
            .find(|entry| entry.header.run_id == "run-1")
            .expect("run summary should exist");

        assert_eq!(summary.seq, 2);
        assert_eq!(summary.created_at, "2026-04-21T10:00:00Z");
        assert_eq!(summary.items_scanned, 12);
        assert_eq!(summary.errors_count, 2);
        assert_eq!(summary.progress_percent, Some(100.0));
        assert_eq!(summary.scan_rate_items_per_sec, 3.5);
    }

    #[test]
    fn scan_run_with_resume_metadata_reports_can_resume_false_when_engine_unsupported() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-21T10:00:00Z".to_string(),
        );
        store
            .record_scan_run_started_with_options(
                "run-1",
                "C:\\scan-root",
                "2026-04-21T09:00:00Z",
                ScanRunStartOptions {
                    current_path: Some("C:\\scan-root\\nested"),
                    target_id: Some("C:\\scan-root"),
                    resumed_from_run_id: None,
                    resume_enabled: true,
                    resume_token: Some("resume-run-1"),
                    resume_expires_at: Some("2099-04-20T09:00:00Z"),
                    resume_payload_json: Some(
                        "{\"currentPath\":\"C:\\\\scan-root\\\\nested\",\"latestSeq\":2}",
                    ),
                    resume_target_fingerprint_json: Some(
                        "{\"rootPath\":\"C:\\\\scan-root\",\"targetId\":\"C:\\\\scan-root\"}",
                    ),
                    privacy_scope_id: Some(DEFAULT_SCAN_RUN_PRIVACY_SCOPE_ID),
                },
            )
            .expect("resume-enabled run should persist");
        store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-21T09:05:00Z".to_string(),
                created_at: "2026-04-21T09:05:00Z".to_string(),
                status: ScanRunStatus::Abandoned,
                files_discovered: 10,
                directories_discovered: 2,
                items_discovered: 12,
                items_scanned: 12,
                errors_count: 0,
                bytes_processed: 4096,
                scan_rate_items_per_sec: 3.5,
                progress_percent: Some(65.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                message: Some("Run marked abandoned during startup reconciliation.".to_string()),
            })
            .expect("abandoned snapshot should append");

        let detail = store.open_scan_run("run-1").expect("run should reopen");

        assert!(detail.has_resume);
        assert!(!detail.can_resume);
    }
}
