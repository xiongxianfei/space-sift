use crate::state::{AppState, DuplicateManager};
use duplicates_core::{
    DuplicateAnalysisFailure, DuplicateAnalysisRequest, DuplicateAnalysisState,
    DuplicateCandidate, DuplicateStatusSnapshot,
};
use scan_core::{CompletedScan, ScanEntryKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDuplicateAnalysisResponse {
    pub analysis_id: String,
}

#[tauri::command]
pub fn start_duplicate_analysis(
    scan_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<StartDuplicateAnalysisResponse, String> {
    let scan = state
        .history_store
        .open_history_entry(&scan_id)
        .map_err(|error| error.to_string())?;
    let request = duplicate_request_from_scan(scan)?;
    let analysis_id = request
        .analysis_id
        .clone()
        .ok_or_else(|| "duplicate analysis identifier was not created".to_string())?;

    state
        .duplicate_manager
        .start(analysis_id.clone(), request.scan_id.clone())?;
    let initial_snapshot = state.duplicate_manager.status()?;
    let _ = emit_snapshot(&app, &initial_snapshot);

    let app_handle = app.clone();
    let duplicate_manager = state.duplicate_manager.clone();
    let history_store = state.history_store.clone();

    tauri::async_runtime::spawn_blocking(move || {
        run_duplicate_task(app_handle, duplicate_manager, history_store, request);
    });

    Ok(StartDuplicateAnalysisResponse { analysis_id })
}

#[tauri::command]
pub fn get_duplicate_analysis_status(
    state: State<'_, AppState>,
) -> Result<DuplicateStatusSnapshot, String> {
    state.duplicate_manager.status()
}

#[tauri::command]
pub fn open_duplicate_analysis(
    analysis_id: String,
    state: State<'_, AppState>,
) -> Result<duplicates_core::CompletedDuplicateAnalysis, String> {
    state
        .duplicate_manager
        .open_completed_analysis(&analysis_id)
}

fn duplicate_request_from_scan(scan: CompletedScan) -> Result<DuplicateAnalysisRequest, String> {
    let candidates = scan
        .entries
        .into_iter()
        .filter(|entry| matches!(entry.kind, ScanEntryKind::File))
        .map(|entry| DuplicateCandidate {
            path: entry.path,
            size_bytes: entry.size_bytes,
        })
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        return Err("A fresh scan is required before duplicate analysis.".to_string());
    }

    let mut request = DuplicateAnalysisRequest::new(scan.scan_id, scan.root_path, candidates);
    request.analysis_id = Some(duplicates_core::make_analysis_id());
    Ok(request)
}

fn run_duplicate_task(
    app: AppHandle,
    duplicate_manager: DuplicateManager,
    history_store: app_db::HistoryStore,
    request: DuplicateAnalysisRequest,
) {
    let analysis_id = request
        .analysis_id
        .clone()
        .unwrap_or_else(duplicates_core::make_analysis_id);
    let scan_id = request.scan_id.clone();

    let result = duplicates_core::analyze_duplicates(&history_store, &request, |snapshot| {
        duplicate_manager.update_snapshot(snapshot.clone());
        if snapshot.state != DuplicateAnalysisState::Completed {
            let _ = emit_snapshot(&app, &snapshot);
        }
    });

    match result {
        Ok(completed_analysis) => {
            let snapshot = duplicate_manager.complete_with_result(completed_analysis);
            let _ = emit_snapshot(&app, &snapshot);
        }
        Err(error) => {
            let snapshot =
                duplicate_manager.fail(analysis_id, scan_id, duplicate_failure_message(error));
            let _ = emit_snapshot(&app, &snapshot);
        }
    }
}

fn duplicate_failure_message(error: DuplicateAnalysisFailure) -> String {
    match error {
        DuplicateAnalysisFailure::InvalidRequest { message } => message,
        DuplicateAnalysisFailure::Internal { message } => message,
    }
}

fn emit_snapshot(app: &AppHandle, snapshot: &DuplicateStatusSnapshot) -> Result<(), String> {
    app.emit("duplicate-progress", snapshot)
        .map_err(|error| error.to_string())
}
