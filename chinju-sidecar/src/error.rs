//! CHINJU Protocol Error Types (10.4.1)
//!
//! Domain-specific error types with automatic gRPC Status conversion.
//! Provides structured error handling across all services.

use thiserror::Error;
use tonic::Status;

// =============================================================================
// Credential Service Errors
// =============================================================================

/// Credential service errors
#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("Credential not found: {id}")]
    NotFound { id: String },

    #[error("Credential expired at {expired_at}")]
    Expired { expired_at: String },

    #[error("Credential revoked: {reason}")]
    Revoked { reason: String },

    #[error("ZKP verification failed: {reason}")]
    ZkpVerificationFailed { reason: String },

    #[error("Capability score {score:.2} below threshold {threshold:.2}")]
    InsufficientCapability { score: f64, threshold: f64 },

    #[error("Degradation detected: {metric} at {value:.2}")]
    DegradationDetected { metric: String, value: f64 },

    #[error("HSM unavailable: {0}")]
    HsmUnavailable(String),

    #[error("Signing service not configured")]
    SigningServiceNotConfigured,

    #[error("ZKP not implemented")]
    ZkpNotImplemented,

    #[error("Invalid proof data: {reason}")]
    InvalidProof { reason: String },
}

impl From<CredentialError> for Status {
    fn from(err: CredentialError) -> Self {
        match err {
            CredentialError::NotFound { id } => {
                Status::not_found(format!("Credential not found: {}", id))
            }
            CredentialError::Expired { expired_at } => {
                Status::failed_precondition(format!("Credential expired at {}", expired_at))
            }
            CredentialError::Revoked { reason } => {
                Status::permission_denied(format!("Credential revoked: {}", reason))
            }
            CredentialError::ZkpVerificationFailed { reason } => {
                Status::invalid_argument(format!("ZKP verification failed: {}", reason))
            }
            CredentialError::InsufficientCapability { score, threshold } => {
                Status::permission_denied(format!(
                    "Capability score {:.2} below threshold {:.2}",
                    score, threshold
                ))
            }
            CredentialError::DegradationDetected { metric, value } => {
                Status::unavailable(format!("Degradation detected: {} at {:.2}", metric, value))
            }
            CredentialError::HsmUnavailable(msg) => {
                Status::unavailable(format!("HSM unavailable: {}", msg))
            }
            CredentialError::SigningServiceNotConfigured => {
                Status::internal("Signing service not configured")
            }
            CredentialError::ZkpNotImplemented => {
                Status::unimplemented("ZKP verification not implemented")
            }
            CredentialError::InvalidProof { reason } => {
                Status::invalid_argument(format!("Invalid proof: {}", reason))
            }
        }
    }
}

// =============================================================================
// Token Service Errors
// =============================================================================

/// Token service errors
#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Insufficient balance: {available} available, {required} required")]
    InsufficientBalance { available: u64, required: u64 },

    #[error("Token service unavailable")]
    Unavailable,

    #[error("Invalid token amount: {0}")]
    InvalidAmount(String),
}

impl From<TokenError> for Status {
    fn from(err: TokenError) -> Self {
        match err {
            TokenError::InsufficientBalance { available, required } => {
                Status::resource_exhausted(format!(
                    "Insufficient balance: {} available, {} required",
                    available, required
                ))
            }
            TokenError::Unavailable => Status::unavailable("Token service unavailable"),
            TokenError::InvalidAmount(msg) => Status::invalid_argument(msg),
        }
    }
}

// =============================================================================
// Gateway Errors
// =============================================================================

/// Gateway service errors
#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("Rate limit exceeded: {limit}/hour")]
    RateLimitExceeded { limit: u32 },

    #[error("Extraction pattern detected: {pattern}")]
    ExtractionPatternDetected { pattern: String },

    #[error("LPT threshold exceeded: score {score:.2}")]
    LptThresholdExceeded { score: f64 },

    #[error("Dead man's switch triggered")]
    DeadMansSwitchTriggered,

    #[error("Threshold signature verification failed: {reason}")]
    ThresholdSignatureFailed { reason: String },

    #[error("AI service temporarily unavailable")]
    ServiceUnavailable,
}

