use crate::state::{ActiveScanHandle, AppState, ScanManager, ScanRunPersistenceCursor};
use app_db::{HistoryStore, PurgedScanRuns};
use chrono::{DateTime, Utc};
use scan_core::{
    CompletedScan, ScanFailure, ScanLifecycleState, ScanRequest, ScanRunSnapshot, ScanRunStatus,
    ScanStatusSnapshot, DEFAULT_TOP_ITEMS_LIMIT,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScanResponse {
    pub scan_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanRunCommandError {
    pub code: &'static str,
    pub message: String,
    pub run_id: Option<String>,
}

const CONTINUITY_ERROR_CODE_SNAPSHOT_WRITE_FAILED: &str = "SNAPSHOT_WRITE_FAILED";
const CONTINUITY_ERROR_CODE_FINALIZATION_FAILED: &str = "FINALIZATION_FAILED";
const HEARTBEAT_INTERVAL_SECONDS: i64 = 30;
const HEARTBEAT_LOOP_POLL_INTERVAL: Duration = Duration::from_secs(1);
const NO_PROGRESS_WARNING_INTERVALS: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FailedRunCause {
    ScanExecution,
    SnapshotWrite,
}

#[tauri::command]
pub fn start_scan(
    root_path: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<StartScanResponse, String> {
    let normalized_root = root_path.trim().to_string();
    if normalized_root.is_empty() {
        return Err("Enter a folder or drive path before starting a scan.".to_string());
    }

    let scan_id = scan_core::make_scan_id();
    let active_scan =
        state
            .scan_manager
            .start(scan_id.clone(), normalized_root.clone())?;
    if let Err(error) = state.history_store.record_scan_run_started(
        &active_scan.scan_id,
        &normalized_root,
        &active_scan.started_at,
        Some(&normalized_root),
    ) {
        state.scan_manager.finish(ScanStatusSnapshot::default());
        return Err(error.to_string());
    }
    let initial_snapshot = state.scan_manager.status()?;
    let _ = emit_snapshot(&app, &initial_snapshot);

    let app_handle = app.clone();
    let scan_manager = state.scan_manager.clone();
    let history_store = state.history_store.clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_scan_task(
            app_handle,
            scan_manager,
            history_store,
            active_scan,
            normalized_root,
        );
    });

    Ok(StartScanResponse { scan_id })
}

#[tauri::command]
pub fn cancel_active_scan(state: State<'_, AppState>) -> Result<(), String> {
    state.scan_manager.cancel_active_scan();
    Ok(())
}

#[tauri::command]
pub fn cancel_scan_run(
    run_id: String,
    state: State<'_, AppState>,
) -> Result<(), ScanRunCommandError> {
    cancel_scan_run_impl(&state.history_store, &state.scan_manager, &run_id)
}

#[tauri::command]
pub fn get_scan_status(state: State<'_, AppState>) -> Result<ScanStatusSnapshot, String> {
    state.scan_manager.status()
}

fn run_scan_task(
    app: AppHandle,
    scan_manager: ScanManager,
    history_store: HistoryStore,
    active_scan: ActiveScanHandle,
    root_path: String,
) {
    let persistence_error = Arc::new(Mutex::new(None::<String>));
    let callback_error = Arc::clone(&persistence_error);
    let callback_cancel = Arc::clone(&active_scan.cancel_flag);
    let callback_scan_manager = scan_manager.clone();
    let callback_history_store = history_store.clone();
    let callback_scan_id = active_scan.scan_id.clone();
    let mut request = ScanRequest::new(PathBuf::from(&root_path));
    request.top_items_limit = DEFAULT_TOP_ITEMS_LIMIT;
    request.scan_id = Some(active_scan.scan_id.clone());
    request.started_at = Some(active_scan.started_at.clone());
    let persistence_gate = Arc::new(Mutex::new(()));
    let heartbeat_stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let heartbeat_handle = spawn_heartbeat_loop(
        app.clone(),
        scan_manager.clone(),
        history_store.clone(),
        active_scan.scan_id.clone(),
        Arc::clone(&active_scan.cancel_flag),
        Arc::clone(&persistence_error),
        Arc::clone(&persistence_gate),
        Arc::clone(&heartbeat_stop),
    );

    let result = scan_core::scan_path(
        &request,
        || active_scan.cancel_flag.load(Ordering::SeqCst),
        |snapshot| {
            match persist_activity_snapshot(
                &callback_scan_manager,
                &callback_history_store,
                &callback_scan_id,
                &snapshot,
                &persistence_gate,
            ) {
                Ok(()) => {
                    let _ = emit_snapshot(&app, &snapshot);
                }
                Err(error) => {
                    if record_persistence_error(&callback_error, error.clone()) {
                        log_scan_run_failure_event(
                            "scan_run_snapshot_write_failed",
                            &callback_scan_id,
                            CONTINUITY_ERROR_CODE_SNAPSHOT_WRITE_FAILED,
                            &error,
                        );
                    }
                    callback_cancel.store(true, Ordering::SeqCst);
                }
            }
        },
    );
    heartbeat_stop.store(true, Ordering::SeqCst);
    let _ = heartbeat_handle.join();

    if let Some(error_message) = take_persistence_error(&persistence_error) {
        finish_failed_run(
            &app,
            &scan_manager,
            &history_store,
            &active_scan.scan_id,
            &root_path,
            &active_scan.started_at,
            FailedRunCause::SnapshotWrite,
            error_message,
        );
        return;
    }

    match result {
        Ok(completed_scan) => {
            finish_completed_scan(&app, &scan_manager, &history_store, completed_scan)
        }
        Err(ScanFailure::Cancelled) => {
            let _ = persist_terminal_run_snapshot(
                &history_store,
                &scan_manager,
                &active_scan.scan_id,
                ScanRunStatus::Cancelled,
                scan_core::current_timestamp(),
                None,
                "Scan cancelled before history save.".to_string(),
            );
            let latest_snapshot = scan_manager.status().unwrap_or_default();
            let snapshot = terminal_snapshot_from_previous(
                &active_scan.scan_id,
                &root_path,
                ScanLifecycleState::Cancelled,
                Some("Scan cancelled before history save.".to_string()),
                &latest_snapshot,
                &active_scan.started_at,
            );
            scan_manager.finish(snapshot.clone());
            let _ = emit_snapshot(&app, &snapshot);
        }
        Err(error) => {
            finish_failed_run(
                &app,
                &scan_manager,
                &history_store,
                &active_scan.scan_id,
                &root_path,
                &active_scan.started_at,
                FailedRunCause::ScanExecution,
                error.to_string(),
            );
        }
    }
}

fn spawn_heartbeat_loop(
    app: AppHandle,
    scan_manager: ScanManager,
    history_store: HistoryStore,
    scan_id: String,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    persistence_error: Arc<Mutex<Option<String>>>,
    persistence_gate: Arc<Mutex<()>>,
    stop_flag: Arc<std::sync::atomic::AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !stop_flag.load(Ordering::SeqCst) {
            thread::sleep(HEARTBEAT_LOOP_POLL_INTERVAL);
            if stop_flag.load(Ordering::SeqCst)
                || cancel_flag.load(Ordering::SeqCst)
                || has_recorded_persistence_error(&persistence_error)
            {
                break;
            }

            let outcome = match run_heartbeat_iteration(
                &scan_manager,
                &history_store,
                &scan_id,
                &cancel_flag,
                &persistence_error,
                &persistence_gate,
            ) {
                Ok(Some(outcome)) => outcome,
                Ok(None) => continue,
                Err(error) => {
                    if record_persistence_error(&persistence_error, error.clone()) {
                        log_scan_run_failure_event(
                            "scan_run_snapshot_write_failed",
                            &scan_id,
                            CONTINUITY_ERROR_CODE_SNAPSHOT_WRITE_FAILED,
                            &error,
                        );
                    }
                    cancel_flag.store(true, Ordering::SeqCst);
                    break;
                }
            };

            let _ = emit_snapshot(&app, &outcome.snapshot);
            if outcome.emit_no_progress_warning {
                log_scan_run_no_progress_warning(
                    &scan_id,
                    outcome.last_progress_at.as_deref(),
                    outcome.unchanged_heartbeat_count,
                );
            }
        }
    })
}

