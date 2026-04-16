use app_db::HistoryStore;
use cleanup_core::{CleanupExecutionResult, CleanupPreview};
use duplicates_core::{
    CompletedDuplicateAnalysis, DuplicateAnalysisStage, DuplicateAnalysisState,
    DuplicateStatusSnapshot,
};
use scan_core::{ScanLifecycleState, ScanStatusSnapshot};
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
}

impl ScanManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ScanRuntimeState {
                status: ScanStatusSnapshot::default(),
                active: None,
            })),
        }
    }

    pub fn start(
        &self,
        scan_id: String,
        root_path: String,
        started_at: String,
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

        let cancel_flag = Arc::new(AtomicBool::new(false));
        runtime.active = Some(ActiveScanRuntime {
            cancel_flag: Arc::clone(&cancel_flag),
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
            runtime.status = snapshot;
        }
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