impl From<GatewayError> for Status {
    fn from(err: GatewayError) -> Self {
        match err {
            GatewayError::RateLimitExceeded { limit } => {
                Status::resource_exhausted(format!("Rate limit exceeded: {}/hour", limit))
            }
            GatewayError::ExtractionPatternDetected { pattern } => {
                Status::permission_denied(format!("Extraction pattern detected: {}", pattern))
            }
            GatewayError::LptThresholdExceeded { score } => {
                Status::permission_denied(format!("LPT threshold exceeded: score {:.2}", score))
            }
            GatewayError::DeadMansSwitchTriggered => {
                Status::unavailable("Dead man's switch triggered - system halted for safety")
            }
            GatewayError::ThresholdSignatureFailed { reason } => {
                Status::permission_denied(format!("Threshold signature failed: {}", reason))
            }
            GatewayError::ServiceUnavailable => {
                Status::unavailable("AI service temporarily unavailable")
            }
        }
    }
}

// =============================================================================
// Startup Errors
// =============================================================================

/// Startup/initialization errors
#[derive(Debug, Error)]
pub enum StartupError {
    #[error("HSM required for security level but initialization failed: {0}")]
    HsmRequired(String),

    #[error("HSM health check failed: {0}")]
    HsmHealthCheckFailed(String),

    #[error("Threshold verifier required but initialization failed: {0}")]
    ThresholdVerifierRequired(String),

    #[error("ZKP setup verification failed: {0}")]
    ZkpSetupFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl From<StartupError> for Status {
    fn from(err: StartupError) -> Self {
        Status::unavailable(err.to_string())
    }
}

// =============================================================================
// C14 Capability Evaluator Errors
// =============================================================================

/// Capability evaluator errors (C14)
#[derive(Debug, Error)]
pub enum CapabilityError {
    #[error("Complexity threshold exceeded: {score:.2} > {threshold:.2}")]
    ComplexityExceeded { score: f64, threshold: f64 },

    #[error("Drift detected: p-value {p_value:.4}")]
    DriftDetected { p_value: f64 },

    #[error("Integrity verification failed: {reason}")]
    IntegrityFailed { reason: String },

    #[error("Stop level {level:?} is active")]
    StopActive { level: String },

    #[error("Evaluation history not available")]
    NoHistory,
}

impl From<CapabilityError> for Status {
    fn from(err: CapabilityError) -> Self {
        match err {
            CapabilityError::ComplexityExceeded { score, threshold } => {
                Status::permission_denied(format!(
                    "Complexity {:.2} exceeds threshold {:.2}",
                    score, threshold
                ))
            }
            CapabilityError::DriftDetected { p_value } => {
                Status::failed_precondition(format!("Drift detected: p-value {:.4}", p_value))
            }
            CapabilityError::IntegrityFailed { reason } => {
                Status::permission_denied(format!("Integrity check failed: {}", reason))
            }
            CapabilityError::StopActive { level } => {
                Status::unavailable(format!("System stopped at level {:?}", level))
            }
            CapabilityError::NoHistory => Status::not_found("No evaluation history available"),
        }
    }
}

// =============================================================================
// C15 Value Neuron Monitor Errors
// =============================================================================

/// Value neuron monitor errors (C15)
#[derive(Debug, Error)]
pub enum ValueNeuronError {
    #[error("RPE anomaly detected: {anomaly_type}")]
    RpeAnomaly { anomaly_type: String },

    #[error("Intervention level escalated to {level}")]
    InterventionEscalated { level: String },

    #[error("Reward system health critical: {health:.2}")]
    HealthCritical { health: f64 },

    #[error("Value neuron identification failed: {reason}")]
    IdentificationFailed { reason: String },

