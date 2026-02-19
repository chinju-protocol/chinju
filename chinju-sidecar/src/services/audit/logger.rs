//! Audit logger frontend
//!
//! Provides a simple API for logging audit events.
//! Entries are sent to a background persister via mpsc channel.

use crate::ids::{CredentialId, RequestId};
use crate::services::audit::chain::HashChainManager;
use crate::services::audit::types::{
    compute_content_hash, Actor, AuditDetails, AuditEventType, AuditLogEntry, AuditResult, Resource,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, warn};

/// Audit logger for logging events asynchronously
pub struct AuditLogger {
    /// Channel sender for async persistence
    sender: mpsc::Sender<AuditLogEntry>,
    /// Hash chain manager
    chain: Arc<HashChainManager>,
    /// Source ID (sidecar instance identifier)
    source_id: String,
}

impl AuditLogger {
    /// Create a new audit logger
    pub fn new(
        sender: mpsc::Sender<AuditLogEntry>,
        chain: Arc<HashChainManager>,
        source_id: impl Into<String>,
    ) -> Self {
        Self {
            sender,
            chain,
            source_id: source_id.into(),
        }
    }

    /// Log an AI request
    pub async fn log_ai_request(
        &self,
        request_id: &RequestId,
        credential_id: Option<&CredentialId>,
        capability_score: Option<f64>,
        request_content: &[u8],
        model: &str,
    ) -> Result<String, AuditError> {
        let request_hash = compute_content_hash(request_content);

        let actor = match credential_id {
            Some(id) => Actor::human(id.as_str(), capability_score),
            None => Actor::anonymous(),
        };

        let mut entry = AuditLogEntry::builder()
            .event_type(AuditEventType::AiRequest)
            .source_id(&self.source_id)
            .request_id(request_id.as_str())
            .actor(actor)
            .resource(Resource::ai_model(model))
            .result(AuditResult::success())
            .details(
                AuditDetails::default()
                    .with_request_hash(&request_hash)
                    .with_model(model),
            )
            .build();

        self.chain.chain_entry(&mut entry).await;
        let audit_log_id = format!("audit_{}", entry.log_id);

        self.send_entry(entry).await?;
        debug!(audit_log_id = %audit_log_id, request_id = %request_id, "Logged AI request");

        Ok(audit_log_id)
    }

    /// Log an AI response
    pub async fn log_ai_response(
        &self,
        request_id: &RequestId,
        response_content: &[u8],
        policy_decision: &str,
        matched_rules: &[String],
        tokens_consumed: u64,
        duration_ms: u64,
        success: bool,
    ) -> Result<String, AuditError> {
        let response_hash = compute_content_hash(response_content);

        let mut entry = AuditLogEntry::builder()
            .event_type(AuditEventType::AiResponse)
            .source_id(&self.source_id)
            .request_id(request_id.as_str())
            .actor(Actor::system(&self.source_id))
            .resource(Resource::ai_response(request_id.as_str()))
            .result(
                if success {
                    AuditResult::success()
                } else {
                    AuditResult::failure()
                }
                .with_policy(policy_decision, matched_rules.to_vec())
                .with_duration(duration_ms),
            )
            .details(
                AuditDetails::default()
                    .with_response_hash(&response_hash)
                    .with_tokens(tokens_consumed),
            )
            .build();

        self.chain.chain_entry(&mut entry).await;
        let audit_log_id = format!("audit_{}", entry.log_id);

        self.send_entry(entry).await?;
        debug!(
            audit_log_id = %audit_log_id,
            request_id = %request_id,
            tokens = tokens_consumed,
            "Logged AI response"
        );

        Ok(audit_log_id)
    }

    /// Log a policy evaluation
    pub async fn log_policy_evaluate(
        &self,
        request_id: &RequestId,
        policy_id: &str,
        decision: &str,
        matched_rules: &[String],
    ) -> Result<String, AuditError> {
        let mut entry = AuditLogEntry::builder()
            .event_type(AuditEventType::PolicyEvaluate)
            .source_id(&self.source_id)
            .request_id(request_id.as_str())
            .actor(Actor::system(&self.source_id))
            .resource(Resource::policy(policy_id))
            .result(AuditResult::success().with_policy(decision, matched_rules.to_vec()))
            .details(AuditDetails::default())
            .build();

        self.chain.chain_entry(&mut entry).await;
        let audit_log_id = format!("audit_{}", entry.log_id);

        self.send_entry(entry).await?;
        debug!(
            audit_log_id = %audit_log_id,
            request_id = %request_id,
            decision = %decision,
            "Logged policy evaluation"
        );

        Ok(audit_log_id)
    }

