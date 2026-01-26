//! Byte array serialization utilities for AT Protocol records.
//!
//! This module provides specialized Serde serialization and deserialization functions
//! for byte arrays (`Vec<u8>`), handling base64 encoding as required by the AT Protocol
//! lexicon specifications for binary data fields.
//!
//! ## Usage
//!
//! The `format` module is designed to be used with Serde's `#[serde(with = "...")]` attribute
//! on fields that contain binary data:
//!
//! ```ignore
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Signature {
//!     #[serde(rename = "$bytes", with = "atproto_record::bytes::format")]
//!     signature: Vec<u8>,
//! }
//! ```
//!
//! ## Base64 Encoding
//!
//! The serialization uses standard base64 encoding (RFC 4648) with padding.
//! The deserialization accepts both padded and unpadded base64 strings for
//! compatibility with various AT Protocol implementations.

/// Base64 serialization format for byte arrays.
///
/// This module provides serde serialization/deserialization for `Vec<u8>` values,
/// encoding them as base64 strings during JSON serialization and decoding them
/// back to byte vectors during deserialization. This is the standard format
/// for binary data in AT Protocol lexicon structures.
pub mod format {
    use serde::{Deserialize, Serialize};
    use serde::{Deserializer, Serializer};

    use base64::{
        Engine,
        engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
    };

    /// Serializes a byte vector to a base64 encoded string.
    pub fn serialize<S: Serializer>(value: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> {
        let encoded_value = STANDARD.encode(value);
        String::serialize(&encoded_value, serializer)
    }

    /// Deserializes a base64 encoded string to a byte vector.
    /// Handles both padded and unpadded base64 strings for compatibility.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let encoded_value = String::deserialize(d)?;

        // Try standard base64 with padding first
        STANDARD
            .decode(&encoded_value)
            .or_else(|_| {
                // If that fails, try without padding requirement
                STANDARD_NO_PAD.decode(&encoded_value)
            })
            .map_err(serde::de::Error::custom)
    }
}