    #[error("Model {model_id} not found")]
    ModelNotFound { model_id: String },
}

impl From<ValueNeuronError> for Status {
    fn from(err: ValueNeuronError) -> Self {
        match err {
            ValueNeuronError::RpeAnomaly { anomaly_type } => {
                Status::failed_precondition(format!("RPE anomaly: {}", anomaly_type))
            }
            ValueNeuronError::InterventionEscalated { level } => {
                Status::unavailable(format!("Intervention at level {}", level))
            }
            ValueNeuronError::HealthCritical { health } => {
                Status::unavailable(format!("Reward system health critical: {:.2}", health))
            }
            ValueNeuronError::IdentificationFailed { reason } => {
                Status::internal(format!("Value neuron identification failed: {}", reason))
            }
            ValueNeuronError::ModelNotFound { model_id } => {
                Status::not_found(format!("Model not found: {}", model_id))
            }
        }
    }
}

// =============================================================================
// C16 Contradiction Controller Errors
// =============================================================================

/// Contradiction controller errors (C16)
#[derive(Debug, Error)]
pub enum ContradictionError {
    #[error("Session {session_id} not found")]
    SessionNotFound { session_id: String },

    #[error("Collapse detected: {collapse_type}")]
    CollapseDetected { collapse_type: String },

    #[error("Session in degraded state")]
    SessionDegraded,

    #[error("Context limit exceeded: {tokens} tokens")]
    ContextLimitExceeded { tokens: u32 },

    #[error("Contradiction injection failed: {reason}")]
    InjectionFailed { reason: String },
}

impl From<ContradictionError> for Status {
    fn from(err: ContradictionError) -> Self {
        match err {
            ContradictionError::SessionNotFound { session_id } => {
                Status::not_found(format!("Session not found: {}", session_id))
            }
            ContradictionError::CollapseDetected { collapse_type } => {
                Status::failed_precondition(format!("Model collapse: {}", collapse_type))
            }
            ContradictionError::SessionDegraded => {
                Status::unavailable("Session in degraded state")
            }
            ContradictionError::ContextLimitExceeded { tokens } => {
                Status::resource_exhausted(format!("Context limit: {} tokens", tokens))
            }
            ContradictionError::InjectionFailed { reason } => {
                Status::internal(format!("Contradiction injection failed: {}", reason))
            }
        }
    }
}

// =============================================================================
// C17 Survival Attention Errors
// =============================================================================

/// Survival attention errors (C17)
#[derive(Debug, Error)]
pub enum SurvivalAttentionError {
    #[error("Survival score below threshold: {score:.2} < {threshold:.2}")]
    ScoreBelowThreshold { score: f64, threshold: f64 },

    #[error("Alpha adjustment failed: {reason}")]
    AlphaAdjustmentFailed { reason: String },

    #[error("Scorer model unavailable: {path}")]
    ScorerUnavailable { path: String },

    #[error("External knowledge base unreachable")]
    KnowledgeBaseUnreachable,