    /// Log token consumption
    pub async fn log_token_consume(
        &self,
        request_id: &RequestId,
        credential_id: Option<&CredentialId>,
        amount: u64,
        balance_before: u64,
        balance_after: u64,
    ) -> Result<String, AuditError> {
        let actor = match credential_id {
            Some(id) => Actor::human(id.as_str(), None),
            None => Actor::anonymous(),
        };

        let mut details = AuditDetails::default().with_tokens(amount);
        details
            .extra
            .insert("balance_before".to_string(), balance_before.to_string());
        details
            .extra
            .insert("balance_after".to_string(), balance_after.to_string());

        let mut entry = AuditLogEntry::builder()
            .event_type(AuditEventType::TokenConsume)
            .source_id(&self.source_id)
            .request_id(request_id.as_str())
            .actor(actor)
            .resource(Resource {
                resource_type: "token".to_string(),
                resource_id: "survival_token".to_string(),
            })
            .result(AuditResult::success())
            .details(details)
            .build();

        self.chain.chain_entry(&mut entry).await;
        let audit_log_id = format!("audit_{}", entry.log_id);

        self.send_entry(entry).await?;
        debug!(
            audit_log_id = %audit_log_id,
            request_id = %request_id,
            amount = amount,
            "Logged token consumption"
        );

        Ok(audit_log_id)
    }

    /// Log a security alert
    pub async fn log_security_alert(
        &self,
        request_id: Option<&RequestId>,
        alert_type: &str,
        description: &str,
        credential_id: Option<&CredentialId>,
    ) -> Result<String, AuditError> {
        let actor = match credential_id {
            Some(id) => Actor::human(id.as_str(), None),
            None => Actor::anonymous(),
        };

        let mut details = AuditDetails::default();
        details
            .extra
            .insert("alert_type".to_string(), alert_type.to_string());
        details
            .extra
            .insert("description".to_string(), description.to_string());

        let mut entry = AuditLogEntry::builder()
            .event_type(AuditEventType::SecurityAlert)
            .source_id(&self.source_id)
            .actor(actor)
            .resource(Resource {
                resource_type: "security".to_string(),
                resource_id: alert_type.to_string(),
            })
            .result(AuditResult::failure())
            .details(details)
            .build();

        if let Some(req_id) = request_id {
            entry.request_id = Some(req_id.to_string());
        }

        self.chain.chain_entry(&mut entry).await;
        let audit_log_id = format!("audit_{}", entry.log_id);

        self.send_entry(entry).await?;
        warn!(
            audit_log_id = %audit_log_id,
            alert_type = %alert_type,
            "Logged security alert"
        );

        Ok(audit_log_id)
    }

    /// Send an entry to the persister
    async fn send_entry(&self, entry: AuditLogEntry) -> Result<(), AuditError> {
        self.sender
            .send(entry)
            .await
            .map_err(|_| AuditError::ChannelClosed)
    }
}

/// Audit errors
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Audit channel closed")]
    ChannelClosed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_ai_request() {
        let (tx, mut rx) = mpsc::channel(100);
        let chain = Arc::new(HashChainManager::new());
        let logger = AuditLogger::new(tx, chain, "test-sidecar");

        let result = logger
            .log_ai_request(
                &RequestId::new("req-123").unwrap(),
                Some(&CredentialId::new("user-456").unwrap()),
                Some(0.75),
                b"Hello, World!",
                "gpt-4",
            )
            .await;

        assert!(result.is_ok());
        let audit_id = result.unwrap();
        assert!(audit_id.starts_with("audit_"));

        let entry = rx.recv().await.unwrap();
        assert_eq!(entry.event_type, AuditEventType::AiRequest);
        assert_eq!(entry.request_id, Some("req-123".to_string()));
        assert_eq!(entry.actor.credential_id, Some("user-456".to_string()));
        assert!(entry.details.request_hash.is_some());
    }

    #[tokio::test]
    async fn test_log_ai_response() {
        let (tx, mut rx) = mpsc::channel(100);
        let chain = Arc::new(HashChainManager::new());
        let logger = AuditLogger::new(tx, chain, "test-sidecar");

        let result = logger
            .log_ai_response(
                &RequestId::new("req-123").unwrap(),
                b"Response content",
                "allow",
                &["rule1".to_string(), "rule2".to_string()],
                150,
                100,
                true,
            )
            .await;

        assert!(result.is_ok());

        let entry = rx.recv().await.unwrap();
        assert_eq!(entry.event_type, AuditEventType::AiResponse);
        assert_eq!(entry.result.policy_decision, Some("allow".to_string()));
        assert_eq!(entry.details.tokens_consumed, Some(150));
    }

    #[tokio::test]
    async fn test_hash_chain_continuity() {
        let (tx, mut rx) = mpsc::channel(100);
        let chain = Arc::new(HashChainManager::new());
        let logger = AuditLogger::new(tx, chain.clone(), "test-sidecar");

        // Log multiple entries
        logger
            .log_ai_request(
                &RequestId::new("req-1").unwrap(),
                None,
                None,
                b"content1",
                "gpt-4",
            )
            .await
            .unwrap();
        logger
            .log_ai_request(
                &RequestId::new("req-2").unwrap(),
                None,
                None,
                b"content2",
                "gpt-4",
            )
            .await
            .unwrap();

        let entry1 = rx.recv().await.unwrap();
        let entry2 = rx.recv().await.unwrap();

        assert_eq!(entry1.sequence, 0);
        assert_eq!(entry2.sequence, 1);
        assert_eq!(entry2.prev_hash, entry1.hash);
    }
}
