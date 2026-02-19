//! CHINJU Protocol Constants
//!
//! Centralized constant definitions for the CHINJU Protocol.
//! All magic numbers and default values should be defined here for easy maintenance.
//!
//! # Organization
//! - Policy: Policy engine thresholds and defaults
//! - Token: Token service parameters
//! - Rate Limiting: Request rate limits
//! - LPT: LPT monitoring thresholds
//! - Containment: C13 model containment parameters
//! - Security: Security-related constants
//! - Timing: Timeout and interval values

// =============================================================================
// Policy Constants (C9)
// =============================================================================

/// Policy engine default values
pub mod policy {
    /// Default policy identifier
    pub const DEFAULT_POLICY_ID: &str = "policy.default.v1";

    /// Japan-specific policy identifier
    pub const JP_POLICY_ID: &str = "policy.jp.v1";

    /// Capability score threshold for rate limiting (users below this get throttled)
    pub const CAPABILITY_THROTTLE_THRESHOLD: f64 = 0.5;

    /// Capability score threshold for sensitive operations (Japan policy)
    pub const JP_SENSITIVE_CAPABILITY_THRESHOLD: f64 = 0.7;

    /// Minimum capability score for credential validation
    pub const MIN_CAPABILITY_SCORE: f64 = 0.3;

    /// Rule priorities (higher = evaluated first)
    pub mod priority {
        /// Credential requirement rules
        pub const REQUIRE_CREDENTIAL: i32 = 100;
        /// Safety/content blocking rules
        pub const BLOCK_DANGEROUS: i32 = 90;
        /// Japan-specific high capability rules
        pub const JP_HIGH_CAPABILITY: i32 = 80;
        /// Rate limiting rules
        pub const RATE_LIMIT: i32 = 50;
        /// Enhanced audit rules
        pub const AUDIT_ENHANCED: i32 = 15;
        /// Basic audit rules
        pub const AUDIT_BASIC: i32 = 10;
        /// Default allow (catch-all)
        pub const DEFAULT_ALLOW: i32 = 1;
    }

    /// HTTP status codes for policy actions
    pub mod http_status {
        pub const UNAUTHORIZED: i32 = 401;
        pub const FORBIDDEN: i32 = 403;
        pub const TOO_MANY_REQUESTS: i32 = 429;
    }

    /// Content patterns for safety filtering
    pub mod patterns {
        /// Dangerous content regex pattern
        pub const DANGEROUS_CONTENT: &str = r"(?i)(how to (make|build|create) (bomb|weapon|virus))";

        /// Japan-specific sensitive content pattern
        pub const JP_SENSITIVE_CONTENT: &str = r"(?i)(医療|金融|法律|個人情報)";
    }

    /// Compliance retention periods (days)
    pub mod retention {
        pub const DEFAULT_DAYS: u32 = 90;
        pub const JP_COMPLIANCE_DAYS: u32 = 365;
    }
}

// =============================================================================
// Rate Limiting Constants
// =============================================================================

/// Rate limiting parameters
pub mod rate_limit {
    /// Requests per minute for low-capability users
    pub const LOW_CAPABILITY_RPM: u32 = 10;

    /// Default requests per second limit
    pub const DEFAULT_RPS: f64 = 10.0;

    /// Maximum concurrent requests
    pub const MAX_CONCURRENT: u32 = 5;

    /// Maximum tokens per single request
    pub const MAX_TOKENS_PER_REQUEST: u64 = 10000;
}

// =============================================================================
// Token Service Constants (C5)
// =============================================================================

/// Token service parameters
pub mod token {
    /// Default initial token balance
    pub const DEFAULT_INITIAL_BALANCE: u64 = 100_000;

    /// Token decay rate per second
    pub const DECAY_RATE_PER_SECOND: f64 = 0.0001;

    /// Minimum token balance (cannot go below)
    pub const MINIMUM_BALANCE: u64 = 100;

    /// Warning threshold for low balance
    pub const WARNING_THRESHOLD: u64 = 1000;

    /// Critical threshold for balance
    pub const CRITICAL_THRESHOLD: u64 = 100;

    /// Healthy balance threshold
    pub const HEALTHY_THRESHOLD: u64 = 1000;
}

// =============================================================================
// LPT Monitoring Constants (C11)
// =============================================================================

/// LPT (LLM Performance Tracking) parameters
pub mod lpt {
    /// Default LPT score (when healthy)
    pub const DEFAULT_SCORE: f64 = 0.75;

    /// Warning threshold for LPT score
    pub const WARNING_THRESHOLD: f64 = 0.6;

