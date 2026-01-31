//! Threshold signature support using FROST
//!
//! This module provides t-of-n threshold signatures using the FROST protocol.
//! FROST (Flexible Round-Optimized Schnorr Threshold) allows a group of signers
//! to collectively sign a message where at least t signers must participate.
//!
//! # Usage
//!
//! ```rust,ignore
//! use chinju_core::hardware::threshold::{FrostCoordinator, FrostParticipant};
//!
//! // Create a coordinator for 3-of-5 threshold
//! let mut coordinator = FrostCoordinator::new(3, 5)?;
//!
//! // Start DKG
//! let round1_packages = coordinator.start_dkg()?;
//!
//! // Exchange round1 packages between participants...
//! // Complete DKG
//! let key_packages = coordinator.complete_dkg(round2_packages)?;
//!
//! // Sign a message with at least t participants
//! let signature = coordinator.sign(&selected_signers, message)?;
//! ```

pub mod ceremony;
pub mod evidence;
mod frost;
mod key_share;

pub use ceremony::{Ceremony, CeremonyPhase, CeremonyRecord, ParticipantRecord};
pub use evidence::{CeremonyEvidence, EvidenceSummary, TimestampProof, TimestampProofType, VideoRecordingHash, WitnessRecord};
pub use frost::{FrostCoordinator, FrostError, FrostParticipant};
pub use key_share::{KeyShare, KeyShareStore};
