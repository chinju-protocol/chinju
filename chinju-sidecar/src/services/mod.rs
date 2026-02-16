//! Service implementations

pub mod audit;
pub mod analog_sanitizer;
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

pub use audit::{
    create_audit_system, create_audit_system_with_restore, AuditLogger, AuditPersister,
    FileStorage, StorageBackend,
};
pub use analog_sanitizer::AnalogSanitizer;
pub use capability_test::{
    CapabilityTestManager, CapabilityTestSession, ChallengeGenerator, HumannessDetector,
    ResponseEvaluator,
};
pub use credential::CredentialServiceImpl;
pub use extraction_deterrent::{
    compute_query_hash, ExtractionDeterrent, ExtractionDeterrentConfig, ExtractionDeterrentError,
};
pub use gateway::{ContainmentConfig, GatewayService};
pub use http_server::{create_router, start_http_server, HttpServerState};
pub use lpt_monitor::{LptConfig, LptMonitor, LptScore, LptState, LptSummary, ResponseRecord};
pub use metrics::{MetricsCollector, MetricsStats};
pub use openai_client::{OpenAiClient, OpenAiClientConfig};
pub use policy::{PolicyEngine, RequestContext};
pub use sanitizer::{OutputSanitizer, SanitizationMode, SanitizerConfig};
pub use nitro::{NitroService, NitroServiceConfig, NitroStatus};
pub use side_channel::{SideChannelBlocker, SideChannelConfig, TimingGuard};
pub use signature::{SigningService, ThresholdConfig, ThresholdVerifier};
pub use token::{TokenService, TokenServiceConfig, TokenServiceImpl};

// C14-C17 services
pub use capability_evaluator::{CapabilityEvaluator, CapabilityEvaluatorConfig, CapabilityEvaluatorImpl};
pub use value_neuron_monitor::{ValueNeuronMonitor, ValueNeuronMonitorConfig, ValueNeuronMonitorImpl};
pub use contradiction_controller::{
    ContradictionController, ContradictionControllerConfig, ContradictionControllerImpl,
};
pub use survival_attention::{SurvivalAttentionService, SurvivalAttentionConfig, SurvivalAttentionServiceImpl};

// C12: ZKP verification
pub use zkp::{verify_humanity_proof, is_zkp_enabled, ZkpError, ZkpVerifier};