fn finish_completed_scan(
    app: &AppHandle,
    scan_manager: &ScanManager,
    history_store: &HistoryStore,
    completed_scan: CompletedScan,
) {
    let latest_snapshot = scan_manager.status().unwrap_or_default();

    let snapshot = completed_snapshot(&completed_scan, &latest_snapshot);
    let finalization = history_store.finalize_completed_scan_run(
        &completed_scan,
        snapshot.current_path.as_deref(),
        snapshot.message.as_deref(),
    );

    match finalization {
        Ok(detail) => {
            let _ = scan_manager.mark_snapshot_persisted(&detail.latest_snapshot);
            scan_manager.finish(snapshot.clone());
            let _ = emit_snapshot(app, &snapshot);
        }
        Err(error) => {
            let error_message = error.to_string();
            log_scan_run_failure_event(
                "scan_run_finalization_failed",
                &completed_scan.scan_id,
                CONTINUITY_ERROR_CODE_FINALIZATION_FAILED,
                &error_message,
            );
            let _ = persist_terminal_run_snapshot(
                history_store,
                scan_manager,
                &completed_scan.scan_id,
                ScanRunStatus::Failed,
                completed_scan.completed_at.clone(),
                Some(CONTINUITY_ERROR_CODE_FINALIZATION_FAILED),
                error_message.clone(),
            );
            let failed_snapshot = terminal_snapshot_from_previous(
                &completed_scan.scan_id,
                &completed_scan.root_path,
                ScanLifecycleState::Failed,
                Some(error_message),
                &latest_snapshot,
                &completed_scan.started_at,
            );
            scan_manager.finish(failed_snapshot.clone());
            let _ = emit_snapshot(app, &failed_snapshot);
        }
    }
}

fn scan_run_command_error(
    error: app_db::HistoryStoreError,
    run_id: Option<&str>,
) -> ScanRunCommandError {
    match error {
        app_db::HistoryStoreError::NotFound { .. } => ScanRunCommandError {
            code: "NOT_FOUND",
            message: match run_id {
                Some(run_id) => format!("scan run not found: {run_id}"),
                None => "scan run not found".to_string(),
            },
            run_id: run_id.map(str::to_string),
        },
        app_db::HistoryStoreError::Conflict { status, .. } => ScanRunCommandError {
            code: "CONFLICT",
            message: match run_id {
                Some(run_id) => format!("scan run cannot be cancelled from status {status}: {run_id}"),
                None => format!("scan run cannot be cancelled from status {status}"),
            },
            run_id: run_id.map(str::to_string),
        },
        other => ScanRunCommandError {
            code: "PERSISTENCE_ERROR",
            message: other.to_string(),
            run_id: run_id.map(str::to_string),
        },
    }
}

