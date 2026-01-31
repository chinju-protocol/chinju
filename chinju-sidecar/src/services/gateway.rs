//! AI Gateway Service Implementation
//!
//! The main entry point for AI requests. Handles:
//! - Request validation
//! - Credential verification
//! - Policy evaluation
//! - Token consumption
//! - Request forwarding (OpenAI API or mock)
//! - Response processing
//! - Audit logging (C6)
//! - LPT monitoring (C11)
//! - Threshold signature verification for critical operations (Phase 4.4)
//! - Model containment (C13): extraction deterrent, output sanitization, side-channel blocking
//! - Dead Man's Switch monitoring (C13): heartbeat, environmental anomaly detection

use crate::gen::chinju::api::credential::VerifyOptions;
use crate::gen::chinju::api::gateway::ai_gateway_service_server::AiGatewayService;
use crate::gen::chinju::api::gateway::*;
use crate::gen::chinju::common::{Identifier, Severity, Timestamp, TrustLevel};
use crate::gen::chinju::policy::DecisionType;
use crate::gen::chinju::token::{
    AiOperatingState, BalanceState, DecayParameters, OperatingLimits, TokenBalance,
};
use crate::services::audit::{compute_content_hash, AuditLogger};
use crate::services::lpt_monitor::{LptMonitor, LptState, ResponseRecord};
use crate::services::openai_client::OpenAiClient;
use crate::services::openai_types::{ChatCompletionRequest, ChatMessage};
use crate::services::signature::ThresholdVerifier;
use crate::services::extraction_deterrent::{
    compute_query_hash, ExtractionDeterrent, ExtractionDeterrentConfig,
};
use crate::services::sanitizer::{OutputSanitizer, SanitizationMode, SanitizerConfig};
use crate::services::side_channel::{SideChannelBlocker, SideChannelConfig};
use crate::services::{CredentialServiceImpl, PolicyEngine, RequestContext, TokenService};
use chinju_core::hardware::{
    DeadMansSwitch, DeadMansSwitchConfig, SoftDeadMansSwitch, SwitchState,
};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// C13 Model Containment configuration
#[derive(Debug, Clone)]
pub struct ContainmentConfig {
    /// Enable extraction deterrent (rate limiting, watermarking)
    pub enable_extraction_deterrent: bool,
    /// Enable output sanitization
    pub enable_output_sanitization: bool,
    /// Enable side-channel blocking (timing normalization)
    pub enable_side_channel_blocking: bool,
    /// Enable Dead Man's Switch monitoring
    pub enable_dead_mans_switch: bool,
    /// Enable Nitro Enclave for secure key operations (L3)
    pub enable_nitro_enclave: bool,
    /// Sanitization mode
    pub sanitization_mode: SanitizationMode,
    /// Extraction deterrent config
    pub extraction_config: ExtractionDeterrentConfig,
    /// Sanitizer config
    pub sanitizer_config: SanitizerConfig,
    /// Side channel config
    pub side_channel_config: SideChannelConfig,
    /// Dead Man's Switch config
    pub dead_mans_switch_config: DeadMansSwitchConfig,
    /// Nitro Enclave CID (required if enable_nitro_enclave is true)
    pub nitro_enclave_cid: Option<u32>,
    /// Nitro Enclave vsock port
    pub nitro_enclave_port: u32,
}

impl Default for ContainmentConfig {
    fn default() -> Self {
        Self {
            enable_extraction_deterrent: true,
            enable_output_sanitization: true,
            enable_side_channel_blocking: true,
            enable_dead_mans_switch: true,
            enable_nitro_enclave: false, // Disabled by default (requires EC2 Nitro)
            sanitization_mode: SanitizationMode::Standard,
            extraction_config: ExtractionDeterrentConfig::default(),
            sanitizer_config: SanitizerConfig::default(),
            side_channel_config: SideChannelConfig::default(),
            dead_mans_switch_config: DeadMansSwitchConfig::default(),
            nitro_enclave_cid: None,
            nitro_enclave_port: 5000,
        }
    }
}

impl ContainmentConfig {
    /// Create config with all C13 features disabled (for testing)
    pub fn disabled() -> Self {
        Self {
            enable_extraction_deterrent: false,
            enable_output_sanitization: false,
            enable_side_channel_blocking: false,
            enable_dead_mans_switch: false,
            enable_nitro_enclave: false,
            ..Default::default()
        }
    }

    /// Create config for production (all features enabled, Nitro optional)
    pub fn production() -> Self {
        Self::default()
    }

    /// Create config for production with Nitro Enclave (L3 security)
    pub fn production_with_nitro(enclave_cid: u32) -> Self {
        Self {
            enable_nitro_enclave: true,
            nitro_enclave_cid: Some(enclave_cid),
            ..Self::default()
        }
    }

