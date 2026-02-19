//! CHINJU Protocol Configuration
//!
//! Unified configuration management for the CHINJU Protocol.
//! All configuration is loaded from environment variables and can be
//! overridden programmatically.
//!
//! # Usage
//!
//! ```rust
//! use chinju_sidecar::config::ChinjuConfig;
//!
//! let config = ChinjuConfig::from_env();
//! println!("Nitro enabled: {}", config.nitro.enabled);
//! ```

use crate::constants::{self, env};
use crate::services::contradiction_controller::ContradictionControllerConfig;
use crate::services::extraction_deterrent::ExtractionDeterrentConfig;
use crate::services::sanitizer::{SanitizationMode, SanitizerConfig};
use crate::services::side_channel::SideChannelConfig;
use chinju_core::hardware::DeadMansSwitchConfig;
use std::time::Duration;
use thiserror::Error;

/// Configuration validation errors
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ConfigValidationError {
    #[error("Invalid value for {field}: {message}")]
    InvalidValue { field: String, message: String },
    #[error("Invalid configuration state: {0}")]
    InvalidState(String),
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Parse boolean from environment variable
fn parse_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}

/// Parse u32 from environment variable
fn parse_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Parse u16 from environment variable
fn parse_u16(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Parse u64 from environment variable
#[allow(dead_code)]
fn parse_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

/// Parse optional u32 from environment variable
fn parse_optional_u32(key: &str) -> Option<u32> {
    std::env::var(key).ok().and_then(|s| s.parse().ok())
}

/// Parse string from environment variable
fn parse_string(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Parse optional string from environment variable
fn parse_optional_string(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

// =============================================================================
// Main Configuration Struct
// =============================================================================

/// CHINJU Protocol unified configuration
#[derive(Debug, Clone)]
pub struct ChinjuConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// Containment (C13) configuration
    pub containment: ContainmentConfig,
    /// Nitro Enclave configuration
    pub nitro: NitroConfig,
    /// Policy engine configuration
    pub policy: PolicyConfig,
    /// Token service configuration
    pub token: TokenConfig,
    /// LPT monitoring configuration
    pub lpt: LptConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// OpenAI client configuration
    pub openai: OpenAiConfig,
}

impl ChinjuConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            server: ServerConfig::from_env(),
            containment: ContainmentConfig::from_env(),
            nitro: NitroConfig::from_env(),
            policy: PolicyConfig::default(),
            token: TokenConfig::default(),
            lpt: LptConfig::default(),
            security: SecurityConfig::from_env(),
            openai: OpenAiConfig::from_env(),
        }
    }

    /// Create default configuration for development
    pub fn development() -> Self {
        Self {
            server: ServerConfig::default(),
            containment: ContainmentConfig::disabled(),
            nitro: NitroConfig::disabled(),
            policy: PolicyConfig::default(),
            token: TokenConfig::default(),
            lpt: LptConfig::default(),
            security: SecurityConfig::development(),
            openai: OpenAiConfig::default(),
        }
    }

    /// Create configuration for production
    pub fn production() -> Self {
        Self {
            server: ServerConfig::default(),
            containment: ContainmentConfig::production(),
            nitro: NitroConfig::from_env(),
            policy: PolicyConfig::default(),
            token: TokenConfig::default(),
            lpt: LptConfig::default(),
            security: SecurityConfig::default(),
            openai: OpenAiConfig::from_env(),
        }
    }

    /// Validate cross-section and per-section configuration consistency
    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        self.server.validate()?;
        self.containment.validate()?;
        self.nitro.validate()?;
        self.security.validate()?;
        self.openai.validate()?;

        if self.containment.enable_nitro_enclave && !self.nitro.enabled {
            return Err(ConfigValidationError::InvalidState(
                "containment.enable_nitro_enclave is true but nitro.enabled is false".to_string(),
            ));
        }

        if self.nitro.enabled && self.nitro.cid.is_none() {
            return Err(ConfigValidationError::InvalidState(
                "nitro.enabled is true but nitro.cid is not set".to_string(),
            ));
        }

        if self.containment.enable_nitro_enclave {
            match (self.containment.nitro_enclave_cid, self.nitro.cid) {
                (Some(containment_cid), Some(nitro_cid)) if containment_cid != nitro_cid => {
                    return Err(ConfigValidationError::InvalidState(format!(
                        "nitro enclave CID mismatch: containment={} nitro={}",
                        containment_cid, nitro_cid
                    )));
                }
                _ => {}
            }
        }

        Ok(())
    }
}

