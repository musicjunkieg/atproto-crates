//! NSID (Namespaced Identifier) syntax validation
//!
//! Validates NSID strings according to the AT Protocol specification.
//! An NSID identifies a lexicon schema with a reversed domain authority.

use std::sync::LazyLock;

use regex::Regex;

use crate::validation::data_errors::DataValidationError;

/// Regex for validating NSID syntax
///
/// Format: <reversed-domain>.<name>
/// - At least 3 segments separated by dots
/// - Authority segments (all but last): start with letter, alphanumeric and hyphens
/// - Name segment (last): starts with a letter, alphanumeric only
/// - Total length max 317 characters (253 for domain + 1 dot + 63 for name)
static NSID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z]([a-zA-Z0-9-]*[a-zA-Z0-9])?(\.[a-zA-Z]([a-zA-Z0-9-]*[a-zA-Z0-9])?)*\.[a-zA-Z][a-zA-Z0-9]*$")
        .expect("NSID regex should compile")
});

/// Validate an NSID string
///
/// A valid NSID must:
/// - Have at least 3 segments separated by dots
/// - Have authority segments that start with a letter
/// - Have a name segment (last) that starts with a letter and contains only alphanumeric chars
/// - Not exceed 317 characters
pub fn validate_nsid(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "nsid".to_string(),
            value: value.to_string(),
            reason: "NSID cannot be empty".to_string(),
        });
    }

    if value.len() > 317 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "nsid".to_string(),
            value: value.to_string(),
            reason: "NSID exceeds maximum length of 317 characters".to_string(),
        });
    }

    let segments: Vec<&str> = value.split('.').collect();

    if segments.len() < 3 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "nsid".to_string(),
            value: value.to_string(),
            reason: "NSID must have at least 3 segments".to_string(),
        });
    }

    // Check for empty segments
    if segments.iter().any(|s| s.is_empty()) {
        return Err(DataValidationError::StringFormatInvalid {
            format: "nsid".to_string(),
            value: value.to_string(),
            reason: "NSID segments must not be empty".to_string(),
        });
    }

    // Each segment max 63 characters
    for segment in &segments {
        if segment.len() > 63 {
            return Err(DataValidationError::StringFormatInvalid {
                format: "nsid".to_string(),
                value: value.to_string(),
                reason: "NSID segments must not exceed 63 characters".to_string(),
            });
        }
    }

    if !NSID_REGEX.is_match(value) {
        return Err(DataValidationError::StringFormatInvalid {
            format: "nsid".to_string(),
            value: value.to_string(),
            reason: "NSID does not match expected syntax".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_nsids() {
        let valid = [
            "app.bsky.feed.post",
            "com.atproto.repo.getRecord",
            "com.example.service.method",
            "a.b.c",
        ];
        for nsid in valid {
            assert!(validate_nsid(nsid).is_ok(), "should be valid: {}", nsid);
        }
    }

    #[test]
    fn test_invalid_nsids() {
        let invalid = [
            "",
            "app.bsky",
            "app",
            "app..feed.post",
            "1app.bsky.feed.post",
            ".bsky.feed.post",
        ];
        for nsid in invalid {
            assert!(validate_nsid(nsid).is_err(), "should be invalid: {}", nsid);
        }
    }
}
