use app_db::HistoryStore;
use chrono::{DateTime, Utc};
use cleanup_core::{CleanupExecutionResult, CleanupPreview};
use duplicates_core::{
    CompletedDuplicateAnalysis, DuplicateAnalysisStage, DuplicateAnalysisState,
    DuplicateStatusSnapshot,
};
use scan_core::{ScanLifecycleState, ScanRunSnapshot, ScanStatusSnapshot};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct AppState {
    pub history_store: HistoryStore,
    pub scan_manager: ScanManager,
    pub duplicate_manager: DuplicateManager,
    pub cleanup_manager: CleanupManager,
}

impl AppState {
    pub fn new(history_store: HistoryStore) -> Self {
        Self {
            history_store,
            scan_manager: ScanManager::new(),
            duplicate_manager: DuplicateManager::new(),
            cleanup_manager: CleanupManager::new(),
        }
    }
}

#[derive(Clone)]
pub struct ScanManager {
    inner: Arc<Mutex<ScanRuntimeState>>,
    now: Arc<dyn Fn() -> String + Send + Sync>,
}

impl ScanManager {
    pub fn new() -> Self {
        Self::with_now(scan_core::current_timestamp)
    }

    pub fn with_now(now: impl Fn() -> String + Send + Sync + 'static) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ScanRuntimeState {
                status: ScanStatusSnapshot::default(),
                active: None,
            })),
            now: Arc::new(now),
        }
    }

    pub fn start(
        &self,
        scan_id: String,
        root_path: String,
    ) -> Result<ActiveScanHandle, String> {
        let mut runtime = self
            .inner
            .lock()
            .map_err(|_| "The scan state lock is poisoned.".to_string())?;

        if runtime.active.is_some() {
            return Err(
                "One scan at a time. Cancel the current scan before starting another.".to_string(),
            );
        }

        let started_at = self.now_timestamp();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        runtime.active = Some(ActiveScanRuntime {
            cancel_flag: Arc::clone(&cancel_flag),
            next_seq: 2,
            last_activity_at: Some(started_at.clone()),
            last_persisted_snapshot_at: started_at.clone(),
            last_persisted_items_scanned: 0,
            unchanged_heartbeat_count: 0,
            no_progress_warning_emitted: false,
        });
        runtime.status = ScanStatusSnapshot {
            scan_id: Some(scan_id.clone()),
            root_path: Some(root_path.clone()),
            state: ScanLifecycleState::Running,
            files_discovered: 0,
            directories_discovered: 0,
            bytes_processed: 0,
            started_at: Some(started_at.clone()),
            updated_at: Some(started_at.clone()),
            current_path: Some(root_path),
            message: None,
            completed_scan_id: None,
        };

        Ok(ActiveScanHandle {
            scan_id,
            started_at,
            cancel_flag,
        })
    }

    pub fn status(&self) -> Result<ScanStatusSnapshot, String> {
        let runtime = self
            .inner
            .lock()
            .map_err(|_| "The scan state lock is poisoned.".to_string())?;

        Ok(runtime.status.clone())
    }

    pub fn update_running_snapshot(&self, snapshot: ScanStatusSnapshot) {
        if let Ok(mut runtime) = self.inner.lock() {
            if let Some(active) = runtime.active.as_mut() {
                active.last_activity_at = snapshot.updated_at.clone();
                active.unchanged_heartbeat_count = 0;
                active.no_progress_warning_emitted = false;
            }
            runtime.status = snapshot;
        }
    }

    pub fn continuity_cursor(&self) -> Result<ScanRunPersistenceCursor, String> {
        let runtime = self
            .inner
            .lock()
            .map_err(|_| "The scan state lock is poisoned.".to_string())?;
        let active = runtime
            .active
            .as_ref()
            .ok_or_else(|| "No active scan run is available.".to_string())?;

        Ok(ScanRunPersistenceCursor {
            next_seq: active.next_seq,
            last_persisted_snapshot_at: active.last_persisted_snapshot_at.clone(),
            last_persisted_items_scanned: active.last_persisted_items_scanned,
        })
    }

    pub fn due_heartbeat(
        &self,
        heartbeat_interval_seconds: i64,
    ) -> Result<Option<ScanHeartbeatCandidate>, String> {
        let now = self.now_timestamp();
        let runtime = self
            .inner
            .lock()
            .map_err(|_| "The scan state lock is poisoned.".to_string())?;
        let active = match runtime.active.as_ref() {
            Some(active) => active,
            None => return Ok(None),
        };

        if runtime.status.state != ScanLifecycleState::Running {
            return Ok(None);
        }

        let last_activity_at = active
            .last_activity_at
            .as_deref()
            .unwrap_or(active.last_persisted_snapshot_at.as_str());
        if !heartbeat_is_due(last_activity_at, &now, heartbeat_interval_seconds)
            || !heartbeat_is_due(
                &active.last_persisted_snapshot_at,
                &now,
                heartbeat_interval_seconds,
            )
        {
            return Ok(None);
        }

        let mut snapshot = runtime.status.clone();
        snapshot.updated_at = Some(now);

        Ok(Some(ScanHeartbeatCandidate {
            snapshot,
            cursor: ScanRunPersistenceCursor {
                next_seq: active.next_seq,
                last_persisted_snapshot_at: active.last_persisted_snapshot_at.clone(),
                last_persisted_items_scanned: active.last_persisted_items_scanned,
            },
        }))
    }

    pub fn mark_snapshot_persisted(&self, snapshot: &ScanRunSnapshot) -> Result<(), String> {
        let mut runtime = self
            .inner
            .lock()
            .map_err(|_| "The scan state lock is poisoned.".to_string())?;
        let active = runtime
            .active
            .as_mut()
            .ok_or_else(|| "No active scan run is available.".to_string())?;

        active.next_seq = snapshot.seq.saturating_add(1);
        active.last_persisted_snapshot_at = snapshot.snapshot_at.clone();
        active.last_persisted_items_scanned = snapshot.items_scanned;
        Ok(())
    }

    pub fn mark_heartbeat_persisted(
        &self,
        ui_snapshot: &ScanStatusSnapshot,
        snapshot: &ScanRunSnapshot,
        warning_after_intervals: u32,
    ) -> Result<ScanHeartbeatPersistenceResult, String> {
        let mut runtime = self
            .inner
            .lock()
            .map_err(|_| "The scan state lock is poisoned.".to_string())?;
        let (emit_no_progress_warning, unchanged_heartbeat_count, last_progress_at) = {
            let active = runtime
                .active
                .as_mut()
                .ok_or_else(|| "No active scan run is available.".to_string())?;

            if snapshot.items_scanned == active.last_persisted_items_scanned {
                active.unchanged_heartbeat_count =
                    active.unchanged_heartbeat_count.saturating_add(1);
            } else {
                active.unchanged_heartbeat_count = 0;
                active.no_progress_warning_emitted = false;
            }

            let emit_no_progress_warning =
                if active.unchanged_heartbeat_count >= warning_after_intervals
                    && !active.no_progress_warning_emitted
                {
                    active.no_progress_warning_emitted = true;
                    true
                } else {
                    false
                };

            active.next_seq = snapshot.seq.saturating_add(1);
            active.last_persisted_snapshot_at = snapshot.snapshot_at.clone();
            active.last_persisted_items_scanned = snapshot.items_scanned;

            (
                emit_no_progress_warning,
                active.unchanged_heartbeat_count,
                active.last_activity_at.clone(),
            )
        };
        runtime.status = ui_snapshot.clone();

        Ok(ScanHeartbeatPersistenceResult {
            emit_no_progress_warning,
            unchanged_heartbeat_count,
            last_progress_at,
        })
    }

    pub fn finish(&self, snapshot: ScanStatusSnapshot) {
        if let Ok(mut runtime) = self.inner.lock() {
            runtime.status = snapshot;
            runtime.active = None;
        }
    }

    pub fn cancel_active_scan(&self) {
        if let Ok(runtime) = self.inner.lock() {
            if let Some(active) = &runtime.active {
                active.cancel_flag.store(true, Ordering::SeqCst);
            }
        }
    }

    pub fn active_scan_id(&self) -> Result<Option<String>, String> {
        let runtime = self
            .inner
            .lock()
            .map_err(|_| "The scan state lock is poisoned.".to_string())?;

        if runtime.active.is_some() {
            Ok(runtime.status.scan_id.clone())
        } else {
            Ok(None)
        }
    }

    fn now_timestamp(&self) -> String {
        (self.now)()
    }
}

