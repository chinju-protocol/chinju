//! AI Gateway Service
//!
//! The main entry point for AI requests.
//!
//! # Architecture
//!
//! The gateway is composed of several specialized components:
//!
//! - `GatewayService`: Main gRPC service implementation
//! - `ContainmentChecker`: C13 model containment checks
//! - `RequestProcessor`: Request processing pipeline components
//!   - `CredentialVerifier`: Credential verification
//!   - `PolicyEvaluator`: Policy evaluation
//!   - `ResponseGenerator`: AI response generation
//!   - `ResponsePostProcessor`: Sanitization and watermarking
//!   - `TokenConsumer`: Token consumption
//!   - `LptRecorder`: LPT monitoring
//!
//! # Usage
//!
//! ```rust,ignore
//! use chinju_sidecar::services::gateway::{GatewayService, ContainmentConfig};
//!
//! let gateway = GatewayService::new(
//!     token_service,
//!     credential_service,
//!     policy_engine,
//! ).await;
//! ```

mod containment_checker;
mod request_processor;
mod service;

// Re-export main types
pub use containment_checker::{ContainmentChecker, PreFlightCheckResult};
pub use request_processor::{
    build_warnings, CredentialVerifier, LptRecorder, PolicyEvaluator, ProcessingContext,
    ProcessingResult, ResponseGenerator, ResponsePostProcessor, TokenConsumer,
};
pub use service::GatewayService;

// Re-export ContainmentConfig from config module for backward compatibility
pub use crate::config::ContainmentConfig;
