//! Storage backends for audit logs

pub mod file;

pub use file::FileStorage;

use crate::services::audit::types::{AuditEventType, AuditLogEntry};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Storage backend trait for audit logs
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Append a single entry
    async fn append(&self, entry: &AuditLogEntry) -> Result<(), StorageError>;

    /// Append multiple entries in batch
    async fn append_batch(&self, entries: &[AuditLogEntry]) -> Result<(), StorageError>;

    /// Get the latest entry (for chain restoration)
    async fn get_latest(&self) -> Result<Option<AuditLogEntry>, StorageError>;

    /// Query entries with filters
    async fn query(&self, query: &AuditQuery) -> Result<Vec<AuditLogEntry>, StorageError>;

    /// Get entries by sequence range (for chain verification)
    async fn get_range(
        &self,
        from_seq: u64,
        to_seq: u64,
    ) -> Result<Vec<AuditLogEntry>, StorageError>;

    /// Rotate logs (archive old entries)
    async fn rotate(&self, cutoff: DateTime<Utc>) -> Result<RotationResult, StorageError>;

    /// Verify integrity of persisted logs.
    ///
    /// Backends that support tamper-evident storage should override this.
    async fn verify_integrity(&self) -> Result<(), StorageError> {
        Ok(())
    }
}

/// Query parameters for audit log search
#[derive(Debug, Default)]
pub struct AuditQuery {
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    pub event_types: Option<Vec<AuditEventType>>,
    pub request_id: Option<String>,
    pub actor_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl AuditQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_time_range(mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        self.time_range = Some((from, to));
        self
    }

    pub fn with_event_types(mut self, types: Vec<AuditEventType>) -> Self {
        self.event_types = Some(types);
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn with_actor_id(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = Some(actor_id.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// Result of log rotation
#[derive(Debug)]
pub struct RotationResult {
    /// Number of entries archived
    pub entries_archived: usize,
    /// Path to the archive file
    pub archive_path: String,
    /// Hash of the archive (for integrity verification)
    pub archive_hash: String,
    /// Optional archive signature (tamper-evident metadata)
    pub archive_signature: Option<String>,
    /// Optional signature file path
    pub archive_signature_path: Option<String>,
}

/// Storage errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Chain integrity error: {0}")]
    ChainIntegrity(String),

    #[error("Storage not available: {0}")]
    NotAvailable(String),
}
