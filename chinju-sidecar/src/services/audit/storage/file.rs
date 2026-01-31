//! File-based storage backend for audit logs
//!
//! Uses JSON Lines format (.jsonl) for append-only, line-by-line storage.

use super::{AuditQuery, RotationResult, StorageBackend, StorageError};
use crate::services::audit::types::AuditLogEntry;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// File-based storage for audit logs
pub struct FileStorage {
    /// Path to the log file
    path: PathBuf,
    /// Archive directory
    archive_dir: PathBuf,
    /// File handle (with mutex for thread safety)
    file: Mutex<Option<BufWriter<File>>>,
    /// Maximum file size before rotation (bytes)
    max_size: u64,
}

impl FileStorage {
    /// Create a new file storage
    pub fn new(path: PathBuf, archive_dir: PathBuf) -> Result<Self, StorageError> {
        // Create directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&archive_dir)?;

        info!(path = %path.display(), "Initializing file-based audit storage");

        Ok(Self {
            path,
            archive_dir,
            file: Mutex::new(None),
            max_size: 100 * 1024 * 1024, // 100MB default
        })
    }

    /// Set maximum file size before rotation
    pub fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = max_size;
        self
    }

    /// Get or open the file handle
    async fn get_writer(&self) -> Result<tokio::sync::MutexGuard<'_, Option<BufWriter<File>>>, StorageError> {
        let mut guard = self.file.lock().await;

        if guard.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            *guard = Some(BufWriter::new(file));
        }

        Ok(guard)
    }

    /// Check if rotation is needed based on file size
    #[allow(dead_code)]
    async fn needs_rotation(&self) -> bool {
        if let Ok(metadata) = fs::metadata(&self.path) {
            metadata.len() >= self.max_size
        } else {
            false
        }
    }

    /// Generate archive filename
    fn archive_filename(&self) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        format!("audit_{}.jsonl.gz", timestamp)
    }
}

#[async_trait]
impl StorageBackend for FileStorage {
    async fn append(&self, entry: &AuditLogEntry) -> Result<(), StorageError> {
        let mut guard = self.get_writer().await?;

        if let Some(ref mut writer) = *guard {
            let line = serde_json::to_string(entry)?;
            writeln!(writer, "{}", line)?;
            writer.flush()?;
            debug!(sequence = entry.sequence, "Audit entry written");
        }

        Ok(())
    }

    async fn append_batch(&self, entries: &[AuditLogEntry]) -> Result<(), StorageError> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut guard = self.get_writer().await?;

        if let Some(ref mut writer) = *guard {
            for entry in entries {
                let line = serde_json::to_string(entry)?;
                writeln!(writer, "{}", line)?;
            }
            writer.flush()?;
            debug!(count = entries.len(), "Audit entries batch written");
        }

