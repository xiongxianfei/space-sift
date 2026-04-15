use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_TOP_ITEMS_LIMIT: usize = 10;

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
}

impl ScanRequest {
    pub fn new(root_path: impl Into<PathBuf>) -> Self {
        Self {
            root_path: root_path.into(),
            top_items_limit: DEFAULT_TOP_ITEMS_LIMIT,
            scan_id: None,
        }
    }
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

pub trait ScanBackend {
    fn describe_path(&self, path: &Path) -> Result<ScanNode, ScanPathError>;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ScanPathError>;
}

#[derive(Default)]
pub struct RecursiveFilesystemBackend;

impl ScanBackend for RecursiveFilesystemBackend {
    fn describe_path(&self, path: &Path) -> Result<ScanNode, ScanPathError> {
        let metadata = std::fs::symlink_metadata(path).map_err(|error| {
            let reason_code = if error.kind() == std::io::ErrorKind::NotFound {
                SkipReasonCode::MissingPath
            } else if error.kind() == std::io::ErrorKind::PermissionDenied {
                SkipReasonCode::PermissionDenied
            } else {
                SkipReasonCode::MetadataError
            };

            ScanPathError::new(path, reason_code, error.to_string())
        })?;

        if is_reparse_point(&metadata) {
            return Ok(ScanNode::reparse_point(path));
        }

        if metadata.is_dir() {
            return Ok(ScanNode::directory(path));
        }

        Ok(ScanNode::file(path, metadata.len()))
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ScanPathError> {
        let entries = std::fs::read_dir(path).map_err(|error| {
            let reason_code = if error.kind() == std::io::ErrorKind::PermissionDenied {
                SkipReasonCode::PermissionDenied
            } else {
                SkipReasonCode::ReadDirError
            };

            ScanPathError::new(path, reason_code, error.to_string())
        })?;

        Ok(entries.filter_map(|entry| entry.ok().map(|value| value.path())).collect())
    }
}

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

pub fn scan_with_backend<B, F, C>(
    backend: &B,
    request: &ScanRequest,
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
    let started_at = current_timestamp();
    let mut reporter = ProgressReporter::new(scan_id.clone(), root_path.clone(), &mut on_progress);
    let mut accumulator = ScanAccumulator::default();

    reporter.emit();

    if is_cancelled() {
        return Err(ScanFailure::Cancelled);
    }

    let total_bytes = scan_directory(
        backend,
        &request.root_path,
        None,
        &is_cancelled,
        &mut reporter,
        &mut accumulator,
    )?;

    if is_cancelled() {
        return Err(ScanFailure::Cancelled);
    }

    let top_items_limit = request.top_items_limit.max(1);
    sort_ranked_paths(&mut accumulator.files);
    sort_ranked_paths(&mut accumulator.directories);
    accumulator.files.truncate(top_items_limit);
    accumulator.directories.truncate(top_items_limit);

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

pub fn make_scan_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
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

#[derive(Default)]
struct ScanAccumulator {
    total_files: u64,
    total_directories: u64,
    files: Vec<SizedPath>,
    directories: Vec<SizedPath>,
    skipped_paths: Vec<SkippedPath>,
    entries: Vec<ScanEntry>,
}

struct ProgressReporter<'a, F>
where
    F: FnMut(ScanStatusSnapshot),
{
    snapshot: ScanStatusSnapshot,
    on_progress: &'a mut F,
}

impl<'a, F> ProgressReporter<'a, F>
where
    F: FnMut(ScanStatusSnapshot),
{
    fn new(scan_id: String, root_path: String, on_progress: &'a mut F) -> Self {
        Self {
            snapshot: ScanStatusSnapshot {
                scan_id: Some(scan_id),
                root_path: Some(root_path),
                state: ScanLifecycleState::Running,
                files_discovered: 0,
                directories_discovered: 0,
                bytes_processed: 0,
                message: None,
                completed_scan_id: None,
            },
            on_progress,
        }
    }

    fn emit(&mut self) {
        (self.on_progress)(self.snapshot.clone());
    }

    fn note_directory(&mut self) {
        self.snapshot.directories_discovered += 1;
        self.emit();
    }

    fn note_file(&mut self, size_bytes: u64) {
        self.snapshot.files_discovered += 1;
        self.snapshot.bytes_processed += size_bytes;
        self.emit();
    }
}

fn scan_directory<B, F, C>(
    backend: &B,
    path: &Path,
    parent_path: Option<&Path>,
    is_cancelled: &C,
    reporter: &mut ProgressReporter<'_, F>,
    accumulator: &mut ScanAccumulator,
) -> Result<u64, ScanFailure>
where
    B: ScanBackend,
    F: FnMut(ScanStatusSnapshot),
    C: Fn() -> bool,
{
    if is_cancelled() {
        return Err(ScanFailure::Cancelled);
    }

    accumulator.total_directories += 1;
    reporter.note_directory();

    let children = match backend.read_dir(path) {
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

    let mut total_bytes = 0_u64;

    for child_path in children {
        if is_cancelled() {
            return Err(ScanFailure::Cancelled);
        }

        let node = match backend.describe_path(&child_path) {
            Ok(node) => node,
            Err(error) => {
                accumulator.skipped_paths.push(error.to_skipped_path());
                continue;
            }
        };

        match node.kind {
            ScanNodeKind::File { size_bytes } => {
                total_bytes += size_bytes;
                accumulator.total_files += 1;
                accumulator.files.push(SizedPath {
                    path: node.path.display().to_string(),
                    size_bytes,
                });
                accumulator.entries.push(ScanEntry {
                    path: node.path.display().to_string(),
                    parent_path: Some(path.display().to_string()),
                    kind: ScanEntryKind::File,
                    size_bytes,
                });
                reporter.note_file(size_bytes);
            }
            ScanNodeKind::Directory => {
                let child_bytes =
                    scan_directory(backend, &node.path, Some(path), is_cancelled, reporter, accumulator)?;
                total_bytes += child_bytes;
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

    accumulator.directories.push(SizedPath {
        path: path.display().to_string(),
        size_bytes: total_bytes,
    });
    accumulator.entries.push(ScanEntry {
        path: path.display().to_string(),
        parent_path: parent_path.map(|value| value.display().to_string()),
        kind: ScanEntryKind::Directory,
        size_bytes: total_bytes,
    });

    Ok(total_bytes)
}

fn sort_ranked_paths(items: &mut [SizedPath]) {
    items.sort_by(|left, right| {
        right
            .size_bytes
            .cmp(&left.size_bytes)
            .then_with(|| left.path.cmp(&right.path))
    });
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
}
