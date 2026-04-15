use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

const DUPLICATE_SOURCE_LABEL: &str = "Duplicate selection";
const BUILTIN_RULE_SPECS: [(&str, &str); 2] = [
    (
        "temp-folder-files.toml",
        include_str!("../../../../src/config/cleanup-rules/temp-folder-files.toml"),
    ),
    (
        "download-partials.toml",
        include_str!("../../../../src/config/cleanup-rules/download-partials.toml"),
    ),
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupRuleDefinition {
    pub rule_id: String,
    pub label: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CleanupIssueCode {
    MissingPath,
    NotAFile,
    OutsideRoot,
    MetadataChanged,
    NotInScan,
    ReadError,
    RequiresElevation,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupIssue {
    pub path: String,
    pub code: CleanupIssueCode,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreviewCandidate {
    pub action_id: String,
    pub path: String,
    pub size_bytes: u64,
    pub source_labels: Vec<String>,
    #[serde(skip_serializing, skip_deserializing, default)]
    pub expected_modified_at_millis: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPreview {
    pub preview_id: String,
    pub scan_id: String,
    pub root_path: String,
    pub generated_at: String,
    pub total_bytes: u64,
    pub duplicate_candidate_count: u64,
    pub rule_candidate_count: u64,
    pub candidates: Vec<CleanupPreviewCandidate>,
    pub issues: Vec<CleanupIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupFileEntry {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CleanupPreviewRequest {
    pub preview_id: Option<String>,
    pub scan_id: String,
    pub root_path: String,
    pub file_entries: Vec<CleanupFileEntry>,
    pub duplicate_delete_paths: Vec<String>,
    pub enabled_rule_ids: Vec<String>,
}

impl CleanupPreviewRequest {
    pub fn new(
        scan_id: impl Into<String>,
        root_path: impl Into<String>,
        file_entries: Vec<CleanupFileEntry>,
    ) -> Self {
        Self {
            preview_id: None,
            scan_id: scan_id.into(),
            root_path: root_path.into(),
            file_entries,
            duplicate_delete_paths: Vec::new(),
            enabled_rule_ids: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupExecutionMode {
    Recycle,
    Permanent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupExecutionItemStatus {
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupExecutionEntry {
    pub action_id: String,
    pub path: String,
    pub status: CleanupExecutionItemStatus,
    pub summary: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupExecutionResult {
    pub execution_id: String,
    pub preview_id: String,
    pub mode: CleanupExecutionMode,
    pub completed_at: String,
    pub completed_count: u64,
    pub failed_count: u64,
    pub entries: Vec<CleanupExecutionEntry>,
}

pub trait CleanupExecutor {
    fn recycle(&self, path: &Path) -> Result<(), String>;
    fn permanent_delete(&self, path: &Path) -> Result<(), String>;
}

#[derive(Default)]
pub struct SystemCleanupExecutor;

impl CleanupExecutor for SystemCleanupExecutor {
    fn recycle(&self, path: &Path) -> Result<(), String> {
        trash::delete(path).map_err(|error| error.to_string())
    }

    fn permanent_delete(&self, path: &Path) -> Result<(), String> {
        std::fs::remove_file(path).map_err(|error| error.to_string())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CleanupFailure {
    #[error("cleanup request is invalid: {message}")]
    InvalidRequest { message: String },
    #[error("cleanup failed: {message}")]
    Internal { message: String },
}

pub fn make_preview_id() -> String {
    format!("preview-{}", Uuid::new_v4())
}

pub fn make_execution_id() -> String {
    format!("execution-{}", Uuid::new_v4())
}

pub fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

pub fn list_cleanup_rules() -> Result<Vec<CleanupRuleDefinition>, CleanupFailure> {
    load_builtin_rules().map(|rules| {
        rules.into_iter()
            .map(|rule| CleanupRuleDefinition {
                rule_id: rule.rule_id,
                label: rule.label,
                description: rule.description,
            })
            .collect()
    })
}

pub fn build_cleanup_preview(
    request: &CleanupPreviewRequest,
) -> Result<CleanupPreview, CleanupFailure> {
    let scan_id = request.scan_id.trim().to_string();
    let root_path = request.root_path.trim().to_string();
    if scan_id.is_empty() || root_path.is_empty() {
        return Err(CleanupFailure::InvalidRequest {
            message: "cleanup preview requires a scan identifier and root path".to_string(),
        });
    }

    if request.file_entries.is_empty() {
        return Err(CleanupFailure::InvalidRequest {
            message: "A fresh scan is required before cleanup preview.".to_string(),
        });
    }

    if request.duplicate_delete_paths.is_empty() && request.enabled_rule_ids.is_empty() {
        return Err(CleanupFailure::InvalidRequest {
            message: "Select at least one cleanup source before previewing.".to_string(),
        });
    }

    let root_key = normalize_path(&root_path);
    let builtin_rules = load_builtin_rules()?;
    let enabled_rules = resolve_enabled_rules(&builtin_rules, &request.enabled_rule_ids)?;
    let entry_by_path = request
        .file_entries
        .iter()
        .map(|entry| (normalize_path(&entry.path), entry))
        .collect::<BTreeMap<_, _>>();

    let mut issues = Vec::new();
    let mut pending = BTreeMap::<String, PendingPreviewCandidate>::new();

    for duplicate_path in dedupe_paths(&request.duplicate_delete_paths) {
        let normalized = normalize_path(&duplicate_path);
        if !is_path_within_root(&normalized, &root_key) {
            issues.push(issue(
                duplicate_path,
                CleanupIssueCode::OutsideRoot,
                "Path is outside the current scan root.",
            ));
            continue;
        }

        let Some(entry) = entry_by_path.get(&normalized) else {
            issues.push(issue(
                duplicate_path,
                CleanupIssueCode::NotInScan,
                "Path is not present in the loaded scan result.",
            ));
            continue;
        };

        let candidate = pending
            .entry(normalized)
            .or_insert_with(|| PendingPreviewCandidate::from_entry(entry));
        candidate.add_source_label(DUPLICATE_SOURCE_LABEL);
        candidate.matched_duplicate = true;
    }

    for rule in enabled_rules {
        for entry in &request.file_entries {
            let normalized = normalize_path(&entry.path);
            if !is_path_within_root(&normalized, &root_key) {
                continue;
            }

            let Some(relative_path) = relative_path_from_root(&entry.path, &root_path) else {
                continue;
            };
            if !rule.matches(relative_path) {
                continue;
            }

            let candidate = pending
                .entry(normalized)
                .or_insert_with(|| PendingPreviewCandidate::from_entry(entry));
            candidate.add_source_label(&rule.label);
            candidate.matched_rule = true;
        }
    }

    let mut candidates = Vec::new();
    for pending_candidate in pending.into_values() {
        match validate_live_candidate(&root_key, pending_candidate) {
            Ok(candidate) => candidates.push(candidate),
            Err(next_issue) => issues.push(next_issue),
        }
    }

    candidates.sort_by(|left, right| left.path.cmp(&right.path));
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.action_id = format!("cleanup-action-{}", index + 1);
    }

    issues.sort_by(|left, right| left.path.cmp(&right.path).then_with(|| left.code.cmp(&right.code)));
    let total_bytes = candidates.iter().map(|candidate| candidate.size_bytes).sum();
    let duplicate_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.source_labels.iter().any(|label| label == DUPLICATE_SOURCE_LABEL))
        .count() as u64;
    let rule_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.source_labels.iter().any(|label| label != DUPLICATE_SOURCE_LABEL))
        .count() as u64;

    Ok(CleanupPreview {
        preview_id: request.preview_id.clone().unwrap_or_else(make_preview_id),
        scan_id,
        root_path,
        generated_at: current_timestamp(),
        total_bytes,
        duplicate_candidate_count,
        rule_candidate_count,
        candidates,
        issues,
    })
}

pub fn execute_cleanup<E>(
    executor: &E,
    preview: &CleanupPreview,
    action_ids: &[String],
    mode: CleanupExecutionMode,
) -> Result<CleanupExecutionResult, CleanupFailure>
where
    E: CleanupExecutor,
{
    if preview.preview_id.trim().is_empty() {
        return Err(CleanupFailure::InvalidRequest {
            message: "cleanup execution requires a preview identifier".to_string(),
        });
    }

    if action_ids.is_empty() {
        return Err(CleanupFailure::InvalidRequest {
            message: "Select at least one cleanup candidate before executing cleanup.".to_string(),
        });
    }

    let requested = action_ids.iter().cloned().collect::<HashSet<_>>();
    let selected = preview
        .candidates
        .iter()
        .filter(|candidate| requested.contains(&candidate.action_id))
        .cloned()
        .collect::<Vec<_>>();

    if selected.len() != requested.len() {
        return Err(CleanupFailure::InvalidRequest {
            message: "Cleanup execution can only target candidates from the active preview.".to_string(),
        });
    }

    let root_key = normalize_path(&preview.root_path);
    let mut entries = Vec::new();
    let mut completed_count = 0_u64;
    let mut failed_count = 0_u64;

    for candidate in selected {
        let validation_error = revalidate_candidate(&root_key, &candidate).err();
        if let Some(error) = validation_error {
            failed_count += 1;
            entries.push(CleanupExecutionEntry {
                action_id: candidate.action_id,
                path: candidate.path,
                status: CleanupExecutionItemStatus::Failed,
                summary: error.summary,
            });
            continue;
        }

        let path = Path::new(&candidate.path);
        let result = match mode {
            CleanupExecutionMode::Recycle => executor.recycle(path),
            CleanupExecutionMode::Permanent => executor.permanent_delete(path),
        };

        match result {
            Ok(()) => {
                completed_count += 1;
                entries.push(CleanupExecutionEntry {
                    action_id: candidate.action_id,
                    path: candidate.path,
                    status: CleanupExecutionItemStatus::Completed,
                    summary: match mode {
                        CleanupExecutionMode::Recycle => "Moved to the Recycle Bin.".to_string(),
                        CleanupExecutionMode::Permanent => "Permanently deleted.".to_string(),
                    },
                });
            }
            Err(error) => {
                failed_count += 1;
                entries.push(CleanupExecutionEntry {
                    action_id: candidate.action_id,
                    path: candidate.path,
                    status: CleanupExecutionItemStatus::Failed,
                    summary: error,
                });
            }
        }
    }

    Ok(CleanupExecutionResult {
        execution_id: make_execution_id(),
        preview_id: preview.preview_id.clone(),
        mode,
        completed_at: current_timestamp(),
        completed_count,
        failed_count,
        entries,
    })
}

#[derive(Clone, Debug)]
struct BuiltinCleanupRule {
    rule_id: String,
    label: String,
    description: String,
    matcher: RuleMatcher,
}

impl BuiltinCleanupRule {
    fn matches(&self, relative_path: &str) -> bool {
        self.matcher.matches(relative_path)
    }
}

#[derive(Clone, Debug)]
enum RuleMatcher {
    PathSegment(Vec<String>),
    Extension(Vec<String>),
}

impl RuleMatcher {
    fn matches(&self, path: &str) -> bool {
        match self {
            Self::PathSegment(values) => {
                let segments = path
                    .split(['\\', '/'])
                    .filter(|segment| !segment.is_empty())
                    .map(|segment| segment.to_lowercase())
                    .collect::<Vec<_>>();
                values.iter().any(|value| segments.iter().any(|segment| segment == value))
            }
            Self::Extension(values) => Path::new(path)
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_lowercase())
                .map(|extension| values.iter().any(|value| value == &extension))
                .unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
struct RawRuleFile {
    id: String,
    label: String,
    description: String,
    matcher: String,
    values: Vec<String>,
}

#[derive(Clone, Debug)]
struct PendingPreviewCandidate {
    path: String,
    size_bytes: u64,
    source_labels: Vec<String>,
    matched_duplicate: bool,
    matched_rule: bool,
}

impl PendingPreviewCandidate {
    fn from_entry(entry: &CleanupFileEntry) -> Self {
        Self {
            path: entry.path.clone(),
            size_bytes: entry.size_bytes,
            source_labels: Vec::new(),
            matched_duplicate: false,
            matched_rule: false,
        }
    }

    fn add_source_label(&mut self, label: &str) {
        if !self.source_labels.iter().any(|current| current == label) {
            self.source_labels.push(label.to_string());
        }
    }
}

fn load_builtin_rules() -> Result<Vec<BuiltinCleanupRule>, CleanupFailure> {
    BUILTIN_RULE_SPECS
        .into_iter()
        .map(|(file_name, raw_rule)| parse_builtin_rule(file_name, raw_rule))
        .collect()
}

fn parse_builtin_rule(file_name: &str, raw_rule: &str) -> Result<BuiltinCleanupRule, CleanupFailure> {
    let parsed = toml::from_str::<RawRuleFile>(raw_rule).map_err(|error| CleanupFailure::Internal {
        message: format!("failed to parse built-in cleanup rule {file_name}: {error}"),
    })?;
    let values = parsed
        .values
        .into_iter()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(CleanupFailure::Internal {
            message: format!("built-in cleanup rule {file_name} has no matcher values"),
        });
    }

    let matcher = match parsed.matcher.trim().to_lowercase().as_str() {
        "path_segment" => RuleMatcher::PathSegment(values),
        "extension" => RuleMatcher::Extension(values),
        other => {
            return Err(CleanupFailure::Internal {
                message: format!("built-in cleanup rule {file_name} uses unsupported matcher {other}"),
            })
        }
    };

    Ok(BuiltinCleanupRule {
        rule_id: parsed.id,
        label: parsed.label,
        description: parsed.description,
        matcher,
    })
}

fn resolve_enabled_rules(
    builtin_rules: &[BuiltinCleanupRule],
    enabled_rule_ids: &[String],
) -> Result<Vec<BuiltinCleanupRule>, CleanupFailure> {
    let enabled = enabled_rule_ids
        .iter()
        .map(|rule_id| rule_id.trim())
        .filter(|rule_id| !rule_id.is_empty())
        .collect::<Vec<_>>();

    let mut resolved = Vec::new();
    for rule_id in enabled {
        let Some(rule) = builtin_rules.iter().find(|rule| rule.rule_id == rule_id) else {
            return Err(CleanupFailure::InvalidRequest {
                message: format!("Unsupported cleanup rule: {rule_id}"),
            });
        };
        resolved.push(rule.clone());
    }

    Ok(resolved)
}

fn dedupe_paths(paths: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for path in paths {
        let normalized = normalize_path(path);
        if !normalized.is_empty() && seen.insert(normalized) {
            result.push(path.clone());
        }
    }
    result
}

fn validate_live_candidate(
    root_key: &str,
    pending: PendingPreviewCandidate,
) -> Result<CleanupPreviewCandidate, CleanupIssue> {
    let normalized = normalize_path(&pending.path);
    if !is_path_within_root(&normalized, root_key) {
        return Err(issue(
            pending.path,
            CleanupIssueCode::OutsideRoot,
            "Path is outside the current scan root.",
        ));
    }

    if elevation_helper::requires_elevation(&pending.path) {
        return Err(issue(
            pending.path,
            CleanupIssueCode::RequiresElevation,
            "Path requires elevation and is excluded from the unprivileged cleanup flow.",
        ));
    }

    let metadata = std::fs::metadata(&pending.path).map_err(|error| {
        issue(
            pending.path.clone(),
            if error.kind() == std::io::ErrorKind::NotFound {
                CleanupIssueCode::MissingPath
            } else {
                CleanupIssueCode::ReadError
            },
            error.to_string(),
        )
    })?;

    if !metadata.is_file() {
        return Err(issue(
            pending.path,
            CleanupIssueCode::NotAFile,
            "Path is no longer a regular file.",
        ));
    }

    if metadata.len() != pending.size_bytes {
        return Err(issue(
            pending.path,
            CleanupIssueCode::MetadataChanged,
            "File size changed after the scan result was created.",
        ));
    }

    let modified_at_millis = system_time_to_millis(
        metadata
            .modified()
            .map_err(|error| issue(pending.path.clone(), CleanupIssueCode::ReadError, error.to_string()))?,
    )
    .map_err(|message| issue(pending.path.clone(), CleanupIssueCode::ReadError, message))?;

    Ok(CleanupPreviewCandidate {
        action_id: String::new(),
        path: pending.path,
        size_bytes: pending.size_bytes,
        source_labels: pending.source_labels,
        expected_modified_at_millis: modified_at_millis,
    })
}

fn revalidate_candidate(root_key: &str, candidate: &CleanupPreviewCandidate) -> Result<(), CleanupIssue> {
    let normalized = normalize_path(&candidate.path);
    if !is_path_within_root(&normalized, root_key) {
        return Err(issue(
            candidate.path.clone(),
            CleanupIssueCode::OutsideRoot,
            "Path is outside the current scan root.",
        ));
    }

    let metadata = std::fs::metadata(&candidate.path).map_err(|error| {
        issue(
            candidate.path.clone(),
            if error.kind() == std::io::ErrorKind::NotFound {
                CleanupIssueCode::MissingPath
            } else {
                CleanupIssueCode::ReadError
            },
            error.to_string(),
        )
    })?;

    if !metadata.is_file() {
        return Err(issue(
            candidate.path.clone(),
            CleanupIssueCode::NotAFile,
            "Path is no longer a regular file.",
        ));
    }

    if metadata.len() != candidate.size_bytes {
        return Err(issue(
            candidate.path.clone(),
            CleanupIssueCode::MetadataChanged,
            "File size changed after cleanup preview was generated.",
        ));
    }

    let modified_at_millis = system_time_to_millis(
        metadata
            .modified()
            .map_err(|error| issue(candidate.path.clone(), CleanupIssueCode::ReadError, error.to_string()))?,
    )
    .map_err(|message| issue(candidate.path.clone(), CleanupIssueCode::ReadError, message))?;

    if modified_at_millis != candidate.expected_modified_at_millis {
        return Err(issue(
            candidate.path.clone(),
            CleanupIssueCode::MetadataChanged,
            "File metadata changed after cleanup preview was generated.",
        ));
    }

    Ok(())
}

fn is_path_within_root(path: &str, root: &str) -> bool {
    path == root || path.starts_with(&format!("{root}\\"))
}

fn normalize_path(value: &str) -> String {
    value.trim().trim_end_matches(['\\', '/']).to_lowercase()
}

fn relative_path_from_root<'a>(path: &'a str, root_path: &str) -> Option<&'a str> {
    let trimmed_path = path.trim();
    let trimmed_root = root_path.trim().trim_end_matches(['\\', '/']);
    if trimmed_path.len() < trimmed_root.len() {
        return None;
    }

    if trimmed_path.eq_ignore_ascii_case(trimmed_root) {
        return Some("");
    }

    if trimmed_path[..trimmed_root.len()].eq_ignore_ascii_case(trimmed_root) {
        let remainder = &trimmed_path[trimmed_root.len()..];
        return Some(remainder.trim_start_matches(['\\', '/']));
    }

    None
}

fn issue(path: impl Into<String>, code: CleanupIssueCode, summary: impl Into<String>) -> CleanupIssue {
    CleanupIssue {
        path: path.into(),
        code,
        summary: summary.into(),
    }
}

fn system_time_to_millis(value: SystemTime) -> Result<i64, String> {
    let timestamp: DateTime<Utc> = value.into();
    Ok(timestamp.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use tempfile::tempdir;

    #[derive(Default)]
    struct RecordingExecutor {
        recycled: RefCell<Vec<String>>,
        permanently_deleted: RefCell<Vec<String>>,
    }

    impl CleanupExecutor for RecordingExecutor {
        fn recycle(&self, path: &Path) -> Result<(), String> {
            self.recycled
                .borrow_mut()
                .push(path.display().to_string());
            Ok(())
        }

        fn permanent_delete(&self, path: &Path) -> Result<(), String> {
            self.permanently_deleted
                .borrow_mut()
                .push(path.display().to_string());
            Ok(())
        }
    }

    #[test]
    fn lists_builtin_rules_from_repo_toml() {
        let rules = list_cleanup_rules().expect("rules should load");
        let rule_ids = rules.iter().map(|rule| rule.rule_id.as_str()).collect::<Vec<_>>();

        assert_eq!(rule_ids, vec!["temp-folder-files", "download-partials"]);
        assert!(rules[0].label.contains("Temp"));
        assert!(rules[1].label.contains("Partial"));
    }

    #[test]
    fn cleanup_preview_deduplicates_duplicate_and_rule_matches() {
        let fixture = tempdir().expect("fixture");
        let root = fixture.path();
        let temp_dir = root.join("Temp");
        std::fs::create_dir_all(&temp_dir).expect("temp dir");

        let overlapping = temp_dir.join("cache.part");
        let keep = root.join("keep.bin");
        std::fs::write(&overlapping, vec![1_u8; 48]).expect("overlapping file");
        std::fs::write(&keep, vec![9_u8; 16]).expect("keep file");

        let mut request = CleanupPreviewRequest::new(
            "scan-1",
            root.display().to_string(),
            vec![
                CleanupFileEntry {
                    path: overlapping.display().to_string(),
                    size_bytes: 48,
                },
                CleanupFileEntry {
                    path: keep.display().to_string(),
                    size_bytes: 16,
                },
            ],
        );
        request.duplicate_delete_paths = vec![overlapping.display().to_string()];
        request.enabled_rule_ids = vec![
            "temp-folder-files".to_string(),
            "download-partials".to_string(),
        ];

        let preview = build_cleanup_preview(&request).expect("preview should build");

        assert_eq!(preview.candidates.len(), 1);
        assert_eq!(preview.total_bytes, 48);
        assert_eq!(preview.duplicate_candidate_count, 1);
        assert_eq!(preview.rule_candidate_count, 1);
        assert_eq!(preview.candidates[0].path, overlapping.display().to_string());
        assert_eq!(
            preview.candidates[0].source_labels,
            vec![
                "Duplicate selection".to_string(),
                "Files in Temp folders".to_string(),
                "Partial downloads".to_string(),
            ]
        );
    }

    #[test]
    fn cleanup_preview_excludes_invalid_or_protected_paths() {
        let fixture = tempdir().expect("fixture");
        let root = fixture.path();
        let live = root.join("left.bin");
        std::fs::write(&live, vec![7_u8; 24]).expect("live file");
        let missing = root.join("missing.bin");
        let outside_dir = tempdir().expect("outside");
        let outside = outside_dir.path().join("other.bin");
        std::fs::write(&outside, vec![8_u8; 12]).expect("outside file");

        let mut request = CleanupPreviewRequest::new(
            "scan-1",
            root.display().to_string(),
            vec![
                CleanupFileEntry {
                    path: live.display().to_string(),
                    size_bytes: 24,
                },
                CleanupFileEntry {
                    path: missing.display().to_string(),
                    size_bytes: 20,
                },
            ],
        );
        request.duplicate_delete_paths = vec![
            missing.display().to_string(),
            outside.display().to_string(),
            live.display().to_string(),
        ];

        let preview = build_cleanup_preview(&request).expect("preview should build");

        assert_eq!(preview.candidates.len(), 1);
        assert_eq!(preview.candidates[0].path, live.display().to_string());
        assert!(preview
            .issues
            .iter()
            .any(|entry| entry.code == CleanupIssueCode::MissingPath));
        assert!(preview
            .issues
            .iter()
            .any(|entry| entry.code == CleanupIssueCode::OutsideRoot));

        let protected_request = CleanupPreviewRequest {
            preview_id: None,
            scan_id: "scan-windows".to_string(),
            root_path: r"C:\Windows".to_string(),
            file_entries: vec![CleanupFileEntry {
                path: r"C:\Windows\Temp\cache.tmp".to_string(),
                size_bytes: 32,
            }],
            duplicate_delete_paths: vec![r"C:\Windows\Temp\cache.tmp".to_string()],
            enabled_rule_ids: Vec::new(),
        };

        let protected_preview =
            build_cleanup_preview(&protected_request).expect("protected preview should build");
        assert!(protected_preview.candidates.is_empty());
        assert!(protected_preview
            .issues
            .iter()
            .any(|entry| entry.code == CleanupIssueCode::RequiresElevation));
    }

    #[test]
    fn cleanup_execute_recycles_and_reports_revalidation_failures() {
        let fixture = tempdir().expect("fixture");
        let root = fixture.path();
        let left = root.join("left.bin");
        let right = root.join("right.bin");
        std::fs::write(&left, vec![1_u8; 16]).expect("left");
        std::fs::write(&right, vec![2_u8; 20]).expect("right");

        let mut request = CleanupPreviewRequest::new(
            "scan-1",
            root.display().to_string(),
            vec![
                CleanupFileEntry {
                    path: left.display().to_string(),
                    size_bytes: 16,
                },
                CleanupFileEntry {
                    path: right.display().to_string(),
                    size_bytes: 20,
                },
            ],
        );
        request.duplicate_delete_paths = vec![
            left.display().to_string(),
            right.display().to_string(),
        ];

        let preview = build_cleanup_preview(&request).expect("preview should build");
        std::fs::write(&right, vec![3_u8; 21]).expect("change metadata");

        let executor = RecordingExecutor::default();
        let action_ids = preview
            .candidates
            .iter()
            .map(|candidate| candidate.action_id.clone())
            .collect::<Vec<_>>();
        let result = execute_cleanup(
            &executor,
            &preview,
            &action_ids,
            CleanupExecutionMode::Recycle,
        )
        .expect("execution should succeed");

        assert_eq!(result.completed_count, 1);
        assert_eq!(result.failed_count, 1);
        assert_eq!(executor.recycled.borrow().len(), 1);
        assert_eq!(
            executor.recycled.borrow()[0],
            left.display().to_string()
        );
        assert!(result
            .entries
            .iter()
            .any(|entry| entry.status == CleanupExecutionItemStatus::Failed));
    }

    #[test]
    fn cleanup_execute_uses_separate_permanent_mode() {
        let fixture = tempdir().expect("fixture");
        let root = fixture.path();
        let target = root.join("target.bin");
        std::fs::write(&target, vec![1_u8; 12]).expect("target");

        let mut request = CleanupPreviewRequest::new(
            "scan-1",
            root.display().to_string(),
            vec![CleanupFileEntry {
                path: target.display().to_string(),
                size_bytes: 12,
            }],
        );
        request.duplicate_delete_paths = vec![target.display().to_string()];

        let preview = build_cleanup_preview(&request).expect("preview should build");
        let executor = RecordingExecutor::default();
        let action_ids = preview
            .candidates
            .iter()
            .map(|candidate| candidate.action_id.clone())
            .collect::<Vec<_>>();

        let result = execute_cleanup(
            &executor,
            &preview,
            &action_ids,
            CleanupExecutionMode::Permanent,
        )
        .expect("execution should succeed");

        assert_eq!(result.completed_count, 1);
        assert_eq!(result.failed_count, 0);
        assert_eq!(executor.recycled.borrow().len(), 0);
        assert_eq!(executor.permanently_deleted.borrow().len(), 1);
        assert_eq!(
            executor.permanently_deleted.borrow()[0],
            target.display().to_string()
        );
    }
}
