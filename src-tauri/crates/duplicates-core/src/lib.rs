use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::time::{Instant, SystemTime};
use std::thread;
use thiserror::Error;
use uuid::Uuid;

#[cfg(windows)]
use std::{ffi::OsStr, os::windows::ffi::OsStrExt};

pub const DEFAULT_PARTIAL_HASH_BYTES: usize = 64 * 1024;
const DUPLICATE_PROGRESS_EMIT_INTERVAL: u64 = 64;
const FULL_HASH_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateAnalysisState {
    Idle,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateAnalysisStage {
    Grouping,
    PartialHash,
    FullHash,
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateIssueCode {
    MissingPath,
    MetadataChanged,
    ReadError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateIssue {
    pub path: String,
    pub code: DuplicateIssueCode,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCandidate {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateAnalysisRequest {
    pub analysis_id: Option<String>,
    pub scan_id: String,
    pub root_path: String,
    pub candidates: Vec<DuplicateCandidate>,
    pub partial_hash_bytes: usize,
}

impl DuplicateAnalysisRequest {
    pub fn new(scan_id: impl Into<String>, root_path: impl Into<String>, candidates: Vec<DuplicateCandidate>) -> Self {
        Self {
            analysis_id: None,
            scan_id: scan_id.into(),
            root_path: root_path.into(),
            candidates,
            partial_hash_bytes: DEFAULT_PARTIAL_HASH_BYTES,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroupMember {
    pub path: String,
    pub size_bytes: u64,
    pub modified_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub group_id: String,
    pub size_bytes: u64,
    pub reclaimable_bytes: u64,
    pub members: Vec<DuplicateGroupMember>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompletedDuplicateAnalysis {
    pub analysis_id: String,
    pub scan_id: String,
    pub root_path: String,
    pub started_at: String,
    pub completed_at: String,
    pub groups: Vec<DuplicateGroup>,
    pub issues: Vec<DuplicateIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateAnalysisMeasurement {
    pub elapsed_millis: u128,
    pub validated_candidate_count: u64,
    pub partial_hash_candidate_count: u64,
    pub full_hash_candidate_count: u64,
    pub cache_lookup_count: u64,
    pub cache_hit_count: u64,
    pub cache_miss_count: u64,
    pub cache_write_count: u64,
    pub partial_cache_lookup_count: u64,
    pub partial_cache_hit_count: u64,
    pub partial_cache_miss_count: u64,
    pub partial_cache_write_count: u64,
    pub full_cache_lookup_count: u64,
    pub full_cache_hit_count: u64,
    pub full_cache_miss_count: u64,
    pub full_cache_write_count: u64,
    pub partial_hash_bytes_read: u64,
    pub full_hash_bytes_read: u64,
    pub progress_event_count: u64,
    pub cancel_to_stop_millis: Option<u128>,
    pub terminal_state: DuplicateAnalysisState,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MeasuredDuplicateAnalysis {
    pub result: Result<CompletedDuplicateAnalysis, DuplicateAnalysisFailure>,
    pub measurement: DuplicateAnalysisMeasurement,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateStatusSnapshot {
    pub analysis_id: Option<String>,
    pub scan_id: Option<String>,
    pub state: DuplicateAnalysisState,
    pub stage: Option<DuplicateAnalysisStage>,
    pub items_processed: u64,
    pub groups_emitted: u64,
    pub message: Option<String>,
    pub completed_analysis_id: Option<String>,
}

impl Default for DuplicateStatusSnapshot {
    fn default() -> Self {
        Self {
            analysis_id: None,
            scan_id: None,
            state: DuplicateAnalysisState::Idle,
            stage: None,
            items_processed: 0,
            groups_emitted: 0,
            message: None,
            completed_analysis_id: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HashCacheKey {
    pub path: String,
    pub size_bytes: u64,
    pub modified_at_millis: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CachedHashes {
    pub partial_hash: Option<String>,
    pub full_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HashCacheWrite {
    pub key: HashCacheKey,
    pub partial_hash: Option<String>,
    pub full_hash: Option<String>,
}

pub trait HashCache {
    fn get_cached_hashes(&self, key: &HashCacheKey) -> Result<Option<CachedHashes>, DuplicateAnalysisFailure>;
    fn save_partial_hash(&self, key: &HashCacheKey, partial_hash: &str) -> Result<(), DuplicateAnalysisFailure>;
    fn save_full_hash(&self, key: &HashCacheKey, full_hash: &str) -> Result<(), DuplicateAnalysisFailure>;
    fn get_cached_hashes_batch(
        &self,
        keys: &[HashCacheKey],
    ) -> Result<HashMap<HashCacheKey, CachedHashes>, DuplicateAnalysisFailure> {
        let mut entries = HashMap::with_capacity(keys.len());
        for key in keys {
            if let Some(cached) = self.get_cached_hashes(key)? {
                entries.insert(key.clone(), cached);
            }
        }
        Ok(entries)
    }
    fn save_hashes_batch(
        &self,
        writes: &[HashCacheWrite],
    ) -> Result<(), DuplicateAnalysisFailure> {
        for write in writes {
            if let Some(partial_hash) = &write.partial_hash {
                self.save_partial_hash(&write.key, partial_hash)?;
            }
            if let Some(full_hash) = &write.full_hash {
                self.save_full_hash(&write.key, full_hash)?;
            }
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct NoopHashCache;

impl HashCache for NoopHashCache {
    fn get_cached_hashes(&self, _key: &HashCacheKey) -> Result<Option<CachedHashes>, DuplicateAnalysisFailure> {
        Ok(None)
    }

    fn save_partial_hash(&self, _key: &HashCacheKey, _partial_hash: &str) -> Result<(), DuplicateAnalysisFailure> {
        Ok(())
    }

    fn save_full_hash(&self, _key: &HashCacheKey, _full_hash: &str) -> Result<(), DuplicateAnalysisFailure> {
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DuplicateAnalysisFailure {
    #[error("duplicate analysis request is invalid: {message}")]
    InvalidRequest { message: String },
    #[error("duplicate analysis cancelled")]
    Cancelled,
    #[error("duplicate analysis failed: {message}")]
    Internal { message: String },
}

pub fn make_analysis_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DuplicateRootPathKind {
    LocalFixed,
    Removable,
    Remote,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DuplicateSchedulingPolicy {
    root_path_kind: DuplicateRootPathKind,
    allow_content_hashing: bool,
    partial_hash_worker_limit: usize,
    full_hash_worker_limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct HashedCandidate {
    candidate: LiveCandidate,
    hash: String,
    bytes_read: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum StageHashOutcome {
    Hashed(HashedCandidate),
    Issue(DuplicateIssue),
    Cancelled,
}

fn scheduling_policy_for_request(request: &DuplicateAnalysisRequest) -> DuplicateSchedulingPolicy {
    let root_path_kind = classify_duplicate_root_path_kind(&request.root_path);
    let available_parallelism = thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1);
    scheduling_policy_for_root_kind(root_path_kind, available_parallelism)
}

fn scheduling_policy_for_root_kind(
    root_path_kind: DuplicateRootPathKind,
    available_parallelism: usize,
) -> DuplicateSchedulingPolicy {
    match root_path_kind {
        DuplicateRootPathKind::LocalFixed => DuplicateSchedulingPolicy {
            root_path_kind,
            allow_content_hashing: true,
            partial_hash_worker_limit: available_parallelism.clamp(1, 2),
            full_hash_worker_limit: available_parallelism.clamp(1, 4),
        },
        DuplicateRootPathKind::Removable | DuplicateRootPathKind::Other => DuplicateSchedulingPolicy {
            root_path_kind,
            allow_content_hashing: true,
            partial_hash_worker_limit: 1,
            full_hash_worker_limit: 1,
        },
        DuplicateRootPathKind::Remote => DuplicateSchedulingPolicy {
            root_path_kind,
            allow_content_hashing: false,
            partial_hash_worker_limit: 1,
            full_hash_worker_limit: 1,
        },
    }
}

fn effective_worker_count(limit: usize, item_count: usize) -> usize {
    limit.max(1).min(item_count.max(1))
}

fn classify_duplicate_root_path_kind(root_path: &str) -> DuplicateRootPathKind {
    let trimmed = root_path.trim();
    if trimmed.starts_with("\\\\") || trimmed.starts_with("//") {
        return DuplicateRootPathKind::Remote;
    }

    #[cfg(windows)]
    {
        if let Some(root) = windows_drive_root(trimmed) {
            let wide_root = to_windows_wide_string(&root);
            let drive_type = unsafe { GetDriveTypeW(wide_root.as_ptr()) };
            return match drive_type {
                DRIVE_FIXED => DuplicateRootPathKind::LocalFixed,
                DRIVE_REMOVABLE => DuplicateRootPathKind::Removable,
                DRIVE_REMOTE => DuplicateRootPathKind::Remote,
                _ => DuplicateRootPathKind::Other,
            };
        }
    }

    DuplicateRootPathKind::Other
}

fn unsupported_non_local_path_issue(
    path: &str,
    root_path_kind: DuplicateRootPathKind,
) -> DuplicateIssue {
    let summary = match root_path_kind {
        DuplicateRootPathKind::Remote => {
            "Duplicate analysis skipped this path to preserve the local-only contract and avoid network-backed reads."
        }
        DuplicateRootPathKind::Removable | DuplicateRootPathKind::Other | DuplicateRootPathKind::LocalFixed => {
            "Duplicate analysis skipped this path because it could not be verified safely within the local-only contract."
        }
    };

    DuplicateIssue {
        path: path.to_string(),
        code: DuplicateIssueCode::ReadError,
        summary: summary.to_string(),
    }
}

fn placeholder_hydration_issue(path: &str) -> DuplicateIssue {
    DuplicateIssue {
        path: path.to_string(),
        code: DuplicateIssueCode::ReadError,
        summary: "Duplicate analysis skipped this file to avoid placeholder hydration and preserve the local-only contract.".to_string(),
    }
}

fn execute_bounded_stage_work<T, R, F>(items: Vec<T>, worker_count: usize, work: &F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(Vec<T>) -> Vec<R> + Sync,
{
    if worker_count <= 1 || items.len() <= 1 {
        return work(items);
    }

    let chunk_size = items.len().div_ceil(worker_count);
    thread::scope(|scope| {
        let mut handles = Vec::new();
        let mut iter = items.into_iter();

        loop {
            let chunk = iter.by_ref().take(chunk_size).collect::<Vec<_>>();
            if chunk.is_empty() {
                break;
            }

            handles.push(scope.spawn(move || work(chunk)));
        }

        let mut results = Vec::new();
        for handle in handles {
            results.extend(handle.join().expect("bounded duplicate stage worker panicked"));
        }
        results
    })
}

fn hash_partial_candidates<S>(
    candidates: Vec<LiveCandidate>,
    partial_hash_bytes: usize,
    worker_count: usize,
    should_cancel: &S,
) -> Vec<StageHashOutcome>
where
    S: Fn() -> bool + Sync,
{
    execute_bounded_stage_work(candidates, worker_count, &|chunk| {
        let mut buffer = Vec::with_capacity(partial_hash_bytes);
        chunk
            .into_iter()
            .map(|candidate| match hash_file_prefix(
                Path::new(&candidate.path),
                partial_hash_bytes,
                should_cancel,
                &mut buffer,
            ) {
                Ok((hash, bytes_read)) => StageHashOutcome::Hashed(HashedCandidate {
                    candidate,
                    hash,
                    bytes_read,
                }),
                Err(DuplicateCandidateFailure::Issue(issue)) => StageHashOutcome::Issue(issue),
                Err(DuplicateCandidateFailure::Cancelled) => StageHashOutcome::Cancelled,
            })
            .collect()
    })
}

fn hash_full_candidates<S>(
    candidates: Vec<LiveCandidate>,
    worker_count: usize,
    should_cancel: &S,
) -> Vec<StageHashOutcome>
where
    S: Fn() -> bool + Sync,
{
    execute_bounded_stage_work(candidates, worker_count, &|chunk| {
        let mut buffer = vec![0_u8; FULL_HASH_BUFFER_BYTES];
        chunk
            .into_iter()
            .map(|candidate| match hash_file_full(
                Path::new(&candidate.path),
                should_cancel,
                buffer.as_mut_slice(),
            ) {
                Ok((hash, bytes_read)) => StageHashOutcome::Hashed(HashedCandidate {
                    candidate,
                    hash,
                    bytes_read,
                }),
                Err(DuplicateCandidateFailure::Issue(issue)) => StageHashOutcome::Issue(issue),
                Err(DuplicateCandidateFailure::Cancelled) => StageHashOutcome::Cancelled,
            })
            .collect()
    })
}

#[cfg(windows)]
const DRIVE_REMOVABLE: u32 = 2;
#[cfg(windows)]
const DRIVE_FIXED: u32 = 3;
#[cfg(windows)]
const DRIVE_REMOTE: u32 = 4;
#[cfg(windows)]
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
#[cfg(windows)]
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
#[cfg(windows)]
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

#[cfg(windows)]
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn GetDriveTypeW(root_path_name: *const u16) -> u32;
}

#[cfg(windows)]
fn windows_drive_root(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && bytes[0].is_ascii_alphabetic() {
        Some(format!("{}:\\", bytes[0] as char))
    } else {
        None
    }
}

#[cfg(windows)]
fn to_windows_wide_string(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn has_unsafe_local_only_file_attributes(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

#[cfg(windows)]
fn candidate_has_unsafe_local_only_metadata(metadata: &std::fs::Metadata) -> bool {
    has_unsafe_local_only_file_attributes(metadata.file_attributes())
}

#[cfg(not(windows))]
fn candidate_has_unsafe_local_only_metadata(_metadata: &std::fs::Metadata) -> bool {
    false
}

pub fn analyze_duplicates<C, S, F>(
    cache: &C,
    request: &DuplicateAnalysisRequest,
    should_cancel: S,
    on_progress: F,
) -> Result<CompletedDuplicateAnalysis, DuplicateAnalysisFailure>
where
    C: HashCache,
    S: Fn() -> bool + Sync,
    F: FnMut(DuplicateStatusSnapshot),
{
    let mut observer = NoopDuplicateAnalysisMetricsObserver;
    analyze_duplicates_with_observer(cache, request, should_cancel, on_progress, &mut observer)
}

pub fn measure_duplicate_analysis<C, S>(
    cache: &C,
    request: &DuplicateAnalysisRequest,
    should_cancel: S,
) -> MeasuredDuplicateAnalysis
where
    C: HashCache,
    S: Fn() -> bool + Sync,
{
    use std::cell::Cell;
    use std::sync::Mutex;

    let progress_event_count = Cell::new(0_u64);
    let cancel_requested_at = Mutex::new(None::<Instant>);
    let started_at = Instant::now();
    let mut observer = DuplicateAnalysisMeasurementObserver::default();
    let result = analyze_duplicates_with_observer(
        cache,
        request,
        || {
            let cancelled = should_cancel();
            if cancelled {
                let mut requested_at = cancel_requested_at.lock().expect("cancel mutex");
                if requested_at.is_none() {
                    *requested_at = Some(Instant::now());
                }
            }
            cancelled
        },
        |_| {
            progress_event_count.set(progress_event_count.get().saturating_add(1));
        },
        &mut observer,
    );
    let finished_at = Instant::now();
    let elapsed = finished_at.saturating_duration_since(started_at);
    let terminal_state = terminal_state_from_result(&result);

    MeasuredDuplicateAnalysis {
        result,
        measurement: DuplicateAnalysisMeasurement {
            elapsed_millis: elapsed.as_millis(),
            validated_candidate_count: observer.validated_candidate_count,
            partial_hash_candidate_count: observer.partial_hash_candidate_count,
            full_hash_candidate_count: observer.full_hash_candidate_count,
            cache_lookup_count: observer.partial_cache_lookup_count + observer.full_cache_lookup_count,
            cache_hit_count: observer.partial_cache_hit_count + observer.full_cache_hit_count,
            cache_miss_count: observer.partial_cache_miss_count + observer.full_cache_miss_count,
            cache_write_count: observer.partial_cache_write_count + observer.full_cache_write_count,
            partial_cache_lookup_count: observer.partial_cache_lookup_count,
            partial_cache_hit_count: observer.partial_cache_hit_count,
            partial_cache_miss_count: observer.partial_cache_miss_count,
            partial_cache_write_count: observer.partial_cache_write_count,
            full_cache_lookup_count: observer.full_cache_lookup_count,
            full_cache_hit_count: observer.full_cache_hit_count,
            full_cache_miss_count: observer.full_cache_miss_count,
            full_cache_write_count: observer.full_cache_write_count,
            partial_hash_bytes_read: observer.partial_hash_bytes_read,
            full_hash_bytes_read: observer.full_hash_bytes_read,
            progress_event_count: progress_event_count.get(),
            cancel_to_stop_millis: cancel_requested_at
                .into_inner()
                .expect("cancel mutex")
                .map(|cancelled_at| finished_at.saturating_duration_since(cancelled_at).as_millis()),
            terminal_state,
        },
    }
}

fn analyze_duplicates_with_observer<C, S, F, M>(
    cache: &C,
    request: &DuplicateAnalysisRequest,
    should_cancel: S,
    mut on_progress: F,
    observer: &mut M,
) -> Result<CompletedDuplicateAnalysis, DuplicateAnalysisFailure>
where
    C: HashCache,
    S: Fn() -> bool + Sync,
    F: FnMut(DuplicateStatusSnapshot),
    M: DuplicateAnalysisMetricsObserver,
{
    let scan_id = request.scan_id.trim().to_string();
    let root_path = request.root_path.trim().to_string();
    if scan_id.is_empty() || root_path.is_empty() {
        return Err(DuplicateAnalysisFailure::InvalidRequest {
            message: "duplicate analysis requires a scan identifier and root path".to_string(),
        });
    }

    let analysis_id = request.analysis_id.clone().unwrap_or_else(make_analysis_id);
    let started_at = current_timestamp();
    let mut reporter = DuplicateProgressReporter::new(analysis_id.clone(), scan_id.clone(), &mut on_progress);
    let scheduling_policy = scheduling_policy_for_request(request);
    reporter.emit();
    ensure_not_cancelled(&should_cancel)?;

    let mut issues = Vec::new();
    let duplicate_size_candidates = duplicate_size_candidates(&request.candidates);
    let validated_candidates = validate_candidates(
        &duplicate_size_candidates,
        &should_cancel,
        &mut reporter,
        &mut issues,
        scheduling_policy,
        observer,
    )?;
    let mut cache_session =
        DuplicateHashCacheSession::preload(cache, &validated_candidates, request.partial_hash_bytes)?;
    let mut groups = Vec::new();

    for size_group in group_by_size(validated_candidates).into_values() {
        ensure_not_cancelled(&should_cancel)?;
        if size_group.len() < 2 {
            continue;
        }

        let partial_groups = if should_use_partial_hash(size_group[0].size_bytes, request.partial_hash_bytes) {
            group_by_partial_hash(
                &mut cache_session,
                size_group,
                request.partial_hash_bytes,
                &should_cancel,
                &mut reporter,
                &mut issues,
                scheduling_policy,
                observer,
            )?
            .into_values()
            .collect::<Vec<_>>()
        } else {
            vec![size_group]
        };
        cache_session.flush_pending_writes(cache)?;

        for partial_group in partial_groups {
            ensure_not_cancelled(&should_cancel)?;
            if partial_group.len() < 2 {
                continue;
            }

            for verified_group in
                group_by_full_hash(
                    &mut cache_session,
                    partial_group,
                    &should_cancel,
                    &mut reporter,
                    &mut issues,
                    scheduling_policy,
                    observer,
                )?
                    .into_values()
            {
                if verified_group.len() < 2 {
                    continue;
                }

                groups.push(verified_group);
                reporter.note_group_emitted();
            }
            cache_session.flush_pending_writes(cache)?;
        }
    }
    cache_session.flush_pending_writes(cache)?;

    let groups = finalize_groups(groups, &analysis_id);
    issues.sort_by(|left, right| left.path.cmp(&right.path).then_with(|| left.code.cmp(&right.code)));

    let result = CompletedDuplicateAnalysis {
        analysis_id: analysis_id.clone(),
        scan_id,
        root_path,
        started_at,
        completed_at: current_timestamp(),
        groups,
        issues,
    };

    reporter.finish(&analysis_id);
    Ok(result)
}

fn validate_candidates<F, M>(
    candidates: &[DuplicateCandidate],
    should_cancel: &impl Fn() -> bool,
    reporter: &mut DuplicateProgressReporter<'_, F>,
    issues: &mut Vec<DuplicateIssue>,
    scheduling_policy: DuplicateSchedulingPolicy,
    observer: &mut M,
) -> Result<Vec<LiveCandidate>, DuplicateAnalysisFailure>
where
    F: FnMut(DuplicateStatusSnapshot),
    M: DuplicateAnalysisMetricsObserver,
{
    let mut unique_paths = HashSet::new();
    let mut validated = Vec::new();

    for candidate in candidates {
        ensure_not_cancelled(should_cancel)?;
        reporter.set_stage(DuplicateAnalysisStage::Grouping);

        if !unique_paths.insert(candidate.path.clone()) {
            reporter.note_item_processed();
            continue;
        }

        if !scheduling_policy.allow_content_hashing {
            issues.push(unsupported_non_local_path_issue(
                &candidate.path,
                scheduling_policy.root_path_kind,
            ));
            reporter.note_item_processed();
            continue;
        }

        match validate_candidate(candidate, scheduling_policy.root_path_kind) {
            Ok(Some(live_candidate)) => {
                observer.on_validated_candidate();
                validated.push(live_candidate);
            }
            Ok(None) => {}
            Err(issue) => issues.push(issue),
        }

        reporter.note_item_processed();
    }

    Ok(validated)
}

fn duplicate_size_candidates(candidates: &[DuplicateCandidate]) -> Vec<DuplicateCandidate> {
    let mut counts = HashMap::new();
    for candidate in candidates {
        *counts.entry(candidate.size_bytes).or_insert(0_u64) += 1;
    }

    candidates
        .iter()
        .filter(|candidate| counts.get(&candidate.size_bytes).copied().unwrap_or(0) > 1)
        .cloned()
        .collect()
}

fn validate_candidate(
    candidate: &DuplicateCandidate,
    root_path_kind: DuplicateRootPathKind,
) -> Result<Option<LiveCandidate>, DuplicateIssue> {
    let path = PathBuf::from(&candidate.path);
    let metadata = std::fs::metadata(&path).map_err(|error| {
        let code = if error.kind() == std::io::ErrorKind::NotFound {
            DuplicateIssueCode::MissingPath
        } else {
            DuplicateIssueCode::ReadError
        };

        DuplicateIssue {
            path: candidate.path.clone(),
            code,
            summary: error.to_string(),
        }
    })?;

    if !metadata.is_file() {
        return Err(DuplicateIssue {
            path: candidate.path.clone(),
            code: DuplicateIssueCode::MetadataChanged,
            summary: "Path is no longer a regular file.".to_string(),
        });
    }

    if metadata.len() != candidate.size_bytes {
        return Err(DuplicateIssue {
            path: candidate.path.clone(),
            code: DuplicateIssueCode::MetadataChanged,
            summary: "File size changed after the scan result was created.".to_string(),
        });
    }

    if root_path_kind == DuplicateRootPathKind::LocalFixed
        && candidate_has_unsafe_local_only_metadata(&metadata)
    {
        return Err(placeholder_hydration_issue(&candidate.path));
    }

    let modified = metadata.modified().map_err(|error| DuplicateIssue {
        path: candidate.path.clone(),
        code: DuplicateIssueCode::ReadError,
        summary: error.to_string(),
    })?;

    let modified_at_millis = system_time_to_millis(modified).map_err(|message| DuplicateIssue {
        path: candidate.path.clone(),
        code: DuplicateIssueCode::ReadError,
        summary: message,
    })?;

    Ok(Some(LiveCandidate {
        path: candidate.path.clone(),
        size_bytes: candidate.size_bytes,
        modified_at_millis,
        modified_at: system_time_to_rfc3339(modified),
    }))
}

fn group_by_size(candidates: Vec<LiveCandidate>) -> HashMap<u64, Vec<LiveCandidate>> {
    let mut groups = HashMap::new();
    for candidate in candidates {
        groups.entry(candidate.size_bytes).or_insert_with(Vec::new).push(candidate);
    }
    groups
}

fn should_use_partial_hash(size_bytes: u64, partial_hash_bytes: usize) -> bool {
    partial_hash_bytes > 0 && size_bytes > partial_hash_bytes as u64
}

fn group_by_partial_hash<F, M>(
    cache_session: &mut DuplicateHashCacheSession,
    candidates: Vec<LiveCandidate>,
    partial_hash_bytes: usize,
    should_cancel: &(impl Fn() -> bool + Sync),
    reporter: &mut DuplicateProgressReporter<'_, F>,
    issues: &mut Vec<DuplicateIssue>,
    scheduling_policy: DuplicateSchedulingPolicy,
    observer: &mut M,
) -> Result<HashMap<String, Vec<LiveCandidate>>, DuplicateAnalysisFailure>
where
    F: FnMut(DuplicateStatusSnapshot),
    M: DuplicateAnalysisMetricsObserver,
{
    let mut groups = HashMap::new();
    let mut misses = Vec::new();

    reporter.set_stage(DuplicateAnalysisStage::PartialHash);
    for candidate in candidates {
        ensure_not_cancelled(should_cancel)?;
        observer.on_partial_hash_candidate();
        let key = candidate.cache_key();
        let partial_hash = cache_session.lookup_partial_hash(&key);
        observer.on_partial_cache_lookup(partial_hash.is_some());
        if let Some(hash) = partial_hash {
            groups.entry(hash).or_insert_with(Vec::new).push(candidate);
            reporter.note_item_processed();
        } else {
            misses.push(candidate);
        }
    }

    let worker_count = effective_worker_count(
        scheduling_policy.partial_hash_worker_limit,
        misses.len(),
    );
    for outcome in hash_partial_candidates(misses, partial_hash_bytes, worker_count, should_cancel) {
        match outcome {
            StageHashOutcome::Hashed(result) => {
                observer.on_partial_hash_bytes_read(result.bytes_read);
                cache_session.record_partial_hash(&result.candidate.cache_key(), &result.hash);
                observer.on_partial_cache_write();
                groups
                    .entry(result.hash)
                    .or_insert_with(Vec::new)
                    .push(result.candidate);
            }
            StageHashOutcome::Issue(issue) => issues.push(issue),
            StageHashOutcome::Cancelled => return Err(DuplicateAnalysisFailure::Cancelled),
        }
        reporter.note_item_processed();
    }

    Ok(groups)
}

fn group_by_full_hash<F, M>(
    cache_session: &mut DuplicateHashCacheSession,
    candidates: Vec<LiveCandidate>,
    should_cancel: &(impl Fn() -> bool + Sync),
    reporter: &mut DuplicateProgressReporter<'_, F>,
    issues: &mut Vec<DuplicateIssue>,
    scheduling_policy: DuplicateSchedulingPolicy,
    observer: &mut M,
) -> Result<HashMap<String, Vec<LiveCandidate>>, DuplicateAnalysisFailure>
where
    F: FnMut(DuplicateStatusSnapshot),
    M: DuplicateAnalysisMetricsObserver,
{
    let mut groups = HashMap::new();
    let mut misses = Vec::new();

    reporter.set_stage(DuplicateAnalysisStage::FullHash);
    for candidate in candidates {
        ensure_not_cancelled(should_cancel)?;
        observer.on_full_hash_candidate();
        let key = candidate.cache_key();
        let full_hash = cache_session.lookup_full_hash(&key);
        observer.on_full_cache_lookup(full_hash.is_some());
        if let Some(hash) = full_hash {
            groups.entry(hash).or_insert_with(Vec::new).push(candidate);
            reporter.note_item_processed();
        } else {
            misses.push(candidate);
        }
    }

    let worker_count = effective_worker_count(
        scheduling_policy.full_hash_worker_limit,
        misses.len(),
    );
    for outcome in hash_full_candidates(misses, worker_count, should_cancel) {
        match outcome {
            StageHashOutcome::Hashed(result) => {
                observer.on_full_hash_bytes_read(result.bytes_read);
                cache_session.record_full_hash(&result.candidate.cache_key(), &result.hash);
                observer.on_full_cache_write();
                groups
                    .entry(result.hash)
                    .or_insert_with(Vec::new)
                    .push(result.candidate);
            }
            StageHashOutcome::Issue(issue) => issues.push(issue),
            StageHashOutcome::Cancelled => return Err(DuplicateAnalysisFailure::Cancelled),
        }
        reporter.note_item_processed();
    }

    Ok(groups)
}

fn hash_file_prefix(
    path: &Path,
    partial_hash_bytes: usize,
    should_cancel: &impl Fn() -> bool,
    buffer: &mut Vec<u8>,
) -> Result<(String, u64), DuplicateCandidateFailure> {
    ensure_candidate_not_cancelled(should_cancel)?;
    let mut file = open_file(path).map_err(DuplicateCandidateFailure::Issue)?;
    buffer.resize(partial_hash_bytes, 0);
    let read = file
        .read(buffer.as_mut_slice())
        .map_err(|error| DuplicateCandidateFailure::Issue(DuplicateIssue {
            path: path.display().to_string(),
            code: DuplicateIssueCode::ReadError,
            summary: error.to_string(),
        }))?;
    buffer.truncate(read);
    Ok((blake3::hash(buffer.as_slice()).to_hex().to_string(), read as u64))
}

fn hash_file_full(
    path: &Path,
    should_cancel: &impl Fn() -> bool,
    buffer: &mut [u8],
) -> Result<(String, u64), DuplicateCandidateFailure> {
    ensure_candidate_not_cancelled(should_cancel)?;
    let mut file = open_file(path).map_err(DuplicateCandidateFailure::Issue)?;
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        DuplicateCandidateFailure::Issue(DuplicateIssue {
            path: path.display().to_string(),
            code: DuplicateIssueCode::ReadError,
            summary: error.to_string(),
        })
    })?;

    let mut hasher = blake3::Hasher::new();
    let mut bytes_read = 0_u64;
    loop {
        ensure_candidate_not_cancelled(should_cancel)?;
        let read = file
            .read(buffer)
            .map_err(|error| DuplicateCandidateFailure::Issue(DuplicateIssue {
                path: path.display().to_string(),
                code: DuplicateIssueCode::ReadError,
                summary: error.to_string(),
            }))?;
        if read == 0 {
            break;
        }
        bytes_read = bytes_read.saturating_add(read as u64);
        hasher.update(&buffer[..read]);
    }

    Ok((hasher.finalize().to_hex().to_string(), bytes_read))
}

fn open_file(path: &Path) -> Result<File, DuplicateIssue> {
    File::open(path).map_err(|error| {
        let code = if error.kind() == std::io::ErrorKind::NotFound {
            DuplicateIssueCode::MissingPath
        } else {
            DuplicateIssueCode::ReadError
        };

        DuplicateIssue {
            path: path.display().to_string(),
            code,
            summary: error.to_string(),
        }
    })
}

fn finalize_groups(groups: Vec<Vec<LiveCandidate>>, analysis_id: &str) -> Vec<DuplicateGroup> {
    let mut result = groups
        .into_iter()
        .map(|mut members| {
            members.sort_by(|left, right| left.path.cmp(&right.path));
            DuplicateGroup {
                group_id: String::new(),
                size_bytes: members[0].size_bytes,
                reclaimable_bytes: members[0]
                    .size_bytes
                    .saturating_mul(members.len().saturating_sub(1) as u64),
                members: members
                    .into_iter()
                    .map(|member| DuplicateGroupMember {
                        path: member.path,
                        size_bytes: member.size_bytes,
                        modified_at: member.modified_at,
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    result.sort_by(|left, right| {
        right
            .reclaimable_bytes
            .cmp(&left.reclaimable_bytes)
            .then_with(|| left.members[0].path.cmp(&right.members[0].path))
    });

    for (index, group) in result.iter_mut().enumerate() {
        group.group_id = format!("{analysis_id}-group-{}", index + 1);
    }

    result
}

fn ensure_not_cancelled(
    should_cancel: &impl Fn() -> bool,
) -> Result<(), DuplicateAnalysisFailure> {
    if should_cancel() {
        return Err(DuplicateAnalysisFailure::Cancelled);
    }

    Ok(())
}

fn ensure_candidate_not_cancelled(
    should_cancel: &impl Fn() -> bool,
) -> Result<(), DuplicateCandidateFailure> {
    if should_cancel() {
        return Err(DuplicateCandidateFailure::Cancelled);
    }

    Ok(())
}

fn system_time_to_millis(value: SystemTime) -> Result<i64, String> {
    let timestamp: DateTime<Utc> = value.into();
    Ok(timestamp.timestamp_millis())
}

fn system_time_to_rfc3339(value: SystemTime) -> String {
    let timestamp: DateTime<Utc> = value.into();
    timestamp.to_rfc3339()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LiveCandidate {
    path: String,
    size_bytes: u64,
    modified_at_millis: i64,
    modified_at: String,
}

impl LiveCandidate {
    fn cache_key(&self) -> HashCacheKey {
        HashCacheKey {
            path: self.path.clone(),
            size_bytes: self.size_bytes,
            modified_at_millis: self.modified_at_millis,
        }
    }
}

#[derive(Debug)]
enum DuplicateCandidateFailure {
    Issue(DuplicateIssue),
    Cancelled,
}

trait DuplicateAnalysisMetricsObserver {
    fn on_validated_candidate(&mut self) {}
    fn on_partial_hash_candidate(&mut self) {}
    fn on_full_hash_candidate(&mut self) {}
    fn on_partial_cache_lookup(&mut self, _hit: bool) {}
    fn on_full_cache_lookup(&mut self, _hit: bool) {}
    fn on_partial_cache_write(&mut self) {}
    fn on_full_cache_write(&mut self) {}
    fn on_partial_hash_bytes_read(&mut self, _bytes_read: u64) {}
    fn on_full_hash_bytes_read(&mut self, _bytes_read: u64) {}
}

struct NoopDuplicateAnalysisMetricsObserver;

impl DuplicateAnalysisMetricsObserver for NoopDuplicateAnalysisMetricsObserver {}

#[derive(Default)]
struct DuplicateAnalysisMeasurementObserver {
    validated_candidate_count: u64,
    partial_hash_candidate_count: u64,
    full_hash_candidate_count: u64,
    partial_cache_lookup_count: u64,
    partial_cache_hit_count: u64,
    partial_cache_miss_count: u64,
    partial_cache_write_count: u64,
    full_cache_lookup_count: u64,
    full_cache_hit_count: u64,
    full_cache_miss_count: u64,
    full_cache_write_count: u64,
    partial_hash_bytes_read: u64,
    full_hash_bytes_read: u64,
}

impl DuplicateAnalysisMetricsObserver for DuplicateAnalysisMeasurementObserver {
    fn on_validated_candidate(&mut self) {
        self.validated_candidate_count = self.validated_candidate_count.saturating_add(1);
    }

    fn on_partial_hash_candidate(&mut self) {
        self.partial_hash_candidate_count = self.partial_hash_candidate_count.saturating_add(1);
    }

    fn on_full_hash_candidate(&mut self) {
        self.full_hash_candidate_count = self.full_hash_candidate_count.saturating_add(1);
    }

    fn on_partial_cache_lookup(&mut self, hit: bool) {
        self.partial_cache_lookup_count = self.partial_cache_lookup_count.saturating_add(1);
        if hit {
            self.partial_cache_hit_count = self.partial_cache_hit_count.saturating_add(1);
        } else {
            self.partial_cache_miss_count = self.partial_cache_miss_count.saturating_add(1);
        }
    }

    fn on_full_cache_lookup(&mut self, hit: bool) {
        self.full_cache_lookup_count = self.full_cache_lookup_count.saturating_add(1);
        if hit {
            self.full_cache_hit_count = self.full_cache_hit_count.saturating_add(1);
        } else {
            self.full_cache_miss_count = self.full_cache_miss_count.saturating_add(1);
        }
    }

    fn on_partial_cache_write(&mut self) {
        self.partial_cache_write_count = self.partial_cache_write_count.saturating_add(1);
    }

    fn on_full_cache_write(&mut self) {
        self.full_cache_write_count = self.full_cache_write_count.saturating_add(1);
    }

    fn on_partial_hash_bytes_read(&mut self, bytes_read: u64) {
        self.partial_hash_bytes_read = self.partial_hash_bytes_read.saturating_add(bytes_read);
    }

    fn on_full_hash_bytes_read(&mut self, bytes_read: u64) {
        self.full_hash_bytes_read = self.full_hash_bytes_read.saturating_add(bytes_read);
    }
}

struct DuplicateHashCacheSession {
    entries: HashMap<HashCacheKey, CachedHashes>,
    pending_writes: HashMap<HashCacheKey, HashCacheWrite>,
}

impl DuplicateHashCacheSession {
    fn preload<C: HashCache>(
        cache: &C,
        candidates: &[LiveCandidate],
        _partial_hash_bytes: usize,
    ) -> Result<Self, DuplicateAnalysisFailure> {
        let mut keys = Vec::with_capacity(candidates.len());
        let mut seen = HashSet::with_capacity(candidates.len());
        for candidate in candidates {
            let key = candidate.cache_key();
            if seen.insert(key.clone()) {
                keys.push(key);
            }
        }

        Ok(Self {
            entries: cache.get_cached_hashes_batch(&keys)?,
            pending_writes: HashMap::new(),
        })
    }

    fn lookup_partial_hash(&self, key: &HashCacheKey) -> Option<String> {
        self.entries.get(key).and_then(|entry| entry.partial_hash.clone())
    }

    fn lookup_full_hash(&self, key: &HashCacheKey) -> Option<String> {
        self.entries.get(key).and_then(|entry| entry.full_hash.clone())
    }

    fn record_partial_hash(&mut self, key: &HashCacheKey, partial_hash: &str) {
        let entry = self.entries.entry(key.clone()).or_insert(CachedHashes {
            partial_hash: None,
            full_hash: None,
        });
        entry.partial_hash = Some(partial_hash.to_string());

        let pending = self
            .pending_writes
            .entry(key.clone())
            .or_insert_with(|| HashCacheWrite {
                key: key.clone(),
                partial_hash: None,
                full_hash: None,
            });
        pending.partial_hash = Some(partial_hash.to_string());
    }

    fn record_full_hash(&mut self, key: &HashCacheKey, full_hash: &str) {
        let entry = self.entries.entry(key.clone()).or_insert(CachedHashes {
            partial_hash: None,
            full_hash: None,
        });
        entry.full_hash = Some(full_hash.to_string());

        let pending = self
            .pending_writes
            .entry(key.clone())
            .or_insert_with(|| HashCacheWrite {
                key: key.clone(),
                partial_hash: None,
                full_hash: None,
            });
        pending.full_hash = Some(full_hash.to_string());
    }

    fn flush_pending_writes<C: HashCache>(
        &mut self,
        cache: &C,
    ) -> Result<(), DuplicateAnalysisFailure> {
        if self.pending_writes.is_empty() {
            return Ok(());
        }

        let mut writes = self.pending_writes.drain().map(|(_, write)| write).collect::<Vec<_>>();
        writes.sort_by(|left, right| {
            left.key
                .path
                .cmp(&right.key.path)
                .then_with(|| left.key.size_bytes.cmp(&right.key.size_bytes))
                .then_with(|| left.key.modified_at_millis.cmp(&right.key.modified_at_millis))
        });
        cache.save_hashes_batch(&writes)
    }
}

struct DuplicateProgressReporter<'a, F>
where
    F: FnMut(DuplicateStatusSnapshot),
{
    snapshot: DuplicateStatusSnapshot,
    on_progress: &'a mut F,
    last_emitted_items_processed: u64,
}

impl<'a, F> DuplicateProgressReporter<'a, F>
where
    F: FnMut(DuplicateStatusSnapshot),
{
    fn new(analysis_id: String, scan_id: String, on_progress: &'a mut F) -> Self {
        Self {
            snapshot: DuplicateStatusSnapshot {
                analysis_id: Some(analysis_id),
                scan_id: Some(scan_id),
                state: DuplicateAnalysisState::Running,
                stage: Some(DuplicateAnalysisStage::Grouping),
                items_processed: 0,
                groups_emitted: 0,
                message: None,
                completed_analysis_id: None,
            },
            on_progress,
            last_emitted_items_processed: 0,
        }
    }

    fn emit(&mut self) {
        self.last_emitted_items_processed = self.snapshot.items_processed;
        (self.on_progress)(self.snapshot.clone());
    }

    fn set_stage(&mut self, stage: DuplicateAnalysisStage) {
        if self.snapshot.stage.as_ref() != Some(&stage) {
            self.snapshot.stage = Some(stage);
            self.emit();
        }
    }

    fn note_item_processed(&mut self) {
        self.snapshot.items_processed += 1;
        if self.snapshot.items_processed <= 4
            || self
                .snapshot
                .items_processed
                .saturating_sub(self.last_emitted_items_processed)
                >= DUPLICATE_PROGRESS_EMIT_INTERVAL
        {
            self.emit();
        }
    }

    fn note_group_emitted(&mut self) {
        self.snapshot.groups_emitted += 1;
        self.emit();
    }

    fn finish(&mut self, analysis_id: &str) {
        self.snapshot.state = DuplicateAnalysisState::Completed;
        self.snapshot.stage = Some(DuplicateAnalysisStage::Completed);
        self.snapshot.completed_analysis_id = Some(analysis_id.to_string());
        self.snapshot.message = Some("Duplicate analysis complete.".to_string());
        self.emit();
    }
}

fn terminal_state_from_result(
    result: &Result<CompletedDuplicateAnalysis, DuplicateAnalysisFailure>,
) -> DuplicateAnalysisState {
    match result {
        Ok(_) => DuplicateAnalysisState::Completed,
        Err(DuplicateAnalysisFailure::Cancelled) => DuplicateAnalysisState::Cancelled,
        Err(_) => DuplicateAnalysisState::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[derive(Default)]
    struct MemoryHashCache {
        entries: RefCell<HashMap<HashCacheKey, CachedHashes>>,
        partial_saves: RefCell<u64>,
        full_saves: RefCell<u64>,
    }

    impl HashCache for MemoryHashCache {
        fn get_cached_hashes(&self, key: &HashCacheKey) -> Result<Option<CachedHashes>, DuplicateAnalysisFailure> {
            Ok(self.entries.borrow().get(key).cloned())
        }

        fn save_partial_hash(&self, key: &HashCacheKey, partial_hash: &str) -> Result<(), DuplicateAnalysisFailure> {
            let mut entries = self.entries.borrow_mut();
            let entry = entries.entry(key.clone()).or_insert(CachedHashes {
                partial_hash: None,
                full_hash: None,
            });
            entry.partial_hash = Some(partial_hash.to_string());
            *self.partial_saves.borrow_mut() += 1;
            Ok(())
        }

        fn save_full_hash(&self, key: &HashCacheKey, full_hash: &str) -> Result<(), DuplicateAnalysisFailure> {
            let mut entries = self.entries.borrow_mut();
            let entry = entries.entry(key.clone()).or_insert(CachedHashes {
                partial_hash: None,
                full_hash: None,
            });
            entry.full_hash = Some(full_hash.to_string());
            *self.full_saves.borrow_mut() += 1;
            Ok(())
        }
    }

    #[derive(Default)]
    struct CountingBatchHashCache {
        entries: RefCell<HashMap<HashCacheKey, CachedHashes>>,
        single_lookup_calls: RefCell<u64>,
        single_partial_save_calls: RefCell<u64>,
        single_full_save_calls: RefCell<u64>,
        batch_lookup_calls: RefCell<u64>,
        batch_save_calls: RefCell<u64>,
    }

    impl HashCache for CountingBatchHashCache {
        fn get_cached_hashes(
            &self,
            key: &HashCacheKey,
        ) -> Result<Option<CachedHashes>, DuplicateAnalysisFailure> {
            *self.single_lookup_calls.borrow_mut() += 1;
            Ok(self.entries.borrow().get(key).cloned())
        }

        fn save_partial_hash(
            &self,
            key: &HashCacheKey,
            partial_hash: &str,
        ) -> Result<(), DuplicateAnalysisFailure> {
            *self.single_partial_save_calls.borrow_mut() += 1;
            let mut entries = self.entries.borrow_mut();
            let entry = entries.entry(key.clone()).or_insert(CachedHashes {
                partial_hash: None,
                full_hash: None,
            });
            entry.partial_hash = Some(partial_hash.to_string());
            Ok(())
        }

        fn save_full_hash(
            &self,
            key: &HashCacheKey,
            full_hash: &str,
        ) -> Result<(), DuplicateAnalysisFailure> {
            *self.single_full_save_calls.borrow_mut() += 1;
            let mut entries = self.entries.borrow_mut();
            let entry = entries.entry(key.clone()).or_insert(CachedHashes {
                partial_hash: None,
                full_hash: None,
            });
            entry.full_hash = Some(full_hash.to_string());
            Ok(())
        }

        fn get_cached_hashes_batch(
            &self,
            keys: &[HashCacheKey],
        ) -> Result<HashMap<HashCacheKey, CachedHashes>, DuplicateAnalysisFailure> {
            *self.batch_lookup_calls.borrow_mut() += 1;
            let entries = self.entries.borrow();
            Ok(keys
                .iter()
                .filter_map(|key| entries.get(key).cloned().map(|value| (key.clone(), value)))
                .collect())
        }

        fn save_hashes_batch(
            &self,
            writes: &[HashCacheWrite],
        ) -> Result<(), DuplicateAnalysisFailure> {
            *self.batch_save_calls.borrow_mut() += 1;
            let mut entries = self.entries.borrow_mut();
            for write in writes {
                let entry = entries.entry(write.key.clone()).or_insert(CachedHashes {
                    partial_hash: None,
                    full_hash: None,
                });
                if let Some(partial_hash) = &write.partial_hash {
                    entry.partial_hash = Some(partial_hash.clone());
                }
                if let Some(full_hash) = &write.full_hash {
                    entry.full_hash = Some(full_hash.clone());
                }
            }
            Ok(())
        }
    }

    #[test]
    fn duplicate_requires_full_hash_confirmation() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let alpha = root.join("alpha.bin");
        let beta = root.join("beta.bin");
        let gamma = root.join("gamma.bin");

        std::fs::write(&alpha, vec![1_u8; 16]).expect("alpha");
        std::fs::write(&beta, vec![1_u8; 16]).expect("beta");
        std::fs::write(&gamma, vec![2_u8; 16]).expect("gamma");

        let request = DuplicateAnalysisRequest::new(
            "scan-1",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: alpha.display().to_string(),
                    size_bytes: 16,
                },
                DuplicateCandidate {
                    path: beta.display().to_string(),
                    size_bytes: 16,
                },
                DuplicateCandidate {
                    path: gamma.display().to_string(),
                    size_bytes: 16,
                },
            ],
        );

        let result =
            analyze_duplicates(&NoopHashCache, &request, || false, |_| {})
                .expect("analysis should succeed");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].members.len(), 2);
        assert!(result.groups[0].members.iter().any(|member| member.path == alpha.display().to_string()));
        assert!(result.groups[0].members.iter().any(|member| member.path == beta.display().to_string()));
        assert!(!result.groups[0].members.iter().any(|member| member.path == gamma.display().to_string()));
    }

    #[test]
    fn excludes_missing_or_changed_files_from_duplicate_groups() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let keep_a = root.join("keep-a.bin");
        let keep_b = root.join("keep-b.bin");
        let changed = root.join("changed.bin");

        std::fs::write(&keep_a, vec![7_u8; 24]).expect("keep_a");
        std::fs::write(&keep_b, vec![7_u8; 24]).expect("keep_b");
        std::fs::write(&changed, vec![7_u8; 24]).expect("changed");

        let request = DuplicateAnalysisRequest::new(
            "scan-1",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: keep_a.display().to_string(),
                    size_bytes: 24,
                },
                DuplicateCandidate {
                    path: keep_b.display().to_string(),
                    size_bytes: 24,
                },
                DuplicateCandidate {
                    path: changed.display().to_string(),
                    size_bytes: 24,
                },
            ],
        );

        std::fs::write(&changed, vec![7_u8; 40]).expect("changed size");

        let result =
            analyze_duplicates(&NoopHashCache, &request, || false, |_| {})
                .expect("analysis should succeed");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].members.len(), 2);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].path, changed.display().to_string());
        assert_eq!(result.issues[0].code, DuplicateIssueCode::MetadataChanged);
    }

    #[test]
    fn reuses_hash_cache_only_for_unchanged_files() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");

        std::fs::write(&left, vec![3_u8; 32]).expect("left");
        std::fs::write(&right, vec![3_u8; 32]).expect("right");

        let cache = MemoryHashCache::default();
        let mut request = DuplicateAnalysisRequest::new(
            "scan-1",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: left.display().to_string(),
                    size_bytes: 32,
                },
                DuplicateCandidate {
                    path: right.display().to_string(),
                    size_bytes: 32,
                },
            ],
        );
        request.analysis_id = Some("analysis-cache".to_string());

        let first =
            analyze_duplicates(&cache, &request, || false, |_| {})
                .expect("first analysis should succeed");
        let first_partial_saves = *cache.partial_saves.borrow();
        let first_full_saves = *cache.full_saves.borrow();

        let second =
            analyze_duplicates(&cache, &request, || false, |_| {})
                .expect("second analysis should succeed");
        assert_eq!(first.groups, second.groups);
        assert_eq!(*cache.partial_saves.borrow(), first_partial_saves);
        assert_eq!(*cache.full_saves.borrow(), first_full_saves);

        std::thread::sleep(std::time::Duration::from_millis(5));
        std::fs::write(&right, vec![4_u8; 32]).expect("right updated");

        let mut changed_request = DuplicateAnalysisRequest::new(
            "scan-1",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: left.display().to_string(),
                    size_bytes: 32,
                },
                DuplicateCandidate {
                    path: right.display().to_string(),
                    size_bytes: 32,
                },
            ],
        );
        changed_request.analysis_id = Some("analysis-cache".to_string());

        let third =
            analyze_duplicates(&cache, &changed_request, || false, |_| {})
                .expect("third analysis should succeed");

        assert!(third.groups.is_empty());
        assert!(*cache.full_saves.borrow() > first_full_saves);
    }

    #[test]
    fn long_duplicate_analysis_progress_is_bounded() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let mut candidates = Vec::new();
        let mut snapshots = Vec::new();

        for index in 0..128 {
            let path = root.join(format!("copy-{index}.bin"));
            std::fs::write(&path, vec![9_u8; 32]).expect("duplicate fixture");
            candidates.push(DuplicateCandidate {
                path: path.display().to_string(),
                size_bytes: 32,
            });
        }

        let request = DuplicateAnalysisRequest::new(
            "scan-progress",
            root.display().to_string(),
            candidates,
        );

        let result = analyze_duplicates(
            &NoopHashCache,
            &request,
            || false,
            |snapshot| snapshots.push(snapshot),
        )
        .expect("analysis should succeed");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].members.len(), 128);
        assert_eq!(
            snapshots.last().expect("completed snapshot").items_processed,
            256
        );
        assert!(
            snapshots.len() < 32,
            "progress emission should stay bounded, got {} snapshots",
            snapshots.len()
        );
    }

    #[test]
    fn duplicate_analysis_can_be_cancelled_before_full_hashing() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");

        std::fs::write(&left, vec![7_u8; 4096]).expect("left");
        std::fs::write(&right, vec![7_u8; 4096]).expect("right");

        let request = DuplicateAnalysisRequest::new(
            "scan-cancel",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: left.display().to_string(),
                    size_bytes: 4096,
                },
                DuplicateCandidate {
                    path: right.display().to_string(),
                    size_bytes: 4096,
                },
            ],
        );

        let cancelled = Arc::new(AtomicBool::new(false));
        let cancel_handle = Arc::clone(&cancelled);

        let result = analyze_duplicates(
            &NoopHashCache,
            &request,
            move || cancel_handle.load(Ordering::SeqCst),
            |snapshot| {
                if snapshot.stage == Some(DuplicateAnalysisStage::FullHash) {
                    cancelled.store(true, Ordering::SeqCst);
                }
            },
        );

        assert_eq!(result, Err(DuplicateAnalysisFailure::Cancelled));
    }

    #[test]
    fn duplicate_analysis_skips_unique_size_candidates_before_validation() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");
        let mut candidates = Vec::new();
        let mut snapshots = Vec::new();

        std::fs::write(&left, vec![5_u8; 4096]).expect("left");
        std::fs::write(&right, vec![5_u8; 4096]).expect("right");
        candidates.push(DuplicateCandidate {
            path: left.display().to_string(),
            size_bytes: 4096,
        });
        candidates.push(DuplicateCandidate {
            path: right.display().to_string(),
            size_bytes: 4096,
        });

        for index in 1..=48 {
            let path = root.join(format!("unique-{index}.bin"));
            std::fs::write(&path, vec![index as u8; index]).expect("unique fixture");
            candidates.push(DuplicateCandidate {
                path: path.display().to_string(),
                size_bytes: index as u64,
            });
        }

        let request = DuplicateAnalysisRequest::new(
            "scan-prefilter",
            root.display().to_string(),
            candidates,
        );

        let result = analyze_duplicates(
            &NoopHashCache,
            &request,
            || false,
            |snapshot| snapshots.push(snapshot),
        )
        .expect("analysis should succeed");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].members.len(), 2);
        assert_eq!(
            snapshots.last().expect("completed snapshot").items_processed,
            4,
            "only repeated-size candidates should be validated and hashed",
        );
    }

    #[test]
    fn duplicate_analysis_batches_cache_preload_and_stage_flushes() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");

        std::fs::write(&left, vec![5_u8; 128]).expect("left");
        std::fs::write(&right, vec![5_u8; 128]).expect("right");

        let cache = CountingBatchHashCache::default();
        let mut request = DuplicateAnalysisRequest::new(
            "scan-batch",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: left.display().to_string(),
                    size_bytes: 128,
                },
                DuplicateCandidate {
                    path: right.display().to_string(),
                    size_bytes: 128,
                },
            ],
        );
        request.partial_hash_bytes = 32;

        let result =
            analyze_duplicates(&cache, &request, || false, |_| {}).expect("analysis should succeed");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(*cache.batch_lookup_calls.borrow(), 1);
        assert_eq!(*cache.single_lookup_calls.borrow(), 0);
        assert_eq!(*cache.batch_save_calls.borrow(), 2);
        assert_eq!(*cache.single_partial_save_calls.borrow(), 0);
        assert_eq!(*cache.single_full_save_calls.borrow(), 0);
    }

    #[test]
    fn duplicate_analysis_uses_preloaded_cache_for_warm_rerun() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");

        std::fs::write(&left, vec![6_u8; 128]).expect("left");
        std::fs::write(&right, vec![6_u8; 128]).expect("right");

        let cache = CountingBatchHashCache::default();
        let mut request = DuplicateAnalysisRequest::new(
            "scan-batch-warm",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: left.display().to_string(),
                    size_bytes: 128,
                },
                DuplicateCandidate {
                    path: right.display().to_string(),
                    size_bytes: 128,
                },
            ],
        );
        request.partial_hash_bytes = 32;

        analyze_duplicates(&cache, &request, || false, |_| {}).expect("cold run should succeed");
        *cache.batch_lookup_calls.borrow_mut() = 0;
        *cache.batch_save_calls.borrow_mut() = 0;

        let result =
            analyze_duplicates(&cache, &request, || false, |_| {}).expect("warm run should succeed");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(*cache.batch_lookup_calls.borrow(), 1);
        assert_eq!(*cache.batch_save_calls.borrow(), 0);
        assert_eq!(*cache.single_lookup_calls.borrow(), 0);
        assert_eq!(*cache.single_partial_save_calls.borrow(), 0);
        assert_eq!(*cache.single_full_save_calls.borrow(), 0);
    }

    #[test]
    fn local_fixed_hash_policy_prefers_bounded_parallelism() {
        let policy = scheduling_policy_for_root_kind(DuplicateRootPathKind::LocalFixed, 8);

        assert!(policy.allow_content_hashing);
        assert_eq!(policy.partial_hash_worker_limit, 2);
        assert_eq!(policy.full_hash_worker_limit, 4);
    }

    #[test]
    fn remote_root_duplicate_analysis_reports_local_only_issues() {
        let request = DuplicateAnalysisRequest::new(
            "scan-remote",
            "\\\\server\\share".to_string(),
            vec![
                DuplicateCandidate {
                    path: "\\\\server\\share\\left.bin".to_string(),
                    size_bytes: 128,
                },
                DuplicateCandidate {
                    path: "\\\\server\\share\\right.bin".to_string(),
                    size_bytes: 128,
                },
            ],
        );

        let result =
            analyze_duplicates(&NoopHashCache, &request, || false, |_| {}).expect("analysis should succeed");

        assert!(result.groups.is_empty());
        assert_eq!(result.issues.len(), 2);
        assert!(result
            .issues
            .iter()
            .all(|issue| issue.summary.contains("local-only")));
    }

    #[cfg(windows)]
    #[test]
    fn placeholder_style_attributes_require_local_only_skip() {
        assert!(has_unsafe_local_only_file_attributes(FILE_ATTRIBUTE_OFFLINE));
        assert!(has_unsafe_local_only_file_attributes(
            FILE_ATTRIBUTE_RECALL_ON_OPEN
        ));
        assert!(has_unsafe_local_only_file_attributes(
            FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS
        ));
        assert!(!has_unsafe_local_only_file_attributes(0));
    }

    #[test]
    fn measured_duplicate_analysis_reports_stage_cache_and_byte_metrics() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");

        std::fs::write(&left, vec![4_u8; 128]).expect("left");
        std::fs::write(&right, vec![4_u8; 128]).expect("right");

        let cache = MemoryHashCache::default();
        let mut request = DuplicateAnalysisRequest::new(
            "scan-measure",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: left.display().to_string(),
                    size_bytes: 128,
                },
                DuplicateCandidate {
                    path: right.display().to_string(),
                    size_bytes: 128,
                },
            ],
        );
        request.partial_hash_bytes = 32;

        let measured = measure_duplicate_analysis(&cache, &request, || false);

        let result = measured.result.expect("analysis should succeed");
        assert_eq!(result.groups.len(), 1);
        assert_eq!(measured.measurement.validated_candidate_count, 2);
        assert_eq!(measured.measurement.partial_hash_candidate_count, 2);
        assert_eq!(measured.measurement.full_hash_candidate_count, 2);
        assert_eq!(measured.measurement.partial_hash_bytes_read, 64);
        assert_eq!(measured.measurement.full_hash_bytes_read, 256);
        assert_eq!(measured.measurement.cache_lookup_count, 4);
        assert_eq!(measured.measurement.cache_hit_count, 0);
        assert_eq!(measured.measurement.cache_miss_count, 4);
        assert_eq!(measured.measurement.cache_write_count, 4);
        assert_eq!(measured.measurement.partial_cache_lookup_count, 2);
        assert_eq!(measured.measurement.partial_cache_hit_count, 0);
        assert_eq!(measured.measurement.partial_cache_miss_count, 2);
        assert_eq!(measured.measurement.partial_cache_write_count, 2);
        assert_eq!(measured.measurement.full_cache_lookup_count, 2);
        assert_eq!(measured.measurement.full_cache_hit_count, 0);
        assert_eq!(measured.measurement.full_cache_miss_count, 2);
        assert_eq!(measured.measurement.full_cache_write_count, 2);
        assert!(measured.measurement.progress_event_count >= 3);
        assert_eq!(measured.measurement.cancel_to_stop_millis, None);
        assert_eq!(
            measured.measurement.terminal_state,
            DuplicateAnalysisState::Completed
        );
    }

    #[test]
    fn measured_duplicate_analysis_reports_cache_hits_on_warm_rerun() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");

        std::fs::write(&left, vec![8_u8; 128]).expect("left");
        std::fs::write(&right, vec![8_u8; 128]).expect("right");

        let cache = MemoryHashCache::default();
        let mut request = DuplicateAnalysisRequest::new(
            "scan-warm",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: left.display().to_string(),
                    size_bytes: 128,
                },
                DuplicateCandidate {
                    path: right.display().to_string(),
                    size_bytes: 128,
                },
            ],
        );
        request.partial_hash_bytes = 32;

        let cold = measure_duplicate_analysis(&cache, &request, || false);
        assert!(cold.result.is_ok(), "cold run should succeed");

        let warm = measure_duplicate_analysis(&cache, &request, || false);

        let result = warm.result.expect("warm analysis should succeed");
        assert_eq!(result.groups.len(), 1);
        assert_eq!(warm.measurement.validated_candidate_count, 2);
        assert_eq!(warm.measurement.partial_hash_candidate_count, 2);
        assert_eq!(warm.measurement.full_hash_candidate_count, 2);
        assert_eq!(warm.measurement.partial_hash_bytes_read, 0);
        assert_eq!(warm.measurement.full_hash_bytes_read, 0);
        assert_eq!(warm.measurement.cache_lookup_count, 4);
        assert_eq!(warm.measurement.cache_hit_count, 4);
        assert_eq!(warm.measurement.cache_miss_count, 0);
        assert_eq!(warm.measurement.cache_write_count, 0);
        assert_eq!(warm.measurement.partial_cache_hit_count, 2);
        assert_eq!(warm.measurement.full_cache_hit_count, 2);
        assert_eq!(
            warm.measurement.terminal_state,
            DuplicateAnalysisState::Completed
        );
    }

    #[test]
    fn measured_duplicate_analysis_reports_cancel_to_stop_latency() {
        let fixture = tempdir().expect("fixture directory");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");

        std::fs::write(&left, vec![6_u8; 256 * 1024]).expect("left");
        std::fs::write(&right, vec![6_u8; 256 * 1024]).expect("right");

        let mut request = DuplicateAnalysisRequest::new(
            "scan-cancel-measure",
            root.display().to_string(),
            vec![
                DuplicateCandidate {
                    path: left.display().to_string(),
                    size_bytes: 256 * 1024,
                },
                DuplicateCandidate {
                    path: right.display().to_string(),
                    size_bytes: 256 * 1024,
                },
            ],
        );
        request.partial_hash_bytes = 64 * 1024;

        let checks = Arc::new(AtomicUsize::new(0));
        let check_handle = Arc::clone(&checks);

        let measured = measure_duplicate_analysis(&NoopHashCache, &request, move || {
            check_handle.fetch_add(1, Ordering::SeqCst) >= 8
        });

        assert_eq!(measured.result, Err(DuplicateAnalysisFailure::Cancelled));
        assert!(measured.measurement.cancel_to_stop_millis.is_some());
        assert_eq!(
            measured.measurement.terminal_state,
            DuplicateAnalysisState::Cancelled
        );
    }
}
