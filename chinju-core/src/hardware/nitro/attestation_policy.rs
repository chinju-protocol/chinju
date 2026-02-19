//! Attestation Policy for Nitro Enclaves
//!
//! This module provides policy definitions and verification for
//! Enclave attestation, enabling PCR-based access control.
//!
//! # PCR Values
//!
//! Nitro Enclaves have three PCR (Platform Configuration Register) values:
//!
//! | PCR | Description |
//! |-----|-------------|
//! | PCR0 | Enclave image file (EIF) hash |
//! | PCR1 | Linux kernel and bootstrap hash |
//! | PCR2 | Application hash (user code) |
//!
//! # KMS Key Policy
//!
//! AWS KMS supports condition keys for Enclave attestation:
//!
//! ```json
//! {
//!   "Condition": {
//!     "StringEquals": {
//!       "kms:RecipientAttestation:ImageSha384": "<PCR0>",
//!       "kms:RecipientAttestation:PCR0": "<PCR0>",
//!       "kms:RecipientAttestation:PCR1": "<PCR1>",
//!       "kms:RecipientAttestation:PCR2": "<PCR2>"
//!     }
//!   }
//! }
//! ```

use super::error::NitroError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// PCR index
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PcrIndex {
    /// PCR0: Enclave image file hash
    Pcr0 = 0,
    /// PCR1: Kernel and bootstrap hash
    Pcr1 = 1,
    /// PCR2: Application hash
    Pcr2 = 2,
}

impl PcrIndex {
    /// Get all PCR indices
    pub fn all() -> [PcrIndex; 3] {
        [PcrIndex::Pcr0, PcrIndex::Pcr1, PcrIndex::Pcr2]
    }

    /// Get the KMS condition key name
    pub fn kms_condition_key(&self) -> &'static str {
        match self {
            PcrIndex::Pcr0 => "kms:RecipientAttestation:PCR0",
            PcrIndex::Pcr1 => "kms:RecipientAttestation:PCR1",
            PcrIndex::Pcr2 => "kms:RecipientAttestation:PCR2",
        }
    }
}

/// PCR value (SHA-384 hash, 48 bytes)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PcrValue(pub [u8; 48]);

impl PcrValue {
    /// Create from hex string
    pub fn from_hex(hex: &str) -> Result<Self, NitroError> {
        let bytes =
            hex::decode(hex).map_err(|e| NitroError::InvalidConfig("Invalid PCR hex string"))?;

        if bytes.len() != 48 {
            return Err(NitroError::InvalidConfig(
                "PCR value must be 48 bytes (SHA-384)",
            ));
        }

        let mut arr = [0u8; 48];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }

    /// Check if all zeros (uninitialized)
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }
}

impl std::fmt::Display for PcrValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Attestation policy configuration
///
/// Defines expected PCR values for an Enclave deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationPolicy {
    /// Policy name/identifier
    pub name: String,
    /// Policy version
    pub version: String,
    /// Expected PCR values
    pub expected_pcrs: HashMap<PcrIndex, PcrValue>,
    /// Whether to allow debug mode Enclaves
    pub allow_debug: bool,
    /// Maximum timestamp age in seconds (for freshness)
    pub max_timestamp_age_secs: u64,
    /// Optional: Allowed signer certificates
    pub allowed_signers: Vec<String>,
}

impl Default for AttestationPolicy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            version: "1.0.0".to_string(),
            expected_pcrs: HashMap::new(),
            allow_debug: false,
            max_timestamp_age_secs: 300, // 5 minutes
            allowed_signers: Vec::new(),
        }
    }
}

impl AttestationPolicy {
    /// Create a new policy
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Create a debug/development policy (allows any PCR values)
    pub fn debug() -> Self {
        Self {
            name: "debug".to_string(),
            allow_debug: true,
            ..Default::default()
        }
    }

    /// Set expected PCR0 (Enclave image hash)
    pub fn with_pcr0(mut self, value: &str) -> Result<Self, NitroError> {
        self.expected_pcrs
            .insert(PcrIndex::Pcr0, PcrValue::from_hex(value)?);
        Ok(self)
    }

