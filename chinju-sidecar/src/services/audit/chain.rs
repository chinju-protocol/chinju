//! Hash chain management for tamper detection
//!
//! Each audit log entry contains the hash of the previous entry,
//! forming an immutable chain that can detect any modifications.

use crate::services::audit::types::AuditLogEntry;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Mutex;

/// Hash chain manager for audit log integrity
pub struct HashChainManager {
    /// Current sequence number
    sequence: AtomicU64,
    /// Hash of the last entry
    last_hash: Mutex<String>,
    /// Genesis hash (starting point of the chain)
    genesis_hash: String,
}

impl HashChainManager {
    /// Create a new hash chain (genesis block)
    pub fn new() -> Self {
        let genesis = Self::compute_genesis_hash();
        Self {
            sequence: AtomicU64::new(0),
            last_hash: Mutex::new(genesis.clone()),
            genesis_hash: genesis,
        }
    }

    /// Restore from existing chain state
    pub fn from_state(next_sequence: u64, last_hash: String) -> Self {
        Self {
            sequence: AtomicU64::new(next_sequence),
            last_hash: Mutex::new(last_hash),
            genesis_hash: "restored".to_string(),
        }
    }

    /// Get the genesis hash
    pub fn genesis_hash(&self) -> &str {
        &self.genesis_hash
    }

    /// Get current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Add chain information to an entry
    pub async fn chain_entry(&self, entry: &mut AuditLogEntry) {
        // Set sequence number
        entry.sequence = self.sequence.fetch_add(1, Ordering::SeqCst);

        // Set prev_hash
        let mut last = self.last_hash.lock().await;
        entry.prev_hash = last.clone();

        // Compute and set this entry's hash
        entry.hash = Self::compute_entry_hash(entry);

        // Update last_hash
        *last = entry.hash.clone();
    }

    /// Compute the hash of an entry
    fn compute_entry_hash(entry: &AuditLogEntry) -> String {
        let mut hasher = Sha256::new();

        // Hash all fields except 'hash' and 'signature'
        hasher.update(entry.schema_version.as_bytes());
        hasher.update(entry.log_id.as_bytes());
        hasher.update(&entry.sequence.to_le_bytes());
        hasher.update(entry.timestamp.to_rfc3339().as_bytes());
        hasher.update(entry.prev_hash.as_bytes());
        hasher.update(format!("{:?}", entry.event_type).as_bytes());
        hasher.update(entry.source_id.as_bytes());

        if let Some(ref req_id) = entry.request_id {
            hasher.update(req_id.as_bytes());
        }

        // Hash actor
        hasher.update(format!("{:?}", entry.actor.actor_type).as_bytes());
        hasher.update(entry.actor.actor_id.as_bytes());
        if let Some(ref cred_id) = entry.actor.credential_id {
            hasher.update(cred_id.as_bytes());
        }
        if let Some(score) = entry.actor.capability_score {
            hasher.update(&score.to_le_bytes());
        }

        // Hash resource
        hasher.update(entry.resource.resource_type.as_bytes());
        hasher.update(entry.resource.resource_id.as_bytes());

        // Hash result
        hasher.update(&[entry.result.success as u8]);
        if let Some(ref decision) = entry.result.policy_decision {
            hasher.update(decision.as_bytes());
        }
        for rule in &entry.result.matched_rules {
            hasher.update(rule.as_bytes());
        }
        hasher.update(&entry.result.duration_ms.to_le_bytes());

        // Hash details
        if let Some(ref req_hash) = entry.details.request_hash {
            hasher.update(req_hash.as_bytes());
        }
        if let Some(ref resp_hash) = entry.details.response_hash {
            hasher.update(resp_hash.as_bytes());
        }
        if let Some(tokens) = entry.details.tokens_consumed {
            hasher.update(&tokens.to_le_bytes());
        }
        if let Some(ref model) = entry.details.model {
            hasher.update(model.as_bytes());
        }

        format!("sha256:{}", hex::encode(hasher.finalize()))
    }

