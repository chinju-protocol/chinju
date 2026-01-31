//! Type conversion utilities between chinju-core and protobuf types

use crate::gen::chinju::common as proto;
use chinju_core::types as core;

// ============================================================================
// Timestamp conversions
// ============================================================================

impl From<core::Timestamp> for proto::Timestamp {
    fn from(ts: core::Timestamp) -> Self {
        proto::Timestamp {
            seconds: ts.seconds,
            nanos: ts.nanos as i32,
        }
    }
}

impl From<proto::Timestamp> for core::Timestamp {
    fn from(ts: proto::Timestamp) -> Self {
        core::Timestamp {
            seconds: ts.seconds,
            nanos: ts.nanos as u32,
        }
    }
}

impl From<&core::Timestamp> for proto::Timestamp {
    fn from(ts: &core::Timestamp) -> Self {
        proto::Timestamp {
            seconds: ts.seconds,
            nanos: ts.nanos as i32,
        }
    }
}

// ============================================================================
// SignatureAlgorithm conversions
// ============================================================================

impl From<core::SignatureAlgorithm> for proto::SignatureAlgorithm {
    fn from(algo: core::SignatureAlgorithm) -> Self {
        match algo {
            core::SignatureAlgorithm::Ed25519 => proto::SignatureAlgorithm::Ed25519,
            core::SignatureAlgorithm::EcdsaP256 => proto::SignatureAlgorithm::EcdsaP256,
            core::SignatureAlgorithm::EcdsaP384 => proto::SignatureAlgorithm::EcdsaP384,
        }
    }
}

impl From<proto::SignatureAlgorithm> for core::SignatureAlgorithm {
    fn from(algo: proto::SignatureAlgorithm) -> Self {
        match algo {
            proto::SignatureAlgorithm::Ed25519 => core::SignatureAlgorithm::Ed25519,
            proto::SignatureAlgorithm::EcdsaP256 => core::SignatureAlgorithm::EcdsaP256,
            proto::SignatureAlgorithm::EcdsaP384 => core::SignatureAlgorithm::EcdsaP384,
            proto::SignatureAlgorithm::Unspecified => core::SignatureAlgorithm::Ed25519, // default
        }
    }
}

// ============================================================================
// Signature conversions
// ============================================================================

impl From<core::Signature> for proto::Signature {
    fn from(sig: core::Signature) -> Self {
        proto::Signature {
            algorithm: proto::SignatureAlgorithm::from(sig.algorithm).into(),
            public_key: sig.public_key,
            signature: sig.signature,
            signed_at: Some(proto::Timestamp::from(sig.signed_at)),
            key_id: sig.key_id.unwrap_or_default(),
        }
    }
}

impl From<proto::Signature> for core::Signature {
    fn from(sig: proto::Signature) -> Self {
        core::Signature {
            algorithm: core::SignatureAlgorithm::from(
                proto::SignatureAlgorithm::try_from(sig.algorithm)
                    .unwrap_or(proto::SignatureAlgorithm::Ed25519),
            ),
            public_key: sig.public_key,
            signature: sig.signature,
            signed_at: sig
                .signed_at
                .map(core::Timestamp::from)
                .unwrap_or_else(core::Timestamp::now),
            key_id: if sig.key_id.is_empty() {
                None
            } else {
                Some(sig.key_id)
            },
        }
    }
}

// ============================================================================
// TrustLevel conversions
// ============================================================================

impl From<core::TrustLevel> for proto::TrustLevel {
    fn from(level: core::TrustLevel) -> Self {
        match level {
            core::TrustLevel::HardwareCritical => proto::TrustLevel::HardwareCritical,
            core::TrustLevel::HardwareEnterprise => proto::TrustLevel::HardwareEnterprise,
            core::TrustLevel::HardwareStandard => proto::TrustLevel::HardwareStandard,
            core::TrustLevel::Software => proto::TrustLevel::Software,
            core::TrustLevel::Mock => proto::TrustLevel::Mock,
        }
    }
}

impl From<proto::TrustLevel> for core::TrustLevel {
    fn from(level: proto::TrustLevel) -> Self {
        match level {
            proto::TrustLevel::HardwareCritical => core::TrustLevel::HardwareCritical,
            proto::TrustLevel::HardwareEnterprise => core::TrustLevel::HardwareEnterprise,
            proto::TrustLevel::HardwareStandard => core::TrustLevel::HardwareStandard,
            proto::TrustLevel::Software => core::TrustLevel::Software,
            proto::TrustLevel::Mock => core::TrustLevel::Mock,
            proto::TrustLevel::Unspecified => core::TrustLevel::Mock, // default
        }
    }
}

// ============================================================================
// HashAlgorithm conversions
// ============================================================================

impl From<core::HashAlgorithm> for proto::HashAlgorithm {
    fn from(algo: core::HashAlgorithm) -> Self {
        match algo {
            core::HashAlgorithm::Sha3_256 => proto::HashAlgorithm::Sha3256,
            core::HashAlgorithm::Sha3_512 => proto::HashAlgorithm::Sha3512,
            core::HashAlgorithm::Blake3 => proto::HashAlgorithm::Blake3,
        }
    }
}

impl From<proto::HashAlgorithm> for core::HashAlgorithm {
    fn from(algo: proto::HashAlgorithm) -> Self {
        match algo {
            proto::HashAlgorithm::Sha3256 => core::HashAlgorithm::Sha3_256,
            proto::HashAlgorithm::Sha3512 => core::HashAlgorithm::Sha3_512,
            proto::HashAlgorithm::Blake3 => core::HashAlgorithm::Blake3,
            proto::HashAlgorithm::Unspecified => core::HashAlgorithm::Sha3_256, // default
        }
    }
}

// ============================================================================
// Hash conversions
// ============================================================================

impl From<core::Hash> for proto::Hash {
    fn from(hash: core::Hash) -> Self {
        proto::Hash {
            algorithm: proto::HashAlgorithm::from(hash.algorithm).into(),
            value: hash.value,
        }
    }
}

impl From<proto::Hash> for core::Hash {
    fn from(hash: proto::Hash) -> Self {
        core::Hash {
            algorithm: core::HashAlgorithm::from(
                proto::HashAlgorithm::try_from(hash.algorithm)
                    .unwrap_or(proto::HashAlgorithm::Sha3256),
            ),
            value: hash.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_conversion() {
        let core_ts = core::Timestamp {
            seconds: 1234567890,
            nanos: 123456789,
        };
        let proto_ts: proto::Timestamp = core_ts.into();
        assert_eq!(proto_ts.seconds, 1234567890);
        assert_eq!(proto_ts.nanos, 123456789);

        let back: core::Timestamp = proto_ts.into();
        assert_eq!(back.seconds, 1234567890);
        assert_eq!(back.nanos, 123456789);
    }

    #[test]
    fn test_signature_algorithm_conversion() {
        assert_eq!(
            proto::SignatureAlgorithm::from(core::SignatureAlgorithm::Ed25519),
            proto::SignatureAlgorithm::Ed25519
        );
        assert_eq!(
            proto::SignatureAlgorithm::from(core::SignatureAlgorithm::EcdsaP256),
            proto::SignatureAlgorithm::EcdsaP256
        );
    }

    #[test]
    fn test_trust_level_conversion() {
        assert_eq!(
            proto::TrustLevel::from(core::TrustLevel::HardwareCritical),
            proto::TrustLevel::HardwareCritical
        );
        assert_eq!(
            core::TrustLevel::from(proto::TrustLevel::Mock),
            core::TrustLevel::Mock
        );
    }
}
