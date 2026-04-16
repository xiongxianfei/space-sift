use cleanup_core::CleanupExecutionResult;
use duplicates_core::{
    CachedHashes, DuplicateAnalysisFailure, HashCache, HashCacheKey, HashCacheWrite,
};
use scan_core::{CompletedScan, ScanHistoryEntry};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct HistoryStore {
    db_path: PathBuf,
}

impl HistoryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { db_path: path.into() }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn initialize(&self) -> Result<(), HistoryStoreError> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        }

        let connection = self.open_connection()?;
        connection
            .execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS scan_history (
                    scan_id TEXT PRIMARY KEY,
                    root_path TEXT NOT NULL,
                    completed_at TEXT NOT NULL,
                    total_bytes INTEGER NOT NULL,
                    scan_json TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS duplicate_hash_cache (
                    path TEXT NOT NULL,
                    size_bytes INTEGER NOT NULL,
                    modified_at_millis INTEGER NOT NULL,
                    partial_hash TEXT,
                    full_hash TEXT,
                    PRIMARY KEY (path, size_bytes, modified_at_millis)
                );

                CREATE TABLE IF NOT EXISTS cleanup_execution_history (
                    execution_id TEXT PRIMARY KEY,
                    preview_id TEXT NOT NULL,
                    mode TEXT NOT NULL,
                    completed_at TEXT NOT NULL,
                    completed_count INTEGER NOT NULL,
                    failed_count INTEGER NOT NULL,
                    execution_json TEXT NOT NULL
                );
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        Ok(())
    }

    pub fn save_completed_scan(&self, scan: &CompletedScan) -> Result<(), HistoryStoreError> {
        self.initialize()?;

        let payload = serde_json::to_string(scan)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let total_bytes = i64::try_from(scan.total_bytes)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let connection = self.open_connection()?;
        connection
            .execute(
                r#"
                INSERT OR REPLACE INTO scan_history (
                    scan_id,
                    root_path,
                    completed_at,
                    total_bytes,
                    scan_json
                ) VALUES (?1, ?2, ?3, ?4, ?5);
                "#,
                rusqlite::params![
                    scan.scan_id,
                    scan.root_path,
                    scan.completed_at,
                    total_bytes,
                    payload
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        Ok(())
    }

    pub fn list_history(&self) -> Result<Vec<ScanHistoryEntry>, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT scan_id, root_path, completed_at, total_bytes
                FROM scan_history
                ORDER BY completed_at DESC;
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let rows = statement
            .query_map([], |row| {
                Ok(ScanHistoryEntry {
                    scan_id: row.get(0)?,
                    root_path: row.get(1)?,
                    completed_at: row.get(2)?,
                    total_bytes: u64::try_from(row.get::<_, i64>(3)?).unwrap_or_default(),
                })
            })
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))
    }

    pub fn open_history_entry(&self, scan_id: &str) -> Result<CompletedScan, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let payload = connection.query_row(
            "SELECT scan_json FROM scan_history WHERE scan_id = ?1;",
            rusqlite::params![scan_id],
            |row| row.get::<_, String>(0),
        );

        match payload {
            Ok(payload) => serde_json::from_str(&payload)
                .map_err(|error| HistoryStoreError::Persistence(error.to_string())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(HistoryStoreError::NotFound {
                scan_id: scan_id.to_string(),
            }),
            Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
        }
    }

    fn open_connection(&self) -> Result<rusqlite::Connection, HistoryStoreError> {
        rusqlite::Connection::open(&self.db_path)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))
    }

    fn load_hash_cache_entry(&self, key: &HashCacheKey) -> Result<Option<CachedHashes>, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let row = connection.query_row(
            r#"
            SELECT partial_hash, full_hash
            FROM duplicate_hash_cache
            WHERE path = ?1 AND size_bytes = ?2 AND modified_at_millis = ?3;
            "#,
            rusqlite::params![
                key.path,
                i64::try_from(key.size_bytes)
                    .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                key.modified_at_millis
            ],
            |row| {
                Ok(CachedHashes {
                    partial_hash: row.get(0)?,
                    full_hash: row.get(1)?,
                })
            },
        );

        match row {
            Ok(entry) => Ok(Some(entry)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
        }
    }

    fn load_hash_cache_entries(
        &self,
        keys: &[HashCacheKey],
    ) -> Result<HashMap<HashCacheKey, CachedHashes>, HistoryStoreError> {
        self.initialize()?;
        if keys.is_empty() {
            return Ok(HashMap::new());
        }

        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT partial_hash, full_hash
                FROM duplicate_hash_cache
                WHERE path = ?1 AND size_bytes = ?2 AND modified_at_millis = ?3;
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let mut entries = HashMap::with_capacity(keys.len());

        for key in keys {
            let row = statement.query_row(
                rusqlite::params![
                    key.path,
                    i64::try_from(key.size_bytes)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    key.modified_at_millis
                ],
                |row| {
                    Ok(CachedHashes {
                        partial_hash: row.get(0)?,
                        full_hash: row.get(1)?,
                    })
                },
            );

            match row {
                Ok(entry) => {
                    entries.insert(key.clone(), entry);
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {}
                Err(error) => return Err(HistoryStoreError::Persistence(error.to_string())),
            }
        }

        Ok(entries)
    }

    pub fn save_cleanup_execution(
        &self,
        result: &CleanupExecutionResult,
    ) -> Result<(), HistoryStoreError> {
        self.initialize()?;

        let payload = serde_json::to_string(result)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let completed_count = i64::try_from(result.completed_count)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let failed_count = i64::try_from(result.failed_count)
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let connection = self.open_connection()?;
        connection
            .execute(
                r#"
                INSERT OR REPLACE INTO cleanup_execution_history (
                    execution_id,
                    preview_id,
                    mode,
                    completed_at,
                    completed_count,
                    failed_count,
                    execution_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);
                "#,
                rusqlite::params![
                    result.execution_id,
                    result.preview_id,
                    serde_json::to_string(&result.mode)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    result.completed_at,
                    completed_count,
                    failed_count,
                    payload
                ],
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        Ok(())
    }

    pub fn open_cleanup_execution(
        &self,
        execution_id: &str,
    ) -> Result<CleanupExecutionResult, HistoryStoreError> {
        self.initialize()?;
        let connection = self.open_connection()?;
        let payload = connection.query_row(
            "SELECT execution_json FROM cleanup_execution_history WHERE execution_id = ?1;",
            rusqlite::params![execution_id],
            |row| row.get::<_, String>(0),
        );

        match payload {
            Ok(payload) => serde_json::from_str(&payload)
                .map_err(|error| HistoryStoreError::Persistence(error.to_string())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Err(HistoryStoreError::NotFound {
                scan_id: execution_id.to_string(),
            }),
            Err(error) => Err(HistoryStoreError::Persistence(error.to_string())),
        }
    }

    fn save_hash_cache_entry(
        &self,
        key: &HashCacheKey,
        partial_hash: Option<&str>,
        full_hash: Option<&str>,
    ) -> Result<(), HistoryStoreError> {
        self.save_hash_cache_entries(&[HashCacheWrite {
            key: key.clone(),
            partial_hash: partial_hash.map(str::to_string),
            full_hash: full_hash.map(str::to_string),
        }])
    }

    fn save_hash_cache_entries(
        &self,
        writes: &[HashCacheWrite],
    ) -> Result<(), HistoryStoreError> {
        self.initialize()?;
        if writes.is_empty() {
            return Ok(());
        }

        let mut connection = self.open_connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        let mut statement = transaction
            .prepare(
                r#"
                INSERT INTO duplicate_hash_cache (
                    path,
                    size_bytes,
                    modified_at_millis,
                    partial_hash,
                    full_hash
                ) VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(path, size_bytes, modified_at_millis) DO UPDATE SET
                    partial_hash = COALESCE(excluded.partial_hash, duplicate_hash_cache.partial_hash),
                    full_hash = COALESCE(excluded.full_hash, duplicate_hash_cache.full_hash);
                "#,
            )
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;

        for write in writes {
            statement
                .execute(rusqlite::params![
                    write.key.path,
                    i64::try_from(write.key.size_bytes)
                        .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?,
                    write.key.modified_at_millis,
                    write.partial_hash,
                    write.full_hash
                ])
                .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        }

        drop(statement);
        transaction
            .commit()
            .map_err(|error| HistoryStoreError::Persistence(error.to_string()))?;
        Ok(())
    }
}

impl HashCache for HistoryStore {
    fn get_cached_hashes(&self, key: &HashCacheKey) -> Result<Option<CachedHashes>, DuplicateAnalysisFailure> {
        self.load_hash_cache_entry(key).map_err(|error| DuplicateAnalysisFailure::Internal {
            message: error.to_string(),
        })
    }

    fn save_partial_hash(&self, key: &HashCacheKey, partial_hash: &str) -> Result<(), DuplicateAnalysisFailure> {
        self.save_hash_cache_entry(key, Some(partial_hash), None)
            .map_err(|error| DuplicateAnalysisFailure::Internal {
                message: error.to_string(),
            })
    }

    fn save_full_hash(&self, key: &HashCacheKey, full_hash: &str) -> Result<(), DuplicateAnalysisFailure> {
        self.save_hash_cache_entry(key, None, Some(full_hash))
            .map_err(|error| DuplicateAnalysisFailure::Internal {
                message: error.to_string(),
            })
    }

    fn get_cached_hashes_batch(
        &self,
        keys: &[HashCacheKey],
    ) -> Result<HashMap<HashCacheKey, CachedHashes>, DuplicateAnalysisFailure> {
        self.load_hash_cache_entries(keys)
            .map_err(|error| DuplicateAnalysisFailure::Internal {
                message: error.to_string(),
            })
    }

    fn save_hashes_batch(
        &self,
        writes: &[HashCacheWrite],
    ) -> Result<(), DuplicateAnalysisFailure> {
        self.save_hash_cache_entries(writes)
            .map_err(|error| DuplicateAnalysisFailure::Internal {
                message: error.to_string(),
            })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HistoryStoreError {
    #[error("history entry not found: {scan_id}")]
    NotFound { scan_id: String },
    #[error("history persistence failed: {0}")]
    Persistence(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use cleanup_core::{CleanupExecutionEntry, CleanupExecutionItemStatus, CleanupExecutionMode};
    use scan_core::{ScanEntry, ScanEntryKind, SizedPath, SkipReasonCode, SkippedPath};
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

    #[test]
    fn persists_and_reopens_completed_scans() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let expected = sample_completed_scan();

        store.initialize().expect("schema initialization");
        store
            .save_completed_scan(&expected)
            .expect("scan should persist successfully");

        let entries = store.list_history().expect("history list");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].scan_id, expected.scan_id);
        assert_eq!(entries[0].total_bytes, expected.total_bytes);

        let reopened = store
            .open_history_entry(&expected.scan_id)
            .expect("stored entry should reopen");
        assert_eq!(reopened, expected);
    }

    #[test]
    fn missing_scan_id_returns_not_found() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        store.initialize().expect("schema initialization");

        let error = store
            .open_history_entry("missing-scan-id")
            .expect_err("missing history entry should fail");

        assert_eq!(
            error,
            HistoryStoreError::NotFound {
                scan_id: "missing-scan-id".to_string(),
            }
        );
    }

    #[test]
    fn reopens_legacy_history_payloads_without_browseable_entries() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        store.initialize().expect("schema initialization");

        let connection =
            rusqlite::Connection::open(store.db_path()).expect("history database should open");
        connection
            .execute(
                r#"
                INSERT INTO scan_history (
                    scan_id,
                    root_path,
                    completed_at,
                    total_bytes,
                    scan_json
                ) VALUES (?1, ?2, ?3, ?4, ?5);
                "#,
                rusqlite::params![
                    "legacy-scan",
                    "C:\\scan-root",
                    "2026-04-15T10:00:05Z",
                    42_i64,
                    r#"{
                        "scanId":"legacy-scan",
                        "rootPath":"C:\\scan-root",
                        "startedAt":"2026-04-15T10:00:00Z",
                        "completedAt":"2026-04-15T10:00:05Z",
                        "totalBytes":42,
                        "totalFiles":3,
                        "totalDirectories":2,
                        "largestFiles":[{"path":"C:\\scan-root\\large.bin","sizeBytes":42}],
                        "largestDirectories":[{"path":"C:\\scan-root","sizeBytes":42}],
                        "skippedPaths":[]
                    }"#
                ],
            )
            .expect("legacy payload insert");

        let reopened = store
            .open_history_entry("legacy-scan")
            .expect("legacy payload should reopen");

        assert!(reopened.entries.is_empty());
    }

    #[test]
    fn stores_and_reuses_duplicate_hash_cache_entries() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let key = HashCacheKey {
            path: "C:\\scan-root\\dup.bin".to_string(),
            size_bytes: 42,
            modified_at_millis: 1234,
        };

        store
            .save_partial_hash(&key, "partial-1")
            .expect("partial hash should persist");
        store
            .save_full_hash(&key, "full-1")
            .expect("full hash should persist");

        let cached = store
            .get_cached_hashes(&key)
            .expect("cache lookup should succeed")
            .expect("cache entry should exist");

        assert_eq!(cached.partial_hash.as_deref(), Some("partial-1"));
        assert_eq!(cached.full_hash.as_deref(), Some("full-1"));
    }

    #[test]
    fn duplicate_hash_cache_key_changes_invalidate_lookup() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let key = HashCacheKey {
            path: "C:\\scan-root\\dup.bin".to_string(),
            size_bytes: 42,
            modified_at_millis: 1234,
        };
        let changed_key = HashCacheKey {
            modified_at_millis: 5678,
            ..key.clone()
        };

        store
            .save_full_hash(&key, "full-1")
            .expect("full hash should persist");

        assert!(store
            .get_cached_hashes(&changed_key)
            .expect("changed lookup should succeed")
            .is_none());
    }

    #[test]
    fn batch_duplicate_hash_cache_save_merges_partial_and_full_hashes() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let key = HashCacheKey {
            path: "C:\\scan-root\\dup.bin".to_string(),
            size_bytes: 42,
            modified_at_millis: 1234,
        };

        store
            .save_hashes_batch(&[
                HashCacheWrite {
                    key: key.clone(),
                    partial_hash: Some("partial-1".to_string()),
                    full_hash: None,
                },
                HashCacheWrite {
                    key: key.clone(),
                    partial_hash: None,
                    full_hash: Some("full-1".to_string()),
                },
            ])
            .expect("batch hash save should persist");

        let cached = store
            .get_cached_hashes(&key)
            .expect("cache lookup should succeed")
            .expect("cache entry should exist");

        assert_eq!(cached.partial_hash.as_deref(), Some("partial-1"));
        assert_eq!(cached.full_hash.as_deref(), Some("full-1"));
    }

    #[test]
    fn batch_duplicate_hash_cache_lookup_returns_only_requested_entries() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let existing = HashCacheKey {
            path: "C:\\scan-root\\dup.bin".to_string(),
            size_bytes: 42,
            modified_at_millis: 1234,
        };
        let missing = HashCacheKey {
            path: "C:\\scan-root\\missing.bin".to_string(),
            size_bytes: 84,
            modified_at_millis: 5678,
        };

        store
            .save_full_hash(&existing, "full-1")
            .expect("full hash should persist");

        let entries = store
            .get_cached_hashes_batch(&[existing.clone(), missing.clone()])
            .expect("batch lookup should succeed");

        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries.get(&existing).and_then(|entry| entry.full_hash.as_deref()),
            Some("full-1")
        );
        assert!(!entries.contains_key(&missing));
    }

    #[test]
    fn persists_cleanup_execution_logs() {
        let fixture = tempdir().expect("db fixture");
        let store = HistoryStore::new(fixture.path().join("history.db"));
        let expected = CleanupExecutionResult {
            execution_id: "execution-1".to_string(),
            preview_id: "preview-1".to_string(),
            mode: CleanupExecutionMode::Recycle,
            completed_at: "2026-04-15T11:03:00Z".to_string(),
            completed_count: 1,
            failed_count: 1,
            entries: vec![
                CleanupExecutionEntry {
                    action_id: "action-1".to_string(),
                    path: "C:\\scan-root\\left.bin".to_string(),
                    status: CleanupExecutionItemStatus::Completed,
                    summary: "Moved to the Recycle Bin.".to_string(),
                },
                CleanupExecutionEntry {
                    action_id: "action-2".to_string(),
                    path: "C:\\scan-root\\right.bin".to_string(),
                    status: CleanupExecutionItemStatus::Failed,
                    summary: "File metadata changed after cleanup preview was generated."
                        .to_string(),
                },
            ],
        };

        store
            .save_cleanup_execution(&expected)
            .expect("cleanup execution should persist");

        let reopened = store
            .open_cleanup_execution(&expected.execution_id)
            .expect("cleanup execution should reopen");

        assert_eq!(reopened, expected);
    }
}
