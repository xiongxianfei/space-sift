use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_TOP_ITEMS_LIMIT: usize = 10;
const PROGRESS_EMIT_ITEM_INTERVAL: u64 = 64;
const PROGRESS_EMIT_BYTE_INTERVAL: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScanLifecycleState {
    Idle,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkipReasonCode {
    Excluded,
    PermissionDenied,
    ReparsePoint,
    MissingPath,
    MetadataError,
    ReadDirError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkippedPath {
    pub path: String,
    pub reason_code: SkipReasonCode,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SizedPath {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScanEntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanEntry {
    pub path: String,
    pub parent_path: Option<String>,
    pub kind: ScanEntryKind,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompletedScan {
    pub scan_id: String,
    pub root_path: String,
    pub started_at: String,
    pub completed_at: String,
    pub total_bytes: u64,
    pub total_files: u64,
    pub total_directories: u64,
    pub largest_files: Vec<SizedPath>,
    pub largest_directories: Vec<SizedPath>,
    pub skipped_paths: Vec<SkippedPath>,
    #[serde(default)]
    pub entries: Vec<ScanEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanHistoryEntry {
    pub scan_id: String,
    pub root_path: String,
    pub completed_at: String,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatusSnapshot {
    pub scan_id: Option<String>,
    pub root_path: Option<String>,
    pub state: ScanLifecycleState,
    pub files_discovered: u64,
    pub directories_discovered: u64,
    pub bytes_processed: u64,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub current_path: Option<String>,
    pub message: Option<String>,
    pub completed_scan_id: Option<String>,
}

impl Default for ScanStatusSnapshot {
    fn default() -> Self {
        Self {
            scan_id: None,
            root_path: None,
            state: ScanLifecycleState::Idle,
            files_discovered: 0,
            directories_discovered: 0,
            bytes_processed: 0,
            started_at: None,
            updated_at: None,
            current_path: None,
            message: None,
            completed_scan_id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanRequest {
    pub root_path: PathBuf,
    pub top_items_limit: usize,
    pub scan_id: Option<String>,
    pub started_at: Option<String>,
}

impl ScanRequest {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            top_items_limit: DEFAULT_TOP_ITEMS_LIMIT,
            scan_id: None,
            started_at: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanMeasurement {
    pub elapsed_millis: u128,
    pub entries_per_second: u64,
    pub describe_path_calls: u64,
    pub read_dir_calls: u64,
    pub progress_event_count: u64,
    pub cancellation_check_count: u64,
    pub cancel_to_stop_millis: Option<u128>,
    pub files_discovered: u64,
    pub directories_discovered: u64,
    pub bytes_processed: u64,
    pub terminal_state: ScanLifecycleState,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MeasuredScan {
    pub result: Result<CompletedScan, ScanFailure>,
    pub measurement: ScanMeasurement,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanNode {
    pub path: PathBuf,
    pub kind: ScanNodeKind,
}

impl ScanNode {
    pub fn directory(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: ScanNodeKind::Directory,
        }
    }

    pub fn file(path: impl Into<PathBuf>, size_bytes: u64) -> Self {
        Self {
            path: path.into(),
            kind: ScanNodeKind::File { size_bytes },
        }
    }

    pub fn reparse_point(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            kind: ScanNodeKind::ReparsePoint,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScanNodeKind {
    File { size_bytes: u64 },
    Directory,
    ReparsePoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanPathError {
    pub path: PathBuf,
    pub reason_code: SkipReasonCode,
    pub summary: String,
}

impl ScanPathError {
    pub fn new(path: impl Into<PathBuf>, reason_code: SkipReasonCode, summary: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason_code,
            summary: summary.into(),
        }
    }

    pub fn to_skipped_path(&self) -> SkippedPath {
        SkippedPath {
            path: self.path.display().to_string(),
            reason_code: self.reason_code.clone(),
            summary: self.summary.clone(),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScanFailure {
    #[error("scan was cancelled")]
    Cancelled,
    #[error("scan root could not be opened: {message}")]
    InvalidRoot { message: String },
    #[error("scan failed: {message}")]
    Internal { message: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RootPathKind {
    LocalFixed,
    Removable,
    Remote,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanBackendPreference {
    OptimizedEmbeddedNodes,
    RecursiveFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectoryTraversalStrategy {
    DepthFirst,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScanSchedulingPolicy {
    max_concurrent_directories: usize,
    directory_traversal: DirectoryTraversalStrategy,
    background_mode_requested: bool,
}

pub trait ScanBackend {
    fn describe_path(&self, path: &Path) -> Result<ScanNode, ScanPathError>;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ScanPathError>;

    fn read_dir_nodes(&self, path: &Path) -> Result<Vec<Result<ScanNode, ScanPathError>>, ScanPathError> {
        let children = self.read_dir(path)?;
        Ok(children.into_iter().map(|child| self.describe_path(&child)).collect())
    }

    fn provides_embedded_directory_nodes(&self) -> bool {
        false
    }
}

#[derive(Default)]
pub struct RecursiveFilesystemBackend;

impl ScanBackend for RecursiveFilesystemBackend {
    fn describe_path(&self, path: &Path) -> Result<ScanNode, ScanPathError> {
        describe_path_with_symlink_metadata(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ScanPathError> {
        let entries = std::fs::read_dir(path).map_err(|error| scan_read_dir_error(path, &error))?;

        Ok(entries.filter_map(|entry| entry.ok().map(|value| value.path())).collect())
    }
}

#[cfg(windows)]
pub fn scan_path<F, C>(
    request: &ScanRequest,
    is_cancelled: C,
    on_progress: F,
) -> Result<CompletedScan, ScanFailure>
where
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    let optimized = WindowsFindFilesystemBackend;
    let fallback = RecursiveFilesystemBackend;
    scan_with_routed_backend(
        classify_root_path(&request.root_path),
        request,
        &optimized,
        &fallback,
        is_cancelled,
        on_progress,
    )
}

#[cfg(not(windows))]
pub fn scan_path<F, C>(
    request: &ScanRequest,
    is_cancelled: C,
    on_progress: F,
) -> Result<CompletedScan, ScanFailure>
where
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    scan_with_backend(&RecursiveFilesystemBackend, request, is_cancelled, on_progress)
}

#[cfg(windows)]
pub fn measure_scan_path<C>(request: &ScanRequest, is_cancelled: C) -> MeasuredScan
where
    C: Fn() -> bool,
{
    let optimized = WindowsFindFilesystemBackend;
    let fallback = RecursiveFilesystemBackend;
    measure_scan_with_routed_backend(
        classify_root_path(&request.root_path),
        request,
        &optimized,
        &fallback,
        is_cancelled,
    )
}

#[cfg(not(windows))]
pub fn measure_scan_path<C>(request: &ScanRequest, is_cancelled: C) -> MeasuredScan
where
    C: Fn() -> bool,
{
    measure_scan_with_backend(&RecursiveFilesystemBackend, request, is_cancelled)
}

fn backend_preference_for_root_kind(root_kind: RootPathKind) -> ScanBackendPreference {
    match root_kind {
        RootPathKind::LocalFixed => ScanBackendPreference::OptimizedEmbeddedNodes,
        RootPathKind::Removable | RootPathKind::Remote | RootPathKind::Other => {
            ScanBackendPreference::RecursiveFallback
        }
    }
}

fn default_scan_scheduling_policy(_root_kind: RootPathKind) -> ScanSchedulingPolicy {
    ScanSchedulingPolicy {
        max_concurrent_directories: 1,
        directory_traversal: DirectoryTraversalStrategy::DepthFirst,
        background_mode_requested: false,
    }
}

fn scan_with_routed_backend<O, R, F, C>(
    root_kind: RootPathKind,
    request: &ScanRequest,
    optimized: &O,
    fallback: &R,
    is_cancelled: C,
    on_progress: F,
) -> Result<CompletedScan, ScanFailure>
where
    O: ScanBackend,
    R: ScanBackend,
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    let scheduling_policy = default_scan_scheduling_policy(root_kind);
    match backend_preference_for_root_kind(root_kind) {
        ScanBackendPreference::OptimizedEmbeddedNodes => {
            scan_with_backend_using_policy(optimized, request, scheduling_policy, is_cancelled, on_progress)
        }
        ScanBackendPreference::RecursiveFallback => {
            scan_with_backend_using_policy(fallback, request, scheduling_policy, is_cancelled, on_progress)
        }
    }
}

fn measure_scan_with_routed_backend<O, R, C>(
    root_kind: RootPathKind,
    request: &ScanRequest,
    optimized: &O,
    fallback: &R,
    is_cancelled: C,
) -> MeasuredScan
where
    O: ScanBackend,
    R: ScanBackend,
    C: Fn() -> bool,
{
    let scheduling_policy = default_scan_scheduling_policy(root_kind);
    match backend_preference_for_root_kind(root_kind) {
        ScanBackendPreference::OptimizedEmbeddedNodes => {
            measure_scan_with_backend_using_policy(optimized, request, scheduling_policy, is_cancelled)
        }
        ScanBackendPreference::RecursiveFallback => {
            measure_scan_with_backend_using_policy(fallback, request, scheduling_policy, is_cancelled)
        }
    }
}

pub fn measure_scan_with_backend<B, C>(backend: &B, request: &ScanRequest, is_cancelled: C) -> MeasuredScan
where
    B: ScanBackend,
    C: Fn() -> bool,
{
    measure_scan_with_backend_using_policy(
        backend,
        request,
        default_scan_scheduling_policy(RootPathKind::Other),
        is_cancelled,
    )
}

fn measure_scan_with_backend_using_policy<B, C>(
    backend: &B,
    request: &ScanRequest,
    scheduling_policy: ScanSchedulingPolicy,
    is_cancelled: C,
) -> MeasuredScan
where
    B: ScanBackend,
    C: Fn() -> bool,
{
    use std::cell::RefCell;

    let describe_path_calls = Cell::new(0_u64);
    let read_dir_calls = Cell::new(0_u64);
    let progress_event_count = Cell::new(0_u64);
    let cancellation_check_count = Cell::new(0_u64);
    let cancel_requested_at = RefCell::new(None::<Instant>);
    let last_snapshot = RefCell::new(ScanStatusSnapshot::default());
    let measuring_backend = MeasuringScanBackend {
        inner: backend,
        describe_path_calls: &describe_path_calls,
        read_dir_calls: &read_dir_calls,
    };

    let started_at = Instant::now();
    let result = scan_with_backend_using_policy(
        &measuring_backend,
        request,
        scheduling_policy,
        || {
            cancellation_check_count.set(cancellation_check_count.get() + 1);
            let cancelled = is_cancelled();
            if cancelled {
                let mut requested_at = cancel_requested_at.borrow_mut();
                if requested_at.is_none() {
                    *requested_at = Some(Instant::now());
                }
            }
            cancelled
        },
        |snapshot| {
            progress_event_count.set(progress_event_count.get() + 1);
            *last_snapshot.borrow_mut() = snapshot;
        },
    );
    let finished_at = Instant::now();
    let last_snapshot = last_snapshot.into_inner();
    let elapsed = finished_at.saturating_duration_since(started_at);
    let terminal_state = terminal_state_from_result(&result);
    let (files_discovered, directories_discovered, bytes_processed) = match &result {
        Ok(completed) => (
            completed.total_files,
            completed.total_directories,
            completed.total_bytes,
        ),
        Err(_) => (
            last_snapshot.files_discovered,
            last_snapshot.directories_discovered,
            last_snapshot.bytes_processed,
        ),
    };

    MeasuredScan {
        result,
        measurement: ScanMeasurement {
            elapsed_millis: elapsed.as_millis(),
            entries_per_second: calculate_entries_per_second(
                files_discovered + directories_discovered,
                elapsed,
            ),
            describe_path_calls: describe_path_calls.get(),
            read_dir_calls: read_dir_calls.get(),
            progress_event_count: progress_event_count.get(),
            cancellation_check_count: cancellation_check_count.get(),
            cancel_to_stop_millis: cancel_requested_at
                .into_inner()
                .map(|cancelled_at| finished_at.saturating_duration_since(cancelled_at).as_millis()),
            files_discovered,
            directories_discovered,
            bytes_processed,
            terminal_state,
        },
    }
}

pub fn scan_with_backend<B, F, C>(
    backend: &B,
    request: &ScanRequest,
    is_cancelled: C,
    on_progress: F,
) -> Result<CompletedScan, ScanFailure>
where
    B: ScanBackend,
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    scan_with_backend_using_policy(
        backend,
        request,
        default_scan_scheduling_policy(RootPathKind::Other),
        is_cancelled,
        on_progress,
    )
}

fn scan_with_backend_using_policy<B, F, C>(
    backend: &B,
    request: &ScanRequest,
    scheduling_policy: ScanSchedulingPolicy,
    is_cancelled: C,
    mut on_progress: F,
) -> Result<CompletedScan, ScanFailure>
where
    B: ScanBackend,
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    let root_node = backend
        .describe_path(&request.root_path)
        .map_err(|error| ScanFailure::InvalidRoot {
            message: error.summary,
        })?;

    if !matches!(root_node.kind, ScanNodeKind::Directory) {
        return Err(ScanFailure::InvalidRoot {
            message: "scan root must be a directory or drive path".to_string(),
        });
    }

    let scan_id = request.scan_id.clone().unwrap_or_else(make_scan_id);
    let root_path = request.root_path.display().to_string();
    let started_at = request.started_at.clone().unwrap_or_else(current_timestamp);
    let top_items_limit = request.top_items_limit.max(1);
    let mut reporter = ProgressReporter::new(
        scan_id.clone(),
        root_path.clone(),
        started_at.clone(),
        &mut on_progress,
    );
    let mut accumulator = ScanAccumulator::new(top_items_limit);

    reporter.emit();

    if is_cancelled() {
        return Err(ScanFailure::Cancelled);
    }

    let total_bytes = scan_directory(
        backend,
        &request.root_path,
        None,
        scheduling_policy,
        &is_cancelled,
        &mut reporter,
        &mut accumulator,
    )?;

    if is_cancelled() {
        return Err(ScanFailure::Cancelled);
    }

    Ok(CompletedScan {
        scan_id,
        root_path,
        started_at,
        completed_at: current_timestamp(),
        total_bytes,
        total_files: accumulator.total_files,
        total_directories: accumulator.total_directories,
        largest_files: accumulator.files,
        largest_directories: accumulator.directories,
        skipped_paths: accumulator.skipped_paths,
        entries: accumulator.entries,
    })
}

struct MeasuringScanBackend<'a, B> {
    inner: &'a B,
    describe_path_calls: &'a Cell<u64>,
    read_dir_calls: &'a Cell<u64>,
}

impl<B> ScanBackend for MeasuringScanBackend<'_, B>
where
    B: ScanBackend,
{
    fn describe_path(&self, path: &Path) -> Result<ScanNode, ScanPathError> {
        self.describe_path_calls
            .set(self.describe_path_calls.get().saturating_add(1));
        self.inner.describe_path(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ScanPathError> {
        self.read_dir_calls
            .set(self.read_dir_calls.get().saturating_add(1));
        self.inner.read_dir(path)
    }

    fn read_dir_nodes(&self, path: &Path) -> Result<Vec<Result<ScanNode, ScanPathError>>, ScanPathError> {
        if self.inner.provides_embedded_directory_nodes() {
            self.read_dir_calls
                .set(self.read_dir_calls.get().saturating_add(1));
            return self.inner.read_dir_nodes(path);
        }

        self.read_dir_calls
            .set(self.read_dir_calls.get().saturating_add(1));
        let children = self.inner.read_dir(path)?;
        let mut nodes = Vec::with_capacity(children.len());
        for child_path in children {
            self.describe_path_calls
                .set(self.describe_path_calls.get().saturating_add(1));
            nodes.push(self.inner.describe_path(&child_path));
        }
        Ok(nodes)
    }

    fn provides_embedded_directory_nodes(&self) -> bool {
        self.inner.provides_embedded_directory_nodes()
    }
}

pub fn make_scan_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn describe_path_with_symlink_metadata(path: &Path) -> Result<ScanNode, ScanPathError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| scan_metadata_error(path, &error))?;
    Ok(scan_node_from_metadata(path, &metadata))
}

fn scan_node_from_metadata(path: &Path, metadata: &std::fs::Metadata) -> ScanNode {
    if is_reparse_point(metadata) {
        return ScanNode::reparse_point(path);
    }

    if metadata.is_dir() {
        return ScanNode::directory(path);
    }

    ScanNode::file(path, metadata.len())
}

fn scan_metadata_error(path: &Path, error: &std::io::Error) -> ScanPathError {
    let reason_code = if error.kind() == std::io::ErrorKind::NotFound {
        SkipReasonCode::MissingPath
    } else if error.kind() == std::io::ErrorKind::PermissionDenied {
        SkipReasonCode::PermissionDenied
    } else {
        SkipReasonCode::MetadataError
    };

    ScanPathError::new(path, reason_code, error.to_string())
}

fn scan_read_dir_error(path: &Path, error: &std::io::Error) -> ScanPathError {
    let reason_code = if error.kind() == std::io::ErrorKind::NotFound {
        SkipReasonCode::MissingPath
    } else if error.kind() == std::io::ErrorKind::PermissionDenied {
        SkipReasonCode::PermissionDenied
    } else {
        SkipReasonCode::ReadDirError
    };

    ScanPathError::new(path, reason_code, error.to_string())
}

#[cfg(windows)]
fn classify_root_path(path: &Path) -> RootPathKind {
    use windows_sys::Win32::Storage::FileSystem::{
        GetDriveTypeW,
    };
    use windows_sys::Win32::System::WindowsProgramming::{
        DRIVE_FIXED, DRIVE_REMOTE, DRIVE_REMOVABLE,
    };

    let Some(root_path) = drive_type_root(path) else {
        return RootPathKind::Other;
    };
    let root_wide = wide_null(root_path.as_os_str());

    match unsafe { GetDriveTypeW(root_wide.as_ptr()) } {
        DRIVE_FIXED => RootPathKind::LocalFixed,
        DRIVE_REMOTE => RootPathKind::Remote,
        DRIVE_REMOVABLE => RootPathKind::Removable,
        _ => RootPathKind::Other,
    }
}

#[cfg(not(windows))]
fn classify_root_path(_path: &Path) -> RootPathKind {
    RootPathKind::Other
}

#[cfg(windows)]
fn drive_type_root(path: &Path) -> Option<PathBuf> {
    use std::path::Component;

    let mut components = path.components();
    let prefix = match components.next()? {
        Component::Prefix(prefix) => PathBuf::from(prefix.as_os_str()),
        _ => return None,
    };

    match components.next()? {
        Component::RootDir => {
            let mut root = prefix;
            root.push(std::path::MAIN_SEPARATOR.to_string());
            Some(root)
        }
        _ => None,
    }
}

#[cfg(windows)]
fn wide_null(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    value.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
#[derive(Default)]
struct WindowsFindFilesystemBackend;

#[cfg(windows)]
impl ScanBackend for WindowsFindFilesystemBackend {
    fn describe_path(&self, path: &Path) -> Result<ScanNode, ScanPathError> {
        describe_path_with_symlink_metadata(path)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ScanPathError> {
        Ok(self
            .read_dir_nodes(path)?
            .into_iter()
            .filter_map(|entry| entry.ok().map(|node| node.path))
            .collect())
    }

    fn read_dir_nodes(&self, path: &Path) -> Result<Vec<Result<ScanNode, ScanPathError>>, ScanPathError> {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::Foundation::{
            ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE,
        };
        use windows_sys::Win32::Storage::FileSystem::{
            FindClose, FindExInfoBasic, FindExSearchNameMatch, FindFirstFileExW, FindNextFileW,
            FIND_FIRST_EX_LARGE_FETCH, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
            WIN32_FIND_DATAW,
        };

        struct FindHandle(windows_sys::Win32::Foundation::HANDLE);

        impl Drop for FindHandle {
            fn drop(&mut self) {
                if self.0 != INVALID_HANDLE_VALUE {
                    unsafe {
                        FindClose(self.0);
                    }
                }
            }
        }

        fn file_name(find_data: &WIN32_FIND_DATAW) -> OsString {
            let len = find_data
                .cFileName
                .iter()
                .position(|value| *value == 0)
                .unwrap_or(find_data.cFileName.len());
            OsString::from_wide(&find_data.cFileName[..len])
        }

        fn node_from_find_data(
            parent: &Path,
            find_data: &WIN32_FIND_DATAW,
        ) -> Option<Result<ScanNode, ScanPathError>> {
            let name = file_name(find_data);
            if name == "." || name == ".." {
                return None;
            }

            let full_path = parent.join(&name);
            let attributes = find_data.dwFileAttributes;
            if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Some(Ok(ScanNode::reparse_point(full_path)));
            }

            if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                return Some(Ok(ScanNode::directory(full_path)));
            }

            let size_bytes = (u64::from(find_data.nFileSizeHigh) << 32) | u64::from(find_data.nFileSizeLow);
            Some(Ok(ScanNode::file(full_path, size_bytes)))
        }

        let search_path = path.join("*");
        let search_wide = wide_null(search_path.as_os_str());
        let mut find_data = WIN32_FIND_DATAW::default();
        let handle = unsafe {
            FindFirstFileExW(
                search_wide.as_ptr(),
                FindExInfoBasic,
                &mut find_data as *mut _ as *mut _,
                FindExSearchNameMatch,
                std::ptr::null(),
                FIND_FIRST_EX_LARGE_FETCH,
            )
        };

        if handle == INVALID_HANDLE_VALUE {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(ERROR_FILE_NOT_FOUND as i32) {
                return Ok(Vec::new());
            }
            return Err(scan_read_dir_error(path, &error));
        }

        let _handle = FindHandle(handle);
        let mut nodes = Vec::new();

        loop {
            if let Some(node) = node_from_find_data(path, &find_data) {
                nodes.push(node);
            }

            let next = unsafe { FindNextFileW(handle, &mut find_data) };
            if next == 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                    break;
                }
                return Err(scan_read_dir_error(path, &error));
            }
        }

        Ok(nodes)
    }

    fn provides_embedded_directory_nodes(&self) -> bool {
        true
    }
}

fn terminal_state_from_result(result: &Result<CompletedScan, ScanFailure>) -> ScanLifecycleState {
    match result {
        Ok(_) => ScanLifecycleState::Completed,
        Err(ScanFailure::Cancelled) => ScanLifecycleState::Cancelled,
        Err(ScanFailure::InvalidRoot { .. } | ScanFailure::Internal { .. }) => ScanLifecycleState::Failed,
    }
}

fn calculate_entries_per_second(total_entries: u64, elapsed: std::time::Duration) -> u64 {
    if total_entries == 0 {
        return 0;
    }

    let elapsed_nanos = elapsed.as_nanos();
    if elapsed_nanos == 0 {
        return total_entries;
    }

    let per_second = (u128::from(total_entries) * 1_000_000_000) / elapsed_nanos;
    per_second.max(1).min(u128::from(u64::MAX)) as u64
}

#[cfg(windows)]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

struct ScanAccumulator {
    top_items_limit: usize,
    total_files: u64,
    total_directories: u64,
    files: Vec<SizedPath>,
    directories: Vec<SizedPath>,
    skipped_paths: Vec<SkippedPath>,
    entries: Vec<ScanEntry>,
}

impl ScanAccumulator {
    fn new(top_items_limit: usize) -> Self {
        Self {
            top_items_limit,
            total_files: 0,
            total_directories: 0,
            files: Vec::new(),
            directories: Vec::new(),
            skipped_paths: Vec::new(),
            entries: Vec::new(),
        }
    }
}

struct ProgressReporter<'a, F>
where
    F: FnMut(ScanStatusSnapshot),
{
    snapshot: ScanStatusSnapshot,
    items_since_emit: u64,
    bytes_since_emit: u64,
    on_progress: &'a mut F,
}

impl<'a, F> ProgressReporter<'a, F>
where
    F: FnMut(ScanStatusSnapshot),
{
    fn new(
        scan_id: String,
        root_path: String,
        started_at: String,
        on_progress: &'a mut F,
    ) -> Self {
        Self {
            snapshot: ScanStatusSnapshot {
                scan_id: Some(scan_id),
                root_path: Some(root_path.clone()),
                state: ScanLifecycleState::Running,
                files_discovered: 0,
                directories_discovered: 0,
                bytes_processed: 0,
                started_at: Some(started_at.clone()),
                updated_at: Some(started_at),
                current_path: Some(root_path),
                message: None,
                completed_scan_id: None,
            },
            items_since_emit: 0,
            bytes_since_emit: 0,
            on_progress,
        }
    }

    fn emit(&mut self) {
        self.snapshot.updated_at = Some(current_timestamp());
        (self.on_progress)(self.snapshot.clone());
        self.items_since_emit = 0;
        self.bytes_since_emit = 0;
    }

    fn note_directory(&mut self, path: &Path) {
        self.snapshot.directories_discovered += 1;
        self.snapshot.current_path = Some(path.display().to_string());
        self.items_since_emit += 1;
        self.emit_if_needed();
    }

    fn note_file(&mut self, path: &Path, size_bytes: u64) {
        self.snapshot.files_discovered += 1;
        self.snapshot.bytes_processed += size_bytes;
        self.snapshot.current_path = Some(path.display().to_string());
        self.items_since_emit += 1;
        self.bytes_since_emit += size_bytes;
        self.emit_if_needed();
    }

    fn emit_if_needed(&mut self) {
        if self.items_since_emit >= PROGRESS_EMIT_ITEM_INTERVAL
            || self.bytes_since_emit >= PROGRESS_EMIT_BYTE_INTERVAL
        {
            self.emit();
        }
    }
}

fn scan_directory<B, F, C>(
    backend: &B,
    path: &Path,
    parent_path: Option<&Path>,
    scheduling_policy: ScanSchedulingPolicy,
    is_cancelled: &C,
    reporter: &mut ProgressReporter<'_, F>,
    accumulator: &mut ScanAccumulator,
) -> Result<u64, ScanFailure>
where
    B: ScanBackend,
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    match scheduling_policy.directory_traversal {
        DirectoryTraversalStrategy::DepthFirst => scan_directory_depth_first(
            backend,
            path,
            parent_path,
            scheduling_policy,
            is_cancelled,
            reporter,
            accumulator,
        ),
    }
}

struct DirectoryFrame {
    path: PathBuf,
    parent_path: Option<PathBuf>,
    children: Vec<Result<ScanNode, ScanPathError>>,
    next_child_index: usize,
    total_bytes: u64,
}

fn scan_directory_depth_first<B, F, C>(
    backend: &B,
    path: &Path,
    parent_path: Option<&Path>,
    scheduling_policy: ScanSchedulingPolicy,
    is_cancelled: &C,
    reporter: &mut ProgressReporter<'_, F>,
    accumulator: &mut ScanAccumulator,
) -> Result<u64, ScanFailure>
where
    B: ScanBackend,
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    if scheduling_policy.max_concurrent_directories != 1 {
        return Err(ScanFailure::Internal {
            message: "only one active directory is supported by the current scheduler".to_string(),
        });
    }

    let mut stack = vec![load_directory_frame(
        backend,
        path,
        parent_path,
        is_cancelled,
        reporter,
        accumulator,
    )?];

    loop {
        if is_cancelled() {
            return Err(ScanFailure::Cancelled);
        }

        let Some(frame) = stack.last_mut() else {
            return Ok(0);
        };

        if frame.next_child_index >= frame.children.len() {
            let finished = stack.pop().expect("frame should exist");
            let total_bytes = finished.total_bytes;

            push_ranked_path(
                &mut accumulator.directories,
                SizedPath {
                    path: finished.path.display().to_string(),
                    size_bytes: total_bytes,
                },
                accumulator.top_items_limit,
            );
            accumulator.entries.push(ScanEntry {
                path: finished.path.display().to_string(),
                parent_path: finished
                    .parent_path
                    .as_ref()
                    .map(|value| value.display().to_string()),
                kind: ScanEntryKind::Directory,
                size_bytes: total_bytes,
            });

            if let Some(parent) = stack.last_mut() {
                parent.total_bytes += total_bytes;
                continue;
            }

            return Ok(total_bytes);
        }

        let next_child = frame.children[frame.next_child_index].clone();
        frame.next_child_index += 1;

        let node = match next_child {
            Ok(node) => node,
            Err(error) => {
                accumulator.skipped_paths.push(error.to_skipped_path());
                continue;
            }
        };

        match node.kind {
            ScanNodeKind::File { size_bytes } => {
                frame.total_bytes += size_bytes;
                accumulator.total_files += 1;
                push_ranked_path(
                    &mut accumulator.files,
                    SizedPath {
                        path: node.path.display().to_string(),
                        size_bytes,
                    },
                    accumulator.top_items_limit,
                );
                accumulator.entries.push(ScanEntry {
                    path: node.path.display().to_string(),
                    parent_path: Some(frame.path.display().to_string()),
                    kind: ScanEntryKind::File,
                    size_bytes,
                });
                reporter.note_file(&node.path, size_bytes);
            }
            ScanNodeKind::Directory => {
                let child_frame = load_directory_frame(
                    backend,
                    &node.path,
                    Some(&frame.path),
                    is_cancelled,
                    reporter,
                    accumulator,
                )?;
                stack.push(child_frame);
            }
            ScanNodeKind::ReparsePoint => {
                accumulator.skipped_paths.push(SkippedPath {
                    path: node.path.display().to_string(),
                    reason_code: SkipReasonCode::ReparsePoint,
                    summary: "Skipped reparse point to avoid recursion loops".to_string(),
                });
            }
        }
    }
}

fn load_directory_frame<B, F, C>(
    backend: &B,
    path: &Path,
    parent_path: Option<&Path>,
    is_cancelled: &C,
    reporter: &mut ProgressReporter<'_, F>,
    accumulator: &mut ScanAccumulator,
) -> Result<DirectoryFrame, ScanFailure>
where
    B: ScanBackend,
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    if is_cancelled() {
        return Err(ScanFailure::Cancelled);
    }

    accumulator.total_directories += 1;
    reporter.note_directory(path);

    let children = match backend.read_dir_nodes(path) {
        Ok(children) => children,
        Err(error) => {
            if parent_path.is_none() {
                return Err(ScanFailure::InvalidRoot {
                    message: error.summary,
                });
            }

            accumulator.skipped_paths.push(error.to_skipped_path());
            Vec::new()
        }
    };

    Ok(DirectoryFrame {
        path: path.to_path_buf(),
        parent_path: parent_path.map(|value| value.to_path_buf()),
        children,
        next_child_index: 0,
        total_bytes: 0,
    })
}

fn sort_ranked_paths(items: &mut [SizedPath]) {
    items.sort_by(|left, right| {
        right
            .size_bytes
            .cmp(&left.size_bytes)
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn push_ranked_path(items: &mut Vec<SizedPath>, candidate: SizedPath, limit: usize) {
    items.push(candidate);
    sort_ranked_paths(items);
    if items.len() > limit {
        items.truncate(limit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tempfile::tempdir;

    struct FakeBackend {
        nodes: HashMap<PathBuf, ScanNode>,
        children: HashMap<PathBuf, Vec<PathBuf>>,
        child_errors: HashMap<PathBuf, ScanPathError>,
    }

    impl FakeBackend {
        fn new() -> Self {
            Self {
                nodes: HashMap::new(),
                children: HashMap::new(),
                child_errors: HashMap::new(),
            }
        }

        fn with_node(mut self, node: ScanNode) -> Self {
            self.nodes.insert(node.path.clone(), node);
            self
        }

        fn with_children(mut self, path: impl Into<PathBuf>, children: Vec<PathBuf>) -> Self {
            self.children.insert(path.into(), children);
            self
        }

        fn with_child_error(mut self, error: ScanPathError) -> Self {
            self.child_errors.insert(error.path.clone(), error);
            self
        }
    }

    impl ScanBackend for FakeBackend {
        fn describe_path(&self, path: &Path) -> Result<ScanNode, ScanPathError> {
            self.nodes
                .get(path)
                .cloned()
                .ok_or_else(|| ScanPathError::new(path, SkipReasonCode::MissingPath, "missing fake node"))
        }

        fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ScanPathError> {
            if let Some(error) = self.child_errors.get(path) {
                return Err(error.clone());
            }

            Ok(self.children.get(path).cloned().unwrap_or_default())
        }
    }

    #[test]
    fn aggregates_nested_directories_and_ranks_results() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path().join("root");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).expect("nested directories");
        std::fs::write(root.join("top.bin"), vec![0_u8; 9]).expect("top file");
        std::fs::write(nested.join("nested.bin"), vec![0_u8; 4]).expect("nested file");

        let request = ScanRequest::new(&root);
        let completed = scan_path(&request, || false, |_| {}).expect("scan should succeed");

        assert_eq!(completed.total_bytes, 13);
        assert_eq!(completed.total_files, 2);
        assert!(completed.total_directories >= 2);
        assert_eq!(completed.largest_files[0].path, root.join("top.bin").display().to_string());
        assert!(completed.largest_directories[0].size_bytes >= 13);
        assert!(completed.entries.iter().any(|entry| {
            entry.path == root.display().to_string()
                && entry.parent_path.is_none()
                && entry.kind == ScanEntryKind::Directory
                && entry.size_bytes == 13
        }));
        assert!(completed.entries.iter().any(|entry| {
            entry.path == nested.display().to_string()
                && entry.parent_path == Some(root.display().to_string())
                && entry.kind == ScanEntryKind::Directory
                && entry.size_bytes == 4
        }));
        assert!(completed.entries.iter().any(|entry| {
            entry.path == root.join("top.bin").display().to_string()
                && entry.parent_path == Some(root.display().to_string())
                && entry.kind == ScanEntryKind::File
                && entry.size_bytes == 9
        }));
        assert!(completed.entries.iter().any(|entry| {
            entry.path == nested.join("nested.bin").display().to_string()
                && entry.parent_path == Some(nested.display().to_string())
                && entry.kind == ScanEntryKind::File
                && entry.size_bytes == 4
        }));
    }

    #[test]
    fn records_skipped_paths_from_backend_errors() {
        let root = PathBuf::from("C:\\scan-root");
        let blocked = root.join("blocked");
        let file = root.join("report.bin");
        let backend = FakeBackend::new()
            .with_node(ScanNode::directory(&root))
            .with_node(ScanNode::directory(&blocked))
            .with_node(ScanNode::file(&file, 6))
            .with_children(root.clone(), vec![blocked.clone(), file.clone()])
            .with_child_error(ScanPathError::new(
                blocked.clone(),
                SkipReasonCode::PermissionDenied,
                "access denied",
            ));

        let completed =
            scan_with_backend(&backend, &ScanRequest::new(&root), || false, |_| {}).expect("scan should succeed");

        assert_eq!(completed.total_bytes, 6);
        assert_eq!(completed.skipped_paths.len(), 1);
        assert_eq!(completed.skipped_paths[0].path, blocked.display().to_string());
        assert_eq!(completed.skipped_paths[0].reason_code, SkipReasonCode::PermissionDenied);
    }

    #[test]
    fn scan_handles_reparse_points() {
        let root = PathBuf::from("C:\\scan-root");
        let reparse = root.join("junction");
        let backend = FakeBackend::new()
            .with_node(ScanNode::directory(&root))
            .with_node(ScanNode::reparse_point(&reparse))
            .with_children(root.clone(), vec![reparse.clone()]);

        let completed =
            scan_with_backend(&backend, &ScanRequest::new(&root), || false, |_| {}).expect("scan should succeed");

        assert_eq!(completed.total_bytes, 0);
        assert_eq!(completed.skipped_paths.len(), 1);
        assert_eq!(completed.skipped_paths[0].reason_code, SkipReasonCode::ReparsePoint);
    }

    #[test]
    fn scan_reports_cancellation_without_completed_result() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path().join("root");
        std::fs::create_dir_all(&root).expect("root directory");
        std::fs::write(root.join("one.bin"), vec![0_u8; 8]).expect("first file");
        std::fs::write(root.join("two.bin"), vec![0_u8; 8]).expect("second file");

        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);
        let request = ScanRequest::new(&root);
        let cancel_from_callback = Arc::clone(&cancelled);
        let result = scan_path(&request, move || flag.load(Ordering::SeqCst), |_| {
            cancel_from_callback.store(true, Ordering::SeqCst);
        });

        assert_eq!(result, Err(ScanFailure::Cancelled));
    }

    #[test]
    fn long_scan_progress_is_bounded_and_monotonic() {
        let root = PathBuf::from("C:\\scan-root");
        let mut children = Vec::new();
        let mut backend = FakeBackend::new().with_node(ScanNode::directory(&root));

        for index in 0..200_u64 {
            let path = root.join(format!("file-{index:03}.bin"));
            children.push(path.clone());
            backend = backend.with_node(ScanNode::file(&path, 1));
        }

        backend = backend.with_children(root.clone(), children);

        let mut snapshots = Vec::new();
        let completed = scan_with_backend(&backend, &ScanRequest::new(&root), || false, |snapshot| {
            snapshots.push(snapshot);
        })
        .expect("scan should succeed");

        assert_eq!(completed.total_files, 200);
        assert!(snapshots.len() > 1, "expected intermediate progress");
        assert!(
            snapshots.len() < 50,
            "expected bounded progress snapshots, got {}",
            snapshots.len()
        );

        let started_at = snapshots[0]
            .started_at
            .clone()
            .expect("running snapshots should include startedAt");
        let root_label = root.display().to_string();

        for pair in snapshots.windows(2) {
            let current = &pair[0];
            let next = &pair[1];
            assert!(current.files_discovered <= next.files_discovered);
            assert!(current.directories_discovered <= next.directories_discovered);
            assert!(current.bytes_processed <= next.bytes_processed);
        }

        for snapshot in &snapshots {
            assert_eq!(snapshot.state, ScanLifecycleState::Running);
            assert_eq!(snapshot.started_at.as_deref(), Some(started_at.as_str()));
            assert!(snapshot.updated_at.is_some(), "running snapshots should include updatedAt");
            assert!(
                snapshot.current_path.as_deref().is_some_and(|path| path.starts_with(&root_label)),
                "expected best-effort currentPath within the scan root, got {:?}",
                snapshot.current_path
            );
        }
    }

    #[test]
    fn measured_scan_reports_baseline_traversal_counters() {
        let root = PathBuf::from("C:\\scan-root");
        let nested = root.join("nested");
        let report = root.join("report.bin");
        let nested_file = nested.join("nested.bin");
        let backend = FakeBackend::new()
            .with_node(ScanNode::directory(&root))
            .with_node(ScanNode::directory(&nested))
            .with_node(ScanNode::file(&report, 9))
            .with_node(ScanNode::file(&nested_file, 4))
            .with_children(root.clone(), vec![nested.clone(), report.clone()])
            .with_children(nested.clone(), vec![nested_file.clone()]);

        let measured = measure_scan_with_backend(&backend, &ScanRequest::new(&root), || false);

        assert_eq!(measured.result.as_ref().map(|scan| scan.total_bytes), Ok(13));
        assert_eq!(measured.measurement.terminal_state, ScanLifecycleState::Completed);
        assert_eq!(measured.measurement.describe_path_calls, 4);
        assert_eq!(measured.measurement.read_dir_calls, 2);
        assert_eq!(measured.measurement.files_discovered, 2);
        assert_eq!(measured.measurement.directories_discovered, 2);
        assert_eq!(measured.measurement.bytes_processed, 13);
        assert!(measured.measurement.progress_event_count >= 1);
        assert!(measured.measurement.cancellation_check_count >= 3);
        assert!(measured.measurement.entries_per_second >= 1);
    }

    #[test]
    fn measured_scan_reports_cancellation_metrics() {
        let root = PathBuf::from("C:\\scan-root");
        let child = root.join("report.bin");
        let backend = FakeBackend::new()
            .with_node(ScanNode::directory(&root))
            .with_node(ScanNode::file(&child, 8))
            .with_children(root.clone(), vec![child]);

        let checks = std::cell::Cell::new(0_u64);
        let measured = measure_scan_with_backend(&backend, &ScanRequest::new(&root), || {
            let next = checks.get() + 1;
            checks.set(next);
            next >= 2
        });

        assert_eq!(measured.result, Err(ScanFailure::Cancelled));
        assert_eq!(measured.measurement.terminal_state, ScanLifecycleState::Cancelled);
        assert_eq!(measured.measurement.describe_path_calls, 1);
        assert_eq!(measured.measurement.read_dir_calls, 0);
        assert!(measured.measurement.cancellation_check_count >= 2);
        assert!(measured.measurement.cancel_to_stop_millis.is_some());
        assert_eq!(measured.measurement.files_discovered, 0);
        assert_eq!(measured.measurement.directories_discovered, 0);
        assert_eq!(measured.measurement.bytes_processed, 0);
    }

    #[test]
    fn default_scan_scheduling_policy_is_conservative() {
        for root_kind in [
            RootPathKind::LocalFixed,
            RootPathKind::Removable,
            RootPathKind::Remote,
            RootPathKind::Other,
        ] {
            let policy = default_scan_scheduling_policy(root_kind);
            assert_eq!(policy.max_concurrent_directories, 1);
            assert_eq!(policy.directory_traversal, DirectoryTraversalStrategy::DepthFirst);
            assert!(!policy.background_mode_requested);
        }
    }

    #[test]
    fn routed_scan_prefers_optimized_backend_for_local_fixed_roots() {
        let root = PathBuf::from("C:\\scan-root");
        let optimized_file = root.join("optimized.bin");
        let fallback_file = root.join("fallback.bin");
        let optimized = FakeBackend::new()
            .with_node(ScanNode::directory(&root))
            .with_node(ScanNode::file(&optimized_file, 9))
            .with_children(root.clone(), vec![optimized_file]);
        let fallback = FakeBackend::new()
            .with_node(ScanNode::directory(&root))
            .with_node(ScanNode::file(&fallback_file, 3))
            .with_children(root.clone(), vec![fallback_file]);

        let completed = scan_with_routed_backend(
            RootPathKind::LocalFixed,
            &ScanRequest::new(&root),
            &optimized,
            &fallback,
            || false,
            |_| {},
        )
        .expect("scan should succeed");

        assert_eq!(completed.total_bytes, 9);
        assert_eq!(completed.largest_files[0].path, root.join("optimized.bin").display().to_string());
    }

    #[test]
    fn routed_scan_uses_recursive_fallback_for_remote_roots() {
        let root = PathBuf::from("C:\\scan-root");
        let optimized_file = root.join("optimized.bin");
        let fallback_file = root.join("fallback.bin");
        let optimized = FakeBackend::new()
            .with_node(ScanNode::directory(&root))
            .with_node(ScanNode::file(&optimized_file, 9))
            .with_children(root.clone(), vec![optimized_file]);
        let fallback = FakeBackend::new()
            .with_node(ScanNode::directory(&root))
            .with_node(ScanNode::file(&fallback_file, 3))
            .with_children(root.clone(), vec![fallback_file]);

        let completed = scan_with_routed_backend(
            RootPathKind::Remote,
            &ScanRequest::new(&root),
            &optimized,
            &fallback,
            || false,
            |_| {},
        )
        .expect("scan should succeed");

        assert_eq!(completed.total_bytes, 3);
        assert_eq!(completed.largest_files[0].path, root.join("fallback.bin").display().to_string());
    }

    #[cfg(windows)]
    #[test]
    fn measured_scan_path_uses_windows_find_backend_for_fixed_roots() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path().join("root");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).expect("nested directories");
        std::fs::write(root.join("top.bin"), vec![0_u8; 9]).expect("top file");
        std::fs::write(nested.join("nested.bin"), vec![0_u8; 4]).expect("nested file");

        let measured = measure_scan_path(&ScanRequest::new(&root), || false);

        assert_eq!(measured.result.as_ref().map(|scan| scan.total_bytes), Ok(13));
        assert_eq!(measured.measurement.describe_path_calls, 1);
        assert_eq!(measured.measurement.read_dir_calls, 2);
        assert_eq!(measured.measurement.files_discovered, 2);
        assert_eq!(measured.measurement.directories_discovered, 2);
    }
}
