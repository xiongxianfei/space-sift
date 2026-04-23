use crate::state::AppState;
use app_db::{HistoryStore, WorkspaceRestoreContext, WorkspaceRestoreContextInput};
use std::path::PathBuf;
use std::process::Command;
use tauri::State;

fn get_workspace_restore_context_impl(
    history_store: &HistoryStore,
) -> Result<Option<WorkspaceRestoreContext>, String> {
    history_store
        .load_workspace_restore_context()
        .map_err(|error| error.to_string())
}

fn save_workspace_restore_context_impl(
    history_store: &HistoryStore,
    input: WorkspaceRestoreContextInput,
) -> Result<WorkspaceRestoreContext, String> {
    history_store
        .save_workspace_restore_context(&input)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_workspace_restore_context(
    state: State<'_, AppState>,
) -> Result<Option<WorkspaceRestoreContext>, String> {
    get_workspace_restore_context_impl(&state.history_store)
}

#[tauri::command]
pub fn save_workspace_restore_context(
    input: WorkspaceRestoreContextInput,
    state: State<'_, AppState>,
) -> Result<WorkspaceRestoreContext, String> {
    save_workspace_restore_context_impl(&state.history_store, input)
}

#[tauri::command]
pub fn open_path_in_explorer(path: String) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Choose a path before opening Explorer.".to_string());
    }

    let candidate = PathBuf::from(trimmed);
    if !candidate.exists() {
        return Err("Path no longer exists.".to_string());
    }

    #[cfg(windows)]
    {
        let mut command = Command::new("explorer.exe");
        if candidate.is_file() {
            command.arg("/select,").arg(&candidate);
        } else {
            command.arg(&candidate);
        }

        command.spawn().map_err(|error| error.to_string())?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let _ = candidate;
        Err("Windows Explorer handoff is only available on Windows.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_db::{HistoryStore, WorkspaceRestoreContext, WorkspaceRestoreContextInput};
    use tempfile::tempdir;

    fn scripted_clock(values: &[&str]) -> impl Fn() -> String + Send + Sync + 'static {
        let values = std::sync::Arc::new(std::sync::Mutex::new(
            values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
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

    #[test]
    fn workspace_restore_context_command_boundary_round_trip() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&["2026-04-22T10:00:00Z"]),
        );

        let saved = save_workspace_restore_context_impl(
            &store,
            WorkspaceRestoreContextInput {
                last_workspace: "explorer".to_string(),
                last_opened_scan_id: Some("scan-1".to_string()),
            },
        )
        .expect("restore context should save");

        assert_eq!(
            saved,
            WorkspaceRestoreContext {
                schema_version: 1,
                last_workspace: "explorer".to_string(),
                last_opened_scan_id: Some("scan-1".to_string()),
                updated_at: "2026-04-22T10:00:00Z".to_string(),
            }
        );
        assert_eq!(
            get_workspace_restore_context_impl(&store).expect("restore context should load"),
            Some(saved)
        );
    }

    #[test]
    fn workspace_restore_context_command_boundary_returns_none_for_unsupported_schema_version() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&["2026-04-22T10:00:00Z"]),
        );

        save_workspace_restore_context_impl(
            &store,
            WorkspaceRestoreContextInput {
                last_workspace: "explorer".to_string(),
                last_opened_scan_id: Some("scan-1".to_string()),
            },
        )
        .expect("restore context should save");

        let connection = rusqlite::Connection::open(store.db_path()).expect("db connection");
        connection
            .execute(
                "UPDATE workspace_restore_context SET schema_version = ?1 WHERE singleton_key = 1;",
                rusqlite::params![2_i64],
            )
            .expect("schema version update");

        assert_eq!(
            get_workspace_restore_context_impl(&store).expect("restore context should load"),
            None
        );
    }
}
