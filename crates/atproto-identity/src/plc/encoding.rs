//! Base32, base64url, and SHA-256 encoding utilities for did:plc operations.
//!
//! Provides encoding and decoding functions used by PLC operations for
//! identifier derivation, signature encoding, and data hashing.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use data_encoding::BASE32_NOPAD;
use sha2::{Digest, Sha256};

use crate::errors::PLCDIDError;

/// Base32 alphabet used for did:plc identifiers (lowercase, excludes 0,1,8,9).
const BASE32_ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz234567";

/// Maximum size for an operation in bytes.
pub const MAX_OPERATION_SIZE: usize = 7500;

/// Encode bytes to base32 using the lowercase alphabet.
pub fn base32_encode(data: &[u8]) -> String {
    BASE32_NOPAD.encode(data).to_lowercase()
}

/// Decode base32 string to bytes.
///
/// Returns `PLCDIDError::InvalidBase32` if the input contains invalid characters.
pub fn base32_decode(s: &str) -> Result<Vec<u8>, PLCDIDError> {
    if !s.chars().all(|c| BASE32_ALPHABET.contains(c)) {
        return Err(PLCDIDError::InvalidBase32 {
            details: format!(
                "String contains invalid characters. Allowed: {}",
                BASE32_ALPHABET
            ),
        });
    }

    BASE32_NOPAD
        .decode(s.to_uppercase().as_bytes())
        .map_err(|e| PLCDIDError::InvalidBase32 {
            details: e.to_string(),
        })
}

/// Encode bytes to base64url without padding.
pub fn base64url_encode(data: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(data)
}

/// Decode base64url string to bytes.
///
/// Returns `PLCDIDError::InvalidBase64Url` if the input is not valid base64url.
pub fn base64url_decode(s: &str) -> Result<Vec<u8>, PLCDIDError> {
    URL_SAFE_NO_PAD
        .decode(s.as_bytes())
        .map_err(|e| PLCDIDError::InvalidBase64Url {
            details: e.to_string(),
        })
}

/// Hash data with SHA-256 and return the digest.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Validate that a string contains only valid base32 characters.
pub fn is_valid_base32(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| BASE32_ALPHABET.contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base32_roundtrip() {
        let data = b"hello world";
        let encoded = base32_encode(data);
        let decoded = base32_decode(&encoded).unwrap();
        assert_eq!(data, decoded.as_slice());
    }

    #[test]
    fn test_base32_invalid_chars() {
        assert!(base32_decode("0189").is_err());
        assert!(base32_decode("ABCD").is_err());
    }

    #[test]
    fn test_base64url_roundtrip() {
        let data = b"hello world";
        let encoded = base64url_encode(data);
        let decoded = base64url_decode(&encoded).unwrap();
        assert_eq!(data, decoded.as_slice());
        assert!(!encoded.contains('='));
    }

    #[test]
    fn test_is_valid_base32() {
        assert!(is_valid_base32("abcdefghijklmnopqrstuvwxyz234567"));
        assert!(!is_valid_base32("0189"));
        assert!(!is_valid_base32("ABCD"));
        assert!(!is_valid_base32(""));
    }

    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = sha256(data);
        assert_eq!(hash.len(), 32);

        // Verify deterministic
        let hash2 = sha256(data);
        assert_eq!(hash, hash2);
    }
}