fn cancel_scan_run_impl(
    history_store: &HistoryStore,
    scan_manager: &ScanManager,
    run_id: &str,
) -> Result<(), ScanRunCommandError> {
    if scan_manager
        .active_scan_id()
        .map_err(|error| ScanRunCommandError {
            code: "RUNTIME_ERROR",
            message: error,
            run_id: Some(run_id.to_string()),
        })?
        .as_deref()
        == Some(run_id)
    {
        scan_manager.cancel_active_scan();
        return Ok(());
    }

    history_store
        .cancel_non_live_scan_run(run_id)
        .map(|_| ())
        .map_err(|error| scan_run_command_error(error, Some(run_id)))
}

fn finish_failed_run(
    app: &AppHandle,
    scan_manager: &ScanManager,
    history_store: &HistoryStore,
    scan_id: &str,
    root_path: &str,
    started_at: &str,
    cause: FailedRunCause,
    error_message: String,
) {
    let _ = persist_failed_run_continuity(
        history_store,
        scan_manager,
        scan_id,
        cause,
        &error_message,
    );
    let latest_snapshot = scan_manager.status().unwrap_or_default();
    let snapshot = terminal_snapshot_from_previous(
        scan_id,
        root_path,
        ScanLifecycleState::Failed,
        Some(error_message),
        &latest_snapshot,
        started_at,
    );
    scan_manager.finish(snapshot.clone());
    let _ = emit_snapshot(app, &snapshot);
}

fn terminal_snapshot_from_previous(
    scan_id: &str,
    root_path: &str,
    state: ScanLifecycleState,
    message: Option<String>,
    previous: &ScanStatusSnapshot,
    started_at: &str,
) -> ScanStatusSnapshot {
    ScanStatusSnapshot {
        scan_id: Some(scan_id.to_string()),
        root_path: Some(root_path.to_string()),
        state,
        files_discovered: previous.files_discovered,
        directories_discovered: previous.directories_discovered,
        bytes_processed: previous.bytes_processed,
        started_at: previous
            .started_at
            .clone()
            .or_else(|| Some(started_at.to_string())),
        updated_at: Some(scan_core::current_timestamp()),
        current_path: previous
            .current_path
            .clone()
            .or_else(|| Some(root_path.to_string())),
        message,
        completed_scan_id: None,
    }
}

fn completed_snapshot(
    completed_scan: &CompletedScan,
    previous: &ScanStatusSnapshot,
) -> ScanStatusSnapshot {
    ScanStatusSnapshot {
        scan_id: Some(completed_scan.scan_id.clone()),
        root_path: Some(completed_scan.root_path.clone()),
        state: ScanLifecycleState::Completed,
        files_discovered: completed_scan.total_files,
        directories_discovered: completed_scan.total_directories,
        bytes_processed: completed_scan.total_bytes,
        started_at: Some(completed_scan.started_at.clone()),
        updated_at: Some(scan_core::current_timestamp()),
        current_path: previous
            .current_path
            .clone()
            .or_else(|| Some(completed_scan.root_path.clone())),
        message: Some("Scan complete.".to_string()),
        completed_scan_id: Some(completed_scan.scan_id.clone()),
    }
}

fn build_running_run_snapshot(
    run_id: &str,
    snapshot: &ScanStatusSnapshot,
    cursor: &ScanRunPersistenceCursor,
) -> Result<ScanRunSnapshot, String> {
    let snapshot_at = snapshot
        .updated_at
        .clone()
        .or_else(|| snapshot.started_at.clone())
        .unwrap_or_else(scan_core::current_timestamp);
    let items_discovered = snapshot.files_discovered.saturating_add(snapshot.directories_discovered);
    let items_scanned = items_discovered;

    Ok(ScanRunSnapshot {
        run_id: run_id.to_string(),
        seq: cursor.next_seq,
        snapshot_at: snapshot_at.clone(),
        created_at: String::new(),
        status: ScanRunStatus::Running,
        files_discovered: snapshot.files_discovered,
        directories_discovered: snapshot.directories_discovered,
        items_discovered,
        items_scanned,
        errors_count: 0,
        bytes_processed: snapshot.bytes_processed,
        scan_rate_items_per_sec: calculate_items_rate_per_second(
            cursor.last_persisted_items_scanned,
            items_scanned,
            &cursor.last_persisted_snapshot_at,
            &snapshot_at,
        ),
        progress_percent: None,
        current_path: snapshot.current_path.clone(),
        message: snapshot.message.clone(),
    })
}