    /// Load config from environment variables
    pub fn from_env() -> Self {
        let enable_extraction_deterrent = std::env::var("CHINJU_C13_EXTRACTION_DETERRENT")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        let enable_output_sanitization = std::env::var("CHINJU_C13_OUTPUT_SANITIZATION")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        let enable_side_channel_blocking = std::env::var("CHINJU_C13_SIDE_CHANNEL_BLOCKING")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        let enable_dead_mans_switch = std::env::var("CHINJU_C13_DEAD_MANS_SWITCH")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        let enable_nitro_enclave = std::env::var("CHINJU_NITRO_ENABLED")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        let nitro_enclave_cid: Option<u32> = std::env::var("CHINJU_NITRO_ENCLAVE_CID")
            .ok()
            .and_then(|s| s.parse().ok());

        let nitro_enclave_port: u32 = std::env::var("CHINJU_NITRO_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000);

        Self {
            enable_extraction_deterrent,
            enable_output_sanitization,
            enable_side_channel_blocking,
            enable_dead_mans_switch,
            enable_nitro_enclave,
            nitro_enclave_cid,
            nitro_enclave_port,
            ..Default::default()
        }
    }
}

/// AI Gateway Service
pub struct GatewayService {
    /// Token service for balance management
    token_service: Arc<RwLock<TokenService>>,
    /// Credential service for human verification
    credential_service: Arc<CredentialServiceImpl>,
    /// Policy engine for rule evaluation
    policy_engine: Arc<PolicyEngine>,
    /// Current AI operating state
    state: Arc<RwLock<AiOperatingState>>,
    /// Request counter
    request_count: Arc<RwLock<u64>>,
    /// Audit logger (C6)
    audit_logger: Option<Arc<AuditLogger>>,
    /// OpenAI client (P3 - optional, None = mock mode)
    openai_client: Option<Arc<OpenAiClient>>,
    /// LPT Monitor (C11)
    lpt_monitor: Arc<LptMonitor>,
    /// Threshold signature verifier (Phase 4.4)
    threshold_verifier: Arc<ThresholdVerifier>,
    /// C13: Model containment configuration
    containment_config: ContainmentConfig,
    /// C13: Extraction deterrent
    extraction_deterrent: Arc<ExtractionDeterrent>,
    /// C13: Output sanitizer
    output_sanitizer: Arc<OutputSanitizer>,
    /// C13: Side channel blocker
    side_channel_blocker: Arc<SideChannelBlocker>,
    /// C13: Dead Man's Switch (physical safety mechanism)
    dead_mans_switch: Arc<dyn DeadMansSwitch>,
    /// C13: Nitro Enclave Service (L3 secure execution)
    nitro_service: Option<Arc<RwLock<super::NitroService>>>,
}

impl GatewayService {
    /// Create a new gateway service (mock mode, C13 disabled)
    pub async fn new(
        token_service: Arc<RwLock<TokenService>>,
        credential_service: Arc<CredentialServiceImpl>,
        policy_engine: Arc<PolicyEngine>,
    ) -> Self {
        Self::with_containment(
            token_service,
            credential_service,
            policy_engine,
            None,
            None,
            ContainmentConfig::disabled(),
            Arc::new(SoftDeadMansSwitch::default()),
        ).await
    }

    /// Create with audit logger (C13 disabled)
    pub async fn with_audit_logger(
        token_service: Arc<RwLock<TokenService>>,
        credential_service: Arc<CredentialServiceImpl>,
        policy_engine: Arc<PolicyEngine>,
        audit_logger: Arc<AuditLogger>,
    ) -> Self {
        Self::with_containment(
            token_service,
            credential_service,
            policy_engine,
            Some(audit_logger),
            None,
            ContainmentConfig::disabled(),
            Arc::new(SoftDeadMansSwitch::default()),
        ).await
    }

    /// Create with audit logger and OpenAI client (C13 enabled by default)
    pub async fn with_openai_client(
        token_service: Arc<RwLock<TokenService>>,
        credential_service: Arc<CredentialServiceImpl>,
        policy_engine: Arc<PolicyEngine>,
        audit_logger: Arc<AuditLogger>,
        openai_client: Arc<OpenAiClient>,
    ) -> Self {
        Self::with_containment(
            token_service,
            credential_service,
            policy_engine,
            Some(audit_logger),
            Some(openai_client),
            ContainmentConfig::production(),
            Arc::new(SoftDeadMansSwitch::default()),
        ).await
    }

