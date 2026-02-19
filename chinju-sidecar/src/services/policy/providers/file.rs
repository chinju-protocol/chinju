//! File-based policy provider
//!
//! Loads policies from JSON files in a directory.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::gen::chinju::common::Identifier;
use crate::gen::chinju::policy::{PolicyMetadata, PolicyPack};

use super::super::provider::{PolicyProvider, PolicyProviderError};

/// File-based policy provider
///
/// Loads policies from a directory containing JSON files.
/// Each policy should be in a separate file named `{namespace}.{id}.v{version}.json`
/// or simply `{filename}.json`.
pub struct FileProvider {
    /// Provider ID
    id: String,
    /// Directory containing policy files
    directory: PathBuf,
    /// Cached policies
    cache: Arc<RwLock<HashMap<String, PolicyPack>>>,
    /// Whether signature verification is required
    verify_signatures: bool,
}

impl FileProvider {
    /// Create a new file provider
    pub fn new(id: impl Into<String>, directory: impl AsRef<Path>) -> Self {
        Self {
            id: id.into(),
            directory: directory.as_ref().to_path_buf(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            verify_signatures: false,
        }
    }

    /// Enable signature verification
    pub fn with_signature_verification(mut self, enabled: bool) -> Self {
        self.verify_signatures = enabled;
        self
    }

    /// Load all policies from the directory
    async fn load_policies(&self) -> Result<Vec<PolicyPack>, PolicyProviderError> {
        if !self.directory.exists() {
            return Err(PolicyProviderError::IoError(format!(
                "Directory does not exist: {}",
                self.directory.display()
            )));
        }

        let mut policies = Vec::new();

        let entries = std::fs::read_dir(&self.directory).map_err(|e| {
            PolicyProviderError::IoError(format!(
                "Failed to read directory {}: {}",
                self.directory.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                PolicyProviderError::IoError(format!("Failed to read entry: {}", e))
            })?;

            let path = entry.path();

            // Only process JSON files
            if path.extension().map_or(false, |ext| ext == "json") {
                match self.load_policy_file(&path).await {
                    Ok(policy) => {
                        debug!("Loaded policy from {:?}", path);
                        policies.push(policy);
                    }
                    Err(e) => {
                        warn!("Failed to load policy from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(policies)
    }

    /// Load a single policy file
    ///
    /// Currently supports protobuf binary format (.bin) only.
    /// JSON support requires serde feature on prost-generated types.
    async fn load_policy_file(&self, path: &Path) -> Result<PolicyPack, PolicyProviderError> {
        use prost::Message;

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "bin" => {
                // Load protobuf binary format
                let content = std::fs::read(path).map_err(|e| {
                    PolicyProviderError::IoError(format!(
                        "Failed to read file {}: {}",
                        path.display(),
                        e
                    ))
                })?;

                PolicyPack::decode(&content[..]).map_err(|e| {
                    PolicyProviderError::ParseError(format!(
                        "Failed to decode protobuf from {}: {}",
                        path.display(),
                        e
                    ))
                })
            }
            "json" => {
                // TODO: JSON support requires serde feature on generated types
                // For now, return an error
                Err(PolicyProviderError::ParseError(format!(
                    "JSON format not yet supported for {}: enable serde feature on prost types",
                    path.display()
                )))
            }
            _ => Err(PolicyProviderError::ParseError(format!(
                "Unknown file format: {}",
                path.display()
            ))),
        }
    }

    /// Get the cache key for a policy identifier
    fn cache_key(id: &Identifier) -> String {
        format!("{}.{}.v{}", id.namespace, id.id, id.version)
    }
}

#[async_trait]
impl PolicyProvider for FileProvider {
    fn provider_id(&self) -> &str {
        &self.id
    }

    fn provider_name(&self) -> &str {
        "File Provider"
    }

    async fn list_policies(&self) -> Result<Vec<PolicyMetadata>, PolicyProviderError> {
        let cache = self.cache.read().await;

        // If cache is empty, load from files
        if cache.is_empty() {
            drop(cache);
            self.refresh().await?;
            let cache = self.cache.read().await;
            return Ok(cache.values().filter_map(|p| p.metadata.clone()).collect());
        }

        Ok(cache.values().filter_map(|p| p.metadata.clone()).collect())
    }

    async fn get_policy(&self, id: &Identifier) -> Result<Option<PolicyPack>, PolicyProviderError> {
        let key = Self::cache_key(id);

        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(policy) = cache.get(&key) {
                return Ok(Some(policy.clone()));
            }
        }

        // Try loading from file
        let filename = format!("{}.json", key);
        let path = self.directory.join(&filename);

        if path.exists() {
            let policy = self.load_policy_file(&path).await?;

            // Cache it
            let mut cache = self.cache.write().await;
            cache.insert(key, policy.clone());

            return Ok(Some(policy));
        }

        Ok(None)
    }

    async fn verify_policy(&self, policy: &PolicyPack) -> Result<bool, PolicyProviderError> {
        if !self.verify_signatures {
            // Skip verification if not required
            return Ok(true);
        }

        // Check if policy has a signature
        if policy.signature.is_none() {
            return Err(PolicyProviderError::SignatureInvalid(
                "Policy has no signature".to_string(),
            ));
        }

        // TODO: Implement actual signature verification using ThresholdVerifier
        // For now, just check that signature exists
        Ok(true)
    }

    async fn is_available(&self) -> bool {
        self.directory.exists()
    }

    async fn refresh(&self) -> Result<(), PolicyProviderError> {
        info!("Refreshing policies from {:?}", self.directory);

        let policies = self.load_policies().await?;

        let mut cache = self.cache.write().await;
        cache.clear();

        for policy in policies {
            if let Some(ref id) = policy.policy_id {
                let key = Self::cache_key(id);
                cache.insert(key, policy);
            }
        }

        info!("Loaded {} policies", cache.len());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_provider_empty_directory() {
        let dir = tempdir().unwrap();
        let provider = FileProvider::new("test", dir.path());

        assert!(provider.is_available().await);

        let policies = provider.list_policies().await.unwrap();
        assert!(policies.is_empty());
    }

    #[tokio::test]
    async fn test_file_provider_nonexistent_directory() {
        let provider = FileProvider::new("test", "/nonexistent/path");

        assert!(!provider.is_available().await);
    }
}