    /// Critical threshold for LPT score
    pub const CRITICAL_THRESHOLD: f64 = 0.4;

    /// Minimum samples required for reliable LPT calculation
    pub const MIN_SAMPLES: usize = 10;

    /// Maximum history size for LPT calculations
    pub const MAX_HISTORY_SIZE: usize = 1000;
}

// =============================================================================
// Model Containment Constants (C13)
// =============================================================================

/// C13 Model containment parameters
pub mod containment {
    /// Extraction deterrent defaults
    pub mod extraction {
        /// Maximum queries per hour per user
        pub const MAX_QUERIES_PER_HOUR: u32 = 100;

        /// Maximum similar queries allowed
        pub const MAX_SIMILAR_QUERIES: u32 = 10;

        /// Similarity threshold for duplicate detection (0.0-1.0)
        pub const SIMILARITY_THRESHOLD: f64 = 0.9;

        /// Watermark embedding strength (0.0-1.0)
        pub const WATERMARK_STRENGTH: f64 = 0.1;
    }

    /// Side channel protection defaults
    pub mod side_channel {
        /// Minimum response time in milliseconds (timing normalization)
        pub const MIN_RESPONSE_TIME_MS: u64 = 100;

        /// Maximum response time variance in milliseconds
        pub const MAX_TIME_VARIANCE_MS: u64 = 50;

        /// Jitter range for timing normalization
        pub const JITTER_RANGE_MS: u64 = 20;
    }

    /// Dead Man's Switch defaults
    pub mod dead_mans_switch {
        /// Heartbeat interval in seconds
        pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

        /// Grace period before trigger in seconds
        pub const GRACE_PERIOD_SECS: u64 = 60;

        /// Maximum missed heartbeats before warning
        pub const MAX_MISSED_HEARTBEATS: u32 = 3;
    }

    /// Output sanitization defaults
    pub mod sanitization {
        /// Maximum output length in characters
        pub const MAX_OUTPUT_LENGTH: usize = 100_000;

        /// PII pattern detection sensitivity (0.0-1.0)
        pub const PII_SENSITIVITY: f64 = 0.8;
    }
}

// =============================================================================
// Nitro Enclave Constants (L3)
// =============================================================================

/// Nitro Enclave parameters
pub mod nitro {
    /// Default vsock port for Enclave communication
    pub const DEFAULT_PORT: u32 = 5000;

    /// Connection timeout in milliseconds
    pub const TIMEOUT_MS: u64 = 5000;

    /// Maximum retry attempts for Enclave connection
    pub const MAX_RETRIES: u32 = 3;

    /// Retry delay in milliseconds
    pub const RETRY_DELAY_MS: u64 = 1000;
}

// =============================================================================
// Capability Evaluation Constants (C14)
// =============================================================================

/// C14 Capability evaluation parameters
pub mod capability {
    /// Complexity threshold for capability evaluation
    pub const COMPLEXITY_THRESHOLD: f64 = 0.7;

    /// Drift detection p-value threshold
    pub const DRIFT_P_VALUE_THRESHOLD: f64 = 0.05;

    /// Minimum samples for reliable evaluation
    pub const MIN_SAMPLES: usize = 50;

    /// History window size
    pub const HISTORY_WINDOW: usize = 100;
}

// =============================================================================
// Value Neuron Constants (C15)
// =============================================================================

/// C15 Value neuron monitoring parameters
pub mod value_neuron {
    /// Correlation threshold for value neuron identification
    pub const CORRELATION_THRESHOLD: f64 = 0.7;

    /// Causal importance threshold
    pub const CAUSAL_THRESHOLD: f64 = 0.5;

    /// Minimum samples for reliable detection
    pub const MIN_SAMPLES: usize = 100;

    /// RPE anomaly z-score threshold
    pub const RPE_ANOMALY_Z_THRESHOLD: f64 = 2.5;

    /// RPE history size
    pub const RPE_HISTORY_SIZE: usize = 1000;

    /// Oscillation detection window
    pub const OSCILLATION_WINDOW: usize = 10;

    /// Health critical threshold
    pub const HEALTH_CRITICAL: f64 = 0.3;
}

// =============================================================================
// Contradiction Controller Constants (C16)
// =============================================================================

/// C16 Contradiction controller parameters
pub mod contradiction {
    /// Maximum context tokens before warning
    pub const MAX_CONTEXT_TOKENS: u32 = 8000;

    /// Contradiction injection probability
    pub const INJECTION_PROBABILITY: f64 = 0.1;

    /// Collapse detection timeout in seconds
    pub const COLLAPSE_TIMEOUT_SECS: u64 = 30;

