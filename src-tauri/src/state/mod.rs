use app_db::HistoryStore;
use scan_core::{ScanLifecycleState, ScanStatusSnapshot};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub struct AppState {
    pub history_store: HistoryStore,
    pub scan_manager: ScanManager,
}

impl AppState {
    pub fn new(history_store: HistoryStore) -> Self {
        Self {
            history_store,
            scan_manager: ScanManager::new(),
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

    pub fn start(&self, scan_id: String, root_path: String) -> Result<ActiveScanHandle, String> {
        let mut runtime = self
            .inner
            .lock()
            .map_err(|_| "The scan state lock is poisoned.".to_string())?;

        if runtime.active.is_some() {
            return Err("One scan at a time. Cancel the current scan before starting another.".to_string());
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        runtime.active = Some(ActiveScanRuntime {
            cancel_flag: Arc::clone(&cancel_flag),
        });
        runtime.status = ScanStatusSnapshot {
            scan_id: Some(scan_id.clone()),
            root_path: Some(root_path),
            state: ScanLifecycleState::Running,
            files_discovered: 0,
            directories_discovered: 0,
            bytes_processed: 0,
            message: None,
            completed_scan_id: None,
        };

        Ok(ActiveScanHandle { scan_id, cancel_flag })
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
pub struct ActiveScanHandle {
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

pub fn history_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let mut path = app
        .path()
        .app_data_dir()
        .map_err(|error: tauri::Error| error.to_string())?;
    std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    path.push("space-sift-history.sqlite3");
    Ok(path)
}