#[derive(Clone)]
pub struct DuplicateManager {
    inner: Arc<Mutex<DuplicateRuntimeState>>,
}

impl DuplicateManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(DuplicateRuntimeState {
                status: DuplicateStatusSnapshot::default(),
                active: None,
                latest_result: None,
            })),
        }
    }

    pub fn start(
        &self,
        analysis_id: String,
        scan_id: String,
    ) -> Result<ActiveDuplicateHandle, String> {
        let mut runtime = self
            .inner
            .lock()
            .map_err(|_| "The duplicate state lock is poisoned.".to_string())?;

        if runtime.active.is_some() {
            return Err(
                "One duplicate analysis at a time. Wait for the current analysis to finish."
                    .to_string(),
            );
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));

        runtime.status = DuplicateStatusSnapshot {
            analysis_id: Some(analysis_id.clone()),
            scan_id: Some(scan_id.clone()),
            state: DuplicateAnalysisState::Running,
            stage: Some(DuplicateAnalysisStage::Grouping),
            items_processed: 0,
            groups_emitted: 0,
            message: None,
            completed_analysis_id: None,
        };
        runtime.active = Some(ActiveDuplicateRuntime {
            cancel_flag: Arc::clone(&cancel_flag),
        });
        runtime.latest_result = None;
        Ok(ActiveDuplicateHandle {
            analysis_id,
            scan_id,
            cancel_flag,
        })
    }

    pub fn status(&self) -> Result<DuplicateStatusSnapshot, String> {
        let runtime = self
            .inner
            .lock()
            .map_err(|_| "The duplicate state lock is poisoned.".to_string())?;

        Ok(runtime.status.clone())
    }

    pub fn update_snapshot(&self, snapshot: DuplicateStatusSnapshot) {
        if let Ok(mut runtime) = self.inner.lock() {
            runtime.status = snapshot;
        }
    }

    pub fn cancel_active_analysis(&self) -> bool {
        if let Ok(runtime) = self.inner.lock() {
            if let Some(active) = &runtime.active {
                active.cancel_flag.store(true, Ordering::SeqCst);
                return true;
            }
        }

        false
    }

    pub fn complete_with_result(
        &self,
        result: CompletedDuplicateAnalysis,
    ) -> DuplicateStatusSnapshot {
        if let Ok(mut runtime) = self.inner.lock() {
            runtime.status = DuplicateStatusSnapshot {
                analysis_id: Some(result.analysis_id.clone()),
                scan_id: Some(result.scan_id.clone()),
                state: DuplicateAnalysisState::Completed,
                stage: Some(DuplicateAnalysisStage::Completed),
                items_processed: runtime.status.items_processed,
                groups_emitted: runtime
                    .status
                    .groups_emitted
                    .max(result.groups.len() as u64),
                message: Some("Duplicate analysis complete.".to_string()),
                completed_analysis_id: Some(result.analysis_id.clone()),
            };
            runtime.active = None;
            runtime.latest_result = Some(result);
            return runtime.status.clone();
        }

        DuplicateStatusSnapshot::default()
    }

    pub fn cancelled(
        &self,
        analysis_id: String,
        scan_id: String,
        message: String,
    ) -> DuplicateStatusSnapshot {
        if let Ok(mut runtime) = self.inner.lock() {
            runtime.status = DuplicateStatusSnapshot {
                analysis_id: Some(analysis_id),
                scan_id: Some(scan_id),
                state: DuplicateAnalysisState::Cancelled,
                stage: None,
                items_processed: runtime.status.items_processed,
                groups_emitted: runtime.status.groups_emitted,
                message: Some(message),
                completed_analysis_id: None,
            };
            runtime.active = None;
            runtime.latest_result = None;
            return runtime.status.clone();
        }

        DuplicateStatusSnapshot::default()
    }

    pub fn fail(
        &self,
        analysis_id: String,
        scan_id: String,
        message: String,
    ) -> DuplicateStatusSnapshot {
        if let Ok(mut runtime) = self.inner.lock() {
            runtime.status = DuplicateStatusSnapshot {
                analysis_id: Some(analysis_id),
                scan_id: Some(scan_id),
                state: DuplicateAnalysisState::Failed,
                stage: None,
                items_processed: runtime.status.items_processed,
                groups_emitted: runtime.status.groups_emitted,
                message: Some(message),
                completed_analysis_id: None,
            };
            runtime.active = None;
            runtime.latest_result = None;
            return runtime.status.clone();
        }

        DuplicateStatusSnapshot::default()
    }

    pub fn open_completed_analysis(
        &self,
        analysis_id: &str,
    ) -> Result<CompletedDuplicateAnalysis, String> {
        let runtime = self
            .inner
            .lock()
            .map_err(|_| "The duplicate state lock is poisoned.".to_string())?;

        match &runtime.latest_result {
            Some(result) if result.analysis_id == analysis_id => Ok(result.clone()),
            _ => Err(format!(
                "duplicate analysis result not found: {analysis_id}"
            )),
        }
    }
}