#[cfg_attr(not(test), allow(dead_code))]
fn build_completed_run_snapshot(
    completed_scan: &CompletedScan,
    cursor: &ScanRunPersistenceCursor,
    current_path: Option<String>,
) -> ScanRunSnapshot {
    let items_scanned = completed_scan
        .total_files
        .saturating_add(completed_scan.total_directories);

    ScanRunSnapshot {
        run_id: completed_scan.scan_id.clone(),
        seq: cursor.next_seq,
        snapshot_at: completed_scan.completed_at.clone(),
        created_at: String::new(),
        status: ScanRunStatus::Completed,
        files_discovered: completed_scan.total_files,
        directories_discovered: completed_scan.total_directories,
        items_discovered: items_scanned,
        items_scanned,
        errors_count: 0,
        bytes_processed: completed_scan.total_bytes,
        scan_rate_items_per_sec: calculate_items_rate_per_second(
            cursor.last_persisted_items_scanned,
            items_scanned,
            &cursor.last_persisted_snapshot_at,
            &completed_scan.completed_at,
        ),
        progress_percent: Some(100.0),
        current_path: current_path.or_else(|| Some(completed_scan.root_path.clone())),
        message: Some("Scan complete.".to_string()),
    }
}

fn build_terminal_run_snapshot(
    run_id: &str,
    status: ScanRunStatus,
    cursor: &ScanRunPersistenceCursor,
    previous: &ScanRunSnapshot,
    snapshot_at: String,
    message: String,
) -> ScanRunSnapshot {
    ScanRunSnapshot {
        run_id: run_id.to_string(),
        seq: cursor.next_seq,
        snapshot_at,
        created_at: String::new(),
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
        message: Some(message),
    }
}

fn calculate_items_rate_per_second(
    previous_items_scanned: u64,
    next_items_scanned: u64,
    previous_snapshot_at: &str,
    snapshot_at: &str,
) -> f64 {
    let Ok(previous) = DateTime::parse_from_rfc3339(previous_snapshot_at) else {
        return 0.0;
    };
    let Ok(next) = DateTime::parse_from_rfc3339(snapshot_at) else {
        return 0.0;
    };

    let elapsed_seconds = next
        .with_timezone(&Utc)
        .signed_duration_since(previous.with_timezone(&Utc))
        .num_seconds();
    if elapsed_seconds <= 0 {
        return 0.0;
    }

    let delta_items = next_items_scanned.saturating_sub(previous_items_scanned) as f64;
    (delta_items / elapsed_seconds as f64).clamp(0.0, 1_000_000.0)
}

#[derive(Debug)]
struct HeartbeatIterationOutcome {
    snapshot: ScanStatusSnapshot,
    emit_no_progress_warning: bool,
    unchanged_heartbeat_count: u32,
    last_progress_at: Option<String>,
}

fn persist_activity_snapshot(
    scan_manager: &ScanManager,
    history_store: &HistoryStore,
    scan_id: &str,
    snapshot: &ScanStatusSnapshot,
    persistence_gate: &Arc<Mutex<()>>,
) -> Result<(), String> {
    let _guard = persistence_gate
        .lock()
        .map_err(|_| "The scan persistence gate is poisoned.".to_string())?;
    let cursor = scan_manager.continuity_cursor()?;
    let run_snapshot = build_running_run_snapshot(scan_id, snapshot, &cursor)?;
    let detail = history_store
        .append_scan_run_snapshot(&run_snapshot)
        .map_err(|error| error.to_string())?;
    scan_manager.mark_snapshot_persisted(&detail.latest_snapshot)?;
    scan_manager.update_running_snapshot(snapshot.clone());
    Ok(())
}

fn run_heartbeat_iteration(
    scan_manager: &ScanManager,
    history_store: &HistoryStore,
    scan_id: &str,
    cancel_flag: &Arc<std::sync::atomic::AtomicBool>,
    persistence_error: &Arc<Mutex<Option<String>>>,
    persistence_gate: &Arc<Mutex<()>>,
) -> Result<Option<HeartbeatIterationOutcome>, String> {
    let _guard = persistence_gate
        .lock()
        .map_err(|_| "The scan persistence gate is poisoned.".to_string())?;
    if cancel_flag.load(Ordering::SeqCst) || has_recorded_persistence_error(persistence_error) {
        return Ok(None);
    }

    let heartbeat = match scan_manager.due_heartbeat(HEARTBEAT_INTERVAL_SECONDS)? {
        Some(heartbeat) => heartbeat,
        None => return Ok(None),
    };
    let heartbeat_snapshot =
        build_running_run_snapshot(scan_id, &heartbeat.snapshot, &heartbeat.cursor)?;
    let detail = history_store
        .append_scan_run_snapshot(&heartbeat_snapshot)
        .map_err(|error| error.to_string())?;
    let heartbeat_result = scan_manager.mark_heartbeat_persisted(
        &heartbeat.snapshot,
        &detail.latest_snapshot,
        NO_PROGRESS_WARNING_INTERVALS,
    )?;

    Ok(Some(HeartbeatIterationOutcome {
        snapshot: heartbeat.snapshot,
        emit_no_progress_warning: heartbeat_result.emit_no_progress_warning,
        unchanged_heartbeat_count: heartbeat_result.unchanged_heartbeat_count,
        last_progress_at: heartbeat_result.last_progress_at,
    }))
}

fn scan_run_failure_event_payload(
    event: &str,
    run_id: &str,
    error_code: &str,
    error: &str,
) -> serde_json::Value {
    serde_json::json!({
        "event": event,
        "runId": run_id,
        "errorCode": error_code,
        "error": error,
        "timestamp": scan_core::current_timestamp(),
    })
}

fn log_scan_run_failure_event(event: &str, run_id: &str, error_code: &str, error: &str) {
    eprintln!(
        "{}",
        scan_run_failure_event_payload(event, run_id, error_code, error)
    );
}

