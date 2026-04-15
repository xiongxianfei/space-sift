use crate::state::AppState;
use cleanup_core::{
    CleanupExecutionMode, CleanupExecutionResult, CleanupFileEntry, CleanupPreview,
    CleanupPreviewRequest, CleanupRuleDefinition, SystemCleanupExecutor,
};
use scan_core::{CompletedScan, ScanEntryKind};
use tauri::State;

#[tauri::command]
pub fn list_cleanup_rules() -> Result<Vec<CleanupRuleDefinition>, String> {
    cleanup_core::list_cleanup_rules().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn preview_cleanup(
    scan_id: String,
    duplicate_delete_paths: Vec<String>,
    enabled_rule_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<CleanupPreview, String> {
    let scan = state
        .history_store
        .open_history_entry(&scan_id)
        .map_err(|error| error.to_string())?;
    let request = cleanup_request_from_scan(scan, duplicate_delete_paths, enabled_rule_ids)?;
    let preview = cleanup_core::build_cleanup_preview(&request).map_err(|error| error.to_string())?;
    state.cleanup_manager.store_preview(preview.clone());
    Ok(preview)
}

#[tauri::command]
pub fn execute_cleanup(
    preview_id: String,
    action_ids: Vec<String>,
    mode: CleanupExecutionMode,
    state: State<'_, AppState>,
) -> Result<CleanupExecutionResult, String> {
    let preview = state.cleanup_manager.open_preview(&preview_id)?;
    let executor = SystemCleanupExecutor;
    let result = cleanup_core::execute_cleanup(&executor, &preview, &action_ids, mode)
        .map_err(|error| error.to_string())?;
    state
        .history_store
        .save_cleanup_execution(&result)
        .map_err(|error| error.to_string())?;
    state.cleanup_manager.store_execution(result.clone());
    Ok(result)
}

fn cleanup_request_from_scan(
    scan: CompletedScan,
    duplicate_delete_paths: Vec<String>,
    enabled_rule_ids: Vec<String>,
) -> Result<CleanupPreviewRequest, String> {
    let file_entries = scan
        .entries
        .into_iter()
        .filter(|entry| matches!(entry.kind, ScanEntryKind::File))
        .map(|entry| CleanupFileEntry {
            path: entry.path,
            size_bytes: entry.size_bytes,
        })
        .collect::<Vec<_>>();

    if file_entries.is_empty() {
        return Err("A fresh scan is required before cleanup preview.".to_string());
    }

    let mut request = CleanupPreviewRequest::new(scan.scan_id, scan.root_path, file_entries);
    request.preview_id = Some(cleanup_core::make_preview_id());
    request.duplicate_delete_paths = duplicate_delete_paths;
    request.enabled_rule_ids = enabled_rule_ids;
    Ok(request)
}
