//! Utility functions and constants for attestation operations.
//!
//! This module provides common utilities used throughout the attestation framework,
//! including base64 encoding/decoding with flexible padding support.

use base64::{
    alphabet::STANDARD as STANDARD_ALPHABET,
    engine::{
        DecodePaddingMode,
        general_purpose::{GeneralPurpose, GeneralPurposeConfig},
    },
};

/// Base64 engine that accepts both padded and unpadded input for maximum compatibility
/// with various AT Protocol implementations. Uses standard encoding with padding for output,
/// but accepts any padding format for decoding.
pub(crate) const BASE64: GeneralPurpose = GeneralPurpose::new(
    &STANDARD_ALPHABET,
    GeneralPurposeConfig::new()
        .with_encode_padding(true)
        .with_decode_padding_mode(DecodePaddingMode::Indifferent),
);