    /// Set expected PCR1 (Kernel hash)
    pub fn with_pcr1(mut self, value: &str) -> Result<Self, NitroError> {
        self.expected_pcrs
            .insert(PcrIndex::Pcr1, PcrValue::from_hex(value)?);
        Ok(self)
    }

    /// Set expected PCR2 (Application hash)
    pub fn with_pcr2(mut self, value: &str) -> Result<Self, NitroError> {
        self.expected_pcrs
            .insert(PcrIndex::Pcr2, PcrValue::from_hex(value)?);
        Ok(self)
    }

    /// Set all PCR values from a map
    pub fn with_pcrs(mut self, pcrs: HashMap<PcrIndex, PcrValue>) -> Self {
        self.expected_pcrs = pcrs;
        self
    }

    /// Load policy from environment variables
    ///
    /// Looks for:
    /// - `CHINJU_ENCLAVE_PCR0`: Expected PCR0 value (hex)
    /// - `CHINJU_ENCLAVE_PCR1`: Expected PCR1 value (hex)
    /// - `CHINJU_ENCLAVE_PCR2`: Expected PCR2 value (hex)
    /// - `CHINJU_ENCLAVE_ALLOW_DEBUG`: Allow debug mode (true/false)
    pub fn from_env() -> Result<Self, NitroError> {
        let mut policy = Self::new("env");

        if let Ok(pcr0) = std::env::var("CHINJU_ENCLAVE_PCR0") {
            policy = policy.with_pcr0(&pcr0)?;
        }

        if let Ok(pcr1) = std::env::var("CHINJU_ENCLAVE_PCR1") {
            policy = policy.with_pcr1(&pcr1)?;
        }

        if let Ok(pcr2) = std::env::var("CHINJU_ENCLAVE_PCR2") {
            policy = policy.with_pcr2(&pcr2)?;
        }

        policy.allow_debug = std::env::var("CHINJU_ENCLAVE_ALLOW_DEBUG")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if policy.expected_pcrs.is_empty() && !policy.allow_debug {
            warn!("No PCR values configured and debug mode disabled - policy will reject all attestations");
        }

        Ok(policy)
    }

    /// Verify PCR values against policy
    pub fn verify_pcrs(&self, pcrs: &HashMap<PcrIndex, PcrValue>) -> Result<(), NitroError> {
        // In debug mode, accept any PCR values
        if self.allow_debug {
            debug!("Debug mode: skipping PCR verification");
            return Ok(());
        }

        for (index, expected) in &self.expected_pcrs {
            let actual = pcrs.get(index).ok_or_else(|| NitroError::PcrMismatch {
                index: *index as usize,
                expected: expected.to_hex(),
                actual: "missing".to_string(),
            })?;

            if actual != expected {
                return Err(NitroError::PcrMismatch {
                    index: *index as usize,
                    expected: expected.to_hex(),
                    actual: actual.to_hex(),
                });
            }
        }

        info!("PCR verification passed");
        Ok(())
    }

    /// Generate KMS key policy conditions for this attestation policy
    ///
    /// Returns a JSON-compatible structure for AWS KMS key policy.
    pub fn to_kms_conditions(&self) -> HashMap<String, HashMap<String, String>> {
        let mut string_equals = HashMap::new();

        for (index, value) in &self.expected_pcrs {
            string_equals.insert(index.kms_condition_key().to_string(), value.to_hex());
        }

        let mut conditions = HashMap::new();
        if !string_equals.is_empty() {
            conditions.insert("StringEquals".to_string(), string_equals);
        }

        conditions
    }

