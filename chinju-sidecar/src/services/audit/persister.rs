//! Background audit log persister
//!
//! Receives audit entries via mpsc channel and writes them to storage
//! in batches for better performance.

use crate::services::audit::storage::StorageBackend;
use crate::services::audit::types::AuditLogEntry;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

/// Configuration for the audit persister
#[derive(Debug, Clone)]
pub struct PersisterConfig {
    /// Maximum batch size before forcing a flush
    pub batch_size: usize,
    /// Maximum time between flushes
    pub flush_interval: Duration,
    /// Number of retries on failure
    pub max_retries: u32,
    /// Delay between retries
    pub retry_delay: Duration,
}

impl Default for PersisterConfig {
    fn default() -> Self {
        Self {
            batch_size: 100,
            flush_interval: Duration::from_secs(1),
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        }
    }
}

/// Background audit log persister
pub struct AuditPersister {
    /// Channel receiver for incoming entries
    receiver: mpsc::Receiver<AuditLogEntry>,
    /// Storage backend
    storage: Arc<dyn StorageBackend>,
    /// Configuration
    config: PersisterConfig,
}

impl AuditPersister {
    /// Create a new persister
    pub fn new(
        receiver: mpsc::Receiver<AuditLogEntry>,
        storage: Arc<dyn StorageBackend>,
    ) -> Self {
        Self {
            receiver,
            storage,
            config: PersisterConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        receiver: mpsc::Receiver<AuditLogEntry>,
        storage: Arc<dyn StorageBackend>,
        config: PersisterConfig,
    ) -> Self {
        Self {
            receiver,
            storage,
            config,
        }
    }

    /// Run the persister as a background task
    pub async fn run(mut self) {
        info!(
            batch_size = self.config.batch_size,
            flush_interval_ms = self.config.flush_interval.as_millis() as u64,
            "Audit persister started"
        );

        let mut buffer: Vec<AuditLogEntry> = Vec::with_capacity(self.config.batch_size);
        let mut flush_timer = interval(self.config.flush_interval);

        loop {
            tokio::select! {
                // Receive new entries
                entry = self.receiver.recv() => {
                    match entry {
                        Some(e) => {
                            buffer.push(e);

                            // Flush if batch size reached
                            if buffer.len() >= self.config.batch_size {
                                self.flush(&mut buffer).await;
                            }
                        }
                        None => {
                            // Channel closed, flush remaining and exit
                            info!("Audit channel closed, flushing remaining entries");
                            self.flush(&mut buffer).await;
                            break;
                        }
                    }
                }

                // Periodic flush
                _ = flush_timer.tick() => {
                    if !buffer.is_empty() {
                        self.flush(&mut buffer).await;
                    }
                }
            }
        }

        info!("Audit persister stopped");
    }

    /// Flush buffer to storage with retries
    async fn flush(&self, buffer: &mut Vec<AuditLogEntry>) {
        if buffer.is_empty() {
            return;
        }

        let count = buffer.len();
        let mut retries = 0;

        loop {
            match self.storage.append_batch(buffer).await {
                Ok(_) => {
                    debug!(count = count, "Audit entries flushed successfully");
                    buffer.clear();
                    return;
                }
                Err(e) => {
                    retries += 1;
                    if retries > self.config.max_retries {
                        error!(
                            error = %e,
                            count = count,
                            retries = retries,
                            "Failed to flush audit entries after max retries, entries may be lost"
                        );
                        // In production, consider sending to a dead letter queue
                        buffer.clear();
                        return;
                    }

                    warn!(
                        error = %e,
                        retry = retries,
                        max_retries = self.config.max_retries,
                        "Failed to flush audit entries, retrying"
                    );

                    tokio::time::sleep(self.config.retry_delay).await;
                }
            }
        }
    }
}

/// Create an audit system with logger and persister
pub fn create_audit_system(
    storage: Arc<dyn StorageBackend>,
    source_id: impl Into<String>,
    buffer_size: usize,
) -> (Arc<crate::services::audit::AuditLogger>, AuditPersister) {
    use crate::services::audit::chain::HashChainManager;
    use crate::services::audit::AuditLogger;

    let (tx, rx) = mpsc::channel(buffer_size);
    let chain = Arc::new(HashChainManager::new());
    let logger = Arc::new(AuditLogger::new(tx, chain, source_id));
    let persister = AuditPersister::new(rx, storage);

    (logger, persister)
}

/// Create an audit system restoring from existing chain state
pub async fn create_audit_system_with_restore(
    storage: Arc<dyn StorageBackend>,
    source_id: impl Into<String>,
    buffer_size: usize,
) -> Result<(Arc<crate::services::audit::AuditLogger>, AuditPersister), crate::services::audit::storage::StorageError>
{
    use crate::services::audit::chain::HashChainManager;
    use crate::services::audit::AuditLogger;

    let (tx, rx) = mpsc::channel(buffer_size);

    // Restore chain state from storage
    let chain = if let Some(latest) = storage.get_latest().await? {
        info!(
            sequence = latest.sequence,
            hash = %latest.hash,
            "Restored audit chain state"
        );
        Arc::new(HashChainManager::from_state(latest.sequence + 1, latest.hash))
    } else {
        info!("Starting new audit chain (genesis)");
        Arc::new(HashChainManager::new())
    };

    let logger = Arc::new(AuditLogger::new(tx, chain, source_id));
    let persister = AuditPersister::new(rx, storage);

    Ok((logger, persister))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit::storage::file::FileStorage;
    use crate::services::audit::types::{Actor, AuditDetails, AuditEventType, AuditResult, Resource};
    use tempfile::TempDir;

    fn create_test_entry(sequence: u64) -> AuditLogEntry {
        let mut entry = AuditLogEntry::builder()
            .event_type(AuditEventType::AiRequest)
            .source_id("test")
            .request_id(format!("req-{}", sequence))
            .actor(Actor::anonymous())
            .resource(Resource::ai_model("gpt-4"))
            .result(AuditResult::success())
            .details(AuditDetails::default())
            .build();

        entry.sequence = sequence;
        entry.prev_hash = "sha256:test".to_string();
        entry.hash = format!("sha256:hash-{}", sequence);
        entry
    }

    #[tokio::test]
    async fn test_persister_flush() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");

        let storage = Arc::new(FileStorage::new(log_path.clone(), archive_dir).unwrap());

        let (tx, rx) = mpsc::channel(100);
        let persister = AuditPersister::with_config(
            rx,
            storage.clone(),
            PersisterConfig {
                batch_size: 5,
                flush_interval: Duration::from_millis(50),
                ..Default::default()
            },
        );

        // Start persister in background
        let handle = tokio::spawn(persister.run());

        // Send entries
        for i in 0..10 {
            tx.send(create_test_entry(i)).await.unwrap();
        }

        // Wait for flush
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Close channel to stop persister
        drop(tx);
        handle.await.unwrap();

        // Verify entries were persisted
        let latest = storage.get_latest().await.unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().sequence, 9);
    }

    #[tokio::test]
    async fn test_create_audit_system() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.jsonl");
        let archive_dir = temp_dir.path().join("archive");

        let storage: Arc<dyn StorageBackend> =
            Arc::new(FileStorage::new(log_path, archive_dir).unwrap());

        let (logger, persister) = create_audit_system(storage.clone(), "test-sidecar", 100);

        let handle = tokio::spawn(persister.run());

        // Log an entry
        let result = logger
            .log_ai_request("req-1", Some("user-1"), Some(0.8), b"test", "gpt-4")
            .await;
        assert!(result.is_ok());

        // Wait for persistence
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Drop logger to close channel
        drop(logger);
        handle.await.unwrap();

        // Verify
        let latest = storage.get_latest().await.unwrap();
        assert!(latest.is_some());
    }
}
