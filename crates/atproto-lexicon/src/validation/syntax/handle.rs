//! Handle syntax validation
//!
//! Validates AT Protocol handle strings according to the specification.
//! A handle is essentially a domain name with additional constraints.

use std::sync::LazyLock;

use regex::Regex;

use crate::validation::data_errors::DataValidationError;

/// Regex for validating handle syntax
///
/// A handle must be a valid domain name:
/// - Labels separated by dots
/// - Each label: starts with alphanumeric, can contain hyphens, ends with alphanumeric
/// - At least two labels
/// - TLD must not be all numeric
/// - Maximum 253 characters total
/// - Each label maximum 63 characters
static HANDLE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$")
        .expect("handle regex should compile")
});

/// Validate a handle string
///
/// A valid handle must:
/// - Be a valid domain name
/// - Have at least two labels (parts separated by dots)
/// - Not exceed 253 characters
/// - Have labels of at most 63 characters each
/// - Have a TLD that is not all numeric
/// - Not start or end with a hyphen in any label
pub fn validate_handle(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "handle".to_string(),
            value: value.to_string(),
            reason: "handle cannot be empty".to_string(),
        });
    }

    if value.len() > 253 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "handle".to_string(),
            value: value.to_string(),
            reason: "handle exceeds maximum length of 253 characters".to_string(),
        });
    }

    let labels: Vec<&str> = value.split('.').collect();

    // Must have at least 2 labels
    if labels.len() < 2 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "handle".to_string(),
            value: value.to_string(),
            reason: "handle must have at least two labels".to_string(),
        });
    }

    // Each label must be 1-63 characters
    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return Err(DataValidationError::StringFormatInvalid {
                format: "handle".to_string(),
                value: value.to_string(),
                reason: "handle labels must be 1-63 characters".to_string(),
            });
        }
    }

    // TLD must not be all numeric
    let tld = labels.last().unwrap();
    if tld.chars().all(|c| c.is_ascii_digit()) {
        return Err(DataValidationError::StringFormatInvalid {
            format: "handle".to_string(),
            value: value.to_string(),
            reason: "handle TLD must not be all numeric".to_string(),
        });
    }

    if !HANDLE_REGEX.is_match(value) {
        return Err(DataValidationError::StringFormatInvalid {
            format: "handle".to_string(),
            value: value.to_string(),
            reason: "handle does not match expected syntax".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_handles() {
        let valid = [
            "john.test",
            "jan.test",
            "a234567890123456789.test",
            "user.bsky.social",
            "example.com",
            "sub.domain.example.com",
        ];
        for handle in valid {
            assert!(
                validate_handle(handle).is_ok(),
                "should be valid: {}",
                handle
            );
        }
    }

    #[test]
    fn test_invalid_handles() {
        let invalid = [
            "",
            "john-.test",
            "john.0",
            "-john.test",
            "john",
            "@handle",
            "john..test",
        ];
        for handle in invalid {
            assert!(
                validate_handle(handle).is_err(),
                "should be invalid: {}",
                handle
            );
        }
    }
}