    /// Generate AWS KMS key policy JSON
    pub fn to_kms_policy_json(&self, key_arn: &str, principal_arn: &str) -> String {
        let pcr_conditions: Vec<String> = self
            .expected_pcrs
            .iter()
            .map(|(index, value)| {
                format!(
                    r#"          "{}": "{}""#,
                    index.kms_condition_key(),
                    value.to_hex()
                )
            })
            .collect();

        let conditions_str = if pcr_conditions.is_empty() {
            "".to_string()
        } else {
            format!(
                r#",
      "Condition": {{
        "StringEquals": {{
{}
        }}
      }}"#,
                pcr_conditions.join(",\n")
            )
        };

        format!(
            r#"{{
  "Version": "2012-10-17",
  "Id": "chinju-enclave-policy",
  "Statement": [
    {{
      "Sid": "AllowAdminAccess",
      "Effect": "Allow",
      "Principal": {{
        "AWS": "{}"
      }},
      "Action": [
        "kms:Create*",
        "kms:Describe*",
        "kms:Enable*",
        "kms:List*",
        "kms:Put*",
        "kms:Update*",
        "kms:Revoke*",
        "kms:Disable*",
        "kms:Get*",
        "kms:Delete*",
        "kms:TagResource",
        "kms:UntagResource",
        "kms:ScheduleKeyDeletion",
        "kms:CancelKeyDeletion"
      ],
      "Resource": "*"
    }},
    {{
      "Sid": "AllowEnclaveDecrypt",
      "Effect": "Allow",
      "Principal": {{
        "AWS": "{}"
      }},
      "Action": [
        "kms:Decrypt",
        "kms:GenerateDataKey"
      ],
      "Resource": "*"{}
    }}
  ]
}}"#,
            principal_arn, principal_arn, conditions_str
        )
    }
}

/// Attestation policy verifier
///
/// Verifies attestation documents against a policy.
pub struct PolicyVerifier {
    policy: AttestationPolicy,
}

impl PolicyVerifier {
    /// Create a new verifier
    pub fn new(policy: AttestationPolicy) -> Self {
        Self { policy }
    }

    /// Create from environment
    pub fn from_env() -> Result<Self, NitroError> {
        Ok(Self::new(AttestationPolicy::from_env()?))
    }

    /// Get the policy
    pub fn policy(&self) -> &AttestationPolicy {
        &self.policy
    }

    /// Verify attestation document PCR values
    pub fn verify(&self, pcrs: &HashMap<PcrIndex, PcrValue>) -> Result<(), NitroError> {
        self.policy.verify_pcrs(pcrs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcr_value_from_hex() {
        let hex = "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let pcr = PcrValue::from_hex(hex).unwrap();
        assert!(pcr.is_zero());
    }

    #[test]
    fn test_pcr_value_roundtrip() {
        let hex = "ea85ddf01145538bc79518181e304f926365a971625f4c32581c7d3e26b60c840000000000000000000000000000000000";
        let pcr = PcrValue::from_hex(hex).unwrap();
        assert_eq!(pcr.to_hex(), hex);
    }

    #[test]
    fn test_policy_with_pcrs() {
        let policy = AttestationPolicy::new("test")
            .with_pcr0("000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
            .unwrap()
            .with_pcr1("111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111")
            .unwrap();

        assert_eq!(policy.expected_pcrs.len(), 2);
    }

    #[test]
    fn test_policy_verify_success() {
        let policy = AttestationPolicy::new("test")
            .with_pcr0("000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

        let mut pcrs = HashMap::new();
        pcrs.insert(
            PcrIndex::Pcr0,
            PcrValue::from_hex("000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
        );

        assert!(policy.verify_pcrs(&pcrs).is_ok());
    }

    #[test]
    fn test_policy_verify_mismatch() {
        let policy = AttestationPolicy::new("test")
            .with_pcr0("000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
            .unwrap();

        let mut pcrs = HashMap::new();
        pcrs.insert(
            PcrIndex::Pcr0,
            PcrValue::from_hex("111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111").unwrap(),
        );

        assert!(policy.verify_pcrs(&pcrs).is_err());
    }

    #[test]
    fn test_debug_policy() {
        let policy = AttestationPolicy::debug();
        let pcrs = HashMap::new();

        // Debug mode should accept empty PCRs
        assert!(policy.verify_pcrs(&pcrs).is_ok());
    }

    #[test]
    fn test_kms_policy_generation() {
        let policy = AttestationPolicy::new("test")
            .with_pcr0("ea85ddf01145538bc79518181e304f926365a971625f4c32581c7d3e26b60c840000000000000000000000000000000000")
            .unwrap();

        let json = policy.to_kms_policy_json(
            "arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012",
            "arn:aws:iam::123456789012:role/EnclaveRole",
        );

        assert!(json.contains("kms:RecipientAttestation:PCR0"));
        assert!(json.contains("ea85ddf01145538bc79518181e304f926365a971625f4c32581c7d3e26b60c84"));
    }
}
