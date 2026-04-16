use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

pub const DEFAULT_PARTIAL_HASH_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateAnalysisState {
    Idle,
    Running,
    Completed,
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

pub trait HashCache {
    fn get_cached_hashes(&self, key: &HashCacheKey) -> Result<Option<CachedHashes>, DuplicateAnalysisFailure>;
    fn save_partial_hash(&self, key: &HashCacheKey, partial_hash: &str) -> Result<(), DuplicateAnalysisFailure>;
    fn save_full_hash(&self, key: &HashCacheKey, full_hash: &str) -> Result<(), DuplicateAnalysisFailure>;
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
    #[error("duplicate analysis failed: {message}")]
    Internal { message: String },
}

pub fn make_analysis_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

pub fn analyze_duplicates<C, F>(
    cache: &C,
    request: &DuplicateAnalysisRequest,
    mut on_progress: F,
) -> Result<CompletedDuplicateAnalysis, DuplicateAnalysisFailure>
where
    C: HashCache,
    F: FnMut(DuplicateStatusSnapshot),
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
    reporter.emit();

    let mut issues = Vec::new();
    let duplicate_size_candidates = duplicate_size_candidates(&request.candidates);
    let validated_candidates =
        validate_candidates(&duplicate_size_candidates, &mut reporter, &mut issues)?;
    let mut groups = Vec::new();

    for size_group in group_by_size(validated_candidates).into_values() {
        if size_group.len() < 2 {
            continue;
        }

        let partial_groups = if should_use_partial_hash(size_group[0].size_bytes, request.partial_hash_bytes) {
            group_by_partial_hash(cache, size_group, request.partial_hash_bytes, &mut reporter, &mut issues)?
                .into_values()
                .collect::<Vec<_>>()
        } else {
            vec![size_group]
        };

        for partial_group in partial_groups {
            if partial_group.len() < 2 {
                continue;
            }

            for verified_group in group_by_full_hash(cache, partial_group, &mut reporter, &mut issues)?
                .into_values()
            {
                if verified_group.len() < 2 {
                    continue;
                }

                groups.push(verified_group);
                reporter.note_group_emitted();
            }
        }
    }

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

fn validate_candidates<F>(
    candidates: &[DuplicateCandidate],
    reporter: &mut DuplicateProgressReporter<'_, F>,
    issues: &mut Vec<DuplicateIssue>,
) -> Result<Vec<LiveCandidate>, DuplicateAnalysisFailure>
where
    F: FnMut(DuplicateStatusSnapshot),
{
    let mut unique_paths = HashSet::new();
    let mut validated = Vec::new();

    for candidate in candidates {
        reporter.set_stage(DuplicateAnalysisStage::Grouping);

        if !unique_paths.insert(candidate.path.clone()) {
            reporter.note_item_processed();
            continue;
        }

        match validate_candidate(candidate) {
            Ok(Some(live_candidate)) => validated.push(live_candidate),
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

fn validate_candidate(candidate: &DuplicateCandidate) -> Result<Option<LiveCandidate>, DuplicateIssue> {
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

fn group_by_partial_hash<C, F>(
    cache: &C,
    candidates: Vec<LiveCandidate>,
    partial_hash_bytes: usize,
    reporter: &mut DuplicateProgressReporter<'_, F>,
    issues: &mut Vec<DuplicateIssue>,
) -> Result<HashMap<String, Vec<LiveCandidate>>, DuplicateAnalysisFailure>
where
    C: HashCache,
    F: FnMut(DuplicateStatusSnapshot),
{
    let mut groups = HashMap::new();

    for candidate in candidates {
        reporter.set_stage(DuplicateAnalysisStage::PartialHash);
        match resolve_partial_hash(cache, &candidate, partial_hash_bytes) {
            Ok(hash) => {
                groups.entry(hash).or_insert_with(Vec::new).push(candidate);
            }
            Err(issue) => issues.push(issue),
        }
        reporter.note_item_processed();
    }

    Ok(groups)
}

fn group_by_full_hash<C, F>(
    cache: &C,
    candidates: Vec<LiveCandidate>,
    reporter: &mut DuplicateProgressReporter<'_, F>,
    issues: &mut Vec<DuplicateIssue>,
) -> Result<HashMap<String, Vec<LiveCandidate>>, DuplicateAnalysisFailure>
where
    C: HashCache,
    F: FnMut(DuplicateStatusSnapshot),
{
    let mut groups = HashMap::new();

    for candidate in candidates {
        reporter.set_stage(DuplicateAnalysisStage::FullHash);
        match resolve_full_hash(cache, &candidate) {
            Ok(hash) => {
                groups.entry(hash).or_insert_with(Vec::new).push(candidate);
            }
            Err(issue) => issues.push(issue),
        }
        reporter.note_item_processed();
    }

    Ok(groups)
}

fn resolve_partial_hash<C>(
    cache: &C,
    candidate: &LiveCandidate,
    partial_hash_bytes: usize,
) -> Result<String, DuplicateIssue>
where
    C: HashCache,
{
    let key = candidate.cache_key();
    let cached = cache
        .get_cached_hashes(&key)
        .map_err(|error| duplicate_issue_from_failure(candidate, error))?;
    if let Some(hash) = cached.and_then(|entry| entry.partial_hash) {
        return Ok(hash);
    }

    let hash = hash_file_prefix(Path::new(&candidate.path), partial_hash_bytes)?;
    cache
        .save_partial_hash(&key, &hash)
        .map_err(|error| duplicate_issue_from_failure(candidate, error))?;
    Ok(hash)
}

fn resolve_full_hash<C>(cache: &C, candidate: &LiveCandidate) -> Result<String, DuplicateIssue>
where
    C: HashCache,
{
    let key = candidate.cache_key();
    let cached = cache
        .get_cached_hashes(&key)
        .map_err(|error| duplicate_issue_from_failure(candidate, error))?;
    if let Some(hash) = cached.and_then(|entry| entry.full_hash) {
        return Ok(hash);
    }

    let hash = hash_file_full(Path::new(&candidate.path))?;
    cache
        .save_full_hash(&key, &hash)
        .map_err(|error| duplicate_issue_from_failure(candidate, error))?;
    Ok(hash)
}

fn hash_file_prefix(path: &Path, partial_hash_bytes: usize) -> Result<String, DuplicateIssue> {
    let mut file = open_file(path)?;
    let mut buffer = vec![0_u8; partial_hash_bytes];
    let read = file.read(&mut buffer).map_err(|error| DuplicateIssue {
        path: path.display().to_string(),
        code: DuplicateIssueCode::ReadError,
        summary: error.to_string(),
    })?;
    buffer.truncate(read);
    Ok(blake3::hash(&buffer).to_hex().to_string())
}

fn hash_file_full(path: &Path) -> Result<String, DuplicateIssue> {
    let mut file = open_file(path)?;
    file.seek(SeekFrom::Start(0)).map_err(|error| DuplicateIssue {
        path: path.display().to_string(),
        code: DuplicateIssueCode::ReadError,
        summary: error.to_string(),
    })?;

    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(|error| DuplicateIssue {
            path: path.display().to_string(),
            code: DuplicateIssueCode::ReadError,
            summary: error.to_string(),
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
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

fn duplicate_issue_from_failure(
    candidate: &LiveCandidate,
    error: DuplicateAnalysisFailure,
) -> DuplicateIssue {
    DuplicateIssue {
        path: candidate.path.clone(),
        code: DuplicateIssueCode::ReadError,
        summary: error.to_string(),
    }
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

struct DuplicateProgressReporter<'a, F>
where
    F: FnMut(DuplicateStatusSnapshot),
{
    snapshot: DuplicateStatusSnapshot,
    on_progress: &'a mut F,
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
        }
    }

    fn emit(&mut self) {
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
        self.emit();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
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

        let result = analyze_duplicates(&NoopHashCache, &request, |_| {}).expect("analysis should succeed");

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

        let result = analyze_duplicates(&NoopHashCache, &request, |_| {}).expect("analysis should succeed");

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

        let first = analyze_duplicates(&cache, &request, |_| {}).expect("first analysis should succeed");
        let first_partial_saves = *cache.partial_saves.borrow();
        let first_full_saves = *cache.full_saves.borrow();

        let second = analyze_duplicates(&cache, &request, |_| {}).expect("second analysis should succeed");
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

        let third = analyze_duplicates(&cache, &changed_request, |_| {}).expect("third analysis should succeed");

        assert!(third.groups.is_empty());
        assert!(*cache.full_saves.borrow() > first_full_saves);
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

        let result = analyze_duplicates(&NoopHashCache, &request, |snapshot| {
            snapshots.push(snapshot);
        })
        .expect("analysis should succeed");

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].members.len(), 2);
        assert_eq!(
            snapshots.last().expect("completed snapshot").items_processed,
            4,
            "only repeated-size candidates should be validated and hashed",
        );
    }
}