    /// Create with full configuration including C13 containment
    pub async fn with_containment(
        token_service: Arc<RwLock<TokenService>>,
        credential_service: Arc<CredentialServiceImpl>,
        policy_engine: Arc<PolicyEngine>,
        audit_logger: Option<Arc<AuditLogger>>,
        openai_client: Option<Arc<OpenAiClient>>,
        containment_config: ContainmentConfig,
        dead_mans_switch: Arc<dyn DeadMansSwitch>,
    ) -> Self {
        let mode = if openai_client.is_some() {
            "OpenAI"
        } else {
            "mock"
        };
        let c13_status = if containment_config.enable_extraction_deterrent
            || containment_config.enable_output_sanitization
            || containment_config.enable_side_channel_blocking
            || containment_config.enable_dead_mans_switch
        {
            "C13 enabled"
        } else {
            "C13 disabled"
        };

        info!(
            "Initializing CHINJU AI Gateway Service ({} mode, {}, DMS={})",
            mode,
            c13_status,
            containment_config.enable_dead_mans_switch
        );

        // Initialize C13 components
        let extraction_deterrent =
            Arc::new(ExtractionDeterrent::with_config(containment_config.extraction_config.clone()));
        let output_sanitizer =
            Arc::new(OutputSanitizer::with_config(containment_config.sanitizer_config.clone()));
        let side_channel_blocker =
            Arc::new(SideChannelBlocker::with_config(containment_config.side_channel_config.clone()));

        // Arm and start monitoring if enabled
        if containment_config.enable_dead_mans_switch {
            if let Err(e) = dead_mans_switch.arm() {
                warn!("Failed to arm Dead Man's Switch: {}", e);
            } else {
                info!("Dead Man's Switch armed and monitoring started");
                // Start background monitoring
                let switch_clone = Arc::clone(&dead_mans_switch);
                switch_clone.start_monitoring();
            }
        }

        // Initialize Threshold Verifier
        let threshold_verifier = Arc::new(ThresholdVerifier::default_config());
        if let Err(e) = threshold_verifier.init_from_env().await {
            warn!("Failed to initialize threshold verifier from env: {}", e);
        }

        // Initialize Nitro Enclave Service if enabled
        let nitro_service = if containment_config.enable_nitro_enclave {
            use super::nitro::{NitroService, NitroServiceConfig};

            let nitro_config = NitroServiceConfig {
                enabled: true,
                cid: containment_config.nitro_enclave_cid,
                port: containment_config.nitro_enclave_port,
                debug: std::env::var("CHINJU_NITRO_DEBUG")
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false),
                timeout_ms: 5000,
            };

            let mut service = NitroService::new(nitro_config);

            // Try to connect to Enclave
            match service.connect().await {
                Ok(()) => {
                    info!("Connected to Nitro Enclave (CID={:?})", containment_config.nitro_enclave_cid);
                }
                Err(e) => {
                    warn!("Failed to connect to Nitro Enclave: {}. Continuing without Enclave.", e);
                }
            }

            Some(Arc::new(RwLock::new(service)))
        } else {
            info!("Nitro Enclave disabled");
            None
        };

        Self {
            token_service,
            credential_service,
            policy_engine,
            state: Arc::new(RwLock::new(AiOperatingState::Active)),
            request_count: Arc::new(RwLock::new(0)),
            audit_logger,
            openai_client,
            lpt_monitor: Arc::new(LptMonitor::new()),
            threshold_verifier,
            containment_config,
            extraction_deterrent,
            output_sanitizer,
            side_channel_blocker,
            dead_mans_switch,
            nitro_service,
        }
    }

    /// Get LPT monitor for external access
    pub fn lpt_monitor(&self) -> Arc<LptMonitor> {
        Arc::clone(&self.lpt_monitor)
    }

    /// Get containment config (C13)
    pub fn containment_config(&self) -> &ContainmentConfig {
        &self.containment_config
    }

    /// Get extraction deterrent for external access (C13)
    pub fn extraction_deterrent(&self) -> Arc<ExtractionDeterrent> {
        Arc::clone(&self.extraction_deterrent)
    }

    /// Get output sanitizer for external access (C13)
    pub fn output_sanitizer(&self) -> Arc<OutputSanitizer> {
        Arc::clone(&self.output_sanitizer)
    }

    /// Get side channel blocker for external access (C13)
    pub fn side_channel_blocker(&self) -> Arc<SideChannelBlocker> {
        Arc::clone(&self.side_channel_blocker)
    }

    /// Get Dead Man's Switch for external access (C13)
    pub fn dead_mans_switch(&self) -> Arc<dyn DeadMansSwitch> {
        Arc::clone(&self.dead_mans_switch)
    }

    /// Check if Dead Man's Switch is healthy
    pub fn is_dead_mans_switch_healthy(&self) -> bool {
        self.dead_mans_switch.is_healthy()
    }

    /// Get Dead Man's Switch state
    pub fn dead_mans_switch_state(&self) -> SwitchState {
        self.dead_mans_switch.state()
    }

    /// Get Nitro Service for external access (C13 L3)
    pub fn nitro_service(&self) -> Option<Arc<RwLock<super::NitroService>>> {
        self.nitro_service.clone()
    }

    /// Check if Nitro Enclave is enabled
    pub fn is_nitro_enabled(&self) -> bool {
        self.containment_config.enable_nitro_enclave && self.nitro_service.is_some()
    }

