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
    let started_at = scan_core::current_timestamp();
    let active_scan = state
        .scan_manager
        .start(scan_id.clone(), normalized_root.clone(), started_at)?;
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
    request.started_at = Some(active_scan.started_at.clone());

    let result = scan_core::scan_path(
        &request,
        || active_scan.cancel_flag.load(Ordering::SeqCst),
        |snapshot| {
            scan_manager.update_running_snapshot(snapshot.clone());
            let _ = emit_snapshot(&app, &snapshot);
        },
    );

    match result {
        Ok(completed_scan) => {
            finish_completed_scan(&app, &scan_manager, &history_store, completed_scan)
        }
        Err(ScanFailure::Cancelled) => {
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
            let latest_snapshot = scan_manager.status().unwrap_or_default();
            let snapshot = terminal_snapshot_from_previous(
                &active_scan.scan_id,
                &root_path,
                ScanLifecycleState::Failed,
                Some(error.to_string()),
                &latest_snapshot,
                &active_scan.started_at,
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
    let latest_snapshot = scan_manager.status().unwrap_or_default();

    if let Err(error) = history_store.save_completed_scan(&completed_scan) {
        let snapshot = terminal_snapshot_from_previous(
            &completed_scan.scan_id,
            &completed_scan.root_path,
            ScanLifecycleState::Failed,
            Some(error.to_string()),
            &latest_snapshot,
            &completed_scan.started_at,
        );
        scan_manager.finish(snapshot.clone());
        let _ = emit_snapshot(app, &snapshot);
        return;
    }

    let snapshot = completed_snapshot(&completed_scan, &latest_snapshot);

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

fn emit_snapshot(app: &AppHandle, snapshot: &ScanStatusSnapshot) -> Result<(), String> {
    app.emit("scan-progress", snapshot)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
