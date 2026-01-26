//! URI syntax validation
//!
//! Validates generic URI strings according to RFC 3986.

use std::sync::LazyLock;

use regex::Regex;

use crate::validation::data_errors::DataValidationError;

/// Regex for validating URI syntax
///
/// A URI must have a scheme followed by ":" and scheme-specific content.
/// Scheme: letter followed by letters, digits, plus, hyphen, or dot.
static URI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z][a-zA-Z0-9+.-]*:.+$").expect("URI regex should compile")
});

/// Validate a URI string
///
/// A valid URI must:
/// - Have a scheme component (e.g., "https:", "at:", "did:")
/// - Have content after the scheme
/// - Not exceed 8192 bytes
pub fn validate_uri(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "uri".to_string(),
            value: value.to_string(),
            reason: "URI cannot be empty".to_string(),
        });
    }

    if value.len() > 8192 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "uri".to_string(),
            value: value.to_string(),
            reason: "URI exceeds maximum length of 8192 bytes".to_string(),
        });
    }

    if !URI_REGEX.is_match(value) {
        return Err(DataValidationError::StringFormatInvalid {
            format: "uri".to_string(),
            value: value.to_string(),
            reason: "URI must have a valid scheme followed by content".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_uris() {
        let valid = [
            "https://example.com",
            "http://example.com/path?query=1#fragment",
            "at://did:plc:asdf123",
            "did:plc:asdf123",
            "ftp://files.example.com/file.txt",
            "mailto:user@example.com",
            "urn:isbn:0451450523",
        ];
        for uri in valid {
            assert!(validate_uri(uri).is_ok(), "should be valid: {}", uri);
        }
    }

    #[test]
    fn test_invalid_uris() {
        let invalid = [
            "",
            "not a uri",
            "://missing-scheme",
            "123:invalid-scheme",
            ":no-scheme",
        ];
        for uri in invalid {
            assert!(validate_uri(uri).is_err(), "should be invalid: {}", uri);
        }
    }
}