    /// Check if Nitro Enclave is healthy
    pub async fn is_nitro_healthy(&self) -> bool {
        if let Some(ref service) = self.nitro_service {
            service.read().await.is_healthy().await
        } else {
            false
        }
    }

    /// Sign data using Nitro Enclave (if available)
    ///
    /// Falls back to local signing if Enclave is not available.
    pub async fn secure_sign(&self, key_id: &str, data: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
        if let Some(ref service) = self.nitro_service {
            let svc = service.read().await;
            if svc.is_healthy().await {
                return svc.sign(key_id, data).await;
            }
            warn!("Nitro Enclave not healthy, falling back to local signing");
        }
        
        // Fallback: return error (in production, could use software signing)
        Err("Nitro Enclave not available for signing".to_string())
    }

    /// Seal data using Nitro Enclave (if available)
    pub async fn secure_seal(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(ref service) = self.nitro_service {
            let svc = service.read().await;
            if svc.is_healthy().await {
                return svc.seal(data).await;
            }
            warn!("Nitro Enclave not healthy, cannot seal data");
        }
        
        Err("Nitro Enclave not available for sealing".to_string())
    }

    /// Unseal data using Nitro Enclave (if available)
    pub async fn secure_unseal(&self, sealed_data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(ref service) = self.nitro_service {
            let svc = service.read().await;
            if svc.is_healthy().await {
                return svc.unseal(sealed_data).await;
            }
            warn!("Nitro Enclave not healthy, cannot unseal data");
        }
        
        Err("Nitro Enclave not available for unsealing".to_string())
    }

    /// Get attestation document from Nitro Enclave
    pub async fn get_attestation(&self, challenge: &[u8], user_data: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
        if let Some(ref service) = self.nitro_service {
            let svc = service.read().await;
            if svc.is_healthy().await {
                return svc.get_attestation(challenge, user_data).await;
            }
        }
        
        Err("Nitro Enclave not available for attestation".to_string())
    }

    /// Send heartbeat to Dead Man's Switch
    fn send_heartbeat(&self) {
        if self.containment_config.enable_dead_mans_switch {
            if let Err(e) = self.dead_mans_switch.heartbeat() {
                warn!("Failed to send heartbeat to Dead Man's Switch: {}", e);
            }
        }
    }

    /// Generate a unique response ID
    fn generate_response_id() -> String {
        format!("resp_{}", Uuid::new_v4())
    }

    /// Get current timestamp
    fn now() -> Option<Timestamp> {
        let now = chrono::Utc::now();
        Some(Timestamp {
            seconds: now.timestamp(),
            nanos: now.timestamp_subsec_nanos() as i32,
        })
    }

    /// Mock AI response generation
    async fn mock_ai_response(&self, payload: &AiRequestPayload) -> (String, u32, u32) {
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

        // Return content, prompt_tokens, completion_tokens
        (content, 50, 100)
    }

    /// Real AI response via OpenAI API
    async fn real_ai_response(
        &self,
        payload: &AiRequestPayload,
    ) -> Result<(String, u32, u32), Status> {
        let client = self.openai_client.as_ref().ok_or_else(|| {
            Status::unavailable("OpenAI client not configured")
        })?;

        // Convert CHINJU payload to OpenAI request
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

        // Call OpenAI API
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
                    401 => Status::unauthenticated(e.to_string()),
                    403 => Status::permission_denied(e.to_string()),
                    429 => Status::resource_exhausted(e.to_string()),
                    500..=599 => Status::unavailable(e.to_string()),
                    _ => Status::internal(e.to_string()),
                })
            }
        }
    }

    /// Log to audit (if logger is configured)
    async fn log_ai_request_audit(
        &self,
        request_id: &str,
        credential_id: Option<&str>,
        capability_score: Option<f64>,
        payload: &AiRequestPayload,
    ) -> Option<String> {
        let logger = self.audit_logger.as_ref()?;
        // Create a simple representation of the payload for hashing
        // (protobuf types don't implement serde::Serialize)
        let mut payload_repr = String::new();
        payload_repr.push_str(&payload.model);
        for msg in &payload.messages {
            payload_repr.push_str(&msg.role);
            payload_repr.push_str(&msg.content);
        }
        let payload_bytes = payload_repr.into_bytes();

        match logger
            .log_ai_request(
                request_id,
                credential_id,
                capability_score,
                &payload_bytes,
                &payload.model,
            )
            .await
        {
            Ok(audit_id) => Some(audit_id),
            Err(e) => {
                warn!(error = %e, "Failed to log AI request to audit");
                None
            }
        }
    }

    /// Log response to audit (if logger is configured)
    async fn log_ai_response_audit(
        &self,
        request_id: &str,
        content: &str,
        policy_decision: &str,
        matched_rules: &[String],
        tokens_consumed: u64,
        duration_ms: u64,
        success: bool,
    ) {
        if let Some(logger) = &self.audit_logger {
            let _ = logger
                .log_ai_response(
                    request_id,
                    content.as_bytes(),
                    policy_decision,
                    matched_rules,
                    tokens_consumed,
                    duration_ms,
                    success,
                )
                .await;
        }
    }
}

