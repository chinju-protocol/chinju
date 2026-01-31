//! TPM 2.0 support for CHINJU Protocol
//!
//! This module provides TPM 2.0 integration using the tss-esapi crate.
//! Compatible with both hardware TPM and software TPM (swtpm).
//!
//! # Features
//! - Key generation and management (RSA, ECC)
//! - Signing and verification
//! - PCR (Platform Configuration Register) operations
//! - Sealing/Unsealing data to PCR values
//! - Remote attestation quotes
//!
//! # Setup (swtpm for development)
//!
//! ```bash
//! # Install swtpm
//! # macOS:
//! brew install swtpm
//!
//! # Linux:
//! apt-get install swtpm swtpm-tools
//!
//! # Start swtpm (socket mode)
//! mkdir -p /tmp/tpm
//! swtpm socket --tpmstate dir=/tmp/tpm \
//!     --ctrl type=tcp,port=2322 \
//!     --server type=tcp,port=2321 \
//!     --flags startup-clear
//!
//! # Or use Docker
//! docker run -d --name swtpm -p 2321:2321 -p 2322:2322 \
//!     ghcr.io/tpm2-software/swtpm:latest
//! ```
//!
//! # Environment Variables
//! - `TPM_INTERFACE`: Connection type ("socket" or "device")
//! - `TPM_HOST`: TPM host address (default: "localhost")
//! - `TPM_PORT`: TPM port (default: 2321)
//! - `TPM_DEVICE`: TPM device path (default: "/dev/tpm0")

mod context;
mod dead_mans_switch;
mod hsm;
mod pcr;

pub use context::{TpmConfig, TpmContext, TpmError, TpmInfo, TpmInterface};
pub use dead_mans_switch::TpmDeadMansSwitch;
pub use hsm::TpmHsm;
pub use pcr::{PcrBank, PcrValue};