impl Default for ChinjuConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

// =============================================================================
// Server Configuration
// =============================================================================

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// gRPC server port
    pub grpc_port: u16,
    /// HTTP server port
    pub http_port: u16,
    /// Log level
    pub log_level: String,
    /// Enable metrics endpoint
    pub enable_metrics: bool,
    /// Enable health endpoint
    pub enable_health: bool,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            grpc_port: parse_u16(env::GRPC_PORT, 50051),
            http_port: parse_u16(env::HTTP_PORT, 8080),
            log_level: parse_string(env::LOG_LEVEL, "info"),
            enable_metrics: true,
            enable_health: true,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.grpc_port == 0 {
            return Err(ConfigValidationError::InvalidValue {
                field: "server.grpc_port".to_string(),
                message: "must be non-zero".to_string(),
            });
        }

        if self.http_port == 0 {
            return Err(ConfigValidationError::InvalidValue {
                field: "server.http_port".to_string(),
                message: "must be non-zero".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

// =============================================================================
// Containment Configuration (C13)
// =============================================================================

/// C13 Model Containment configuration
#[derive(Debug, Clone)]
pub struct ContainmentConfig {
    /// Enable extraction deterrent (rate limiting, watermarking)
    pub enable_extraction_deterrent: bool,
    /// Enable output sanitization
    pub enable_output_sanitization: bool,
    /// Enable side-channel blocking (timing normalization)
    pub enable_side_channel_blocking: bool,
    /// Enable Dead Man's Switch monitoring
    pub enable_dead_mans_switch: bool,
    /// Enable Nitro Enclave for secure key operations (L3)
    pub enable_nitro_enclave: bool,
    /// Enable C16 structural contradiction injection
    pub enable_contradiction: bool,
    /// Sanitization mode
    pub sanitization_mode: SanitizationMode,
    /// Extraction deterrent config
    pub extraction_config: ExtractionDeterrentConfig,
    /// Sanitizer config
    pub sanitizer_config: SanitizerConfig,
    /// Side channel config
    pub side_channel_config: SideChannelConfig,
    /// Dead Man's Switch config
    pub dead_mans_switch_config: DeadMansSwitchConfig,
    /// C16 contradiction controller config
    pub contradiction_config: ContradictionControllerConfig,
    /// Nitro Enclave CID (required if enable_nitro_enclave is true)
    pub nitro_enclave_cid: Option<u32>,
    /// Nitro Enclave vsock port
    pub nitro_enclave_port: u32,
}

impl ContainmentConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        Self {
            enable_extraction_deterrent: parse_bool(env::C13_EXTRACTION_DETERRENT, true),
            enable_output_sanitization: parse_bool(env::C13_OUTPUT_SANITIZATION, true),
            enable_side_channel_blocking: parse_bool(env::C13_SIDE_CHANNEL_BLOCKING, true),
            enable_dead_mans_switch: parse_bool(env::C13_DEAD_MANS_SWITCH, true),
            enable_nitro_enclave: parse_bool(env::NITRO_ENABLED, false),
            enable_contradiction: parse_bool(env::C16_CONTRADICTION, false),
            sanitization_mode: SanitizationMode::Standard,
            extraction_config: ExtractionDeterrentConfig::default(),
            sanitizer_config: SanitizerConfig::default(),
            side_channel_config: SideChannelConfig::default(),
            dead_mans_switch_config: DeadMansSwitchConfig::default(),
            contradiction_config: ContradictionControllerConfig::default(),
            nitro_enclave_cid: parse_optional_u32(env::NITRO_ENCLAVE_CID),
            nitro_enclave_port: parse_u32(env::NITRO_PORT, constants::nitro::DEFAULT_PORT),
        }
    }

    /// Create config with all C13/C16 features disabled (for testing)
    pub fn disabled() -> Self {
        Self {
            enable_extraction_deterrent: false,
            enable_output_sanitization: false,
            enable_side_channel_blocking: false,
            enable_dead_mans_switch: false,
            enable_nitro_enclave: false,
            enable_contradiction: false,
            sanitization_mode: SanitizationMode::Standard,
            extraction_config: ExtractionDeterrentConfig::default(),
            sanitizer_config: SanitizerConfig::default(),
            side_channel_config: SideChannelConfig::default(),
            dead_mans_switch_config: DeadMansSwitchConfig::default(),
            contradiction_config: ContradictionControllerConfig::default(),
            nitro_enclave_cid: None,
            nitro_enclave_port: constants::nitro::DEFAULT_PORT,
        }
    }

    /// Create config for production (all features enabled)
    pub fn production() -> Self {
        Self::from_env()
    }

    /// Create config for production with Nitro Enclave (L3 security)
    pub fn production_with_nitro(enclave_cid: u32) -> Self {
        Self {
            enable_nitro_enclave: true,
            nitro_enclave_cid: Some(enclave_cid),
            ..Self::from_env()
        }
    }

    /// Check if any containment feature is enabled
    pub fn any_enabled(&self) -> bool {
        self.enable_extraction_deterrent
            || self.enable_output_sanitization
            || self.enable_side_channel_blocking
            || self.enable_dead_mans_switch
    }

    /// Get status string for logging
    pub fn status_string(&self) -> &'static str {
        if self.any_enabled() {
            "C13 enabled"
        } else {
            "C13 disabled"
        }
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.enable_nitro_enclave && self.nitro_enclave_cid.is_none() {
            return Err(ConfigValidationError::InvalidState(
                "containment.enable_nitro_enclave is true but containment.nitro_enclave_cid is not set".to_string(),
            ));
        }

        if self.enable_nitro_enclave && self.nitro_enclave_port == 0 {
            return Err(ConfigValidationError::InvalidValue {
                field: "containment.nitro_enclave_port".to_string(),
                message: "must be non-zero when nitro enclave is enabled".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for ContainmentConfig {
    fn default() -> Self {
        Self {
            enable_extraction_deterrent: true,
            enable_output_sanitization: true,
            enable_side_channel_blocking: true,
            enable_dead_mans_switch: true,
            enable_nitro_enclave: false,
            enable_contradiction: false,
            sanitization_mode: SanitizationMode::Standard,
            extraction_config: ExtractionDeterrentConfig::default(),
            sanitizer_config: SanitizerConfig::default(),
            side_channel_config: SideChannelConfig::default(),
            dead_mans_switch_config: DeadMansSwitchConfig::default(),
            contradiction_config: ContradictionControllerConfig::default(),
            nitro_enclave_cid: None,
            nitro_enclave_port: constants::nitro::DEFAULT_PORT,
        }
    }
}

// =============================================================================
// Nitro Enclave Configuration
// =============================================================================

/// Nitro Enclave configuration
#[derive(Debug, Clone)]
pub struct NitroConfig {
    /// Enable Nitro Enclave for secure key operations
    pub enabled: bool,
    /// Enclave CID (required if enabled)
    pub cid: Option<u32>,
    /// Enclave vsock port
    pub port: u32,
    /// Enable debug mode
    pub debug: bool,
    /// Connection timeout in milliseconds
    pub timeout_ms: u64,
}

impl NitroConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        Self {
            enabled: parse_bool(env::NITRO_ENABLED, false),
            cid: parse_optional_u32(env::NITRO_ENCLAVE_CID),
            port: parse_u32(env::NITRO_PORT, constants::nitro::DEFAULT_PORT),
            debug: parse_bool(env::NITRO_DEBUG, false),
            timeout_ms: constants::nitro::TIMEOUT_MS,
        }
    }

    /// Create disabled configuration
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            cid: None,
            port: constants::nitro::DEFAULT_PORT,
            debug: false,
            timeout_ms: constants::nitro::TIMEOUT_MS,
        }
    }

    /// Create configuration with specific CID
    pub fn with_cid(cid: u32) -> Self {
        Self {
            enabled: true,
            cid: Some(cid),
            port: constants::nitro::DEFAULT_PORT,
            debug: false,
            timeout_ms: constants::nitro::TIMEOUT_MS,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.enabled && self.cid.is_none() {
            return Err(ConfigValidationError::InvalidState(
                "nitro.enabled is true but nitro.cid is not set".to_string(),
            ));
        }

        if self.enabled && self.port == 0 {
            return Err(ConfigValidationError::InvalidValue {
                field: "nitro.port".to_string(),
                message: "must be non-zero when enabled".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for NitroConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

// =============================================================================
// Policy Configuration
// =============================================================================

/// Policy engine configuration
#[derive(Debug, Clone)]
pub struct PolicyConfig {
    /// Default policy ID
    pub default_policy_id: String,
    /// Enable Japan-specific policies
    pub enable_jp_policy: bool,
    /// Capability throttle threshold
    pub capability_throttle_threshold: f64,
    /// Japan sensitive capability threshold
    pub jp_sensitive_threshold: f64,
    /// Minimum capability score
    pub min_capability_score: f64,
    /// Enable content filtering
    pub enable_content_filter: bool,
    /// Custom dangerous content pattern (optional)
    pub dangerous_content_pattern: Option<String>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            default_policy_id: constants::policy::DEFAULT_POLICY_ID.to_string(),
            enable_jp_policy: true,
            capability_throttle_threshold: constants::policy::CAPABILITY_THROTTLE_THRESHOLD,
            jp_sensitive_threshold: constants::policy::JP_SENSITIVE_CAPABILITY_THRESHOLD,
            min_capability_score: constants::policy::MIN_CAPABILITY_SCORE,
            enable_content_filter: true,
            dangerous_content_pattern: None,
        }
    }
}

// =============================================================================
// Token Configuration
// =============================================================================

/// Token service configuration
#[derive(Debug, Clone)]
pub struct TokenConfig {
    /// Initial token balance
    pub initial_balance: u64,
    /// Decay rate per second
    pub decay_rate: f64,
    /// Minimum balance
    pub min_balance: u64,
    /// Warning threshold
    pub warning_threshold: u64,
    /// Enable auto-replenishment
    pub enable_auto_replenish: bool,
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self {
            initial_balance: constants::token::DEFAULT_INITIAL_BALANCE,
            decay_rate: constants::token::DECAY_RATE_PER_SECOND,
            min_balance: constants::token::MINIMUM_BALANCE,
            warning_threshold: constants::token::WARNING_THRESHOLD,
            enable_auto_replenish: false,
        }
    }
}

// =============================================================================
// LPT Configuration
// =============================================================================

/// LPT monitoring configuration
#[derive(Debug, Clone)]
pub struct LptConfig {
    /// Warning threshold
    pub warning_threshold: f64,
    /// Critical threshold
    pub critical_threshold: f64,
    /// Minimum samples for calculation
    pub min_samples: usize,
    /// Maximum history size
    pub max_history: usize,
}

impl Default for LptConfig {
    fn default() -> Self {
        Self {
            warning_threshold: constants::lpt::WARNING_THRESHOLD,
            critical_threshold: constants::lpt::CRITICAL_THRESHOLD,
            min_samples: constants::lpt::MIN_SAMPLES,
            max_history: constants::lpt::MAX_HISTORY_SIZE,
        }
    }
}

// =============================================================================
// Security Configuration
// =============================================================================

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Allow unverified emergency halt (DANGEROUS)
    pub allow_unverified_halt: bool,
    /// Require hardware attestation
    pub require_hardware_attestation: bool,
    /// Minimum trust chain length
    pub min_chain_length: usize,
    /// Threshold signature t value
    pub threshold_t: usize,
    /// Threshold signature n value
    pub threshold_n: usize,
    /// Session expiry duration
    pub session_expiry: Duration,
}

impl SecurityConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        Self {
            allow_unverified_halt: parse_bool(env::ALLOW_UNVERIFIED_HALT, false),
            require_hardware_attestation: false,
            min_chain_length: constants::security::MIN_CHAIN_LENGTH,
            threshold_t: constants::security::DEFAULT_THRESHOLD_T,
            threshold_n: constants::security::DEFAULT_THRESHOLD_N,
            session_expiry: Duration::from_secs(constants::security::SESSION_EXPIRY_SECS),
        }
    }

    /// Development configuration (relaxed security)
    pub fn development() -> Self {
        Self {
            allow_unverified_halt: true,
            require_hardware_attestation: false,
            min_chain_length: 0,
            threshold_t: 1,
            threshold_n: 1,
            session_expiry: Duration::from_secs(constants::security::SESSION_EXPIRY_SECS * 24),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.threshold_t == 0 || self.threshold_n == 0 {
            return Err(ConfigValidationError::InvalidValue {
                field: "security.threshold".to_string(),
                message: "threshold values must be non-zero".to_string(),
            });
        }

        if self.threshold_t > self.threshold_n {
            return Err(ConfigValidationError::InvalidValue {
                field: "security.threshold".to_string(),
                message: "threshold_t must be <= threshold_n".to_string(),
            });
        }

        Ok(())
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_unverified_halt: false,
            require_hardware_attestation: false,
            min_chain_length: constants::security::MIN_CHAIN_LENGTH,
            threshold_t: constants::security::DEFAULT_THRESHOLD_T,
            threshold_n: constants::security::DEFAULT_THRESHOLD_N,
            session_expiry: Duration::from_secs(constants::security::SESSION_EXPIRY_SECS),
        }
    }
}

// =============================================================================
// OpenAI Configuration
// =============================================================================

/// OpenAI client configuration
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// API key (optional - mock mode if not set)
    pub api_key: Option<String>,
    /// Base URL (optional - use default if not set)
    pub base_url: Option<String>,
    /// Request timeout
    pub timeout: Duration,
    /// Enable mock mode
    pub mock_mode: bool,
}