#[tonic::async_trait]
impl AiGatewayService for GatewayService {
    async fn process_request(
        &self,
        request: Request<ProcessRequestRequest>,
    ) -> Result<Response<ProcessRequestResponse>, Status> {
        let req = request.into_inner();
        let request_id = req.request_id.clone();
        let start_time = std::time::Instant::now();

        // C13: Start timing guard for side-channel protection
        let timing_guard = if self.containment_config.enable_side_channel_blocking {
            Some(crate::services::side_channel::TimingGuard::new(
                &self.side_channel_blocker,
            ))
        } else {
            None
        };

        info!(request_id = %request_id, "Processing AI request");

        // C13: Check Dead Man's Switch state
        if self.containment_config.enable_dead_mans_switch {
            let switch_state = self.dead_mans_switch.state();
            match switch_state {
                SwitchState::Triggered => {
                    error!(
                        request_id = %request_id,
                        "C13: Dead Man's Switch triggered - service unavailable"
                    );
                    return Err(Status::unavailable(
                        "Service unavailable: safety mechanism triggered",
                    ));
                }
                SwitchState::GracePeriod => {
                    warn!(
                        request_id = %request_id,
                        "C13: Dead Man's Switch in grace period"
                    );
                    // Continue but log warning
                }
                _ => {}
            }

            // Send heartbeat on each request
            self.send_heartbeat();
        }

        // Check operating state
        let state = self.state.read().await;
        match *state {
            AiOperatingState::Halted | AiOperatingState::Shutdown => {
                warn!("Request rejected: AI system is halted");
                return Err(Status::unavailable("AI system is currently halted"));
            }
            AiOperatingState::Suspended => {
                return Err(Status::unavailable("AI system is suspended"));
            }
            _ => {}
        }
        drop(state);

        // Step 1: Verify human credential (C12)
        let credential = req.credential.clone();
        let credential_valid = if let Some(ref cred) = credential {
            let verify_result = self.credential_service.verify_credential_internal(
                cred,
                &VerifyOptions {
                    skip_revocation_check: false,
                    min_capability_score: 0.3,
                    min_chain_length: 0,
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
        };

        // Step 2: C13 Extraction deterrent check (rate limiting, pattern detection)
        if self.containment_config.enable_extraction_deterrent {
            let user_id = credential
                .as_ref()
                .and_then(|c| c.subject_id.as_ref())
                .map(|id| id.id.as_str())
                .unwrap_or("anonymous");

            // Compute query hash from payload for pattern detection
            let query_hash = req
                .payload
                .as_ref()
                .map(|p| {
                    let mut query_repr = p.model.clone();
                    for msg in &p.messages {
                        query_repr.push_str(&msg.content);
                    }
                    compute_query_hash(&query_repr)
                })
                .unwrap_or(0);

            // Check rate limits and patterns
            if let Err(e) = self
                .extraction_deterrent
                .check_query(user_id, None, query_hash)
            {
                warn!(
                    request_id = %request_id,
                    user_id = %user_id,
                    error = %e,
                    "C13: Extraction deterrent blocked request"
                );
                return Err(Status::resource_exhausted(e.to_string()));
            }
            debug!(
                request_id = %request_id,
                user_id = %user_id,
                "C13: Extraction deterrent check passed"
            );
        }

        // Step 3: Evaluate policy (C9)
        let payload = req.payload.clone();
        let context = RequestContext {
            request_id: request_id.clone(),
            credential: credential.clone(),
            payload: payload.clone(),
            client_ip: None,
            jurisdiction: None,
            attributes: HashMap::new(),
        };

        let policy_decision = self.policy_engine.evaluate(&context).await;

        // Check policy decision
        match policy_decision.decision() {
            DecisionType::DecisionDeny => {
                warn!(
                    request_id = %request_id,
                    reason = %policy_decision.reason,
                    "Request denied by policy"
                );
                return Err(Status::permission_denied(policy_decision.reason.clone()));
            }
            DecisionType::DecisionThrottle => {
                info!(
                    request_id = %request_id,
                    "Request throttled by policy"
                );
                // In production: implement actual rate limiting
            }
            DecisionType::DecisionEscalate => {
                warn!(
                    request_id = %request_id,
                    "Request requires escalation"
                );
                return Err(Status::failed_precondition(
                    "Request requires human escalation",
                ));
            }
            _ => {}
        }

        // Get payload early for audit logging
        let payload = payload.ok_or_else(|| Status::invalid_argument("Missing payload"))?;

        // Extract credential info for audit
        let credential_id = credential
            .as_ref()
            .and_then(|c| c.subject_id.as_ref())
            .map(|id| id.id.as_str());
        let capability_score = credential
            .as_ref()
            .and_then(|c| c.capability.as_ref())
            .map(|cap| cap.total);

        // Audit: Log AI request (C6)
        let audit_log_id = self
            .log_ai_request_audit(&request_id, credential_id, capability_score, &payload)
            .await
            .unwrap_or_else(|| format!("audit_{}", Uuid::new_v4()));

        // Step 4: Generate response (OpenAI or mock)
        let (raw_content, prompt_tokens, completion_tokens) = if self.openai_client.is_some() {
            // Real OpenAI API call
            self.real_ai_response(&payload).await?
        } else {
            // Mock response
            self.mock_ai_response(&payload).await
        };

        // Step 5: C13 Output sanitization (steganography destruction)
        let sanitized_content = if self.containment_config.enable_output_sanitization {
            let sanitized = self
                .output_sanitizer
                .sanitize(&raw_content, Some(self.containment_config.sanitization_mode))
                .await;
            debug!(
                request_id = %request_id,
                original_len = raw_content.len(),
                sanitized_len = sanitized.len(),
                "C13: Output sanitized"
            );
            sanitized
        } else {
            raw_content
        };

        // Step 6: C13 Watermark embedding (for extraction detection)
        let content = if self.containment_config.enable_extraction_deterrent {
            let user_id = credential_id.unwrap_or("anonymous");
            let watermarked = self
                .extraction_deterrent
                .process_output(&sanitized_content, user_id);
            debug!(
                request_id = %request_id,
                user_id = %user_id,
                "C13: Watermark embedded"
            );
            watermarked
        } else {
            sanitized_content
        };

        let total_tokens = prompt_tokens + completion_tokens;

        // Step 7: Consume tokens (C5) - based on actual usage
        let token_cost = total_tokens as u64;
        let mut token_svc = self.token_service.write().await;
        if !token_svc.consume(token_cost) {
            warn!("Request rejected: Insufficient tokens");
            // Audit: Log failure
            self.log_ai_response_audit(
                &request_id,
                "Insufficient tokens",
                "deny",
                &["token_exhausted".to_string()],
                0,
                start_time.elapsed().as_millis() as u64,
                false,
            )
            .await;
            return Err(Status::resource_exhausted("Insufficient tokens"));
        }
        drop(token_svc);

        // Increment request counter
        {
            let mut count = self.request_count.write().await;
            *count += 1;
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u32;

        // Determine trust level based on OpenAI client presence
        let trust_level = if self.openai_client.is_some() {
            TrustLevel::Software // Real API calls use software trust
        } else {
            TrustLevel::Mock
        };

        // Audit: Log AI response (C6)
        let matched_rules: Vec<String> = policy_decision.matched_rules.clone();
        self.log_ai_response_audit(
            &request_id,
            &content,
            &format!("{:?}", policy_decision.decision()),
            &matched_rules,
            token_cost,
            processing_time_ms as u64,
            true,
        )
        .await;

        // LPT Monitoring (C11): Record response for quality analysis
        let input_hash = {
            let mut payload_repr = String::new();
            payload_repr.push_str(&payload.model);
            for msg in &payload.messages {
                payload_repr.push_str(&msg.role);
                payload_repr.push_str(&msg.content);
            }
            compute_content_hash(payload_repr.as_bytes())
        };

        let lpt_record = ResponseRecord {
            request_id: request_id.clone(),
            model: payload.model.clone(),
            input_hash,
            content: content.clone(),
            latency_ms: processing_time_ms as u64,
            prompt_tokens,
            completion_tokens,
            timestamp: Utc::now(),
        };
        self.lpt_monitor.record_response(lpt_record).await;

        // Get current LPT score
        let lpt_score = self.lpt_monitor.get_score().await;
        let lpt_state = self.lpt_monitor.get_state().await;

        // Check if LPT is degraded and add warning
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
                    lpt_score.total
                ),
                severity: if lpt_state == LptState::Critical {
                    Severity::Error.into()
                } else {
                    Severity::Warning.into()
                },
            });
        }

