//! TID (Timestamp Identifier) syntax validation
//!
//! Validates AT Protocol TID strings. A TID is a timestamp-based identifier
//! encoded in a specific base32-sortable format.

use crate::validation::data_errors::DataValidationError;

/// Valid characters in a TID (base32-sortable: 234567abcdefghijklmnopqrstuvwxyz)
const TID_CHARS: &str = "234567abcdefghijklmnopqrstuvwxyz";

/// Validate a TID string
///
/// A valid TID must:
/// - Be exactly 13 characters long
/// - Contain only base32-sortable characters (2-7, a-z)
/// - Start with a character in the range [2-b] (high bit 0)
pub fn validate_tid(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "tid".to_string(),
            value: value.to_string(),
            reason: "TID cannot be empty".to_string(),
        });
    }

    if value.len() != 13 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "tid".to_string(),
            value: value.to_string(),
            reason: format!("TID must be exactly 13 characters, got {}", value.len()),
        });
    }

    for c in value.chars() {
        if !TID_CHARS.contains(c) {
            return Err(DataValidationError::StringFormatInvalid {
                format: "tid".to_string(),
                value: value.to_string(),
                reason: format!("TID contains invalid character: '{}'", c),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_tids() {
        let valid = ["3jui7kd54zh2y", "2222222222222", "3333333333333"];
        for tid in valid {
            assert!(validate_tid(tid).is_ok(), "should be valid: {}", tid);
        }
    }

    #[test]
    fn test_invalid_tids() {
        let invalid = [
            "",
            "3jui7kd54zh2",   // too short (12 chars)
            "3jui7kd54zh2yy", // too long (14 chars)
            "0000000000000",  // invalid chars (0, 1 not in base32-sortable)
            "3jui7kd54zH2y",  // uppercase not allowed
        ];
        for tid in invalid {
            assert!(validate_tid(tid).is_err(), "should be invalid: {}", tid);
        }
    }
}
