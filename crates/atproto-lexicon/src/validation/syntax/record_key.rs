//! Record key syntax validation
//!
//! Validates AT Protocol record key strings (rkey).

use crate::validation::data_errors::DataValidationError;

/// Validate a record key string
///
/// A valid record key must:
/// - Be 1-512 characters long
/// - Contain only alphanumeric characters, dots, hyphens, underscores, tildes, and colons
/// - Not be "." or ".."
pub fn validate_record_key(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "record-key".to_string(),
            value: value.to_string(),
            reason: "record key cannot be empty".to_string(),
        });
    }

    if value.len() > 512 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "record-key".to_string(),
            value: value.to_string(),
            reason: "record key exceeds maximum length of 512 characters".to_string(),
        });
    }

    // Must not be "." or ".."
    if value == "." || value == ".." {
        return Err(DataValidationError::StringFormatInvalid {
            format: "record-key".to_string(),
            value: value.to_string(),
            reason: "record key must not be '.' or '..'".to_string(),
        });
    }

    // Must only contain valid characters
    for c in value.chars() {
        if !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '_' && c != '~' && c != ':' {
            return Err(DataValidationError::StringFormatInvalid {
                format: "record-key".to_string(),
                value: value.to_string(),
                reason: format!("record key contains invalid character: '{}'", c),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_record_keys() {
        let valid = [
            "3jui7kd54zh2y",
            "self",
            "example.com",
            "key-with-hyphens",
            "key_with_underscores",
            "key~with~tildes",
            "key:with:colons",
        ];
        for key in valid {
            assert!(validate_record_key(key).is_ok(), "should be valid: {}", key);
        }
    }

    #[test]
    fn test_invalid_record_keys() {
        let invalid = ["", ".", "..", "key with spaces", "key/with/slashes"];
        for key in invalid {
            assert!(
                validate_record_key(key).is_err(),
                "should be invalid: {}",
                key
            );
        }
    }
}
