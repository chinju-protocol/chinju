//! Strongly typed identifiers used across sidecar services.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

const MAX_ID_LEN: usize = 256;

/// Identifier validation errors.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum IdError {
    #[error("{kind} must not be empty")]
    Empty { kind: &'static str },
    #[error("{kind} exceeds maximum length ({max})")]
    TooLong { kind: &'static str, max: usize },
    #[error("{kind} contains invalid characters")]
    InvalidChars { kind: &'static str },
}

fn validate_id(kind: &'static str, value: &str) -> Result<(), IdError> {
    if value.is_empty() {
        return Err(IdError::Empty { kind });
    }
    if value.len() > MAX_ID_LEN {
        return Err(IdError::TooLong {
            kind,
            max: MAX_ID_LEN,
        });
    }
    if value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(IdError::InvalidChars { kind });
    }
    Ok(())
}

macro_rules! define_id {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
                let value = value.into();
                validate_id($kind, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = IdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

define_id!(RequestId, "request_id");
define_id!(CredentialId, "credential_id");
define_id!(UserId, "user_id");

impl UserId {
    pub fn anonymous() -> Self {
        // Static literal validated by construction rules.
        Self("anonymous".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_validates() {
        assert!(RequestId::new("req-1").is_ok());
        assert!(CredentialId::new("cred_abc").is_ok());
        assert!(UserId::new("user@example.com").is_ok());
    }

    #[test]
    fn id_rejects_empty_and_space() {
        assert!(RequestId::new("").is_err());
        assert!(RequestId::new("has space").is_err());
    }
}
