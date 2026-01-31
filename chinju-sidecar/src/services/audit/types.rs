//! Audit log type definitions for C6 compliance
//!
//! Privacy protection: Request/response contents are NOT stored,
//! only their hashes are recorded for integrity verification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Audit log entry with hash chain support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Schema version for forward compatibility
    pub schema_version: String,

    /// Log ID (UUIDv7 for timestamp-sortable ordering)
    pub log_id: Uuid,

    /// Sequence number for gap detection
    pub sequence: u64,

    /// Timestamp in UTC
    pub timestamp: DateTime<Utc>,

    /// Hash of the previous entry (chain)
    pub prev_hash: String,

    /// Event type
    pub event_type: AuditEventType,

    /// Source identifier (sidecar instance)
    pub source_id: String,

    /// Request ID (correlation with GatewayService)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// Actor information
    pub actor: Actor,

    /// Resource information
    pub resource: Resource,

    /// Result information
    pub result: AuditResult,

    /// Detail data (content hashes only, not actual content)
    pub details: AuditDetails,

    /// Hash of this entry
    pub hash: String,

    /// Signature (only for critical events)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

impl AuditLogEntry {
    /// Create a new audit log entry builder
    pub fn builder() -> AuditLogEntryBuilder {
        AuditLogEntryBuilder::new()
    }
}

/// Builder for AuditLogEntry
pub struct AuditLogEntryBuilder {
    event_type: AuditEventType,
    source_id: String,
    request_id: Option<String>,
    actor: Actor,
    resource: Resource,
    result: AuditResult,
    details: AuditDetails,
}

impl AuditLogEntryBuilder {
    pub fn new() -> Self {
        Self {
            event_type: AuditEventType::AiRequest,
            source_id: String::new(),
            request_id: None,
            actor: Actor::default(),
            resource: Resource::default(),
            result: AuditResult::default(),
            details: AuditDetails::default(),
        }
    }

    pub fn event_type(mut self, event_type: AuditEventType) -> Self {
        self.event_type = event_type;
        self
    }

    pub fn source_id(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = source_id.into();
        self
    }

    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    pub fn actor(mut self, actor: Actor) -> Self {
        self.actor = actor;
        self
    }

    pub fn resource(mut self, resource: Resource) -> Self {
        self.resource = resource;
        self
    }

    pub fn result(mut self, result: AuditResult) -> Self {
        self.result = result;
        self
    }

    pub fn details(mut self, details: AuditDetails) -> Self {
        self.details = details;
        self
    }

    /// Build the entry (sequence, prev_hash, hash will be set by HashChainManager)
    pub fn build(self) -> AuditLogEntry {
        AuditLogEntry {
            schema_version: "audit_log_v1".to_string(),
            log_id: Uuid::now_v7(),
            sequence: 0,
            timestamp: Utc::now(),
            prev_hash: String::new(),
            event_type: self.event_type,
            source_id: self.source_id,
            request_id: self.request_id,
            actor: self.actor,
            resource: self.resource,
            result: self.result,
            details: self.details,
            hash: String::new(),
            signature: None,
        }
    }
}

impl Default for AuditLogEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Event types for audit logging
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AuditEventType {
    // AI Operations
    AiRequest,
    AiResponse,

    // Authentication
    Authentication,
    Authorization,

    // Token operations
    TokenGrant,
    TokenConsume,

    // Policy operations
    PolicyEvaluate,
    PolicyUpdate,

    // Security events
    AnomalyDetected,
    SecurityAlert,

    // System events
    ConfigChange,
    KeyRotation,
}

/// Actor information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Actor {
    pub actor_type: ActorType,
    pub actor_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_score: Option<f64>,
}

impl Actor {
    pub fn human(credential_id: impl Into<String>, capability_score: Option<f64>) -> Self {
        let id = credential_id.into();
        Self {
            actor_type: ActorType::Human,
            actor_id: id.clone(),
            credential_id: Some(id),
            capability_score,
        }
    }

    pub fn anonymous() -> Self {
        Self {
            actor_type: ActorType::Unknown,
            actor_id: "anonymous".to_string(),
            credential_id: None,
            capability_score: None,
        }
    }

    pub fn system(source_id: impl Into<String>) -> Self {
        Self {
            actor_type: ActorType::AiSystem,
            actor_id: source_id.into(),
            credential_id: None,
            capability_score: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    Human,
    AiSystem,
    Service,
    #[default]
    Unknown,
}

/// Resource information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Resource {
    pub resource_type: String,
    pub resource_id: String,
}

impl Resource {
    pub fn ai_model(model: impl Into<String>) -> Self {
        Self {
            resource_type: "ai_model".to_string(),
            resource_id: model.into(),
        }
    }

    pub fn ai_response(request_id: impl Into<String>) -> Self {
        Self {
            resource_type: "ai_response".to_string(),
            resource_id: request_id.into(),
        }
    }

    pub fn policy(policy_id: impl Into<String>) -> Self {
        Self {
            resource_type: "policy".to_string(),
            resource_id: policy_id.into(),
        }
    }
}

/// Result information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_decision: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub matched_rules: Vec<String>,
    pub duration_ms: u64,
}

impl AuditResult {
    pub fn success() -> Self {
        Self {
            success: true,
            ..Default::default()
        }
    }

    pub fn failure() -> Self {
        Self {
            success: false,
            ..Default::default()
        }
    }

    pub fn with_policy(mut self, decision: impl Into<String>, rules: Vec<String>) -> Self {
        self.policy_decision = Some(decision.into());
        self.matched_rules = rules;
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Detail data (privacy-preserving: only hashes, not content)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditDetails {
    /// Hash of request content (content itself is NOT stored)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_hash: Option<String>,

    /// Hash of response content (content itself is NOT stored)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_hash: Option<String>,

    /// Tokens consumed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_consumed: Option<u64>,

    /// Model ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Custom attributes
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty", default)]
    pub extra: HashMap<String, String>,
}

impl AuditDetails {
    pub fn with_request_hash(mut self, hash: impl Into<String>) -> Self {
        self.request_hash = Some(hash.into());
        self
    }

    pub fn with_response_hash(mut self, hash: impl Into<String>) -> Self {
        self.response_hash = Some(hash.into());
        self
    }

    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.tokens_consumed = Some(tokens);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

/// Compute SHA-256 hash of content (for privacy-preserving logging)
pub fn compute_content_hash(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry_builder() {
        let entry = AuditLogEntry::builder()
            .event_type(AuditEventType::AiRequest)
            .source_id("test-sidecar")
            .request_id("req-123")
            .actor(Actor::human("user-456", Some(0.75)))
            .resource(Resource::ai_model("gpt-4"))
            .result(AuditResult::success())
            .details(
                AuditDetails::default()
                    .with_request_hash("sha256:abc123")
                    .with_model("gpt-4"),
            )
            .build();

        assert_eq!(entry.event_type, AuditEventType::AiRequest);
        assert_eq!(entry.source_id, "test-sidecar");
        assert_eq!(entry.request_id, Some("req-123".to_string()));
        assert_eq!(entry.actor.actor_type, ActorType::Human);
        assert_eq!(entry.actor.capability_score, Some(0.75));
    }

    #[test]
    fn test_content_hash() {
        let content = b"Hello, World!";
        let hash = compute_content_hash(content);
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn test_serialization() {
        let entry = AuditLogEntry::builder()
            .event_type(AuditEventType::AiRequest)
            .source_id("test")
            .build();

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("AI_REQUEST"));

        let parsed: AuditLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_type, AuditEventType::AiRequest);
    }
}
