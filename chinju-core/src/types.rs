//! Common types for CHINJU Protocol

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Universal identifier with namespace
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identifier {
    /// Namespace (e.g., "chinju", "user", "policy")
    pub namespace: String,
    /// Unique ID within namespace
    pub id: String,
    /// Optional version number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
}

impl Identifier {
    /// Create a new identifier
    pub fn new(namespace: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            id: id.into(),
            version: None,
        }
    }

    /// Create identifier with version
    pub fn with_version(namespace: impl Into<String>, id: impl Into<String>, version: u64) -> Self {
        Self {
            namespace: namespace.into(),
            id: id.into(),
            version: Some(version),
        }
    }

    /// Convert to string representation
    pub fn to_string_repr(&self) -> String {
        match self.version {
            Some(v) => format!("{}:{}:v{}", self.namespace, self.id, v),
            None => format!("{}:{}", self.namespace, self.id),
        }
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_repr())
    }
}

/// Timestamp with nanosecond precision
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp {
    /// Seconds since Unix epoch
    pub seconds: i64,
    /// Nanoseconds (0-999999999)
    pub nanos: u32,
}

impl Timestamp {
    /// Create timestamp from current time
    pub fn now() -> Self {
        let now = Utc::now();
        Self {
            seconds: now.timestamp(),
            nanos: now.timestamp_subsec_nanos(),
        }
    }

    /// Create from DateTime<Utc>
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self {
            seconds: dt.timestamp(),
            nanos: dt.timestamp_subsec_nanos(),
        }
    }

    /// Convert to DateTime<Utc>
    pub fn to_datetime(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp(self.seconds, self.nanos)
    }
}

/// Validity period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidityPeriod {
    pub not_before: Timestamp,
    pub not_after: Timestamp,
}

impl ValidityPeriod {
    /// Check if the current time is within the validity period
    pub fn is_valid_now(&self) -> bool {
        let now = Timestamp::now();
        now >= self.not_before && now <= self.not_after
    }
}

/// Signature algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignatureAlgorithm {
    Ed25519,
    EcdsaP256,
    EcdsaP384,
}

/// Digital signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub algorithm: SignatureAlgorithm,
    #[serde(with = "base64_bytes")]
    pub public_key: Vec<u8>,
    #[serde(with = "base64_bytes")]
    pub signature: Vec<u8>,
    pub signed_at: Timestamp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

/// Hash algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    Sha3_256,
    Sha3_512,
    Blake3,
}

/// Hash value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hash {
    pub algorithm: HashAlgorithm,
    #[serde(with = "base64_bytes")]
    pub value: Vec<u8>,
}

/// Threshold signature (t-of-n)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdSignature {
    /// Required number of signatures (t)
    pub threshold: u32,
    /// Total number of signers (n)
    pub total: u32,
    /// Collected signatures
    pub signatures: Vec<Signature>,
}

impl ThresholdSignature {
    /// Check if threshold is met
    pub fn is_threshold_met(&self) -> bool {
        self.signatures.len() >= self.threshold as usize
    }
}

/// Trust level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    /// L4: Critical infrastructure
    HardwareCritical,
    /// L3: Enterprise
    HardwareEnterprise,
    /// L2: Standard
    HardwareStandard,
    /// L1: Basic
    Software,
    /// L0: Development
    Mock,
}

/// Hardware attestation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareAttestation {
    pub trust_level: TrustLevel,
    pub hardware_type: String,
    #[serde(with = "base64_bytes")]
    pub attestation_data: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_signature: Option<Signature>,
    pub attested_at: Timestamp,
}

/// Severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Error detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub suggestions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    pub severity: Severity,
}

// Helper module for base64 serialization
mod base64_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    use base64::{engine::general_purpose::STANDARD, Engine};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        STANDARD.decode(&s).map_err(serde::de::Error::custom)
    }
}
