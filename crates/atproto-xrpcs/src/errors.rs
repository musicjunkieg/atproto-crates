//! # Structured Error Types for XRPC Services
//!
//! Comprehensive error handling for AT Protocol XRPC service operations using structured error types
//! with the `thiserror` library. All errors follow the project convention of prefixed error codes
//! with descriptive messages.
//!
//! ## Error Categories
//!
//! - **`AuthorizationError`** (authorization-1 to authorization-15): JWT validation, DID resolution, and authorization errors
//!
//! ## Error Format
//!
//! All errors use the standardized format: `error-atproto-xrpcs-{domain}-{number} {message}: {details}`

use thiserror::Error;

/// Error types that can occur during XRPC authorization operations.
///
/// These errors represent failures in JWT validation, DID document resolution,
/// and cryptographic verification during authorization processing.
#[derive(Debug, Error)]
pub enum AuthorizationError {
    /// Occurs when JWT does not have the expected 3-part format (header.payload.signature)
    #[error("error-atproto-xrpcs-authorization-1 Invalid JWT format: expected 3 parts")]
    InvalidJWTFormat,

    /// Occurs when JWT claims cannot be base64 decoded
    #[error("error-atproto-xrpcs-authorization-2 Failed to decode JWT claims: {error}")]
    ClaimsDecodeError {
        /// The underlying base64 decode error
        error: base64::DecodeError,
    },

    /// Occurs when JWT claims cannot be parsed as JSON
    #[error("error-atproto-xrpcs-authorization-3 Failed to parse JWT claims: {error}")]
    ClaimsParseError {
        /// The underlying JSON parse error
        error: serde_json::Error,
    },

    /// Occurs when no issuer is found in JWT claims
    #[error("error-atproto-xrpcs-authorization-4 No issuer found in JWT claims")]
    NoIssuerInClaims,

    /// Occurs when no verification keys are found in DID document
    #[error("error-atproto-xrpcs-authorization-5 No verification keys found in DID document")]
    NoVerificationKeys,

    /// Occurs when JWT header cannot be base64 decoded
    #[error("error-atproto-xrpcs-authorization-6 Failed to decode JWT header: {error}")]
    HeaderDecodeError {
        /// The underlying base64 decode error
        error: base64::DecodeError,
    },

    /// Occurs when JWT header cannot be parsed as JSON
    #[error("error-atproto-xrpcs-authorization-7 Failed to parse JWT header: {error}")]
    HeaderParseError {
        /// The underlying JSON parse error
        error: serde_json::Error,
    },

    /// Occurs when JWT validation fails with all available keys
    #[error("error-atproto-xrpcs-authorization-8 JWT validation failed with all available keys")]
    ValidationFailedAllKeys,

    /// Occurs when subject resolution fails during DID document lookup
    #[error("error-atproto-xrpcs-authorization-9 Subject resolution failed: {issuer} {error}")]
    SubjectResolutionFailed {
        /// The issuer that failed to resolve
        issuer: String,
        /// The underlying resolution error
        error: anyhow::Error,
    },
}
