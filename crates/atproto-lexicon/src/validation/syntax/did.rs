//! DID (Decentralized Identifier) syntax validation
//!
//! Validates DID strings according to the W3C DID specification
//! and AT Protocol requirements.

use std::sync::LazyLock;

use regex::Regex;

use crate::validation::data_errors::DataValidationError;

/// Regex for validating DID syntax
///
/// Format: did:<method>:<method-specific-id>
/// - method: lowercase letters and digits
/// - method-specific-id: alphanumeric, dots, hyphens, underscores, colons, percent-encoded
static DID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^did:[a-z]+:[a-zA-Z0-9._:%-]+$").expect("DID regex should compile")
});

/// Validate a DID string
///
/// A valid DID must:
/// - Start with "did:"
/// - Have a method name of lowercase letters
/// - Have a method-specific identifier
/// - Not exceed 2048 characters
/// - Not end with ":"
pub fn validate_did(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "did".to_string(),
            value: value.to_string(),
            reason: "DID cannot be empty".to_string(),
        });
    }

    if !value.starts_with("did:") {
        return Err(DataValidationError::StringFormatInvalid {
            format: "did".to_string(),
            value: value.to_string(),
            reason: "DID must start with 'did:'".to_string(),
        });
    }

    if value.len() > 2048 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "did".to_string(),
            value: value.to_string(),
            reason: "DID exceeds maximum length of 2048 characters".to_string(),
        });
    }

    if value.ends_with(':') {
        return Err(DataValidationError::StringFormatInvalid {
            format: "did".to_string(),
            value: value.to_string(),
            reason: "DID must not end with ':'".to_string(),
        });
    }

    if !DID_REGEX.is_match(value) {
        return Err(DataValidationError::StringFormatInvalid {
            format: "did".to_string(),
            value: value.to_string(),
            reason: "DID does not match expected syntax".to_string(),
        });
    }

    // Must have at least 3 parts when split by ':'
    let parts: Vec<&str> = value.splitn(3, ':').collect();
    if parts.len() < 3 || parts[2].is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "did".to_string(),
            value: value.to_string(),
            reason: "DID must have a method-specific identifier".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_dids() {
        let valid = [
            "did:plc:7iza6de2dwap2sbkpav7c6c6",
            "did:web:example.com",
            "did:method:val",
            "did:method:VAL",
            "did:method:val123",
            "did:method:val:sub:path",
            "did:key:zQ3shZc2QzFh7MC8g...",
        ];
        for did in valid {
            assert!(validate_did(did).is_ok(), "should be valid: {}", did);
        }
    }

    #[test]
    fn test_invalid_dids() {
        let invalid = [
            "",
            "did",
            "did:",
            "did:method:",
            "not:a:did",
            "did:METHOD:val", // method must be lowercase
        ];
        for did in invalid {
            assert!(validate_did(did).is_err(), "should be invalid: {}", did);
        }
    }
}