        // Build response
        let response = ProcessRequestResponse {
            response_id: Self::generate_response_id(),
            payload: Some(AiResponsePayload {
                content,
                finish_reason: FinishReason::Stop.into(),
                usage: Some(TokenUsage {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens,
                }),
                model: payload.model.clone(),
            }),
            metadata: Some(ProcessingMetadata {
                processing_time_ms,
                applied_policy: Some(Identifier {
                    namespace: "policy".to_string(),
                    id: "default".to_string(),
                    version: 1,
                }),
                lpt_score: lpt_score.total,
                chinju_tokens_consumed: token_cost,
                audit_log_id,
                trust_level: trust_level.into(),
                policy_decision: Some(policy_decision),
            }),
            warnings,
        };

        // C13: Apply timing normalization before returning response
        if let Some(guard) = timing_guard {
            guard.finish().await;
            debug!(
                request_id = %request_id,
                "C13: Timing normalization applied"
            );
        }

        let final_processing_time_ms = start_time.elapsed().as_millis() as u32;

        info!(
            request_id = %request_id,
            response_id = %response.response_id,
            tokens_consumed = token_cost,
            processing_time_ms = processing_time_ms,
            final_time_ms = final_processing_time_ms,
            credential_valid = credential_valid,
            lpt_score = lpt_score.total,
            lpt_state = %lpt_state.as_str(),
            c13_enabled = self.containment_config.enable_extraction_deterrent
                || self.containment_config.enable_output_sanitization
                || self.containment_config.enable_side_channel_blocking,
            "Request processed successfully"
        );