impl OpenAiConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        let api_key = parse_optional_string(env::OPENAI_API_KEY);
        let mock_mode = api_key.is_none();

        Self {
            api_key,
            base_url: parse_optional_string(env::OPENAI_BASE_URL),
            timeout: Duration::from_millis(constants::timing::REQUEST_TIMEOUT_MS),
            mock_mode,
        }
    }

    /// Check if mock mode is active
    pub fn is_mock(&self) -> bool {
        self.mock_mode || self.api_key.is_none()
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if !self.is_mock() && self.api_key.as_deref().unwrap_or("").is_empty() {
            return Err(ConfigValidationError::InvalidState(
                "openai.mock_mode is false but openai.api_key is empty".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            timeout: Duration::from_millis(constants::timing::REQUEST_TIMEOUT_MS),
            mock_mode: true,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ChinjuConfig::development();
        assert!(!config.containment.enable_extraction_deterrent);
        assert!(!config.nitro.enabled);
    }

    #[test]
    fn test_containment_disabled() {
        let config = ContainmentConfig::disabled();
        assert!(!config.any_enabled());
    }

    #[test]
    fn test_containment_production() {
        let config = ContainmentConfig::default();
        assert!(config.any_enabled());
    }

    #[test]
    fn test_policy_defaults() {
        let config = PolicyConfig::default();
        assert_eq!(
            config.capability_throttle_threshold,
            constants::policy::CAPABILITY_THROTTLE_THRESHOLD
        );
    }

    #[test]
    fn test_openai_mock_mode() {
        let config = OpenAiConfig::default();
        assert!(config.is_mock());
    }

    #[test]
    fn test_validate_nitro_requires_cid() {
        let mut config = ChinjuConfig::development();
        config.nitro.enabled = true;
        config.nitro.cid = None;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_threshold_relation() {
        let mut config = ChinjuConfig::development();
        config.security.threshold_t = 3;
        config.security.threshold_n = 2;
        assert!(config.validate().is_err());
    }
}