    /// Verify chain integrity for a sequence of entries
    pub fn verify_chain(entries: &[AuditLogEntry]) -> Result<(), ChainError> {
        if entries.is_empty() {
            return Ok(());
        }

        for i in 0..entries.len() {
            let curr = &entries[i];

            // Verify hash is correct
            let computed = Self::compute_entry_hash(curr);
            if computed != curr.hash {
                return Err(ChainError::HashInvalid {
                    sequence: curr.sequence,
                    expected: computed,
                    actual: curr.hash.clone(),
                });
            }

            // Verify chain linkage (skip first entry)
            if i > 0 {
                let prev = &entries[i - 1];

                // Check prev_hash matches previous entry's hash
                if curr.prev_hash != prev.hash {
                    return Err(ChainError::HashMismatch {
                        sequence: curr.sequence,
                        expected: prev.hash.clone(),
                        actual: curr.prev_hash.clone(),
                    });
                }

                // Check sequence is continuous
                if curr.sequence != prev.sequence + 1 {
                    return Err(ChainError::SequenceGap {
                        expected: prev.sequence + 1,
                        actual: curr.sequence,
                    });
                }
            }
        }

        Ok(())
    }

    /// Compute genesis hash
    fn compute_genesis_hash() -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"CHINJU_AUDIT_GENESIS_V1");
        // Use a fixed timestamp for reproducibility
        hasher.update(b"2026-01-01T00:00:00Z");
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }
}

impl Default for HashChainManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain verification errors
#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("Hash mismatch at sequence {sequence}: expected {expected}, got {actual}")]
    HashMismatch {
        sequence: u64,
        expected: String,
        actual: String,
    },

    #[error("Sequence gap: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },

    #[error("Invalid hash at sequence {sequence}: computed {expected}, stored {actual}")]
    HashInvalid {
        sequence: u64,
        expected: String,
        actual: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::audit::types::{
        Actor, AuditDetails, AuditEventType, AuditResult, Resource,
    };

    fn create_test_entry() -> AuditLogEntry {
        AuditLogEntry::builder()
            .event_type(AuditEventType::AiRequest)
            .source_id("test-sidecar")
            .request_id("req-123")
            .actor(Actor::human("user-456", Some(0.75)))
            .resource(Resource::ai_model("gpt-4"))
            .result(AuditResult::success())
            .details(AuditDetails::default().with_model("gpt-4"))
            .build()
    }

    #[tokio::test]
    async fn test_chain_entry() {
        let chain = HashChainManager::new();

        let mut entry1 = create_test_entry();
        chain.chain_entry(&mut entry1).await;

        assert_eq!(entry1.sequence, 0);
        assert!(!entry1.hash.is_empty());
        assert!(entry1.hash.starts_with("sha256:"));

        let mut entry2 = create_test_entry();
        chain.chain_entry(&mut entry2).await;

        assert_eq!(entry2.sequence, 1);
        assert_eq!(entry2.prev_hash, entry1.hash);
    }

    #[tokio::test]
    async fn test_verify_chain_valid() {
        let chain = HashChainManager::new();

        let mut entries = Vec::new();
        for _ in 0..5 {
            let mut entry = create_test_entry();
            chain.chain_entry(&mut entry).await;
            entries.push(entry);
        }

        let result = HashChainManager::verify_chain(&entries);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_chain_tampered() {
        let chain = HashChainManager::new();

        let mut entries = Vec::new();
        for _ in 0..3 {
            let mut entry = create_test_entry();
            chain.chain_entry(&mut entry).await;
            entries.push(entry);
        }

        // Tamper with an entry
        entries[1].details.tokens_consumed = Some(9999);

        let result = HashChainManager::verify_chain(&entries);
        assert!(matches!(result, Err(ChainError::HashInvalid { .. })));
    }

    #[test]
    fn test_genesis_hash_deterministic() {
        let hash1 = HashChainManager::compute_genesis_hash();
        let hash2 = HashChainManager::compute_genesis_hash();
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_from_state() {
        let chain = HashChainManager::from_state(100, "sha256:abc123".to_string());
        assert_eq!(chain.current_sequence(), 100);

        let mut entry = create_test_entry();
        chain.chain_entry(&mut entry).await;

        assert_eq!(entry.sequence, 100);
        assert_eq!(entry.prev_hash, "sha256:abc123");
    }
}
