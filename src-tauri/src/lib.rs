mod commands;
mod state;

use crate::state::{history_db_path, AppState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let history_path =
                history_db_path(&app.handle()).map_err(std::io::Error::other)?;
            let history_store = app_db::HistoryStore::new(history_path);
            history_store
                .initialize()
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            app.manage(AppState::new(history_store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::scan::start_scan,
            commands::scan::cancel_active_scan,
            commands::scan::get_scan_status,
            commands::history::list_scan_history,
            commands::history::open_scan_history,
            commands::shell::open_path_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("error while running Space Sift");
}
