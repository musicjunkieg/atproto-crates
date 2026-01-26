//! CID (Content Identifier) syntax validation
//!
//! Validates CID strings by parsing them with `atproto_dasl::Cid`, which performs
//! full multibase, multicodec, and multihash validation rather than just checking
//! character sets.

use crate::validation::data_errors::DataValidationError;

/// Validate a CID string
///
/// Parses the string as a CIDv1 in base32lower encoding using `atproto_dasl::Cid`.
/// This validates the full CID structure including version, codec, and multihash,
/// not just the character set.
pub fn validate_cid(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "cid".to_string(),
            value: value.to_string(),
            reason: "CID cannot be empty".to_string(),
        });
    }

    value
        .parse::<atproto_dasl::Cid>()
        .map_err(|e| DataValidationError::StringFormatInvalid {
            format: "cid".to_string(),
            value: value.to_string(),
            reason: e.to_string(),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cids() {
        let valid = [
            "bafyreidfayvfkicffzikfhbdrqvjobjzpmvcfc7kzbih2noclhf3vqywue",
            "bafyreie5cvv4h45feadgeuwhbcutmh6t7ceseocckahdoe6uat64zmz454",
        ];
        for cid in valid {
            assert!(validate_cid(cid).is_ok(), "should be valid: {}", cid);
        }
    }

    #[test]
    fn test_invalid_cids() {
        let invalid = [
            "",
            "not-a-cid",
            "QmYtUc4iTCbbfVSDNKvtQqrfyezPPnFvE33wFmutw9PBBk", // CIDv0 base58btc
            "baaaaaaa",   // valid base32lower chars but not a real CID
            "bafyfoobar", // starts with 'b' but invalid CID structure
        ];
        for cid in invalid {
            assert!(validate_cid(cid).is_err(), "should be invalid: {}", cid);
        }
    }
}
