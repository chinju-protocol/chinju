//! Policy Provider trait and common types
//!
//! This module defines the abstraction for loading policies from various sources.

use async_trait::async_trait;
use std::sync::Arc;

use crate::gen::chinju::common::Identifier;
use crate::gen::chinju::policy::{PolicyMetadata, PolicyPack};

/// Error type for policy provider operations
#[derive(Debug, Clone)]
pub enum PolicyProviderError {
    /// Policy not found
    NotFound(String),
    /// IO error
    IoError(String),
    /// Parse error
    ParseError(String),
    /// Signature verification failed
    SignatureInvalid(String),
    /// Provider unavailable
    Unavailable(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for PolicyProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "Policy not found: {}", id),
            Self::IoError(msg) => write!(f, "IO error: {}", msg),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::SignatureInvalid(msg) => write!(f, "Invalid signature: {}", msg),
            Self::Unavailable(msg) => write!(f, "Provider unavailable: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for PolicyProviderError {}

/// Policy update notification
#[derive(Debug, Clone)]
pub enum PolicyUpdate {
    /// Policy was added or updated
    Updated(PolicyPack),
    /// Policy was removed
    Removed(Identifier),
    /// Policy was revoked
    Revoked {
        policy_id: Identifier,
        reason: String,
    },
}

/// Trait for policy providers
///
/// A policy provider is responsible for loading policies from a specific source
/// (e.g., local files, remote registry, Git repository).
#[async_trait]
pub trait PolicyProvider: Send + Sync {
    /// Get the provider ID
    fn provider_id(&self) -> &str;

    /// Get the provider name for display
    fn provider_name(&self) -> &str;

    /// List available policies (metadata only)
    async fn list_policies(&self) -> Result<Vec<PolicyMetadata>, PolicyProviderError>;

    /// Get a specific policy by ID
    async fn get_policy(&self, id: &Identifier) -> Result<Option<PolicyPack>, PolicyProviderError>;

    /// Verify policy signature
    async fn verify_policy(&self, policy: &PolicyPack) -> Result<bool, PolicyProviderError>;

    /// Check if the provider is available
    async fn is_available(&self) -> bool;

    /// Refresh/reload policies from source
    async fn refresh(&self) -> Result<(), PolicyProviderError>;
}

/// Collection of policy providers
pub struct PolicyProviderRegistry {
    providers: Vec<Arc<dyn PolicyProvider>>,
}

impl PolicyProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Add a provider to the registry
    pub fn add_provider(&mut self, provider: Arc<dyn PolicyProvider>) {
        self.providers.push(provider);
    }

    /// Get all providers
    pub fn providers(&self) -> &[Arc<dyn PolicyProvider>] {
        &self.providers
    }

    /// Find a provider by ID
    pub fn get_provider(&self, id: &str) -> Option<Arc<dyn PolicyProvider>> {
        self.providers
            .iter()
            .find(|p| p.provider_id() == id)
            .cloned()
    }

    /// List all policies from all providers
    pub async fn list_all_policies(&self) -> Vec<(String, Vec<PolicyMetadata>)> {
        let mut results = Vec::new();

        for provider in &self.providers {
            if provider.is_available().await {
                if let Ok(policies) = provider.list_policies().await {
                    results.push((provider.provider_id().to_string(), policies));
                }
            }
        }

        results
    }

    /// Get a policy from any provider
    pub async fn get_policy(&self, id: &Identifier) -> Option<PolicyPack> {
        for provider in &self.providers {
            if provider.is_available().await {
                if let Ok(Some(policy)) = provider.get_policy(id).await {
                    return Some(policy);
                }
            }
        }
        None
    }
}

impl Default for PolicyProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry_creation() {
        let registry = PolicyProviderRegistry::new();
        assert!(registry.providers().is_empty());
    }
}
