//! Axum state management for OAuth configuration.
//!
//! Request extractors and HTTP client wrappers for OAuth
//! client configuration injection in handlers.

use atproto_identity::key::KeyData;
use axum::extract::{FromRef, FromRequestParts};
use http::request::Parts;
use std::convert::Infallible;

#[cfg(feature = "zeroize")]
use zeroize::{Zeroize, ZeroizeOnDrop};

/// OAuth client configuration for Axum handlers.
///
/// Contains the essential configuration needed for OAuth client operations.
#[derive(Clone, Default)]
#[cfg_attr(feature = "zeroize", derive(Zeroize, ZeroizeOnDrop))]
pub struct OAuthClientConfig {
    /// OAuth client identifier
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub client_id: String,

    /// Allowed OAuth redirect URIs
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub redirect_uris: String,

    /// JSON Web Key Set URI for public keys
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub jwks_uri: Option<String>,

    /// Signing keys for JWT operations
    pub signing_keys: Vec<KeyData>,

    /// OAuth scope, defaults to "atproto transition:generic"
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub scope: Option<String>,

    /// Optional human-readable client name
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub client_name: Option<String>,

    /// Optional client website URI
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub client_uri: Option<String>,

    /// Optional client logo URI
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub logo_uri: Option<String>,

    /// Optional terms of service URI
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub tos_uri: Option<String>,

    /// Optional privacy policy URI
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub policy_uri: Option<String>,
}

impl OAuthClientConfig {
    /// Returns the OAuth scope, using the default "atproto transition:generic" if not set.
    pub fn scope(&self) -> &str {
        self.scope
            .as_deref()
            .unwrap_or("atproto transition:generic")
    }
}

impl<S> FromRequestParts<S> for OAuthClientConfig
where
    OAuthClientConfig: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    /// Extracts OAuth client configuration from Axum application state.
    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let oauth_client_config = OAuthClientConfig::from_ref(state);
        Ok(oauth_client_config)
    }
}

/// HTTP client wrapper for dependency injection.
///
/// Wraps a reqwest::Client for use in Axum extractors.
#[derive(Clone)]
pub struct HttpClient(pub reqwest::Client);

impl std::ops::Deref for HttpClient {
    type Target = reqwest::Client;

    /// Provides direct access to the underlying reqwest::Client.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for HttpClient
where
    reqwest::Client: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Infallible;

    /// Extracts HTTP client from Axum application state.
    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let client = reqwest::Client::from_ref(state);
        Ok(HttpClient(client))
    }
}
