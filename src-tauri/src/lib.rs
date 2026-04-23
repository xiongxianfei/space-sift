mod commands;
mod state;

use crate::state::{history_db_path, AppState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let history_path = history_db_path(&app.handle()).map_err(std::io::Error::other)?;
            let history_store = app_db::HistoryStore::new(history_path);
            history_store
                .initialize()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            history_store
                .reconcile_scan_runs()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            let purged = history_store
                .purge_expired_scan_runs()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            commands::scan::log_scan_run_purged(&purged);
            app.manage(AppState::new(history_store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan::start_scan,
            commands::scan::resume_scan_run,
            commands::scan::cancel_active_scan,
            commands::scan::cancel_scan_run,
            commands::scan::get_scan_status,
            commands::duplicates::start_duplicate_analysis,
            commands::duplicates::cancel_duplicate_analysis,
            commands::duplicates::get_duplicate_analysis_status,
            commands::duplicates::open_duplicate_analysis,
            commands::cleanup::list_cleanup_rules,
            commands::cleanup::preview_cleanup,
            commands::cleanup::execute_cleanup,
            commands::history::list_scan_history,
            commands::history::open_scan_history,
            commands::history::list_scan_runs,
            commands::history::open_scan_run,
            commands::privileged::get_privileged_cleanup_capability,
            commands::shell::get_workspace_restore_context,
            commands::shell::open_path_in_explorer,
            commands::shell::save_workspace_restore_context
        ])
        .run(tauri::generate_context!())
        .expect("error while running Space Sift");
}
