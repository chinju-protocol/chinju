//! Hardware provider factory
//!
//! This module provides factory functions to create hardware implementations
//! based on configuration and available features.

use crate::hardware::traits::{
    HardwareError, HardwareSecurityModule, ImmutableStorage, RandomSource,
};
use crate::hardware::dead_mans_switch::{
    DeadMansSwitch, DeadMansSwitchConfig, SoftDeadMansSwitch,
};
#[cfg(all(feature = "tpm", target_os = "linux"))]
use crate::hardware::tpm::{TpmConfig, TpmDeadMansSwitch};
use crate::types::TrustLevel;

/// HSM backend configuration
#[derive(Debug, Clone)]
pub enum HsmBackend {
    /// Mock implementation for testing (L0)
    Mock,
    /// SoftHSM2 via PKCS#11 (L1)
    #[cfg(feature = "softhsm")]
    SoftHsm {
        /// Path to PKCS#11 module library
        module_path: String,
        /// Slot number
        slot: u64,
        /// PIN for authentication
        pin: String,
    },
    /// TPM 2.0 (L2) - Linux only
    #[cfg(all(feature = "tpm", target_os = "linux"))]
    Tpm {
        /// TPM interface type ("socket", "device", "tabrmd")
        interface: String,
        /// TPM host (for socket interface)
        host: String,
        /// TPM port (for socket interface)
        port: u16,
        /// Device path (for device interface)
        device_path: String,
    },
    /// YubiHSM 2 (L3)
    #[cfg(feature = "yubihsm")]
    YubiHsm {
        /// Connector URL (e.g., "http://localhost:12345")
        connector_url: String,
        /// Authentication key ID
        auth_key_id: u16,
        /// Password
        password: String,
    },
    /// AWS Nitro Enclaves (L3)
    #[cfg(all(feature = "nitro", target_os = "linux"))]
    Nitro {
        /// Enclave CID (Context ID)
        cid: u32,
        /// vsock port
        port: u32,
        /// Debug mode (skips PCR verification)
        debug: bool,
    },
}

impl Default for HsmBackend {
    fn default() -> Self {
        Self::Mock
    }
}

/// Random source backend configuration
#[derive(Debug, Clone)]
pub enum RandomBackend {
    /// Mock implementation using system CSPRNG
    Mock,
    /// System /dev/urandom
    System,
    /// Hardware TRNG (if available)
    #[cfg(feature = "hardware")]
    Trng,
}

impl Default for RandomBackend {
    fn default() -> Self {
        Self::Mock
    }
}

/// OTP storage backend configuration
#[derive(Debug, Clone)]
pub enum OtpBackend {
    /// Mock in-memory implementation
    Mock,
    /// File-based persistent storage
    File { path: String },
    /// TPM-based storage (Linux only)
    #[cfg(all(feature = "tpm", target_os = "linux"))]
    Tpm,
}

impl Default for OtpBackend {
    fn default() -> Self {
        Self::Mock
    }
}

/// Hardware provider configuration
#[derive(Debug, Clone, Default)]
pub struct HardwareConfig {
    /// HSM backend selection
    pub hsm: HsmBackend,
    /// Random source backend selection
    pub random: RandomBackend,
    /// OTP storage backend selection
    pub otp: OtpBackend,
}

