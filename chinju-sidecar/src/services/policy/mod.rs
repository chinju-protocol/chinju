//! Policy management subsystem
//!
//! This module provides policy evaluation, provider management,
//! and federated policy support.

mod engine;
pub mod provider;
pub mod providers;
pub mod revocation;
pub mod signature;
pub mod versioning;

// Re-export main types from engine (original policy.rs)
pub use engine::{PolicyEngine, RequestContext};

// Re-export provider types
pub use provider::{PolicyProvider, PolicyProviderError, PolicyProviderRegistry, PolicyUpdate};
pub use providers::FileProvider;

// Re-export signature types
pub use signature::PolicySigner;

// Re-export versioning types
pub use versioning::{PolicyVersion, PolicyVersionStore, VersionState};

// Re-export revocation types
pub use revocation::{
    PolicyIdentifier, RevocationCache, RevocationNotice, RevocationPropagator, RevokeReason,
};
