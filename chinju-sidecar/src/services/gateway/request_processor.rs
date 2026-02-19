//! Request Processing Logic
//!
//! Handles the core request processing pipeline:
//! - Credential verification
//! - Policy evaluation
//! - AI response generation
//! - Response post-processing

use crate::config::ContainmentConfig;
use crate::constants::{mock, policy};
use crate::error::{ChinjuError, GatewayError};
use crate::gen::chinju::api::credential::VerifyOptions;
use crate::gen::chinju::api::gateway::*;
use crate::gen::chinju::common::Severity;
use crate::gen::chinju::credential::HumanCredential;
use crate::gen::chinju::policy::DecisionType;
use crate::ids::{CredentialId, RequestId, UserId};
use crate::services::audit::compute_content_hash;
use crate::services::extraction_deterrent::ExtractionDeterrent;
use crate::services::lpt_monitor::{LptMonitor, LptState, ResponseRecord};
use crate::services::openai_client::OpenAiClient;
use crate::services::openai_types::{ChatCompletionRequest, ChatMessage};
use crate::services::sanitizer::OutputSanitizer;
use crate::services::{CredentialServiceImpl, PolicyEngine, RequestContext, TokenService};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::Status;
use tracing::{debug, warn};

/// Request processing context
pub struct ProcessingContext<'a> {
    pub request_id: &'a RequestId,
    pub credential: Option<&'a HumanCredential>,
    pub payload: &'a AiRequestPayload,
    pub start_time: std::time::Instant,
}

/// Request processing result
pub struct ProcessingResult {
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub credential_valid: bool,
    pub policy_decision: crate::gen::chinju::policy::PolicyDecision,
    pub audit_log_id: String,
}

/// Handles credential verification
pub struct CredentialVerifier {
    credential_service: Arc<CredentialServiceImpl>,
}

impl CredentialVerifier {
    pub fn new(credential_service: Arc<CredentialServiceImpl>) -> Self {
        Self { credential_service }
    }

    /// Verify a credential
    pub fn verify(&self, credential: Option<&HumanCredential>, request_id: &RequestId) -> bool {
        if let Some(cred) = credential {
            let verify_result = self.credential_service.verify_credential_internal(
                cred,
                &VerifyOptions {
                    skip_revocation_check: false,
                    min_capability_score: policy::MIN_CAPABILITY_SCORE,
                    min_chain_length: crate::constants::security::MIN_CHAIN_LENGTH as u64,
                    require_hardware_attestation: false,
                },
            );
            if !verify_result.valid {
                warn!(
                    request_id = %request_id,
                    errors = ?verify_result.errors,
                    "Credential verification failed"
                );
            }
            verify_result.valid
        } else {
            warn!(request_id = %request_id, "No credential provided");
            false
        }
    }

    /// Extract credential ID
    pub fn extract_credential_id(credential: Option<&HumanCredential>) -> Option<CredentialId> {
        credential
            .and_then(|c| c.subject_id.as_ref())
            .and_then(|id| CredentialId::new(id.id.clone()).ok())
    }

    /// Extract capability score
    pub fn extract_capability_score(credential: Option<&HumanCredential>) -> Option<f64> {
        credential
            .and_then(|c| c.capability.as_ref())
            .map(|cap| cap.total)
    }
}

/// Handles policy evaluation
pub struct PolicyEvaluator {
    policy_engine: Arc<PolicyEngine>,
}

impl PolicyEvaluator {
    pub fn new(policy_engine: Arc<PolicyEngine>) -> Self {
        Self { policy_engine }
    }

    /// Evaluate request against policies
    pub async fn evaluate(
        &self,
        context: &ProcessingContext<'_>,
    ) -> Result<crate::gen::chinju::policy::PolicyDecision, Status> {
        let request_context = RequestContext {
            request_id: context.request_id.clone(),
            credential: context.credential.cloned(),
            payload: Some(context.payload.clone()),
            client_ip: None,
            jurisdiction: None,
            attributes: HashMap::new(),
        };

        let decision = self.policy_engine.evaluate(&request_context).await;

        // Check policy decision
        match decision.decision() {
            DecisionType::DecisionDeny => {
                warn!(
                    request_id = %context.request_id,
                    reason = %decision.reason,
                    "Request denied by policy"
                );
                return Err(Status::permission_denied(decision.reason.clone()));
            }
            DecisionType::DecisionThrottle => {
                tracing::info!(
                    request_id = %context.request_id,
                    "Request throttled by policy"
                );
                // In production: implement actual rate limiting
            }
            DecisionType::DecisionEscalate => {
                warn!(
                    request_id = %context.request_id,
                    "Request requires escalation"
                );
                return Err(Status::failed_precondition(
                    "Request requires human escalation",
                ));
            }
            _ => {}
        }

        Ok(decision)
    }
}

