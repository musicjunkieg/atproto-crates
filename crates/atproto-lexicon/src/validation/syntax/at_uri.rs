//! AT-URI syntax validation
//!
//! An AT-URI has the format: `at://<authority>/<collection>/<rkey>`
//! where authority is a DID or handle, collection is an NSID, and rkey is a record key.

use crate::validation::data_errors::DataValidationError;
use crate::validation::syntax::{validate_at_identifier, validate_nsid, validate_record_key};

/// Validate an AT-URI string
pub fn validate_at_uri(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "at-uri".to_string(),
            value: value.to_string(),
            reason: "AT-URI cannot be empty".to_string(),
        });
    }

    // Must start with "at://"
    let rest =
        value
            .strip_prefix("at://")
            .ok_or_else(|| DataValidationError::StringFormatInvalid {
                format: "at-uri".to_string(),
                value: value.to_string(),
                reason: "AT-URI must start with 'at://'".to_string(),
            })?;

    if rest.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "at-uri".to_string(),
            value: value.to_string(),
            reason: "AT-URI must have an authority".to_string(),
        });
    }

    // Must not have a fragment
    if value.contains('#') {
        return Err(DataValidationError::StringFormatInvalid {
            format: "at-uri".to_string(),
            value: value.to_string(),
            reason: "AT-URI must not contain a fragment".to_string(),
        });
    }

    // Must not have query parameters
    if value.contains('?') {
        return Err(DataValidationError::StringFormatInvalid {
            format: "at-uri".to_string(),
            value: value.to_string(),
            reason: "AT-URI must not contain query parameters".to_string(),
        });
    }

    // Split into path segments
    let segments: Vec<&str> = rest.splitn(3, '/').collect();

    // Validate authority (first segment)
    let authority = segments[0];
    validate_at_identifier(authority).map_err(|_| DataValidationError::StringFormatInvalid {
        format: "at-uri".to_string(),
        value: value.to_string(),
        reason: format!("invalid authority: {}", authority),
    })?;

    // If there's a collection, validate it
    if segments.len() > 1 && !segments[1].is_empty() {
        let collection = segments[1];
        validate_nsid(collection).map_err(|_| DataValidationError::StringFormatInvalid {
            format: "at-uri".to_string(),
            value: value.to_string(),
            reason: format!("invalid collection NSID: {}", collection),
        })?;

        // If there's a record key, validate it
        if segments.len() > 2 && !segments[2].is_empty() {
            let rkey = segments[2];
            validate_record_key(rkey).map_err(|_| DataValidationError::StringFormatInvalid {
                format: "at-uri".to_string(),
                value: value.to_string(),
                reason: format!("invalid record key: {}", rkey),
            })?;
        }
    }

    // Enforce max length of 8 KiB
    if value.len() > 8192 {
        return Err(DataValidationError::StringFormatInvalid {
            format: "at-uri".to_string(),
            value: value.to_string(),
            reason: "AT-URI exceeds maximum length of 8192 bytes".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_at_uris() {
        let valid = [
            "at://did:plc:asdf123",
            "at://did:plc:asdf123/app.bsky.feed.post",
            "at://did:plc:asdf123/app.bsky.feed.post/3jui7kd54zh2y",
            "at://user.bsky.social",
            "at://user.bsky.social/app.bsky.feed.post",
        ];
        for uri in valid {
            assert!(validate_at_uri(uri).is_ok(), "should be valid: {}", uri);
        }
    }

    #[test]
    fn test_invalid_at_uris() {
        let invalid = [
            "",
            "http://example.com",
            "at://",
            "at://did:plc:asdf123#fragment",
            "at://did:plc:asdf123?query=1",
            "at://invalid",
        ];
        for uri in invalid {
            assert!(validate_at_uri(uri).is_err(), "should be invalid: {}", uri);
        }
    }
}
