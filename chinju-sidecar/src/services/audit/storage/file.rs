//! File-based storage backend for audit logs
//!
//! Uses JSON Lines format (.jsonl) for append-only, line-by-line storage.

use super::{AuditQuery, RotationResult, StorageBackend, StorageError};
use crate::services::audit::chain::HashChainManager;
use crate::services::audit::types::AuditLogEntry;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
struct ChainState {
    sequence: u64,
    hash: String,
}

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
    /// Last known chain state for continuity checks
    last_state: Mutex<Option<ChainState>>,
    /// HMAC key for archive signature metadata (optional)
    archive_hmac_key: Option<Vec<u8>>,
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

        let archive_hmac_key = std::env::var("CHINJU_AUDIT_ARCHIVE_HMAC_KEY")
            .ok()
            .and_then(|v| {
                if v.is_empty() {
                    None
                } else {
                    Some(v.into_bytes())
                }
            });

        Ok(Self {
            path,
            archive_dir,
            file: Mutex::new(None),
            max_size: 100 * 1024 * 1024, // 100MB default
            last_state: Mutex::new(None),
            archive_hmac_key,
        })
    }

    /// Set maximum file size before rotation
    pub fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = max_size;
        self
    }

    /// Get or open the file handle
    async fn get_writer(
        &self,
    ) -> Result<tokio::sync::MutexGuard<'_, Option<BufWriter<File>>>, StorageError> {
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

    fn read_entries_from_disk(&self) -> Result<Vec<AuditLogEntry>, StorageError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<AuditLogEntry>(&line) {
                Ok(entry) => entries.push(entry),
                Err(e) => warn!(error = %e, "Failed to parse audit log line"),
            }
        }
        Ok(entries)
    }

    fn read_latest_from_disk(&self) -> Result<Option<AuditLogEntry>, StorageError> {
        let mut entries = self.read_entries_from_disk()?;
        Ok(entries.pop())
    }

    async fn ensure_last_state(&self) -> Result<Option<ChainState>, StorageError> {
        let mut state = self.last_state.lock().await;
        if state.is_none() {
            *state = self.read_latest_from_disk()?.map(|entry| ChainState {
                sequence: entry.sequence,
                hash: entry.hash,
            });
        }
        Ok(state.clone())
    }

    async fn set_last_state_from_entry(&self, entry: &AuditLogEntry) {
        let mut state = self.last_state.lock().await;
        *state = Some(ChainState {
            sequence: entry.sequence,
            hash: entry.hash.clone(),
        });
    }

    async fn validate_chain_continuity(
        &self,
        entries: &[AuditLogEntry],
    ) -> Result<(), StorageError> {
        if entries.is_empty() {
            return Ok(());
        }

        let current = self.ensure_last_state().await?;
        let mut expected_seq = match &current {
            Some(s) => s.sequence + 1,
            None => entries[0].sequence,
        };
        let mut expected_prev_hash = current.as_ref().map(|s| s.hash.clone());

        for (idx, entry) in entries.iter().enumerate() {
            if entry.sequence != expected_seq {
                return Err(StorageError::ChainIntegrity(format!(
                    "Sequence continuity violation at index {}: expected {}, got {}",
                    idx, expected_seq, entry.sequence
                )));
            }

            if let Some(prev_hash) = &expected_prev_hash {
                if entry.prev_hash != *prev_hash {
                    return Err(StorageError::ChainIntegrity(format!(
                        "Prev hash continuity violation at sequence {}",
                        entry.sequence
                    )));
                }
            }

            expected_seq += 1;
            expected_prev_hash = Some(entry.hash.clone());
        }

        Ok(())
    }

    fn sign_archive_hash(
        &self,
        archive_filename: &str,
        archive_hash: &str,
    ) -> Result<Option<(String, PathBuf)>, StorageError> {
        let key = match &self.archive_hmac_key {
            Some(key) => key,
            None => return Ok(None),
        };

        let mut mac = HmacSha256::new_from_slice(key)
            .map_err(|e| StorageError::Database(format!("Invalid archive HMAC key: {}", e)))?;
        mac.update(archive_filename.as_bytes());
        mac.update(b"\n");
        mac.update(archive_hash.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        let signature_path = PathBuf::from(format!(
            "{}/{}.sig",
            self.archive_dir.display(),
            archive_filename
        ));
        let payload = serde_json::json!({
            "archive_file": archive_filename,
            "archive_hash": archive_hash,
            "algorithm": "hmac-sha256",
            "signature": signature,
            "generated_at": Utc::now().to_rfc3339(),
        });
        fs::write(&signature_path, serde_json::to_vec_pretty(&payload)?)?;

        Ok(Some((signature, signature_path)))
    }
}