/// Handles AI response generation
pub struct ResponseGenerator {
    openai_client: Option<Arc<OpenAiClient>>,
}

impl ResponseGenerator {
    pub fn new(openai_client: Option<Arc<OpenAiClient>>) -> Self {
        Self { openai_client }
    }

    /// Generate AI response (mock or real)
    pub async fn generate(
        &self,
        payload: &AiRequestPayload,
    ) -> Result<(String, u32, u32), ChinjuError> {
        if self.openai_client.is_some() {
            self.real_response(payload).await
        } else {
            Ok(self.mock_response(payload))
        }
    }

    /// Generate mock response
    fn mock_response(&self, payload: &AiRequestPayload) -> (String, u32, u32) {
        let model = &payload.model;
        let message_count = payload.messages.len();

        let content = format!(
            "[CHINJU Mock Response]\n\
             Model: {}\n\
             Messages received: {}\n\
             This is a mock response. Set OPENAI_API_KEY to enable real API calls.\n\
             CHINJU Protocol ensures safe AI operation through:\n\
             - Human credential verification\n\
             - Token-based resource control\n\
             - Policy enforcement\n\
             - Comprehensive audit logging",
            model, message_count
        );

        (content, mock::PROMPT_TOKENS, mock::COMPLETION_TOKENS)
    }

    /// Generate real response via OpenAI API
    async fn real_response(
        &self,
        payload: &AiRequestPayload,
    ) -> Result<(String, u32, u32), ChinjuError> {
        let client = self
            .openai_client
            .as_ref()
            .ok_or_else(|| ChinjuError::from(GatewayError::ServiceUnavailable))?;

        let openai_request = ChatCompletionRequest {
            model: payload.model.clone(),
            messages: payload
                .messages
                .iter()
                .map(|m| ChatMessage {
                    role: m.role.clone(),
                    content: m.content.clone(),
                    name: if m.name.is_empty() {
                        None
                    } else {
                        Some(m.name.clone())
                    },
                })
                .collect(),
            temperature: payload.parameters.as_ref().map(|p| p.temperature),
            max_tokens: payload.parameters.as_ref().and_then(|p| {
                if p.max_tokens > 0 {
                    Some(p.max_tokens)
                } else {
                    None
                }
            }),
            top_p: payload.parameters.as_ref().map(|p| p.top_p),
            frequency_penalty: payload.parameters.as_ref().map(|p| p.frequency_penalty),
            presence_penalty: payload.parameters.as_ref().map(|p| p.presence_penalty),
            stop: payload.parameters.as_ref().and_then(|p| {
                if p.stop_sequences.is_empty() {
                    None
                } else {
                    Some(crate::services::openai_types::StopSequence::Multiple(
                        p.stop_sequences.clone(),
                    ))
                }
            }),
            stream: false,
            user: None,
        };

        match client.chat_completion(&openai_request).await {
            Ok(response) => {
                let content = response
                    .choices
                    .first()
                    .map(|c| c.message.content.clone())
                    .unwrap_or_default();

                let (prompt_tokens, completion_tokens) = response
                    .usage
                    .map(|u| (u.prompt_tokens, u.completion_tokens))
                    .unwrap_or((0, 0));

                debug!(
                    model = %payload.model,
                    prompt_tokens = prompt_tokens,
                    completion_tokens = completion_tokens,
                    "OpenAI API call successful"
                );

                Ok((content, prompt_tokens, completion_tokens))
            }
            Err(e) => {
                warn!(error = %e, "OpenAI API call failed");
                Err(match e.to_status_code() {
                    401 => GatewayError::UpstreamAuthenticationFailed.into(),
                    403 => GatewayError::UpstreamForbidden.into(),
                    429 => GatewayError::UpstreamRateLimited.into(),
                    500..=599 => GatewayError::UpstreamUnavailable.into(),
                    _ => GatewayError::UpstreamRequestFailed.into(),
                })
            }
        }
    }

    /// Check if using mock mode
    pub fn is_mock(&self) -> bool {
        self.openai_client.is_none()
    }
}

