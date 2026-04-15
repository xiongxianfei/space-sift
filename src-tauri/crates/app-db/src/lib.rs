use scan_core::{CompletedScan, ScanHistoryEntry};
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
    use scan_core::{ScanEntry, ScanEntryKind, SizedPath, SkippedPath, SkipReasonCode};
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
}