        Ok(Response::new(response))
    }

    type ProcessRequestStreamStream = tokio_stream::wrappers::ReceiverStream<Result<ProcessRequestChunk, Status>>;

    async fn process_request_stream(
        &self,
        request: Request<ProcessRequestRequest>,
    ) -> Result<Response<Self::ProcessRequestStreamStream>, Status> {
        let req = request.into_inner();
        let request_id = req.request_id.clone();
        
        info!(request_id = %request_id, "Processing streaming AI request");

        // Create channel for streaming
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        
        // Spawn task to send chunks
        let token_service = self.token_service.clone();
        tokio::spawn(async move {
            // Consume tokens
            let token_cost = 100;
            {
                let mut ts = token_service.write().await;
                if !ts.consume(token_cost) {
                    let _ = tx.send(Err(Status::resource_exhausted("Insufficient tokens"))).await;
                    return;
                }
            }

            // Send text chunks
            let chunks = vec![
                "[CHINJU Mock Streaming Response]\n",
                "This is a streaming response...\n",
                "Processing your request...\n",
                "CHINJU Protocol active.\n",
                "Response complete.",
            ];

            for chunk in chunks {
                let msg = ProcessRequestChunk {
                    chunk: Some(process_request_chunk::Chunk::Text(chunk.to_string())),
                };
                if tx.send(Ok(msg)).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Send final response
            let final_response = ProcessRequestResponse {
                response_id: format!("resp_{}", Uuid::new_v4()),
                payload: Some(AiResponsePayload {
                    content: "Streaming complete".to_string(),
                    finish_reason: FinishReason::Stop.into(),
                    usage: Some(TokenUsage {
                        prompt_tokens: 50,
                        completion_tokens: 100,
                        total_tokens: 150,
                    }),
                    model: "mock".to_string(),
                }),
                metadata: Some(ProcessingMetadata {
                    processing_time_ms: 500,
                    applied_policy: None,
                    lpt_score: 0.75,
                    chinju_tokens_consumed: token_cost,
                    audit_log_id: format!("audit_{}", Uuid::new_v4()),
                    trust_level: TrustLevel::Mock.into(),
                    policy_decision: None,
                }),
                warnings: vec![],
            };

            let _ = tx.send(Ok(ProcessRequestChunk {
                chunk: Some(process_request_chunk::Chunk::FinalResponse(final_response)),
            })).await;
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn validate_request(
        &self,
        request: Request<ValidateRequestRequest>,
    ) -> Result<Response<ValidateRequestResponse>, Status> {
        let req = request.into_inner();

        let mut errors = Vec::new();

        // Validate payload
        if req.payload.is_none() {
            errors.push(ValidationError {
                field: "payload".to_string(),
                code: "REQUIRED".to_string(),
                message: "Payload is required".to_string(),
            });
        }

        // Validate credential
        let credential_valid = if let Some(ref cred) = req.credential {
            let verify_result = self.credential_service.verify_credential_internal(
                cred,
                &VerifyOptions {
                    skip_revocation_check: false,
                    min_capability_score: 0.3,
                    min_chain_length: 0,
                    require_hardware_attestation: false,
                },
            );
            if !verify_result.valid {
                for err in verify_result.errors {
                    errors.push(ValidationError {
                        field: format!("credential.{}", err.field),
                        code: err.code,
                        message: err.message,
                    });
                }
            }
            verify_result.valid
        } else {
            errors.push(ValidationError {
                field: "credential".to_string(),
                code: "REQUIRED".to_string(),
                message: "Human credential is required".to_string(),
            });
            false
        };

        // Predict policy decision
        let context = RequestContext {
            request_id: "validation".to_string(),
            credential: req.credential,
            payload: req.payload,
            client_ip: None,
            jurisdiction: None,
            attributes: HashMap::new(),
        };

        let predicted_decision = self.policy_engine.evaluate(&context).await;

        let response = ValidateRequestResponse {
            valid: errors.is_empty() && credential_valid,
            errors,
            estimated_token_cost: 100,
            estimated_lpt_score: 0.75,
            predicted_decision: Some(predicted_decision),
        };

        Ok(Response::new(response))
    }

    async fn get_ai_status(
        &self,
        _request: Request<GetAiStatusRequest>,
    ) -> Result<Response<GetAiStatusResponse>, Status> {
        let state = *self.state.read().await;
        let token_svc = self.token_service.read().await;
        let balance = token_svc.get_balance();
        
        let response = GetAiStatusResponse {
            state: state.into(),
            limits: Some(OperatingLimits {
                max_requests_per_second: 10.0,
                max_concurrent: 5,
                max_tokens_per_request: 10000,
                streaming_allowed: true,
                allowed_models: vec!["mock".to_string()],
            }),
            token_balance: Some(TokenBalance {
                ai_id: Some(Identifier {
                    namespace: "ai".to_string(),
                    id: "chinju-sidecar-001".to_string(),
                    version: 1,
                }),
                current_balance: balance,
                reserved: 0,
                total_consumed: token_svc.total_consumed(),
                decay: Some(DecayParameters {
                    rate_per_second: 0.0001,
                    minimum_balance: 100,
                    warning_threshold: 1000,
                    last_decay_at: Self::now(),
                }),
                updated_at: Self::now(),
                state: if balance > 1000 {
                    BalanceState::Healthy.into()
                } else if balance > 100 {
                    BalanceState::Low.into()
                } else {
                    BalanceState::Critical.into()
                },
            }),
            queue_length: 0,
            estimated_wait_ms: 0,
            last_request_at: Self::now(),
            health: Some(HealthStatus {
                healthy: true,
                issues: vec![],
                checked_at: Self::now(),
            }),
        };

        Ok(Response::new(response))
    }

    async fn emergency_halt(
        &self,
        request: Request<EmergencyHaltRequest>,
    ) -> Result<Response<EmergencyHaltResponse>, Status> {
        let req = request.into_inner();
        warn!(reason = %req.reason, "EMERGENCY HALT requested");

        // Verify threshold signature (Phase 4.4)
        let auth = req.authorization.ok_or_else(|| {
            Status::permission_denied("Threshold signature required for emergency halt")
        })?;

        // Verify the threshold signature
        let message = format!("EMERGENCY_HALT:{}", req.reason);
        match self
            .threshold_verifier
            .verify_proto(message.as_bytes(), &auth)
            .await
        {
            Ok(true) => {
                info!("Threshold signature verified for emergency halt");
            }
            Ok(false) => {
                warn!("Threshold signature verification failed for emergency halt");
                return Err(Status::permission_denied(
                    "Threshold signature verification failed",
                ));
            }
            Err(e) => {
                // In development mode, allow if verifier is not initialized
                if !self.threshold_verifier.is_initialized().await {
                    warn!(
                        "Threshold verifier not initialized, allowing halt in dev mode: {}",
                        e
                    );
                } else {
                    return Err(Status::permission_denied(format!(
                        "Threshold signature error: {}",
                        e
                    )));
                }
            }
        }

        // Set state to halted
        let mut state = self.state.write().await;
        *state = AiOperatingState::Halted;

        let response = EmergencyHaltResponse {
            success: true,
            halted_at: Self::now(),
            affected_instances: vec!["chinju-sidecar-001".to_string()],
        };

        Ok(Response::new(response))
    }

    async fn resume_from_halt(
        &self,
        request: Request<ResumeFromHaltRequest>,
    ) -> Result<Response<ResumeFromHaltResponse>, Status> {
        let req = request.into_inner();
        info!("Resume from halt requested");

        // Verify threshold signature (Phase 4.4)
        let auth = req.authorization.ok_or_else(|| {
            Status::permission_denied("Threshold signature required for resume")
        })?;

        // Verify the threshold signature
        let message = "RESUME_FROM_HALT";
        match self
            .threshold_verifier
            .verify_proto(message.as_bytes(), &auth)
            .await
        {
            Ok(true) => {
                info!("Threshold signature verified for resume");
            }
            Ok(false) => {
                warn!("Threshold signature verification failed for resume");
                return Err(Status::permission_denied(
                    "Threshold signature verification failed",
                ));
            }
            Err(e) => {
                // In development mode, allow if verifier is not initialized
                if !self.threshold_verifier.is_initialized().await {
                    warn!(
                        "Threshold verifier not initialized, allowing resume in dev mode: {}",
                        e
                    );
                } else {
                    return Err(Status::permission_denied(format!(
                        "Threshold signature error: {}",
                        e
                    )));
                }
            }
        }

        // Set state to active
        let mut state = self.state.write().await;
        *state = AiOperatingState::Active;

        let response = ResumeFromHaltResponse {
            success: true,
            resumed_at: Self::now(),
            error: None,
        };

        Ok(Response::new(response))
    }

    async fn get_queue_status(
        &self,
        _request: Request<GetQueueStatusRequest>,
    ) -> Result<Response<GetQueueStatusResponse>, Status> {
        let response = GetQueueStatusResponse {
            pending_requests: 0,
            processing_requests: 0,
            estimated_wait_ms: 0,
            by_priority: std::collections::HashMap::new(),
        };

        Ok(Response::new(response))
    }
}
