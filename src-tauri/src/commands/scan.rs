use crate::state::{ActiveScanHandle, AppState, ScanManager};
use app_db::HistoryStore;
use scan_core::{
    CompletedScan, ScanFailure, ScanLifecycleState, ScanRequest, ScanStatusSnapshot,
    DEFAULT_TOP_ITEMS_LIMIT,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartScanResponse {
    pub scan_id: String,
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
    let active_scan = state
        .scan_manager
        .start(scan_id.clone(), normalized_root.clone())?;
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
    let mut request = ScanRequest::new(PathBuf::from(&root_path));
    request.top_items_limit = DEFAULT_TOP_ITEMS_LIMIT;
    request.scan_id = Some(active_scan.scan_id.clone());

    let result = scan_core::scan_path(
        &request,
        || active_scan.cancel_flag.load(Ordering::SeqCst),
        |snapshot| {
            scan_manager.update_running_snapshot(snapshot.clone());
            let _ = emit_snapshot(&app, &snapshot);
        },
    );

    match result {
        Ok(completed_scan) => finish_completed_scan(&app, &scan_manager, &history_store, completed_scan),
        Err(ScanFailure::Cancelled) => {
            let snapshot = terminal_snapshot(
                &active_scan.scan_id,
                &root_path,
                ScanLifecycleState::Cancelled,
                Some("Scan cancelled before history save.".to_string()),
                None,
                None,
            );
            scan_manager.finish(snapshot.clone());
            let _ = emit_snapshot(&app, &snapshot);
        }
        Err(error) => {
            let snapshot = terminal_snapshot(
                &active_scan.scan_id,
                &root_path,
                ScanLifecycleState::Failed,
                Some(error.to_string()),
                None,
                None,
            );
            scan_manager.finish(snapshot.clone());
            let _ = emit_snapshot(&app, &snapshot);
        }
    }
}

fn finish_completed_scan(
    app: &AppHandle,
    scan_manager: &ScanManager,
    history_store: &HistoryStore,
    completed_scan: CompletedScan,
) {
    if let Err(error) = history_store.save_completed_scan(&completed_scan) {
        let snapshot = terminal_snapshot(
            &completed_scan.scan_id,
            &completed_scan.root_path,
            ScanLifecycleState::Failed,
            Some(error.to_string()),
            Some(completed_scan.total_files),
            Some(completed_scan.total_directories),
        );
        scan_manager.finish(snapshot.clone());
        let _ = emit_snapshot(app, &snapshot);
        return;
    }

    let snapshot = terminal_snapshot(
        &completed_scan.scan_id,
        &completed_scan.root_path,
        ScanLifecycleState::Completed,
        Some("Scan complete.".to_string()),
        Some(completed_scan.total_files),
        Some(completed_scan.total_directories),
    )
    .with_bytes(completed_scan.total_bytes)
    .with_completed_scan_id(completed_scan.scan_id.clone());

    scan_manager.finish(snapshot.clone());
    let _ = emit_snapshot(app, &snapshot);
}

fn terminal_snapshot(
    scan_id: &str,
    root_path: &str,
    state: ScanLifecycleState,
    message: Option<String>,
    total_files: Option<u64>,
    total_directories: Option<u64>,
) -> ScanStatusSnapshot {
    ScanStatusSnapshot {
        scan_id: Some(scan_id.to_string()),
        root_path: Some(root_path.to_string()),
        state,
        files_discovered: total_files.unwrap_or_default(),
        directories_discovered: total_directories.unwrap_or_default(),
        bytes_processed: 0,
        message,
        completed_scan_id: None,
    }
}

trait SnapshotExt {
    fn with_bytes(self, bytes_processed: u64) -> Self;
    fn with_completed_scan_id(self, scan_id: String) -> Self;
}

impl SnapshotExt for ScanStatusSnapshot {
    fn with_bytes(mut self, bytes_processed: u64) -> Self {
        self.bytes_processed = bytes_processed;
        self
    }

    fn with_completed_scan_id(mut self, scan_id: String) -> Self {
        self.completed_scan_id = Some(scan_id);
        self
    }
}

fn emit_snapshot(app: &AppHandle, snapshot: &ScanStatusSnapshot) -> Result<(), String> {
    app.emit("scan-progress", snapshot)
        .map_err(|error| error.to_string())
}