/// Handles response post-processing (sanitization, watermarking)
pub struct ResponsePostProcessor {
    containment_config: ContainmentConfig,
    output_sanitizer: Arc<OutputSanitizer>,
    extraction_deterrent: Arc<ExtractionDeterrent>,
}

impl ResponsePostProcessor {
    pub fn new(
        containment_config: ContainmentConfig,
        output_sanitizer: Arc<OutputSanitizer>,
        extraction_deterrent: Arc<ExtractionDeterrent>,
    ) -> Self {
        Self {
            containment_config,
            output_sanitizer,
            extraction_deterrent,
        }
    }

    /// Process response (sanitize and watermark)
    pub async fn process(
        &self,
        raw_content: String,
        user_id: &UserId,
        request_id: &RequestId,
    ) -> String {
        let mut content = raw_content;

        // C13: Output sanitization
        if self.containment_config.enable_output_sanitization {
            let sanitized = self
                .output_sanitizer
                .sanitize(&content, Some(self.containment_config.sanitization_mode))
                .await;
            debug!(
                request_id = %request_id,
                original_len = content.len(),
                sanitized_len = sanitized.len(),
                "C13: Output sanitized"
            );
            content = sanitized;
        }

        // C13: Watermark embedding
        if self.containment_config.enable_extraction_deterrent {
            let watermarked = self
                .extraction_deterrent
                .process_output(&content, user_id.as_str());
            debug!(
                request_id = %request_id,
                user_id = %user_id,
                "C13: Watermark embedded"
            );
            content = watermarked;
        }

        content
    }
}

/// Handles token consumption
pub struct TokenConsumer {
    token_service: Arc<RwLock<TokenService>>,
}

impl TokenConsumer {
    pub fn new(token_service: Arc<RwLock<TokenService>>) -> Self {
        Self { token_service }
    }

    /// Consume tokens, returns (success, remaining_balance)
    pub async fn consume(&self, cost: u64) -> (bool, u64) {
        let mut token_svc = self.token_service.write().await;
        let success = token_svc.consume(cost);
        let remaining = token_svc.get_balance();
        (success, remaining)
    }
}

/// Handles LPT monitoring
pub struct LptRecorder {
    lpt_monitor: Arc<LptMonitor>,
}

impl LptRecorder {
    pub fn new(lpt_monitor: Arc<LptMonitor>) -> Self {
        Self { lpt_monitor }
    }

    /// Record response for LPT analysis
    pub async fn record(
        &self,
        request_id: &RequestId,
        payload: &AiRequestPayload,
        content: &str,
        processing_time_ms: u64,
        prompt_tokens: u32,
        completion_tokens: u32,
    ) {
        let input_hash = {
            let mut payload_repr = String::new();
            payload_repr.push_str(&payload.model);
            for msg in &payload.messages {
                payload_repr.push_str(&msg.role);
                payload_repr.push_str(&msg.content);
            }
            compute_content_hash(payload_repr.as_bytes())
        };

        let record = ResponseRecord {
            request_id: request_id.to_string(),
            model: payload.model.clone(),
            input_hash,
            content: content.to_string(),
            latency_ms: processing_time_ms,
            prompt_tokens,
            completion_tokens,
            timestamp: Utc::now(),
        };
        self.lpt_monitor.record_response(record).await;
    }

    /// Get current LPT score and state
    pub async fn get_status(&self) -> (crate::services::lpt_monitor::LptScore, LptState) {
        let score = self.lpt_monitor.get_score().await;
        let state = self.lpt_monitor.get_state().await;
        (score, state)
    }
}

/// Build warnings for response
pub fn build_warnings(credential_valid: bool, lpt_state: LptState, lpt_score: f64) -> Vec<Warning> {
    let mut warnings = Vec::new();

    if !credential_valid {
        warnings.push(Warning {
            code: "CREDENTIAL_INVALID".to_string(),
            message: "Credential verification failed or not provided".to_string(),
            severity: Severity::Warning.into(),
        });
    }

    if lpt_state == LptState::Warning || lpt_state == LptState::Critical {
        warnings.push(Warning {
            code: "LPT_DEGRADED".to_string(),
            message: format!(
                "LLM quality degradation detected (state: {}, score: {:.2})",
                lpt_state.as_str(),
                lpt_score
            ),
            severity: if lpt_state == LptState::Critical {
                Severity::Error.into()
            } else {
                Severity::Warning.into()
            },
        });
    }

    warnings
}