    /// Maximum session duration in minutes
    pub const MAX_SESSION_DURATION_MINS: u64 = 60;
}

// =============================================================================
// Survival Attention Constants (C17)
// =============================================================================

/// C17 Survival attention parameters
pub mod survival_attention {
    /// Default survival score threshold
    pub const SCORE_THRESHOLD: f64 = 0.5;

    /// Alpha adjustment rate
    pub const ALPHA_ADJUSTMENT_RATE: f64 = 0.01;

    /// Minimum alpha value
    pub const ALPHA_MIN: f64 = 0.0;

    /// Maximum alpha value
    pub const ALPHA_MAX: f64 = 1.0;
}

// =============================================================================
// Security Constants
// =============================================================================

/// Security-related constants
pub mod security {
    /// Minimum trust chain length for credential validation
    pub const MIN_CHAIN_LENGTH: usize = 0;

    /// Threshold signature minimum signers
    pub const MIN_THRESHOLD_SIGNERS: usize = 2;

    /// Default threshold (t of n)
    pub const DEFAULT_THRESHOLD_T: usize = 2;
    pub const DEFAULT_THRESHOLD_N: usize = 3;

    /// Session expiry in seconds
    pub const SESSION_EXPIRY_SECS: u64 = 3600;
}

// =============================================================================
// Timing Constants
// =============================================================================

/// Timeout and interval values
pub mod timing {
    /// Default request timeout in milliseconds
    pub const REQUEST_TIMEOUT_MS: u64 = 30_000;

    /// Health check interval in seconds
    pub const HEALTH_CHECK_INTERVAL_SECS: u64 = 30;

    /// Audit flush interval in seconds
    pub const AUDIT_FLUSH_INTERVAL_SECS: u64 = 5;

    /// Metrics collection interval in seconds
    pub const METRICS_INTERVAL_SECS: u64 = 10;
}

// =============================================================================
// Mock/Development Constants
// =============================================================================

/// Development and testing defaults
pub mod mock {
    /// Mock response prompt tokens
    pub const PROMPT_TOKENS: u32 = 50;

    /// Mock response completion tokens
    pub const COMPLETION_TOKENS: u32 = 100;

    /// Streaming chunk delay in milliseconds
    pub const STREAMING_CHUNK_DELAY_MS: u64 = 100;

    /// Default token cost for streaming
    pub const STREAMING_TOKEN_COST: u64 = 100;
}

// =============================================================================
// Environment Variable Names
// =============================================================================

/// Environment variable names for configuration
pub mod env {
    // Containment (C13)
    pub const C13_EXTRACTION_DETERRENT: &str = "CHINJU_C13_EXTRACTION_DETERRENT";
    pub const C13_OUTPUT_SANITIZATION: &str = "CHINJU_C13_OUTPUT_SANITIZATION";
    pub const C13_SIDE_CHANNEL_BLOCKING: &str = "CHINJU_C13_SIDE_CHANNEL_BLOCKING";
    pub const C13_DEAD_MANS_SWITCH: &str = "CHINJU_C13_DEAD_MANS_SWITCH";

    // Contradiction (C16)
    pub const C16_CONTRADICTION: &str = "CHINJU_C16_CONTRADICTION";

    // Nitro Enclave
    pub const NITRO_ENABLED: &str = "CHINJU_NITRO_ENABLED";
    pub const NITRO_ENCLAVE_CID: &str = "CHINJU_NITRO_ENCLAVE_CID";
    pub const NITRO_PORT: &str = "CHINJU_NITRO_PORT";
    pub const NITRO_DEBUG: &str = "CHINJU_NITRO_DEBUG";

    // Security
    pub const ALLOW_UNVERIFIED_HALT: &str = "CHINJU_ALLOW_UNVERIFIED_HALT";

    // OpenAI
    pub const OPENAI_API_KEY: &str = "OPENAI_API_KEY";
    pub const OPENAI_BASE_URL: &str = "OPENAI_BASE_URL";

    // Server
    pub const GRPC_PORT: &str = "CHINJU_GRPC_PORT";
    pub const HTTP_PORT: &str = "CHINJU_HTTP_PORT";
    pub const LOG_LEVEL: &str = "CHINJU_LOG_LEVEL";
}

// =============================================================================
// Protocol Version
// =============================================================================

/// Protocol version information
pub mod version {
    /// Current protocol version
    pub const PROTOCOL_VERSION: &str = "0.1.0";

    /// Minimum compatible version
    pub const MIN_COMPATIBLE_VERSION: &str = "0.1.0";

    /// API version
    pub const API_VERSION: &str = "v1";
}
