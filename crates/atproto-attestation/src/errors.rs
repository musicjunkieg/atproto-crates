//! Errors that can occur during attestation preparation and verification.
//!
//! Covers CID construction, `$sig` metadata validation, inline attestation
//! structure checks, and identity/key resolution failures.

use thiserror::Error;

/// Errors that can occur during attestation preparation and verification.
#[derive(Debug, Error)]
pub enum AttestationError {
    /// Error when the record value is not a JSON object.
    #[error("error-atproto-attestation-1 Record must be a JSON object")]
    RecordMustBeObject,

    /// Error when the record omits the `$type` discriminator.
    #[error("error-atproto-attestation-1 Record must include a string `$type` field")]
    RecordMissingType,

    /// Error when attestation metadata is not a JSON object.
    #[error("error-atproto-attestation-2 Attestation metadata must be a JSON object")]
    MetadataMustBeObject,

    /// Error when attestation metadata is missing a required field.
    #[error("error-atproto-attestation-3 Attestation metadata missing required field: {field}")]
    MetadataMissingField {
        /// Name of the missing field.
        field: String,
    },

    /// Error when attestation metadata omits the `$type` discriminator.
    #[error("error-atproto-attestation-4 Attestation metadata must include a string `$type` field")]
    MetadataMissingSigType,

    /// Error when the record does not contain a signatures array.
    #[error("error-atproto-attestation-5 Signatures array not found on record")]
    SignaturesArrayMissing,

    /// Error when the signatures field exists but is not an array.
    #[error("error-atproto-attestation-6 Signatures field must be an array")]
    SignaturesFieldInvalid,

    /// Error when attempting to verify a signature at an invalid index.
    #[error("error-atproto-attestation-7 Signature index {index} out of bounds")]
    SignatureIndexOutOfBounds {
        /// Index that was requested.
        index: usize,
    },

    /// Error when a signature object is missing a required field.
    #[error("error-atproto-attestation-8 Signature object missing required field: {field}")]
    SignatureMissingField {
        /// Field name that was expected.
        field: String,
    },

    /// Error when a signature object uses an invalid `$type` for inline attestations.
    #[error(
        "error-atproto-attestation-9 Inline attestation `$type` cannot be `com.atproto.repo.strongRef`"
    )]
    InlineAttestationTypeInvalid,

    /// Error when a remote attestation entry does not use the strongRef type.
    #[error(
        "error-atproto-attestation-10 Remote attestation entries must use `com.atproto.repo.strongRef`"
    )]
    RemoteAttestationTypeInvalid,

    /// Error when a remote attestation entry is missing a CID.
    #[error(
        "error-atproto-attestation-11 Remote attestation entries must include a string `cid` field"
    )]
    RemoteAttestationMissingCid,

    /// Error when signature bytes are not provided using the `$bytes` wrapper.
    #[error(
        "error-atproto-attestation-12 Signature bytes must be encoded as `{{\"$bytes\": \"...\"}}`"
    )]
    SignatureBytesFormatInvalid,

    /// Error when record serialization to DAG-CBOR fails.
    #[error("error-atproto-attestation-13 Record serialization failed: {error}")]
    RecordSerializationFailed {
        /// Underlying serialization error.
        #[from]
        error: atproto_dasl::EncodeError,
    },

    /// Error when `$sig` metadata is missing from the record before CID creation.
    #[error("error-atproto-attestation-14 `$sig` metadata must be present before generating a CID")]
    SigMetadataMissing,

    /// Error when `$sig` metadata is not an object.
    #[error("error-atproto-attestation-15 `$sig` metadata must be a JSON object")]
    SigMetadataNotObject,

    /// Error when `$sig` metadata omits the `$type` discriminator.
    #[error("error-atproto-attestation-16 `$sig` metadata must include a string `$type` field")]
    SigMetadataMissingType,

    /// Error when metadata omits the `$type` discriminator.
    #[error("error-atproto-attestation-18 Metadata must include a string `$type` field")]
    MetadataMissingType,

    /// Error when a key resolver is required but not provided.
    #[error("error-atproto-attestation-17 Key resolver required to resolve key reference: {key}")]
    KeyResolverRequired {
        /// Key reference that required resolution.
        key: String,
    },

    /// Error when key resolution using the provided resolver fails.
    #[error("error-atproto-attestation-18 Failed to resolve key reference {key}: {error}")]
    KeyResolutionFailed {
        /// Key reference that was being resolved.
        key: String,
        /// Underlying resolution error.
        #[source]
        error: anyhow::Error,
    },

    /// Error when the key type is unsupported for inline attestations.
    #[error("error-atproto-attestation-21 Unsupported key type for attestation: {key_type}")]
    UnsupportedKeyType {
        /// Unsupported key type.
        key_type: atproto_identity::key::KeyType,
    },

    /// Error when signature decoding fails.
    #[error("error-atproto-attestation-22 Signature decoding failed: {error}")]
    SignatureDecodingFailed {
        /// Underlying base64 decoding error.
        #[from]
        error: base64::DecodeError,
    },

    /// Error when signature length does not match the expected size.
    #[error(
        "error-atproto-attestation-23 Signature length invalid: expected {expected} bytes, found {actual}"
    )]
    SignatureLengthInvalid {
        /// Expected signature length.
        expected: usize,
        /// Actual signature length.
        actual: usize,
    },

    /// Error when signature is not normalized to low-S form.
    #[error("error-atproto-attestation-24 Signature must be normalized to low-S form")]
    SignatureNotNormalized,

    /// Error when cryptographic verification fails.
    #[error("error-atproto-attestation-25 Signature verification failed: {error}")]
    SignatureValidationFailed {
        /// Underlying key validation error.
        #[source]
        error: atproto_identity::errors::KeyError,
    },

    /// Error when signature creation fails during inline attestation.
    #[error("error-atproto-attestation-27 Signature creation failed: {error}")]
    SignatureCreationFailed {
        /// Underlying signing error.
        #[source]
        error: atproto_identity::errors::KeyError,
    },

    /// Error when fetching a remote attestation proof record fails.
    #[error("error-atproto-attestation-28 Failed to fetch remote attestation from {uri}: {error}")]
    RemoteAttestationFetchFailed {
        /// AT-URI that failed to resolve.
        uri: String,
        /// Underlying fetch error.
        #[source]
        error: anyhow::Error,
    },

    /// Error when the CID of a remote attestation proof record doesn't match expected.
    #[error(
        "error-atproto-attestation-29 Remote attestation CID mismatch: expected {expected}, got {actual}"
    )]
    RemoteAttestationCidMismatch {
        /// Expected CID.
        expected: String,
        /// Actual CID.
        actual: String,
    },

    /// Error when parsing a CID string fails.
    #[error("error-atproto-attestation-30 Invalid CID format: {cid}")]
    InvalidCid {
        /// Invalid CID string.
        cid: String,
    },
}
