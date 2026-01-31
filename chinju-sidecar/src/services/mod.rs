//! Service implementations

pub mod audit;
pub mod capability_test;
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
pub mod token;

pub use audit::{
    create_audit_system, create_audit_system_with_restore, AuditLogger, AuditPersister,
    FileStorage, StorageBackend,
};
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
pub use token::TokenService;