impl HardwareConfig {
    /// Create a new configuration with mock backends
    pub fn mock() -> Self {
        Self {
            hsm: HsmBackend::Mock,
            random: RandomBackend::Mock,
            otp: OtpBackend::Mock,
        }
    }

    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self, HardwareError> {
        let hsm_backend = std::env::var("CHINJU_HSM_BACKEND").unwrap_or_else(|_| "mock".into());

        let hsm = match hsm_backend.as_str() {
            "mock" => HsmBackend::Mock,
            #[cfg(feature = "softhsm")]
            "softhsm" => {
                let module_path = std::env::var("PKCS11_MODULE").map_err(|_| {
                    HardwareError::InvalidData("PKCS11_MODULE not set".into())
                })?;
                let slot = std::env::var("PKCS11_SLOT")
                    .unwrap_or_else(|_| "0".into())
                    .parse()
                    .map_err(|_| HardwareError::InvalidData("Invalid PKCS11_SLOT".into()))?;
                let pin = std::env::var("PKCS11_PIN")
                    .map_err(|_| HardwareError::InvalidData("PKCS11_PIN not set".into()))?;
                HsmBackend::SoftHsm {
                    module_path,
                    slot,
                    pin,
                }
            }
            #[cfg(all(feature = "tpm", target_os = "linux"))]
            "tpm" => {
                let interface = std::env::var("TPM_INTERFACE")
                    .unwrap_or_else(|_| "socket".into());
                let host = std::env::var("TPM_HOST")
                    .unwrap_or_else(|_| "localhost".into());
                let port: u16 = std::env::var("TPM_PORT")
                    .unwrap_or_else(|_| "2321".into())
                    .parse()
                    .map_err(|_| HardwareError::InvalidData("Invalid TPM_PORT".into()))?;
                let device_path = std::env::var("TPM_DEVICE")
                    .unwrap_or_else(|_| "/dev/tpm0".into());
                HsmBackend::Tpm {
                    interface,
                    host,
                    port,
                    device_path,
                }
            }
            #[cfg(feature = "yubihsm")]
            "yubihsm" => {
                let connector_url = std::env::var("YUBIHSM_CONNECTOR")
                    .unwrap_or_else(|_| "http://localhost:12345".into());
                let auth_key_id: u16 = std::env::var("YUBIHSM_AUTH_KEY_ID")
                    .unwrap_or_else(|_| "1".into())
                    .parse()
                    .map_err(|_| HardwareError::InvalidData("Invalid YUBIHSM_AUTH_KEY_ID".into()))?;
                let password = std::env::var("YUBIHSM_PASSWORD")
                    .map_err(|_| HardwareError::InvalidData("YUBIHSM_PASSWORD not set".into()))?;
                HsmBackend::YubiHsm {
                    connector_url,
                    auth_key_id,
                    password,
                }
            }
            #[cfg(all(feature = "nitro", target_os = "linux"))]
            "nitro" => {
                let cid: u32 = std::env::var("CHINJU_NITRO_ENCLAVE_CID")
                    .map_err(|_| HardwareError::InvalidData("CHINJU_NITRO_ENCLAVE_CID not set".into()))?
                    .parse()
                    .map_err(|_| HardwareError::InvalidData("Invalid CHINJU_NITRO_ENCLAVE_CID".into()))?;
                let port: u32 = std::env::var("CHINJU_NITRO_PORT")
                    .unwrap_or_else(|_| "5000".into())
                    .parse()
                    .map_err(|_| HardwareError::InvalidData("Invalid CHINJU_NITRO_PORT".into()))?;
                let debug = std::env::var("CHINJU_NITRO_DEBUG")
                    .map(|v| v == "true" || v == "1")
                    .unwrap_or(false);
                HsmBackend::Nitro { cid, port, debug }
            }
            _ => {
                tracing::warn!(
                    "Unknown HSM backend '{}', falling back to mock",
                    hsm_backend
                );
                HsmBackend::Mock
            }
        };

        Ok(Self {
            hsm,
            random: RandomBackend::Mock,
            otp: OtpBackend::Mock,
        })
    }

    /// Get the security level for this configuration
    pub fn security_level(&self) -> TrustLevel {
        match &self.hsm {
            HsmBackend::Mock => TrustLevel::Mock,
            #[cfg(feature = "softhsm")]
            HsmBackend::SoftHsm { .. } => TrustLevel::Software, // L1
            #[cfg(all(feature = "tpm", target_os = "linux"))]
            HsmBackend::Tpm { interface, .. } => {
                // Hardware TPM is L2, software TPM is L1
                if interface == "device" {
                    TrustLevel::HardwareStandard // L2
                } else {
                    TrustLevel::Software // L1 (swtpm)
                }
            }
            #[cfg(feature = "yubihsm")]
            HsmBackend::YubiHsm { .. } => TrustLevel::HardwareEnterprise, // L3
            #[cfg(all(feature = "nitro", target_os = "linux"))]
            HsmBackend::Nitro { debug, .. } => {
                if *debug {
                    TrustLevel::Software // L1 in debug mode
                } else {
                    TrustLevel::HardwareEnterprise // L3 in production
                }
            }
        }
    }
}

/// Create an HSM instance based on configuration
pub fn create_hsm(
    config: &HardwareConfig,
) -> Result<Box<dyn HardwareSecurityModule>, HardwareError> {
    match &config.hsm {
        HsmBackend::Mock => {
            use crate::hardware::mock::MockHsm;
            Ok(Box::new(MockHsm::new()))
        }
        #[cfg(feature = "softhsm")]
        HsmBackend::SoftHsm {
            module_path,
            slot,
            pin,
        } => {
            use crate::hardware::softhsm::SoftHsm;
            Ok(Box::new(SoftHsm::new(module_path, *slot, pin)?))
        }
        #[cfg(all(feature = "tpm", target_os = "linux"))]
        HsmBackend::Tpm {
            interface,
            host,
            port,
            device_path,
        } => {
            use crate::hardware::tpm::{TpmConfig, TpmHsm};
            let tpm_config = match interface.as_str() {
                "device" => TpmConfig::device(device_path),
                _ => TpmConfig::socket(host, *port),
            };
            Ok(Box::new(TpmHsm::new(tpm_config)?))
        }
        #[cfg(feature = "yubihsm")]
        HsmBackend::YubiHsm {
            connector_url,
            auth_key_id,
            password,
        } => {
            // YubiHSM implementation would go here
            Err(HardwareError::NotSupported)
        }
    }
}

/// Create a random source instance based on configuration
pub fn create_random(config: &HardwareConfig) -> Result<Box<dyn RandomSource>, HardwareError> {
    match &config.random {
        RandomBackend::Mock | RandomBackend::System => {
            use crate::hardware::mock::MockRandom;
            Ok(Box::new(MockRandom::new()))
        }
        #[cfg(feature = "hardware")]
        RandomBackend::Trng => {
            // TRNG implementation would go here
            Err(HardwareError::NotSupported)
        }
    }
}