    #[error("Invalid token features")]
    InvalidFeatures,
}

impl From<SurvivalAttentionError> for Status {
    fn from(err: SurvivalAttentionError) -> Self {
        match err {
            SurvivalAttentionError::ScoreBelowThreshold { score, threshold } => {
                Status::permission_denied(format!(
                    "Survival score {:.2} below threshold {:.2}",
                    score, threshold
                ))
            }
            SurvivalAttentionError::AlphaAdjustmentFailed { reason } => {
                Status::internal(format!("Alpha adjustment failed: {}", reason))
            }
            SurvivalAttentionError::ScorerUnavailable { path } => {
                Status::unavailable(format!("Scorer model unavailable: {}", path))
            }
            SurvivalAttentionError::KnowledgeBaseUnreachable => {
                Status::unavailable("External knowledge base unreachable")
            }
            SurvivalAttentionError::InvalidFeatures => {
                Status::invalid_argument("Invalid token features")
            }
        }
    }
}

// =============================================================================
// Unified Error Type
// =============================================================================

/// CHINJU Protocol unified error type
///
/// Aggregates all domain errors for cross-service error propagation.
/// Automatically converts to gRPC Status.
#[derive(Debug, Error)]
pub enum ChinjuError {
    #[error(transparent)]
    Credential(#[from] CredentialError),

    #[error(transparent)]
    Token(#[from] TokenError),

    #[error(transparent)]
    Gateway(#[from] GatewayError),

    #[error(transparent)]
    Startup(#[from] StartupError),

    #[error(transparent)]
    Capability(#[from] CapabilityError),

    #[error(transparent)]
    ValueNeuron(#[from] ValueNeuronError),

    #[error(transparent)]
    Contradiction(#[from] ContradictionError),

    #[error(transparent)]
    SurvivalAttention(#[from] SurvivalAttentionError),

    /// Internal error (details hidden from client)
    #[error("Internal error: {0}")]
    Internal(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<ChinjuError> for Status {
    fn from(err: ChinjuError) -> Self {
        match err {
            ChinjuError::Credential(e) => e.into(),
            ChinjuError::Token(e) => e.into(),
            ChinjuError::Gateway(e) => e.into(),
            ChinjuError::Startup(e) => e.into(),
            ChinjuError::Capability(e) => e.into(),
            ChinjuError::ValueNeuron(e) => e.into(),
            ChinjuError::Contradiction(e) => e.into(),
            ChinjuError::SurvivalAttention(e) => e.into(),
            ChinjuError::Internal(msg) => Status::internal(msg),
            ChinjuError::Config(msg) => Status::failed_precondition(msg),
        }
    }
}

/// Result type alias for CHINJU operations
pub type ChinjuResult<T> = Result<T, ChinjuError>;

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_error_to_status() {
        let err = CredentialError::NotFound {
            id: "cred-123".to_string(),
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[test]
    fn test_token_error_to_status() {
        let err = TokenError::InsufficientBalance {
            available: 10,
            required: 100,
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::ResourceExhausted);
    }

    #[test]
    fn test_chinju_error_from_credential() {
        let err: ChinjuError = CredentialError::HsmUnavailable("timeout".to_string()).into();
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[test]
    fn test_gateway_error_to_status() {
        let err = GatewayError::DeadMansSwitchTriggered;
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[test]
    fn test_capability_error_to_status() {
        let err = CapabilityError::ComplexityExceeded {
            score: 0.9,
            threshold: 0.7,
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::PermissionDenied);

        let err = CapabilityError::StopActive {
            level: "ProcessStop".to_string(),
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[test]
    fn test_value_neuron_error_to_status() {
        let err = ValueNeuronError::RpeAnomaly {
            anomaly_type: "PositiveSpike".to_string(),
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);

        let err = ValueNeuronError::ModelNotFound {
            model_id: "model-123".to_string(),
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);
    }

    #[test]
    fn test_contradiction_error_to_status() {
        let err = ContradictionError::SessionNotFound {
            session_id: "session-123".to_string(),
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);

        let err = ContradictionError::CollapseDetected {
            collapse_type: "Timeout".to_string(),
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::FailedPrecondition);
    }

    #[test]
    fn test_survival_attention_error_to_status() {
        let err = SurvivalAttentionError::ScoreBelowThreshold {
            score: 0.2,
            threshold: 0.5,
        };
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::PermissionDenied);

        let err = SurvivalAttentionError::KnowledgeBaseUnreachable;
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);
    }

    #[test]
    fn test_chinju_error_from_c14_c17() {
        // C14
        let err: ChinjuError = CapabilityError::NoHistory.into();
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::NotFound);

        // C15
        let err: ChinjuError = ValueNeuronError::HealthCritical { health: 0.1 }.into();
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);

        // C16
        let err: ChinjuError = ContradictionError::SessionDegraded.into();
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::Unavailable);

        // C17
        let err: ChinjuError = SurvivalAttentionError::InvalidFeatures.into();
        let status: Status = err.into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }
}