fn scan_run_no_progress_warning_payload(
    run_id: &str,
    last_progress_at: Option<&str>,
    unchanged_heartbeat_count: u32,
) -> serde_json::Value {
    serde_json::json!({
        "event": "scan_run_no_progress_warning",
        "runId": run_id,
        "lastProgressAt": last_progress_at,
        "unchangedHeartbeatCount": unchanged_heartbeat_count,
        "timestamp": scan_core::current_timestamp(),
    })
}

fn log_scan_run_no_progress_warning(
    run_id: &str,
    last_progress_at: Option<&str>,
    unchanged_heartbeat_count: u32,
) {
    eprintln!(
        "{}",
        scan_run_no_progress_warning_payload(
            run_id,
            last_progress_at,
            unchanged_heartbeat_count,
        )
    );
}

fn scan_run_purged_payload(purged: &PurgedScanRuns) -> serde_json::Value {
    serde_json::json!({
        "event": "scan_run_purged",
        "purgedCount": purged.purged_count,
        "runIds": purged.deleted_run_ids,
        "timestamp": scan_core::current_timestamp(),
    })
}

pub(crate) fn log_scan_run_purged(purged: &PurgedScanRuns) {
    if purged.purged_count == 0 {
        return;
    }

    eprintln!("{}", scan_run_purged_payload(purged));
}

fn record_persistence_error(target: &Arc<Mutex<Option<String>>>, message: String) -> bool {
    if let Ok(mut slot) = target.lock() {
        if slot.is_none() {
            *slot = Some(message);
            return true;
        }
    }

    false
}

fn has_recorded_persistence_error(target: &Arc<Mutex<Option<String>>>) -> bool {
    target
        .lock()
        .map(|slot| slot.is_some())
        .unwrap_or(true)
}

fn failed_run_error_code(cause: FailedRunCause) -> Option<&'static str> {
    match cause {
        FailedRunCause::ScanExecution => None,
        FailedRunCause::SnapshotWrite => Some(CONTINUITY_ERROR_CODE_SNAPSHOT_WRITE_FAILED),
    }
}

fn persist_failed_run_continuity(
    history_store: &HistoryStore,
    scan_manager: &ScanManager,
    run_id: &str,
    cause: FailedRunCause,
    error_message: &str,
) -> Result<(), String> {
    persist_terminal_run_snapshot(
        history_store,
        scan_manager,
        run_id,
        ScanRunStatus::Failed,
        scan_core::current_timestamp(),
        failed_run_error_code(cause),
        error_message.to_string(),
    )
}

fn take_persistence_error(target: &Arc<Mutex<Option<String>>>) -> Option<String> {
    target.lock().ok().and_then(|mut slot| slot.take())
}

fn persist_terminal_run_snapshot(
    history_store: &HistoryStore,
    scan_manager: &ScanManager,
    run_id: &str,
    status: ScanRunStatus,
    snapshot_at: String,
    error_code: Option<&str>,
    message: String,
) -> Result<(), String> {
    let detail = history_store
        .open_scan_run(run_id)
        .map_err(|error| error.to_string())?;
    let cursor = scan_manager.continuity_cursor()?;
    let terminal = build_terminal_run_snapshot(
        run_id,
        status,
        &cursor,
        &detail.latest_snapshot,
        snapshot_at,
        message,
    );
    let detail = history_store
        .append_scan_run_snapshot_with_error_code(&terminal, error_code)
        .map_err(|error| error.to_string())?;
    scan_manager.mark_snapshot_persisted(&detail.latest_snapshot)?;
    Ok(())
}

