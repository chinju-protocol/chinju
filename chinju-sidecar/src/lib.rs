//! CHINJU Protocol Sidecar
//!
//! AI Gateway and Policy Enforcement proxy that sits between
//! clients and AI systems.
//!
//! ## Architecture
//!
//! ```text
//! Client → Sidecar → AI Model
//!              ↓
//!          Policy Engine
//!              ↓
//!          Token Manager
//!              ↓
//!          Audit Log
//! ```

pub mod error;
pub mod gen;
pub mod lock_order;
pub mod services;

// Re-export common types
pub use chinju_core as core;

// Re-export error types (10.4.1)
pub use error::{ChinjuError, ChinjuResult, CredentialError, GatewayError, StartupError, TokenError};