#[derive(Clone)]
pub struct CleanupManager {
    inner: Arc<Mutex<CleanupRuntimeState>>,
}

impl CleanupManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CleanupRuntimeState {
                latest_preview: None,
                latest_execution: None,
            })),
        }
    }

    pub fn store_preview(&self, preview: CleanupPreview) {
        if let Ok(mut runtime) = self.inner.lock() {
            runtime.latest_preview = Some(preview);
            runtime.latest_execution = None;
        }
    }

    pub fn open_preview(&self, preview_id: &str) -> Result<CleanupPreview, String> {
        let runtime = self
            .inner
            .lock()
            .map_err(|_| "The cleanup state lock is poisoned.".to_string())?;

        match &runtime.latest_preview {
            Some(preview) if preview.preview_id == preview_id => Ok(preview.clone()),
            _ => Err(format!("cleanup preview not found: {preview_id}")),
        }
    }

    pub fn store_execution(&self, result: CleanupExecutionResult) {
        if let Ok(mut runtime) = self.inner.lock() {
            runtime.latest_execution = Some(result);
        }
    }
}

#[derive(Clone)]
pub struct ActiveScanHandle {
    pub scan_id: String,
    pub started_at: String,
    pub cancel_flag: Arc<AtomicBool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScanRunPersistenceCursor {
    pub next_seq: u64,
    pub last_persisted_snapshot_at: String,
    pub last_persisted_items_scanned: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScanHeartbeatCandidate {
    pub snapshot: ScanStatusSnapshot,
    pub cursor: ScanRunPersistenceCursor,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanHeartbeatPersistenceResult {
    pub emit_no_progress_warning: bool,
    pub unchanged_heartbeat_count: u32,
    pub last_progress_at: Option<String>,
}

#[derive(Clone)]
pub struct ActiveDuplicateHandle {
    pub analysis_id: String,
    pub scan_id: String,
    pub cancel_flag: Arc<AtomicBool>,
}

struct ScanRuntimeState {
    status: ScanStatusSnapshot,
    active: Option<ActiveScanRuntime>,
}

struct ActiveScanRuntime {
    cancel_flag: Arc<AtomicBool>,
    #[allow(dead_code)]
    next_seq: u64,
    last_activity_at: Option<String>,
    last_persisted_snapshot_at: String,
    last_persisted_items_scanned: u64,
    unchanged_heartbeat_count: u32,
    no_progress_warning_emitted: bool,
}

struct DuplicateRuntimeState {
    status: DuplicateStatusSnapshot,
    active: Option<ActiveDuplicateRuntime>,
    latest_result: Option<CompletedDuplicateAnalysis>,
}

struct ActiveDuplicateRuntime {
    cancel_flag: Arc<AtomicBool>,
}

struct CleanupRuntimeState {
    latest_preview: Option<CleanupPreview>,
    latest_execution: Option<CleanupExecutionResult>,
}

pub fn history_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|error: tauri::Error| error.to_string())?;
    std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    path.push("space-sift-history.sqlite3");
    Ok(path)
}

fn heartbeat_is_due(previous: &str, now: &str, heartbeat_interval_seconds: i64) -> bool {
    elapsed_seconds(previous, now)
        .map(|elapsed| elapsed >= heartbeat_interval_seconds)
        .unwrap_or(false)
}

fn elapsed_seconds(previous: &str, now: &str) -> Option<i64> {
    let previous = DateTime::parse_from_rfc3339(previous).ok()?;
    let now = DateTime::parse_from_rfc3339(now).ok()?;

    Some(
        now.with_timezone(&Utc)
            .signed_duration_since(previous.with_timezone(&Utc))
            .num_seconds(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuity_time_scan_manager_start_uses_injected_clock() {
        let manager = ScanManager::with_now(|| "2026-04-18T10:00:00Z".to_string());

        let active = manager
            .start("scan-1".to_string(), "C:\\scan-root".to_string())
            .expect("scan should start");
        let status = manager.status().expect("scan status");

        assert_eq!(active.started_at, "2026-04-18T10:00:00Z");
        assert_eq!(status.started_at, Some("2026-04-18T10:00:00Z".to_string()));
        assert_eq!(status.updated_at, Some("2026-04-18T10:00:00Z".to_string()));
    }

    #[test]
    fn continuity_time_scan_manager_seeds_next_seq_and_last_activity() {
        let manager = ScanManager::with_now(|| "2026-04-18T10:00:00Z".to_string());

        manager
            .start("scan-1".to_string(), "C:\\scan-root".to_string())
            .expect("scan should start");

        let runtime = manager.inner.lock().expect("runtime lock");
        let active = runtime.active.as_ref().expect("active runtime");

        assert_eq!(active.next_seq, 2);
        assert_eq!(
            active.last_activity_at.as_deref(),
            Some("2026-04-18T10:00:00Z")
        );
    }

    #[test]
    fn continuity_heartbeat_scan_manager_reports_due_heartbeat_on_cadence() {
        let times = Arc::new(Mutex::new(vec![
            "2026-04-18T10:00:00Z".to_string(),
            "2026-04-18T10:00:29Z".to_string(),
            "2026-04-18T10:00:30Z".to_string(),
            "2026-04-18T10:00:59Z".to_string(),
            "2026-04-18T10:01:00Z".to_string(),
        ]));
        let clock = Arc::clone(&times);
        let manager = ScanManager::with_now(move || {
            clock
                .lock()
                .expect("clock lock")
                .remove(0)
        });

        manager
            .start("scan-1".to_string(), "C:\\scan-root".to_string())
            .expect("scan should start");

        assert!(
            manager
                .due_heartbeat(30)
                .expect("heartbeat check")
                .is_none()
        );

        let first = manager
            .due_heartbeat(30)
            .expect("heartbeat check")
            .expect("heartbeat should be due");
        assert_eq!(first.cursor.next_seq, 2);
        assert_eq!(
            first.snapshot.updated_at,
            Some("2026-04-18T10:00:30Z".to_string())
        );
        assert_eq!(first.snapshot.state, ScanLifecycleState::Running);

        let first_persisted = ScanRunSnapshot {
            run_id: "scan-1".to_string(),
            seq: 2,
            snapshot_at: "2026-04-18T10:00:30Z".to_string(),
            created_at: "2026-04-18T10:00:30Z".to_string(),
            status: scan_core::ScanRunStatus::Running,
            files_discovered: 0,
            directories_discovered: 0,
            items_discovered: 0,
            items_scanned: 0,
            errors_count: 0,
            bytes_processed: 0,
            scan_rate_items_per_sec: 0.0,
            progress_percent: None,
            current_path: Some("C:\\scan-root".to_string()),
            message: None,
        };
        manager
            .mark_heartbeat_persisted(&first.snapshot, &first_persisted, 4)
            .expect("heartbeat persistence should update runtime");

        assert!(
            manager
                .due_heartbeat(30)
                .expect("heartbeat check")
                .is_none()
        );

        let second = manager
            .due_heartbeat(30)
            .expect("heartbeat check")
            .expect("second heartbeat should be due");
        assert_eq!(second.cursor.next_seq, 3);
        assert_eq!(
            second.snapshot.updated_at,
            Some("2026-04-18T10:01:00Z".to_string())
        );
    }

    #[test]
    fn continuity_no_progress_scan_manager_emits_warning_after_four_unchanged_heartbeats() {
        let manager = ScanManager::with_now(|| "2026-04-18T10:00:00Z".to_string());

        manager
            .start("scan-1".to_string(), "C:\\scan-root".to_string())
            .expect("scan should start");

        let base_snapshot = manager.status().expect("scan status");
        let heartbeat_times = [
            "2026-04-18T10:00:30Z",
            "2026-04-18T10:01:00Z",
            "2026-04-18T10:01:30Z",
            "2026-04-18T10:02:00Z",
            "2026-04-18T10:02:30Z",
        ];

        let mut warnings = Vec::new();
        for (index, snapshot_at) in heartbeat_times.iter().enumerate() {
            let ui_snapshot = ScanStatusSnapshot {
                updated_at: Some((*snapshot_at).to_string()),
                ..base_snapshot.clone()
            };
            let persisted = ScanRunSnapshot {
                run_id: "scan-1".to_string(),
                seq: (index as u64) + 2,
                snapshot_at: (*snapshot_at).to_string(),
                created_at: (*snapshot_at).to_string(),
                status: scan_core::ScanRunStatus::Running,
                files_discovered: 0,
                directories_discovered: 0,
                items_discovered: 0,
                items_scanned: 0,
                errors_count: 0,
                bytes_processed: 0,
                scan_rate_items_per_sec: 0.0,
                progress_percent: None,
                current_path: Some("C:\\scan-root".to_string()),
                message: Some("still scanning".to_string()),
            };

            let result = manager
                .mark_heartbeat_persisted(&ui_snapshot, &persisted, 4)
                .expect("heartbeat persistence should update runtime");
            warnings.push(result.emit_no_progress_warning);
            assert_eq!(
                result.last_progress_at.as_deref(),
                Some("2026-04-18T10:00:00Z")
            );
        }

        assert_eq!(warnings, vec![false, false, false, true, false]);
        assert_eq!(
            manager
                .status()
                .expect("scan status")
                .updated_at
                .as_deref(),
            Some("2026-04-18T10:02:30Z")
        );
    }
}
