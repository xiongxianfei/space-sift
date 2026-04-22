use crate::state::AppState;
use serde::Serialize;
use scan_core::{CompletedScan, ScanHistoryEntry, ScanRunDetail, ScanRunSummary};
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCommandError {
    code: &'static str,
    message: String,
    run_id: Option<String>,
}

fn run_history_error(error: app_db::HistoryStoreError, run_id: Option<&str>) -> HistoryCommandError {
    match error {
        app_db::HistoryStoreError::NotFound { .. } => HistoryCommandError {
            code: "NOT_FOUND",
            message: match run_id {
                Some(run_id) => format!("scan run not found: {run_id}"),
                None => "scan run not found".to_string(),
            },
            run_id: run_id.map(str::to_string),
        },
        other => HistoryCommandError {
            code: "PERSISTENCE_ERROR",
            message: other.to_string(),
            run_id: run_id.map(str::to_string),
        },
    }
}

fn list_scan_runs_impl(
    history_store: &app_db::HistoryStore,
) -> Result<Vec<ScanRunSummary>, HistoryCommandError> {
    history_store
        .list_scan_runs()
        .map_err(|error| run_history_error(error, None))
}

fn open_scan_run_impl(
    history_store: &app_db::HistoryStore,
    run_id: &str,
    page: Option<u32>,
    page_size: Option<u32>,
) -> Result<ScanRunDetail, HistoryCommandError> {
    history_store
        .open_scan_run_paged(run_id, page.unwrap_or(1), page_size.unwrap_or(20))
        .map_err(|error| run_history_error(error, Some(run_id)))
}

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

#[tauri::command]
pub fn list_scan_runs(
    state: State<'_, AppState>,
) -> Result<Vec<ScanRunSummary>, HistoryCommandError> {
    list_scan_runs_impl(&state.history_store)
}

#[tauri::command]
pub fn open_scan_run(
    run_id: String,
    page: Option<u32>,
    page_size: Option<u32>,
    state: State<'_, AppState>,
) -> Result<ScanRunDetail, HistoryCommandError> {
    open_scan_run_impl(&state.history_store, &run_id, page, page_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scan_core::{
        CompletedScan, ScanEntry, ScanEntryKind, ScanRunSnapshot, ScanRunStatus, SizedPath,
        SkipReasonCode, SkippedPath,
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

    #[test]
    fn continuity_open_run_surfaces_reconciled_run_detail_and_last_progress_at() {
        let fixture = tempdir().expect("db fixture");
        let store = app_db::HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&[
                "2026-04-19T10:00:01Z",
                "2026-04-19T10:00:31Z",
                "2026-04-19T10:03:00Z",
                "2026-04-19T10:03:00Z",
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
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-19T10:00:30Z".to_string(),
                created_at: "2026-04-19T10:00:31Z".to_string(),
                status: ScanRunStatus::Running,
                files_discovered: 9,
                directories_discovered: 3,
                items_discovered: 12,
                items_scanned: 9,
                errors_count: 0,
                bytes_processed: 512,
                scan_rate_items_per_sec: 128.0,
                progress_percent: Some(75.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                message: None,
            })
            .expect("progress snapshot should append");
        store
            .reconcile_scan_runs()
            .expect("reconciliation should succeed");

        let detail =
            open_scan_run_impl(&store, "run-1", None, None).expect("run detail should open");

        assert_eq!(detail.header.status, ScanRunStatus::Stale);
        assert_eq!(detail.latest_snapshot.status, ScanRunStatus::Stale);
        assert_eq!(detail.header.last_progress_at, "2026-04-19T10:00:30Z");
        assert_eq!(detail.header.last_snapshot_at, "2026-04-19T10:03:00Z");
        assert_eq!(detail.snapshot_preview_page, 1);
        assert_eq!(detail.snapshot_preview_page_size, 20);
        assert_eq!(detail.snapshot_preview_total, 3);
        assert_eq!(detail.snapshot_preview.len(), 3);
        assert_eq!(detail.snapshot_preview[0].status, ScanRunStatus::Stale);
        assert!(!detail.has_resume);
        assert!(!detail.can_resume);
    }

    #[test]
    fn continuity_open_run_legacy_history_remains_available_without_continuity_rows() {
        let fixture = tempdir().expect("db fixture");
        let store = app_db::HistoryStore::new(fixture.path().join("history.db"));
        let completed = sample_completed_scan();

        store.initialize().expect("schema initialization");
        store
            .save_completed_scan(&completed)
            .expect("completed history should persist");

        let runs = list_scan_runs_impl(&store).expect("run list should load");
        assert!(runs.is_empty());

        let reopened = store
            .open_history_entry(&completed.scan_id)
            .expect("legacy history should still reopen");
        assert_eq!(reopened, completed);
    }

    #[test]
    fn continuity_open_run_returns_machine_readable_not_found_error() {
        let fixture = tempdir().expect("db fixture");
        let store = app_db::HistoryStore::new(fixture.path().join("history.db"));
        store.initialize().expect("schema initialization");

        let error = open_scan_run_impl(&store, "missing-run", None, None)
            .expect_err("missing run should return a machine-readable error");

        assert_eq!(
            error,
            HistoryCommandError {
                code: "NOT_FOUND",
                message: "scan run not found: missing-run".to_string(),
                run_id: Some("missing-run".to_string()),
            }
        );
    }

    #[test]
    fn continuity_list_run_summaries_include_snapshot_preview() {
        let fixture = tempdir().expect("db fixture");
        let store = app_db::HistoryStore::with_now(
            fixture.path().join("history.db"),
            scripted_clock(&["2026-04-19T10:00:01Z", "2026-04-19T10:00:31Z"]),
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
            .append_scan_run_snapshot(&ScanRunSnapshot {
                run_id: "run-1".to_string(),
                seq: 2,
                snapshot_at: "2026-04-19T10:00:30Z".to_string(),
                created_at: "2026-04-19T10:00:31Z".to_string(),
                status: ScanRunStatus::Running,
                files_discovered: 9,
                directories_discovered: 3,
                items_discovered: 12,
                items_scanned: 9,
                errors_count: 0,
                bytes_processed: 512,
                scan_rate_items_per_sec: 128.0,
                progress_percent: Some(75.0),
                current_path: Some("C:\\scan-root\\nested".to_string()),
                message: None,
            })
            .expect("progress snapshot should append");

        let summaries = list_scan_runs_impl(&store).expect("run summaries should load");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].header.run_id, "run-1");
        assert_eq!(summaries[0].latest_snapshot.seq, 2);
        assert_eq!(summaries[0].snapshot_preview.len(), 2);
        assert_eq!(summaries[0].snapshot_preview[0].seq, 2);
        assert_eq!(summaries[0].snapshot_preview[1].seq, 1);
        assert!(!summaries[0].has_resume);
        assert!(!summaries[0].can_resume);
    }
}
