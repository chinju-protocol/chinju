//! Emergency policy revocation
//!
//! This module provides functionality for revoking policies and
//! propagating revocation notices.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::gen::chinju::common::{Identifier, ThresholdSignature};

/// Reason for policy revocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevokeReason {
    /// Security vulnerability discovered
    SecurityVulnerability,
    /// Policy conflicts with regulations
    RegulatoryConflict,
    /// Policy was issued in error
    IssuedInError,
    /// Policy has been superseded
    Superseded,
    /// Manual administrative revocation
    Administrative,
    /// Other reason with description
    Other(String),
}

impl std::fmt::Display for RevokeReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SecurityVulnerability => write!(f, "Security vulnerability"),
            Self::RegulatoryConflict => write!(f, "Regulatory conflict"),
            Self::IssuedInError => write!(f, "Issued in error"),
            Self::Superseded => write!(f, "Superseded"),
            Self::Administrative => write!(f, "Administrative"),
            Self::Other(desc) => write!(f, "Other: {}", desc),
        }
    }
}

/// Policy identifier for revocation (serde-compatible)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PolicyIdentifier {
    pub namespace: String,
    pub id: String,
    pub version: u64,
}

impl From<&Identifier> for PolicyIdentifier {
    fn from(id: &Identifier) -> Self {
        Self {
            namespace: id.namespace.clone(),
            id: id.id.clone(),
            version: id.version,
        }
    }
}

impl From<Identifier> for PolicyIdentifier {
    fn from(id: Identifier) -> Self {
        Self {
            namespace: id.namespace,
            id: id.id,
            version: id.version,
        }
    }
}

/// Revocation notice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationNotice {
    /// Policy being revoked
    pub policy_id: PolicyIdentifier,
    /// Reason for revocation
    pub reason: RevokeReason,
    /// When the revocation was issued (Unix timestamp)
    pub revoked_at_seconds: i64,
    /// Authority signature (threshold) - not serialized
    #[serde(skip)]
    pub authority_signature: Option<ThresholdSignature>,
    /// Propagation ID for tracking
    pub propagation_id: String,
    /// Additional notes
    pub notes: Option<String>,
}

impl RevocationNotice {
    /// Create a new revocation notice
    pub fn new(policy_id: impl Into<PolicyIdentifier>, reason: RevokeReason) -> Self {
        Self {
            policy_id: policy_id.into(),
            reason,
            revoked_at_seconds: chrono::Utc::now().timestamp(),
            authority_signature: None,
            propagation_id: uuid::Uuid::new_v4().to_string(),
            notes: None,
        }
    }

    /// Add notes to the notice
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    /// Set the authority signature
    pub fn with_signature(mut self, signature: ThresholdSignature) -> Self {
        self.authority_signature = Some(signature);
        self
    }

    /// Get the policy key
    pub fn policy_key(&self) -> String {
        format!(
            "{}.{}.v{}",
            self.policy_id.namespace, self.policy_id.id, self.policy_id.version
        )
    }
}

/// Cache of revoked policies
pub struct RevocationCache {
    /// Set of revoked policy keys
    revoked: Arc<RwLock<HashSet<String>>>,
    /// Stored revocation notices
    notices: Arc<RwLock<Vec<RevocationNotice>>>,
}

impl RevocationCache {
    /// Create a new revocation cache
    pub fn new() -> Self {
        Self {
            revoked: Arc::new(RwLock::new(HashSet::new())),
            notices: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a revocation notice
    pub async fn add_revocation(&self, notice: RevocationNotice) {
        let key = notice.policy_key();

        let mut revoked = self.revoked.write().await;
        revoked.insert(key);

        let mut notices = self.notices.write().await;
        notices.push(notice);
    }

    /// Check if a policy is revoked
    pub async fn is_revoked(&self, policy_id: &Identifier) -> bool {
        let key = format!(
            "{}.{}.v{}",
            policy_id.namespace, policy_id.id, policy_id.version
        );

        let revoked = self.revoked.read().await;
        revoked.contains(&key)
    }

    /// Get revocation notice for a policy
    pub async fn get_notice(&self, policy_id: &Identifier) -> Option<RevocationNotice> {
        let key = format!(
            "{}.{}.v{}",
            policy_id.namespace, policy_id.id, policy_id.version
        );

        let notices = self.notices.read().await;
        notices.iter().find(|n| n.policy_key() == key).cloned()
    }

    /// List all revoked policy keys
    pub async fn list_revoked(&self) -> Vec<String> {
        let revoked = self.revoked.read().await;
        revoked.iter().cloned().collect()
    }

    /// Get all notices
    pub async fn all_notices(&self) -> Vec<RevocationNotice> {
        let notices = self.notices.read().await;
        notices.clone()
    }
}

impl Default for RevocationCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Revocation propagator for distributing revocation notices
pub struct RevocationPropagator {
    cache: Arc<RevocationCache>,
}

impl RevocationPropagator {
    /// Create a new propagator
    pub fn new(cache: Arc<RevocationCache>) -> Self {
        Self { cache }
    }

    /// Broadcast a revocation notice
    ///
    /// In a full implementation, this would send to peer nodes.
    /// For now, it just adds to the local cache.
    pub async fn broadcast(&self, notice: RevocationNotice) {
        tracing::warn!(
            "Broadcasting revocation for policy {}: {}",
            notice.policy_key(),
            notice.reason
        );

        self.cache.add_revocation(notice).await;

        // TODO: In a distributed system, send to peers:
        // for peer in &self.peers {
        //     peer.send_revocation(&notice).await;
        // }
    }

    /// Receive a revocation notice from a peer
    pub async fn receive(&self, notice: RevocationNotice) -> Result<(), String> {
        // Verify the notice has a valid signature
        if notice.authority_signature.is_none() {
            return Err("Revocation notice has no authority signature".to_string());
        }

        // TODO: Verify the signature using ThresholdVerifier

        tracing::info!(
            "Received revocation for policy {}: {}",
            notice.policy_key(),
            notice.reason
        );

        self.cache.add_revocation(notice).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_revocation_cache() {
        let cache = RevocationCache::new();

        let notice = RevocationNotice::new(
            PolicyIdentifier {
                namespace: "test".to_string(),
                id: "policy1".to_string(),
                version: 1,
            },
            RevokeReason::SecurityVulnerability,
        );

        cache.add_revocation(notice).await;

        let policy_id = Identifier {
            namespace: "test".to_string(),
            id: "policy1".to_string(),
            version: 1,
        };

        assert!(cache.is_revoked(&policy_id).await);
    }

    #[test]
    fn test_revoke_reason_display() {
        assert_eq!(
            RevokeReason::SecurityVulnerability.to_string(),
            "Security vulnerability"
        );
        assert_eq!(
            RevokeReason::Other("Custom reason".to_string()).to_string(),
            "Other: Custom reason"
        );
    }
}
