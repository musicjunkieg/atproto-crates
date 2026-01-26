//! Raw DAG-CBOR message type for delayed decoding.
//!
//! `RawMessage` stores pre-encoded DAG-CBOR bytes that can be included
//! in larger structures without deserializing. This enables efficient
//! partial processing of CBOR data.

use crate::errors::{DecodeError, EncodeError};
use serde::{Deserialize, Serialize, de, ser};
use std::fmt;

/// A raw DAG-CBOR message that delays deserialization.
///
/// `RawMessage` holds pre-encoded DAG-CBOR bytes. When serialized, the raw
/// bytes are written directly. When deserialized, bytes are captured without
/// interpretation. Use [`decode`](RawMessage::decode) to deserialize the
/// contained value on demand.
///
/// # Example
///
/// ```rust
/// use atproto_dasl::drisl::raw::RawMessage;
/// use atproto_dasl::{to_vec, from_slice};
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Envelope {
///     version: u32,
///     payload: RawMessage,
/// }
///
/// #[derive(Serialize, Deserialize, Debug, PartialEq)]
/// struct Payload {
///     data: String,
/// }
///
/// let payload = Payload { data: "hello".into() };
/// let envelope = Envelope {
///     version: 1,
///     payload: RawMessage::encode(&payload).unwrap(),
/// };
///
/// let bytes = to_vec(&envelope).unwrap();
/// let decoded: Envelope = from_slice(&bytes).unwrap();
/// let payload: Payload = decoded.payload.decode().unwrap();
/// assert_eq!(payload.data, "hello");
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RawMessage(Vec<u8>);

impl RawMessage {
    /// Create a RawMessage from pre-encoded DAG-CBOR bytes.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Encode a value to DAG-CBOR and wrap as RawMessage.
    ///
    /// # Errors
    ///
    /// Returns `EncodeError` if serialization fails.
    pub fn encode<T: serde::Serialize>(value: &T) -> Result<Self, EncodeError> {
        let bytes = crate::drisl::to_vec(value)?;
        Ok(Self(bytes))
    }

    /// Decode the raw bytes to a typed value.
    ///
    /// # Errors
    ///
    /// Returns `DecodeError` if deserialization fails.
    pub fn decode<T: serde::de::DeserializeOwned>(&self) -> Result<T, DecodeError> {
        crate::drisl::from_slice(&self.0)
    }

    /// Get the raw DAG-CBOR bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consume and return the raw DAG-CBOR bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Returns true if the raw message is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the byte length of the raw message.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl fmt::Debug for RawMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RawMessage({} bytes)", self.0.len())
    }
}

impl Serialize for RawMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        // Write raw bytes directly — the serializer's write_raw will pass them through
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for RawMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        Ok(Self(bytes))
    }
}

impl From<Vec<u8>> for RawMessage {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl AsRef<[u8]> for RawMessage {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Inner {
        value: i32,
        name: String,
    }

    #[test]
    fn test_raw_message_encode_decode() {
        let inner = Inner {
            value: 42,
            name: "test".into(),
        };
        let raw = RawMessage::encode(&inner).unwrap();
        assert!(!raw.is_empty());

        let decoded: Inner = raw.decode().unwrap();
        assert_eq!(decoded, inner);
    }

    #[test]
    fn test_raw_message_from_bytes() {
        let data = crate::drisl::to_vec(&42i32).unwrap();
        let raw = RawMessage::from_bytes(data.clone());
        assert_eq!(raw.as_bytes(), &data);
        assert_eq!(raw.len(), data.len());
    }

    #[test]
    fn test_raw_message_into_bytes() {
        let data = crate::drisl::to_vec(&"hello").unwrap();
        let raw = RawMessage::from_bytes(data.clone());
        assert_eq!(raw.into_bytes(), data);
    }
}