#[async_trait]
impl StorageBackend for FileStorage {
    async fn append(&self, entry: &AuditLogEntry) -> Result<(), StorageError> {
        self.validate_chain_continuity(std::slice::from_ref(entry))
            .await?;
        let mut guard = self.get_writer().await?;

        if let Some(ref mut writer) = *guard {
            let line = serde_json::to_string(entry)?;
            writeln!(writer, "{}", line)?;
            writer.flush()?;
            debug!(sequence = entry.sequence, "Audit entry written");
        }
        self.set_last_state_from_entry(entry).await;

        Ok(())
    }

    async fn append_batch(&self, entries: &[AuditLogEntry]) -> Result<(), StorageError> {
        if entries.is_empty() {
            return Ok(());
        }
        self.validate_chain_continuity(entries).await?;

        let mut guard = self.get_writer().await?;

        if let Some(ref mut writer) = *guard {
            for entry in entries {
                let line = serde_json::to_string(entry)?;
                writeln!(writer, "{}", line)?;
            }
            writer.flush()?;
            debug!(count = entries.len(), "Audit entries batch written");
        }
        if let Some(last) = entries.last() {
            self.set_last_state_from_entry(last).await;
        }

        Ok(())
    }

    async fn get_latest(&self) -> Result<Option<AuditLogEntry>, StorageError> {
        self.read_latest_from_disk()
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
                archive_signature: None,
                archive_signature_path: None,
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
                archive_signature: None,
                archive_signature_path: None,
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
        let archive_signature = self.sign_archive_hash(&archive_filename, &archive_hash)?;

        // Rewrite the main file with remaining entries
        let main_file = File::create(&self.path)?;
        let mut main_writer = BufWriter::new(main_file);

        for line in &to_keep {
            writeln!(main_writer, "{}", line)?;
        }
        main_writer.flush()?;

        // Refresh continuity state after rotation.
        {
            let mut state = self.last_state.lock().await;
            *state = to_keep.last().and_then(|line| {
                serde_json::from_str::<AuditLogEntry>(line)
                    .ok()
                    .map(|entry| ChainState {
                        sequence: entry.sequence,
                        hash: entry.hash,
                    })
            });
        }

        info!(
            entries_archived = entries_archived,
            archive_path = %archive_path.display(),
            "Audit log rotated"
        );

        Ok(RotationResult {
            entries_archived,
            archive_path: archive_path.to_string_lossy().to_string(),
            archive_hash,
            archive_signature: archive_signature.as_ref().map(|(sig, _)| sig.clone()),
            archive_signature_path: archive_signature
                .as_ref()
                .map(|(_, p)| p.to_string_lossy().to_string()),
        })
    }

    async fn verify_integrity(&self) -> Result<(), StorageError> {
        let entries = self.read_entries_from_disk()?;
        HashChainManager::verify_chain(&entries)
            .map_err(|e| StorageError::ChainIntegrity(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit::chain::HashChainManager;
    use crate::services::audit::types::{
        Actor, AuditDetails, AuditEventType, AuditResult, Resource,
    };
    use tempfile::TempDir;

    fn create_test_entry(sequence: u64) -> AuditLogEntry {
        AuditLogEntry::builder()
            .event_type(AuditEventType::AiRequest)
            .source_id("test-sidecar")
            .request_id(format!("req-{}", sequence))
            .actor(Actor::human("user-456", Some(0.75)))
            .resource(Resource::ai_model("gpt-4"))
            .result(AuditResult::success())
            .details(AuditDetails::default().with_model("gpt-4"))
            .build()
    }

    async fn create_chained_entries(count: u64) -> Vec<AuditLogEntry> {
        let chain = HashChainManager::new();
        let mut entries = Vec::new();
        for i in 0..count {
            let mut entry = create_test_entry(i);
            chain.chain_entry(&mut entry).await;
            entries.push(entry);
        }
        entries
    }

    #[tokio::test]
    async fn test_append_and_get_latest() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");

        let storage = FileStorage::new(log_path, archive_dir).unwrap();

        let mut entries = create_chained_entries(1).await;
        let entry = entries.remove(0);
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

        let entries = create_chained_entries(10).await;
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

        let entries = create_chained_entries(20).await;
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

        let entries = create_chained_entries(10).await;
        storage.append_batch(&entries).await.unwrap();

        let range = storage.get_range(3, 7).await.unwrap();
        assert_eq!(range.len(), 5);
        assert_eq!(range[0].sequence, 3);
        assert_eq!(range[4].sequence, 7);
    }

    #[tokio::test]
    async fn test_verify_integrity_detects_tamper() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");
        let storage = FileStorage::new(log_path.clone(), archive_dir).unwrap();

        let entries = create_chained_entries(3).await;
        storage.append_batch(&entries).await.unwrap();

        // Tamper one line on disk.
        let content = std::fs::read_to_string(&log_path).unwrap();
        let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
        lines[1] = lines[1].replace("\"success\":true", "\"success\":false");
        std::fs::write(&log_path, format!("{}\n", lines.join("\n"))).unwrap();

        let verified = storage.verify_integrity().await;
        assert!(verified.is_err());
    }
}
