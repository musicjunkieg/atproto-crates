//! Base64 encoding utilities for OAuth operations.
//!
//! URL-safe base64 encoding/decoding for JWT headers and claims
//! following RFC 7515 specifications.

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Trait for converting types to base64-encoded JSON.
pub trait ToBase64 {
    /// Convert the type to a base64-encoded JSON string.
    fn to_base64(&self) -> Result<Cow<'_, str>>;
}

impl<T: Serialize> ToBase64 for T {
    fn to_base64(&self) -> Result<Cow<'_, str>> {
        let json_bytes = serde_json::to_vec(&self)?;
        let encoded_json_bytes = general_purpose::URL_SAFE_NO_PAD.encode(json_bytes);
        Ok(Cow::Owned(encoded_json_bytes))
    }
}

/// Trait for converting from base64-encoded JSON to types.
pub trait FromBase64: Sized {
    /// Convert from a base64-encoded JSON string to the type.
    fn from_base64<Input: ?Sized + AsRef<[u8]>>(raw: &Input) -> Result<Self>;
}

impl<T: for<'de> Deserialize<'de> + Sized> FromBase64 for T {
    fn from_base64<Input: ?Sized + AsRef<[u8]>>(raw: &Input) -> Result<Self> {
        let content = general_purpose::URL_SAFE_NO_PAD.decode(raw)?;
        serde_json::from_slice(&content).context("unable to deserialize json")
    }
}
