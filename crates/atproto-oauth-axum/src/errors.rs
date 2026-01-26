//! # Structured Error Types for OAuth Axum Handlers
//!
//! Comprehensive error handling for AT Protocol OAuth Axum web handlers using structured error types
//! with the `thiserror` library. All errors follow the project convention of prefixed error codes
//! with descriptive messages.
//!
//! ## Error Categories
//!
//! - **`OAuthCallbackError`** (callback-1 to callback-7): OAuth callback handler errors
//! - **`OAuthLoginError`** (login-1 to login-11): OAuth login CLI tool errors
//!
//! ## Error Format
//!
//! All errors use the standardized format: `error-atproto-oauth-axum-{domain}-{number} {message}: {details}`

use thiserror::Error;

/// Error types that can occur during OAuth callback handling.
///
/// These errors represent failures in the OAuth authorization callback flow
/// including request validation and token exchange operations.
#[derive(Debug, Error)]
pub enum OAuthCallbackError {
    /// Occurs when no OAuth request is found for the provided state parameter
    #[error("error-atproto-oauth-axum-callback-1 No OAuth request found for state")]
    NoOAuthRequestFound,

    /// Occurs when the issuer in the callback doesn't match the stored OAuth request
    #[error(
        "error-atproto-oauth-axum-callback-2 Invalid issuer: expected {expected}, got {actual}"
    )]
    InvalidIssuer {
        /// The expected issuer from the stored OAuth request
        expected: String,
        /// The actual issuer from the callback
        actual: String,
    },

    /// Occurs when no DID document is found for the OAuth request
    #[error("error-atproto-oauth-axum-callback-3 No DID document found for OAuth request")]
    NoDIDDocumentFound,

    /// Occurs when no signing key is found for the OAuth request
    #[error("error-atproto-oauth-axum-callback-4 No signing key found for OAuth request")]
    NoSigningKeyFound,

    /// Occurs when an underlying operation fails with an anyhow error
    #[error("error-atproto-oauth-axum-callback-5 Operation failed: {error}")]
    OperationFailed {
        /// The underlying anyhow error
        error: anyhow::Error,
    },

    /// Occurs when key operations fail
    #[error("error-atproto-oauth-axum-callback-6 Key operation failed: {error}")]
    KeyOperationFailed {
        /// The underlying key error
        error: atproto_identity::errors::KeyError,
    },

    /// Occurs when OAuth client operations fail
    #[error("error-atproto-oauth-axum-callback-7 OAuth client operation failed: {error}")]
    OAuthClientOperationFailed {
        /// The underlying OAuth client error
        error: atproto_oauth::errors::OAuthClientError,
    },
}

impl From<anyhow::Error> for OAuthCallbackError {
    fn from(error: anyhow::Error) -> Self {
        OAuthCallbackError::OperationFailed { error }
    }
}

impl From<atproto_identity::errors::KeyError> for OAuthCallbackError {
    fn from(error: atproto_identity::errors::KeyError) -> Self {
        OAuthCallbackError::KeyOperationFailed { error }
    }
}

impl From<atproto_oauth::errors::OAuthClientError> for OAuthCallbackError {
    fn from(error: atproto_oauth::errors::OAuthClientError) -> Self {
        OAuthCallbackError::OAuthClientOperationFailed { error }
    }
}

/// Error types that can occur during OAuth login CLI operations.
///
/// These errors represent failures in the OAuth login command-line tool
/// including subject resolution, DID operations, and OAuth flow initiation.
#[derive(Debug, Error)]
pub enum OAuthLoginError {
    /// Occurs when subject resolution fails
    #[error("error-atproto-oauth-axum-login-1 Failed to resolve subject: {error}")]
    SubjectResolutionFailed {
        /// The underlying resolution error
        error: anyhow::Error,
    },

    /// Occurs when PLC directory query fails
    #[error("error-atproto-oauth-axum-login-2 Failed to query PLC directory: {error}")]
    PLCQueryFailed {
        /// The underlying PLC error
        error: anyhow::Error,
    },

    /// Occurs when web DID query fails
    #[error("error-atproto-oauth-axum-login-3 Failed to query web DID: {error}")]
    WebDIDQueryFailed {
        /// The underlying web DID error
        error: anyhow::Error,
    },

    /// Occurs when an unsupported DID method is encountered
    #[error("error-atproto-oauth-axum-login-4 Unsupported DID method: {did}")]
    UnsupportedDIDMethod {
        /// The unsupported DID identifier
        did: String,
    },

    /// Occurs when no PDS endpoint is found in the DID document
    #[error("error-atproto-oauth-axum-login-5 No PDS endpoint found in DID document")]
    NoPDSEndpointFound,

    /// Occurs when PDS resources retrieval fails
    #[error("error-atproto-oauth-axum-login-6 Failed to get PDS resources: {error}")]
    PDSResourcesFailed {
        /// The underlying PDS resources error
        error: anyhow::Error,
    },

    /// Occurs when DPoP key generation fails
    #[error("error-atproto-oauth-axum-login-7 Failed to generate DPoP key: {error}")]
    DPoPKeyGenerationFailed {
        /// The underlying key generation error
        error: anyhow::Error,
    },

    /// Occurs when private signing key parsing fails
    #[error("error-atproto-oauth-axum-login-8 Invalid private signing key: {error}")]
    InvalidPrivateSigningKey {
        /// The underlying key parsing error
        error: anyhow::Error,
    },

    /// Occurs when OAuth initialization fails
    #[error("error-atproto-oauth-axum-login-9 OAuth init failed: {error}")]
    OAuthInitFailed {
        /// The underlying OAuth initialization error
        error: anyhow::Error,
    },

    /// Occurs when public key derivation fails
    #[error("error-atproto-oauth-axum-login-10 Failed to derive public key: {error}")]
    PublicKeyDerivationFailed {
        /// The underlying key derivation error
        error: anyhow::Error,
    },

    /// Occurs when OAuth request storage fails
    #[error("error-atproto-oauth-axum-login-11 Failed to store OAuth request: {error}")]
    OAuthRequestStorageFailed {
        /// The underlying storage error
        error: anyhow::Error,
    },
}
