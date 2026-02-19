//! Service implementations
//!
//! This module contains all service implementations for the CHINJU Protocol Sidecar.
//!
//! # Core Services
//!
//! - `gateway`: AI Gateway service (main entry point)
//! - `policy`: Policy engine for request control
//! - `credential`: Human credential verification
//! - `token`: Token-based resource management
//! - `audit`: Audit logging (C6)
//!
//! # Safety Services (C13)
//!
//! - `extraction_deterrent`: Rate limiting and watermarking
//! - `sanitizer`: Output sanitization
//! - `side_channel`: Timing normalization
//!
//! # Monitoring Services
//!
//! - `lpt_monitor`: LLM Performance Tracking (C11)
//! - `capability_evaluator`: Capability evaluation (C14)
//! - `value_neuron_monitor`: Value neuron monitoring (C15)
//! - `contradiction_controller`: Structural contradiction (C16)
//! - `survival_attention`: Survival attention (C17)

pub mod analog_sanitizer;
pub mod audit;
pub mod capability_evaluator;
pub mod capability_test;
pub mod contradiction_controller;
pub mod credential;
pub mod extraction_deterrent;
pub mod gateway;
pub mod http_server;
pub mod lpt_monitor;
pub mod metrics;
pub mod nitro;
pub mod openai_client;
pub mod openai_types;
pub mod policy;
pub mod sanitizer;
pub mod side_channel;
pub mod signature;
pub mod survival_attention;
pub mod token;
pub mod value_neuron_monitor;
pub mod zkp;

// Core service re-exports
pub use audit::{
    create_audit_system, create_audit_system_with_restore, AuditLogger, AuditPersister,
    FileStorage, StorageBackend,
};
pub use credential::CredentialServiceImpl;
pub use gateway::{ContainmentChecker, ContainmentConfig, GatewayService};
pub use policy::{PolicyEngine, RequestContext};
pub use token::{TokenService, TokenServiceConfig, TokenServiceImpl};

// Safety service re-exports (C13)
pub use analog_sanitizer::AnalogSanitizer;
pub use extraction_deterrent::{
    compute_query_hash, ExtractionDeterrent, ExtractionDeterrentConfig, ExtractionDeterrentError,
};
pub use sanitizer::{OutputSanitizer, SanitizationMode, SanitizerConfig};
pub use side_channel::{SideChannelBlocker, SideChannelConfig, TimingGuard};

// Infrastructure re-exports
pub use http_server::{create_router, start_http_server, HttpServerState};
pub use metrics::{MetricsCollector, MetricsStats};
pub use nitro::{NitroService, NitroServiceConfig, NitroStatus};
pub use openai_client::{OpenAiClient, OpenAiClientConfig};
pub use signature::{SigningService, ThresholdConfig, ThresholdVerifier};

// Monitoring service re-exports
pub use capability_test::{
    CapabilityTestManager, CapabilityTestSession, ChallengeGenerator, HumannessDetector,
    ResponseEvaluator,
};
pub use lpt_monitor::{LptConfig, LptMonitor, LptScore, LptState, LptSummary, ResponseRecord};

// C14-C17 services
pub use capability_evaluator::{
    CapabilityEvaluator, CapabilityEvaluatorConfig, CapabilityEvaluatorImpl,
};
pub use contradiction_controller::{
    ContradictionController, ContradictionControllerConfig, ContradictionControllerImpl,
};
pub use survival_attention::{
    SurvivalAttentionConfig, SurvivalAttentionService, SurvivalAttentionServiceImpl,
};
pub use value_neuron_monitor::{
    ValueNeuronMonitor, ValueNeuronMonitorConfig, ValueNeuronMonitorImpl,
};

// C12: ZKP verification
pub use zkp::{is_zkp_enabled, verify_humanity_proof, ZkpError, ZkpVerifier};
