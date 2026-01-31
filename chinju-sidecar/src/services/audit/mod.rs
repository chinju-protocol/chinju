//! Audit logging module for C6 compliance
//!
//! Provides tamper-evident audit logging with:
//! - Hash chain for integrity verification
//! - Privacy-preserving logging (content hashes only)
//! - Asynchronous persistence
//! - File-based storage with rotation support

pub mod chain;
pub mod logger;
pub mod persister;
pub mod storage;
pub mod types;

pub use chain::{ChainError, HashChainManager};
pub use logger::{AuditError, AuditLogger};
pub use persister::{create_audit_system, create_audit_system_with_restore, AuditPersister, PersisterConfig};
pub use storage::{AuditQuery, FileStorage, RotationResult, StorageBackend, StorageError};
pub use types::{
    compute_content_hash, Actor, ActorType, AuditDetails, AuditEventType, AuditLogEntry,
    AuditResult, Resource,
};
