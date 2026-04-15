use crate::state::AppState;
use scan_core::{CompletedScan, ScanHistoryEntry};
use tauri::State;

#[tauri::command]
pub fn list_scan_history(state: State<'_, AppState>) -> Result<Vec<ScanHistoryEntry>, String> {
    state
        .history_store
        .list_history()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn open_scan_history(
    scan_id: String,
    state: State<'_, AppState>,
) -> Result<CompletedScan, String> {
    state
        .history_store
        .open_history_entry(&scan_id)
        .map_err(|error| error.to_string())
}
