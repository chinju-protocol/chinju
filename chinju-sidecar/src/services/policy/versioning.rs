//! Policy versioning management
//!
//! This module provides version tracking and management for policies.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::gen::chinju::common::{Identifier, Timestamp};
use crate::gen::chinju::policy::PolicyPack;

/// Policy state (simplified local version)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionState {
    Draft,
    Active,
    Published,
    Superseded,
    Revoked,
}

/// Policy version information
#[derive(Debug, Clone)]
pub struct PolicyVersion {
    /// Policy identifier
    pub policy_id: Identifier,
    /// Version number
    pub version: u64,
    /// Content hash for integrity verification
    pub content_hash: Vec<u8>,
    /// Parent version (if this is an update)
    pub parent_version: Option<Identifier>,
    /// When this version becomes effective
    pub effective_from: Option<Timestamp>,
    /// When this version expires
    pub effective_until: Option<Timestamp>,
    /// Current state
    pub state: VersionState,
    /// When this version was created
    pub created_at: Timestamp,
}

impl PolicyVersion {
    /// Create a new version from a policy
    pub fn from_policy(policy: &PolicyPack, content_hash: Vec<u8>) -> Option<Self> {
        let policy_id = policy.policy_id.clone()?;

        Some(Self {
            policy_id: policy_id.clone(),
            version: policy_id.version,
            content_hash,
            parent_version: policy.parent_policy_id.clone(),
            effective_from: policy.validity.as_ref().and_then(|v| v.not_before.clone()),
            effective_until: policy.validity.as_ref().and_then(|v| v.not_after.clone()),
            state: VersionState::Active, // Default to Active
            created_at: policy
                .metadata
                .as_ref()
                .and_then(|m| m.created_at.clone())
                .unwrap_or_else(|| Timestamp {
                    seconds: chrono::Utc::now().timestamp(),
                    nanos: 0,
                }),
        })
    }

    /// Check if this version is currently effective
    pub fn is_effective(&self) -> bool {
        let now = chrono::Utc::now().timestamp();

        // Check state
        if self.state != VersionState::Active && self.state != VersionState::Published {
            return false;
        }

        // Check effective_from
        if let Some(ref from) = self.effective_from {
            if from.seconds > now {
                return false;
            }
        }

        // Check effective_until
        if let Some(ref until) = self.effective_until {
            if until.seconds < now {
                return false;
            }
        }

        true
    }

    /// Get the version key
    pub fn version_key(&self) -> String {
        format!(
            "{}.{}.v{}",
            self.policy_id.namespace, self.policy_id.id, self.version
        )
    }
}

/// Store for tracking policy versions
pub struct PolicyVersionStore {
    /// All known versions
    versions: Arc<RwLock<HashMap<String, PolicyVersion>>>,
    /// Currently active version per policy (namespace.id -> version key)
    active: Arc<RwLock<HashMap<String, String>>>,
}

impl PolicyVersionStore {
    /// Create a new version store
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(HashMap::new())),
            active: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new policy version
    pub async fn register_version(&self, version: PolicyVersion) {
        let version_key = version.version_key();
        let policy_key = format!("{}.{}", version.policy_id.namespace, version.policy_id.id);

        let mut versions = self.versions.write().await;
        versions.insert(version_key.clone(), version.clone());

        // Update active if this version is effective and newer
        if version.is_effective() {
            let mut active = self.active.write().await;
            if let Some(current_key) = active.get(&policy_key) {
                if let Some(current) = versions.get(current_key) {
                    if version.version > current.version {
                        active.insert(policy_key, version_key);
                    }
                }
            } else {
                active.insert(policy_key, version_key);
            }
        }
    }

    /// Get a specific version
    pub async fn get_version(&self, version_key: &str) -> Option<PolicyVersion> {
        let versions = self.versions.read().await;
        versions.get(version_key).cloned()
    }

    /// Get the active version for a policy
    pub async fn get_active_version(&self, namespace: &str, id: &str) -> Option<PolicyVersion> {
        let policy_key = format!("{}.{}", namespace, id);

        let active = self.active.read().await;
        let version_key = active.get(&policy_key)?;

        let versions = self.versions.read().await;
        versions.get(version_key).cloned()
    }

    /// List all versions for a policy
    pub async fn list_versions(&self, namespace: &str, id: &str) -> Vec<PolicyVersion> {
        let prefix = format!("{}.{}", namespace, id);

        let versions = self.versions.read().await;
        versions
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v.clone())
            .collect()
    }

    /// Mark a version as superseded
    pub async fn mark_superseded(&self, version_key: &str) {
        let mut versions = self.versions.write().await;
        if let Some(version) = versions.get_mut(version_key) {
            version.state = VersionState::Superseded;
        }
    }

    /// Mark a version as revoked
    pub async fn mark_revoked(&self, version_key: &str) {
        let mut versions = self.versions.write().await;
        if let Some(version) = versions.get_mut(version_key) {
            version.state = VersionState::Revoked;
        }

        // Remove from active if it was active
        let mut active = self.active.write().await;
        active.retain(|_, v| v != version_key);
    }
}

impl Default for PolicyVersionStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_version_store() {
        let store = PolicyVersionStore::new();

        let version = PolicyVersion {
            policy_id: Identifier {
                namespace: "test".to_string(),
                id: "policy1".to_string(),
                version: 1,
            },
            version: 1,
            content_hash: vec![1, 2, 3],
            parent_version: None,
            effective_from: None,
            effective_until: None,
            state: VersionState::Active,
            created_at: Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            },
        };

        store.register_version(version.clone()).await;

        let retrieved = store.get_version("test.policy1.v1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().version, 1);
    }
}
