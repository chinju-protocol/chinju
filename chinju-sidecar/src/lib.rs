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

pub mod gen;
pub mod services;

// Re-export common types
pub use chinju_core as core;