        Ok(())
    }

    async fn get_latest(&self) -> Result<Option<AuditLogEntry>, StorageError> {
        if !self.path.exists() {
            return Ok(None);
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut last_entry = None;
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<AuditLogEntry>(&line) {
                    Ok(entry) => last_entry = Some(entry),
                    Err(e) => {
                        warn!(error = %e, "Failed to parse audit log line");
                    }
                }
            }
        }

        Ok(last_entry)
    }

    async fn query(&self, query: &AuditQuery) -> Result<Vec<AuditLogEntry>, StorageError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut results = Vec::new();
        let mut skipped = 0;

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: AuditLogEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Apply filters
            if let Some((from, to)) = &query.time_range {
                if entry.timestamp < *from || entry.timestamp > *to {
                    continue;
                }
            }

            if let Some(ref types) = query.event_types {
                if !types.contains(&entry.event_type) {
                    continue;
                }
            }

            if let Some(ref req_id) = query.request_id {
                if entry.request_id.as_ref() != Some(req_id) {
                    continue;
                }
            }

            if let Some(ref actor_id) = query.actor_id {
                if &entry.actor.actor_id != actor_id {
                    continue;
                }
            }

            // Handle offset
            if let Some(offset) = query.offset {
                if skipped < offset {
                    skipped += 1;
                    continue;
                }
            }

            results.push(entry);

            // Handle limit
            if let Some(limit) = query.limit {
                if results.len() >= limit {
                    break;
                }
            }
        }

        Ok(results)
    }

    async fn get_range(
        &self,
        from_seq: u64,
        to_seq: u64,
    ) -> Result<Vec<AuditLogEntry>, StorageError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut results = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: AuditLogEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            let seq = entry.sequence;
            if seq >= from_seq && seq <= to_seq {
                results.push(entry);
            }

            if seq > to_seq {
                break;
            }
        }

        // Sort by sequence to ensure order
        results.sort_by_key(|e| e.sequence);

        Ok(results)
    }

    async fn rotate(&self, cutoff: DateTime<Utc>) -> Result<RotationResult, StorageError> {
        if !self.path.exists() {
            return Ok(RotationResult {
                entries_archived: 0,
                archive_path: String::new(),
                archive_hash: String::new(),
            });
        }

        // Close current file handle
        {
            let mut guard = self.file.lock().await;
            if let Some(ref mut writer) = *guard {
                writer.flush()?;
            }
            *guard = None;
        }

        // Read all entries
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);

        let mut to_archive = Vec::new();
        let mut to_keep = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let entry: AuditLogEntry = match serde_json::from_str(&line) {
                Ok(e) => e,
                Err(_) => continue,
            };

            if entry.timestamp < cutoff {
                to_archive.push(line);
            } else {
                to_keep.push(line);
            }
        }

        let entries_archived = to_archive.len();

        if entries_archived == 0 {
            return Ok(RotationResult {
                entries_archived: 0,
                archive_path: String::new(),
                archive_hash: String::new(),
            });
        }

        // Write archive file
        let archive_filename = self.archive_filename();
        let archive_path = self.archive_dir.join(&archive_filename);

        let archive_file = File::create(&archive_path)?;
        let mut archive_writer = BufWriter::new(archive_file);
        let mut hasher = Sha256::new();

        for line in &to_archive {
            writeln!(archive_writer, "{}", line)?;
            hasher.update(line.as_bytes());
            hasher.update(b"\n");
        }
        archive_writer.flush()?;

        let archive_hash = format!("sha256:{}", hex::encode(hasher.finalize()));

        // Rewrite the main file with remaining entries
        let main_file = File::create(&self.path)?;
        let mut main_writer = BufWriter::new(main_file);

        for line in &to_keep {
            writeln!(main_writer, "{}", line)?;
        }
        main_writer.flush()?;

        info!(
            entries_archived = entries_archived,
            archive_path = %archive_path.display(),
            "Audit log rotated"
        );

        Ok(RotationResult {
            entries_archived,
            archive_path: archive_path.to_string_lossy().to_string(),
            archive_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit::types::{Actor, AuditDetails, AuditEventType, AuditResult, Resource};
    use tempfile::TempDir;

    fn create_test_entry(sequence: u64) -> AuditLogEntry {
        let mut entry = AuditLogEntry::builder()
            .event_type(AuditEventType::AiRequest)
            .source_id("test-sidecar")
            .request_id(format!("req-{}", sequence))
            .actor(Actor::human("user-456", Some(0.75)))
            .resource(Resource::ai_model("gpt-4"))
            .result(AuditResult::success())
            .details(AuditDetails::default().with_model("gpt-4"))
            .build();

        entry.sequence = sequence;
        entry.prev_hash = "sha256:test".to_string();
        entry.hash = format!("sha256:hash-{}", sequence);
        entry
    }

    #[tokio::test]
    async fn test_append_and_get_latest() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");

        let storage = FileStorage::new(log_path, archive_dir).unwrap();

        let entry = create_test_entry(0);
        storage.append(&entry).await.unwrap();

        let latest = storage.get_latest().await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().sequence, 0);
    }

    #[tokio::test]
    async fn test_append_batch() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");

        let storage = FileStorage::new(log_path, archive_dir).unwrap();

        let entries: Vec<_> = (0..10).map(create_test_entry).collect();
        storage.append_batch(&entries).await.unwrap();

        let latest = storage.get_latest().await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().sequence, 9);
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");

        let storage = FileStorage::new(log_path, archive_dir).unwrap();

        let entries: Vec<_> = (0..20).map(create_test_entry).collect();
        storage.append_batch(&entries).await.unwrap();

        let query = AuditQuery::new().with_limit(5);
        let results = storage.query(&query).await.unwrap();

        assert_eq!(results.len(), 5);
    }

    #[tokio::test]
    async fn test_get_range() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");

        let storage = FileStorage::new(log_path, archive_dir).unwrap();

        let entries: Vec<_> = (0..10).map(create_test_entry).collect();
        storage.append_batch(&entries).await.unwrap();

        let range = storage.get_range(3, 7).await.unwrap();
        assert_eq!(range.len(), 5);
        assert_eq!(range[0].sequence, 3);
        assert_eq!(range[4].sequence, 7);
    }
}