fn emit_snapshot(app: &AppHandle, snapshot: &ScanStatusSnapshot) -> Result<(), String> {
    app.emit("scan-progress", snapshot)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ScanRunPersistenceCursor;
    use scan_core::ScanRunStatus;
    use tempfile::tempdir;

    fn make_previous_running_snapshot() -> ScanStatusSnapshot {
        ScanStatusSnapshot {
            scan_id: Some("scan-1".to_string()),
            root_path: Some("C:\\Users\\xiongxianfei\\Downloads".to_string()),
            state: ScanLifecycleState::Running,
            files_discovered: 8,
            directories_discovered: 3,
            bytes_processed: 4096,
            started_at: Some("2026-04-16T10:00:00Z".to_string()),
            updated_at: Some("2026-04-16T10:00:05Z".to_string()),
            current_path: Some("C:\\Users\\xiongxianfei\\Downloads\\nested".to_string()),
            message: None,
            completed_scan_id: None,
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

    #[test]
    fn continuity_seq_running_snapshot_uses_next_seq_and_rate_context() {
        let snapshot = build_running_run_snapshot(
            "scan-1",
            &make_previous_running_snapshot(),
            &ScanRunPersistenceCursor {
                next_seq: 3,
                last_persisted_snapshot_at: "2026-04-16T10:00:00Z".to_string(),
                last_persisted_items_scanned: 5,
            },
        )
        .expect("running snapshot should build");

        assert_eq!(snapshot.run_id, "scan-1");
        assert_eq!(snapshot.seq, 3);
        assert_eq!(snapshot.status, ScanRunStatus::Running);
        assert_eq!(snapshot.items_discovered, 11);
        assert_eq!(snapshot.items_scanned, 11);
        assert_eq!(snapshot.errors_count, 0);
        assert_eq!(snapshot.snapshot_at, "2026-04-16T10:00:05Z");
        assert!((snapshot.scan_rate_items_per_sec - 1.2).abs() < f64::EPSILON);
    }

    #[test]
    fn continuity_finalization_completed_run_snapshot_uses_completed_totals_and_next_seq() {
        let completed_scan = CompletedScan {
            scan_id: "scan-1".to_string(),
            root_path: "C:\\Users\\xiongxianfei\\Downloads".to_string(),
            started_at: "2026-04-16T10:00:00Z".to_string(),
            completed_at: "2026-04-16T10:01:00Z".to_string(),
            total_bytes: 8192,
            total_files: 12,
            total_directories: 4,
            largest_files: Vec::new(),
            largest_directories: Vec::new(),
            skipped_paths: Vec::new(),
            entries: Vec::new(),
        };

        let snapshot = build_completed_run_snapshot(
            &completed_scan,
            &ScanRunPersistenceCursor {
                next_seq: 4,
                last_persisted_snapshot_at: "2026-04-16T10:00:05Z".to_string(),
                last_persisted_items_scanned: 11,
            },
            Some("C:\\Users\\xiongxianfei\\Downloads\\nested".to_string()),
        );

        assert_eq!(snapshot.run_id, "scan-1");
        assert_eq!(snapshot.seq, 4);
        assert_eq!(snapshot.status, ScanRunStatus::Completed);
        assert_eq!(snapshot.files_discovered, 12);
        assert_eq!(snapshot.directories_discovered, 4);
        assert_eq!(snapshot.items_discovered, 16);
        assert_eq!(snapshot.items_scanned, 16);
        assert_eq!(snapshot.bytes_processed, 8192);
        assert_eq!(snapshot.progress_percent, Some(100.0));
        assert_eq!(snapshot.current_path, Some("C:\\Users\\xiongxianfei\\Downloads\\nested".to_string()));
    }

    #[test]
    fn terminal_snapshot_preserves_latest_progress_context() {
        let previous = make_previous_running_snapshot();

        let snapshot = terminal_snapshot_from_previous(
            "scan-1",
            "C:\\Users\\xiongxianfei\\Downloads",
            ScanLifecycleState::Cancelled,
            Some("Scan cancelled before history save.".to_string()),
            &previous,
            "2026-04-16T10:00:00Z",
        );

        assert_eq!(snapshot.state, ScanLifecycleState::Cancelled);
        assert_eq!(snapshot.files_discovered, 8);
        assert_eq!(snapshot.directories_discovered, 3);
        assert_eq!(snapshot.bytes_processed, 4096);
        assert_eq!(snapshot.started_at, previous.started_at);
        assert_eq!(snapshot.current_path, previous.current_path);
        assert!(snapshot.updated_at.is_some());
        assert_eq!(
            snapshot.message,
            Some("Scan cancelled before history save.".to_string())
        );
    }

    #[test]
    fn completed_snapshot_uses_completed_scan_totals() {
        let previous = make_previous_running_snapshot();
        let completed_scan = CompletedScan {
            scan_id: "scan-1".to_string(),
            root_path: "C:\\Users\\xiongxianfei\\Downloads".to_string(),
            started_at: "2026-04-16T10:00:00Z".to_string(),
            completed_at: "2026-04-16T10:01:00Z".to_string(),
            total_bytes: 8192,
            total_files: 12,
            total_directories: 4,
            largest_files: Vec::new(),
            largest_directories: Vec::new(),
            skipped_paths: Vec::new(),
            entries: Vec::new(),
        };

        let snapshot = completed_snapshot(&completed_scan, &previous);

        assert_eq!(snapshot.state, ScanLifecycleState::Completed);
        assert_eq!(snapshot.files_discovered, 12);
        assert_eq!(snapshot.directories_discovered, 4);
        assert_eq!(snapshot.bytes_processed, 8192);
        assert_eq!(
            snapshot.started_at,
            Some("2026-04-16T10:00:00Z".to_string())
        );
        assert_eq!(snapshot.completed_scan_id, Some("scan-1".to_string()));
        assert_eq!(snapshot.current_path, previous.current_path);
        assert_eq!(snapshot.message, Some("Scan complete.".to_string()));
    }

    #[test]
    fn scan_run_failure_event_payload_is_structured_and_named() {
        let payload = scan_run_failure_event_payload(
            "scan_run_snapshot_write_failed",
            "scan-1",
            CONTINUITY_ERROR_CODE_SNAPSHOT_WRITE_FAILED,
            "disk error",
        );

        assert_eq!(payload["event"], "scan_run_snapshot_write_failed");
        assert_eq!(payload["runId"], "scan-1");
        assert_eq!(payload["errorCode"], CONTINUITY_ERROR_CODE_SNAPSHOT_WRITE_FAILED);
        assert_eq!(payload["error"], "disk error");
        assert!(payload["timestamp"].as_str().is_some());
    }

    #[test]
    fn record_persistence_error_keeps_the_first_failure() {
        let slot = Arc::new(Mutex::new(None));

        assert!(record_persistence_error(&slot, "first".to_string()));
        assert!(!record_persistence_error(&slot, "second".to_string()));
        assert_eq!(
            take_persistence_error(&slot),
            Some("first".to_string())
        );
    }

    #[test]
    fn persist_failed_run_continuity_leaves_error_code_empty_for_scan_execution_failures() {
        let db_path = std::env::temp_dir().join(format!(
            "space-sift-continuity-{}.db",
            scan_core::make_scan_id()
        ));
        let history_store = HistoryStore::with_now(
            db_path.clone(),
            || "2026-04-19T10:00:02Z".to_string(),
        );
        let scan_manager = ScanManager::with_now(|| "2026-04-19T10:00:00Z".to_string());
        let _active = scan_manager
            .start("scan-1".to_string(), "C:\\scan-root".to_string())
            .expect("active scan should start");
        history_store
            .record_scan_run_started(
                "scan-1",
                "C:\\scan-root",
                "2026-04-19T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        persist_failed_run_continuity(
            &history_store,
            &scan_manager,
            "scan-1",
            FailedRunCause::ScanExecution,
            "walk failed",
        )
        .expect("generic failure snapshot should persist");

        let detail = history_store
            .open_scan_run("scan-1")
            .expect("failed run should reopen");
        assert_eq!(detail.header.status, ScanRunStatus::Failed);
        assert_eq!(detail.header.error_code, None);
        assert_eq!(
            detail.header.error_message,
            Some("walk failed".to_string())
        );

        drop(history_store);
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn continuity_heartbeat_iteration_uses_shared_gate_and_repository_path() {
        let db_path = std::env::temp_dir().join(format!(
            "space-sift-heartbeat-{}.db",
            scan_core::make_scan_id()
        ));
        let history_store = HistoryStore::with_now(
            db_path.clone(),
            || "2026-04-19T10:00:01Z".to_string(),
        );
        let times = Arc::new(Mutex::new(vec![
            "2026-04-19T10:00:00Z".to_string(),
            "2026-04-19T10:00:50Z".to_string(),
        ]));
        let clock = Arc::clone(&times);
        let scan_manager = ScanManager::with_now(move || {
            clock.lock().expect("clock lock").remove(0)
        });
        let active = scan_manager
            .start("scan-1".to_string(), "C:\\scan-root".to_string())
            .expect("scan should start");
        history_store
            .record_scan_run_started(
                "scan-1",
                "C:\\scan-root",
                "2026-04-19T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        let persistence_gate = Arc::new(Mutex::new(()));
        let persistence_error = Arc::new(Mutex::new(None));
        let activity_snapshot = ScanStatusSnapshot {
            scan_id: Some("scan-1".to_string()),
            root_path: Some("C:\\scan-root".to_string()),
            state: ScanLifecycleState::Running,
            files_discovered: 3,
            directories_discovered: 1,
            bytes_processed: 512,
            started_at: Some("2026-04-19T10:00:00Z".to_string()),
            updated_at: Some("2026-04-19T10:00:20Z".to_string()),
            current_path: Some("C:\\scan-root\\nested".to_string()),
            message: Some("scanning".to_string()),
            completed_scan_id: None,
        };

        persist_activity_snapshot(
            &scan_manager,
            &history_store,
            "scan-1",
            &activity_snapshot,
            &persistence_gate,
        )
        .expect("activity snapshot should persist");

        let outcome = run_heartbeat_iteration(
            &scan_manager,
            &history_store,
            "scan-1",
            &active.cancel_flag,
            &persistence_error,
            &persistence_gate,
        )
        .expect("heartbeat iteration should succeed")
        .expect("heartbeat should be emitted");

        assert_eq!(
            outcome.snapshot.updated_at,
            Some("2026-04-19T10:00:50Z".to_string())
        );
        let detail = history_store
            .open_scan_run("scan-1")
            .expect("scan run should reopen");
        assert_eq!(detail.header.latest_seq, 3);
        assert_eq!(detail.header.last_snapshot_at, "2026-04-19T10:00:50Z");
        assert_eq!(detail.header.last_progress_at, "2026-04-19T10:00:20Z");
        assert_eq!(detail.latest_snapshot.seq, 3);
        assert_eq!(detail.latest_snapshot.items_scanned, 4);

        drop(history_store);
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn continuity_heartbeat_iteration_stops_after_persistence_error_is_recorded() {
        let db_path = std::env::temp_dir().join(format!(
            "space-sift-heartbeat-stop-{}.db",
            scan_core::make_scan_id()
        ));
        let history_store = HistoryStore::with_now(
            db_path.clone(),
            || "2026-04-19T10:00:01Z".to_string(),
        );
        let times = Arc::new(Mutex::new(vec![
            "2026-04-19T10:00:00Z".to_string(),
            "2026-04-19T10:00:50Z".to_string(),
        ]));
        let clock = Arc::clone(&times);
        let scan_manager = ScanManager::with_now(move || {
            clock.lock().expect("clock lock").remove(0)
        });
        let active = scan_manager
            .start("scan-1".to_string(), "C:\\scan-root".to_string())
            .expect("scan should start");
        history_store
            .record_scan_run_started(
                "scan-1",
                "C:\\scan-root",
                "2026-04-19T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");

        let persistence_gate = Arc::new(Mutex::new(()));
        let persistence_error = Arc::new(Mutex::new(None));
        assert!(record_persistence_error(
            &persistence_error,
            "write failed".to_string()
        ));

        let outcome = run_heartbeat_iteration(
            &scan_manager,
            &history_store,
            "scan-1",
            &active.cancel_flag,
            &persistence_error,
            &persistence_gate,
        )
        .expect("heartbeat iteration should not fail");

        assert!(outcome.is_none());
        let detail = history_store
            .open_scan_run("scan-1")
            .expect("scan run should reopen");
        assert_eq!(detail.header.latest_seq, 1);

        drop(history_store);
        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn continuity_cancel_run_delegates_live_runs_to_active_runtime() {
        let fixture = tempdir().expect("db fixture");
        let history_store = HistoryStore::new(fixture.path().join("history.db"));
        let scan_manager = ScanManager::with_now(|| "2026-04-19T10:00:00Z".to_string());
        let active = scan_manager
            .start("scan-1".to_string(), "C:\\scan-root".to_string())
            .expect("scan should start");

        cancel_scan_run_impl(&history_store, &scan_manager, "scan-1")
            .expect("live run cancel should delegate");

        assert!(active.cancel_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn continuity_cancel_run_returns_not_found_for_unknown_run() {
        let fixture = tempdir().expect("db fixture");
        let history_store = HistoryStore::new(fixture.path().join("history.db"));
        history_store.initialize().expect("schema initialization");
        let scan_manager = ScanManager::new();

        let error = cancel_scan_run_impl(&history_store, &scan_manager, "missing-run")
            .expect_err("missing run should return a not-found error");

        assert_eq!(
            error,
            ScanRunCommandError {
                code: "NOT_FOUND",
                message: "scan run not found: missing-run".to_string(),
                run_id: Some("missing-run".to_string()),
            }
        );
    }

    #[test]
    fn continuity_cancel_run_returns_conflict_for_terminal_run() {
        let fixture = tempdir().expect("db fixture");
        let history_store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-19T10:00:01Z".to_string(),
        );
        let scan_manager = ScanManager::new();
        history_store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-19T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        history_store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-19T10:00:01Z".to_string(),
                created_at: "2026-04-19T10:00:01Z".to_string(),
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

        let error = cancel_scan_run_impl(&history_store, &scan_manager, "run-1")
            .expect_err("terminal run should return a conflict error");

        assert_eq!(error.code, "CONFLICT");
        assert_eq!(error.run_id, Some("run-1".to_string()));
    }

    #[test]
    fn continuity_cancel_run_appends_non_live_cancelled_snapshot() {
        let fixture = tempdir().expect("db fixture");
        let history_store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-19T10:03:00Z".to_string(),
        );
        let scan_manager = ScanManager::new();
        history_store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-19T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        history_store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-19T10:01:00Z".to_string(),
                created_at: "2026-04-19T10:01:00Z".to_string(),
                status: ScanRunStatus::Stale,
                files_discovered: 1,
                directories_discovered: 1,
                items_discovered: 2,
                items_scanned: 2,
                errors_count: 0,
                bytes_processed: 64,
                scan_rate_items_per_sec: 0.0,
                progress_percent: Some(50.0),
                current_path: Some("C:\\scan-root".to_string()),
                message: Some("Run marked stale during startup reconciliation.".to_string()),
            })
            .expect("stale snapshot should append");

        cancel_scan_run_impl(&history_store, &scan_manager, "run-1")
            .expect("stale run should cancel");

        let detail = history_store
            .open_scan_run("run-1")
            .expect("cancelled run should reopen");
        assert_eq!(detail.header.status, ScanRunStatus::Cancelled);
        assert_eq!(detail.latest_snapshot.status, ScanRunStatus::Cancelled);
        assert_eq!(detail.latest_snapshot.seq, 3);
    }

    #[test]
    fn continuity_audit_non_live_cancel_writes_audit_row() {
        let fixture = tempdir().expect("db fixture");
        let history_store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            || "2026-04-19T10:03:00Z".to_string(),
        );
        let scan_manager = ScanManager::new();
        history_store
            .record_scan_run_started(
                "run-1",
                "C:\\scan-root",
                "2026-04-19T10:00:00Z",
                Some("C:\\scan-root"),
            )
            .expect("scan run should persist");
        history_store
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-19T10:01:00Z".to_string(),
                created_at: "2026-04-19T10:01:00Z".to_string(),
                status: ScanRunStatus::Abandoned,
                files_discovered: 1,
                directories_discovered: 1,
                items_discovered: 2,
                items_scanned: 2,
                errors_count: 0,
                bytes_processed: 64,
                scan_rate_items_per_sec: 0.0,
                progress_percent: Some(50.0),
                current_path: Some("C:\\scan-root".to_string()),
                message: Some("Run marked abandoned during startup reconciliation.".to_string()),
            })
            .expect("abandoned snapshot should append");

        cancel_scan_run_impl(&history_store, &scan_manager, "run-1")
            .expect("abandoned run should cancel");

        assert_eq!(
            load_audit_reason_codes(&history_store, "run-1"),
            vec!["USER_CANCELLED".to_string()]
        );
    }

    #[test]
    fn continuity_audit_purge_signal_payload_is_structured_and_named() {
        let payload = scan_run_purged_payload(&PurgedScanRuns {
            purged_count: 2,
            deleted_run_ids: vec!["run-1".to_string(), "run-2".to_string()],
        });

        assert_eq!(payload["event"], "scan_run_purged");
        assert_eq!(payload["purgedCount"], 2);
        assert_eq!(payload["runIds"][0], "run-1");
        assert_eq!(payload["runIds"][1], "run-2");
        assert!(payload["timestamp"].as_str().is_some());
    }
}
