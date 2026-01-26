//! JSON Web Key Set (JWKS) endpoint handler.
//!
//! Serve OAuth client public keys for JWT signature verification
//! by authorization servers.

use atproto_oauth::jwk::{WrappedJsonWebKey, generate};
use axum::{Json, response::IntoResponse};
use serde::Serialize;

use crate::state::OAuthClientConfig;

/// JSON Web Key Set response structure.
///
/// Contains a collection of public keys for JWT signature verification.
#[derive(Serialize)]
pub struct WrappedJsonWebKeySet {
    /// Array of JSON Web Keys
    pub keys: Vec<WrappedJsonWebKey>,
}

/// Handles requests for the OAuth JWKS (JSON Web Key Set) endpoint.
///
/// Returns the public keys used by this OAuth client for JWT signature verification.
pub async fn handle_oauth_jwks(oauth_client_config: OAuthClientConfig) -> impl IntoResponse {
    let mut jwks = Vec::new();
    for key_data in &oauth_client_config.signing_keys {
        if let Ok(jwk) = generate(key_data) {
            jwks.push(jwk);
        }
    }
    Json(WrappedJsonWebKeySet { keys: jwks })
}
