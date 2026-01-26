//! AT-Identifier syntax validation
//!
//! An AT-Identifier is either a DID or a Handle

use crate::validation::data_errors::DataValidationError;
use crate::validation::syntax::{validate_did, validate_handle};

/// Validate an AT-Identifier (either a DID or Handle)
pub fn validate_at_identifier(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "at-identifier".to_string(),
            value: value.to_string(),
            reason: "at-identifier cannot be empty".to_string(),
        });
    }
    if value.starts_with("did:") {
        return validate_did(value).map_err(|_| DataValidationError::StringFormatInvalid {
            format: "at-identifier".to_string(),
            value: value.to_string(),
            reason: "invalid DID".to_string(),
        });
    }
    validate_handle(value).map_err(|_| DataValidationError::StringFormatInvalid {
        format: "at-identifier".to_string(),
        value: value.to_string(),
        reason: "invalid handle".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_valid_at_identifiers() {
        let valid = [
            "john.test",
            "jan.test",
            "a234567890123456789.test",
            "did:method:val",
            "did:method:VAL",
            "did:plc:7iza6de2dwap2sbkpav7c6c6",
        ];
        for id in valid {
            assert!(
                validate_at_identifier(id).is_ok(),
                "should be valid: {}",
                id
            );
        }
    }
    #[test]
    fn test_invalid_at_identifiers() {
        let invalid = [
            "did",
            "didmethodval",
            "john-.test",
            "john.0",
            "",
            "email@example.com",
            "@handle",
        ];
        for id in invalid {
            assert!(
                validate_at_identifier(id).is_err(),
                "should be invalid: {}",
                id
            );
        }
    }
}