/// Create an OTP storage instance based on configuration
pub fn create_otp(config: &HardwareConfig) -> Result<Box<dyn ImmutableStorage>, HardwareError> {
    match &config.otp {
        OtpBackend::Mock => {
            use crate::hardware::mock::MockOtp;
            Ok(Box::new(MockOtp::new()))
        }
        OtpBackend::File { path } => {
            use crate::hardware::mock::MockOtp;
            use std::path::PathBuf;
            Ok(Box::new(MockOtp::with_persistence(PathBuf::from(path))))
        }
        #[cfg(all(feature = "tpm", target_os = "linux"))]
        OtpBackend::Tpm => {
            // TPM-based OTP implementation would go here
            Err(HardwareError::NotSupported)
        }
    }
}

/// Create a secure execution instance based on configuration
///
/// This function creates an instance that implements `SecureExecution` trait,
/// which is used for TEE-based operations like seal/unseal and remote attestation.
#[cfg(all(feature = "nitro", target_os = "linux"))]
pub fn create_secure_execution(
    config: &HardwareConfig,
) -> Result<Box<dyn crate::hardware::traits::SecureExecution>, HardwareError> {
    match &config.hsm {
        HsmBackend::Nitro { cid, port, debug } => {
            use crate::hardware::nitro::{NitroConfig, NitroHsm, VsockConfig};

            let nitro_config = NitroConfig {
                vsock: VsockConfig {
                    cid: *cid,
                    port: *port,
                    timeout_ms: 5000,
                },
                attestation: if *debug {
                    crate::hardware::nitro::AttestationVerificationConfig::debug()
                } else {
                    crate::hardware::nitro::AttestationVerificationConfig::from_env()
                        .map_err(|e| HardwareError::InvalidData(e.to_string()))?
                },
                debug_mode: *debug,
            };

            Ok(Box::new(NitroHsm::new(nitro_config)?))
        }
        _ => Err(HardwareError::NotSupported),
    }
}

/// Create a Dead Man's Switch instance based on configuration
pub fn create_dead_mans_switch(
    config: &HardwareConfig,
) -> Result<Box<dyn DeadMansSwitch>, HardwareError> {
    match &config.hsm {
        HsmBackend::Mock => {
            Ok(Box::new(SoftDeadMansSwitch::default()))
        }
        #[cfg(all(feature = "tpm", target_os = "linux"))]
        HsmBackend::Tpm {
            interface,
            host,
            port,
            device_path,
        } => {
            let tpm_config = match interface.as_str() {
                "device" => TpmConfig::device(device_path),
                _ => TpmConfig::socket(host, *port),
            };
            Ok(Box::new(TpmDeadMansSwitch::default_config(tpm_config)))
        }
        // Fallback to soft switch for other backends for now
        _ => {
            Ok(Box::new(SoftDeadMansSwitch::default()))
        }
    }
}

/// Hardware provider that manages all hardware components
pub struct HardwareProvider {
    config: HardwareConfig,
    hsm: Box<dyn HardwareSecurityModule>,
    random: Box<dyn RandomSource>,
    otp: Box<dyn ImmutableStorage>,
}

impl HardwareProvider {
    /// Create a new hardware provider with the given configuration
    pub fn new(config: HardwareConfig) -> Result<Self, HardwareError> {
        let hsm = create_hsm(&config)?;
        let random = create_random(&config)?;
        let otp = create_otp(&config)?;

        Ok(Self {
            config,
            hsm,
            random,
            otp,
        })
    }

    /// Create a mock hardware provider for testing
    pub fn mock() -> Result<Self, HardwareError> {
        Self::new(HardwareConfig::mock())
    }

    /// Create a hardware provider from environment variables
    pub fn from_env() -> Result<Self, HardwareError> {
        Self::new(HardwareConfig::from_env()?)
    }

    /// Get the HSM instance
    pub fn hsm(&self) -> &dyn HardwareSecurityModule {
        self.hsm.as_ref()
    }

    /// Get the random source instance
    pub fn random(&self) -> &dyn RandomSource {
        self.random.as_ref()
    }

    /// Get the OTP storage instance
    pub fn otp(&self) -> &dyn ImmutableStorage {
        self.otp.as_ref()
    }

    /// Get the security level
    pub fn security_level(&self) -> TrustLevel {
        self.config.security_level()
    }

    /// Get the configuration
    pub fn config(&self) -> &HardwareConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider() {
        let provider = HardwareProvider::mock().unwrap();
        assert_eq!(provider.security_level(), TrustLevel::Mock);
        assert!(!provider.hsm().is_hardware_backed());
    }

    #[test]
    fn test_config_from_env_defaults_to_mock() {
        // Without env vars set, should default to mock
        std::env::remove_var("CHINJU_HSM_BACKEND");
        let config = HardwareConfig::from_env().unwrap();
        assert!(matches!(config.hsm, HsmBackend::Mock));
    }
}
