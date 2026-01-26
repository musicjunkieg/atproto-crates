# atproto-oauth-axum

Axum web framework integration for AT Protocol OAuth.

## Overview

Production-ready OAuth handlers for authorization flows, callbacks, JWKS endpoints, and metadata with secure state management.

## Features

- **OAuth endpoint handlers**: Complete Axum handlers for authorization flows, callbacks, and metadata endpoints
- **JWKS endpoint**: JSON Web Key Set endpoint for public key distribution to authorization servers
- **Client metadata**: RFC 7591 compliant OAuth client metadata endpoint for dynamic registration
- **Callback processing**: OAuth authorization callback handler with state validation and token exchange
- **State management**: Secure OAuth state and request management with Axum extractors
- **Error handling**: Comprehensive error handling with proper HTTP status codes

## CLI Tools

The following command-line tool is available when built with the `clap` feature:

- **`atproto-oauth-tool`**: Complete OAuth login workflow tool for AT Protocol services with local callback server

## Usage

### Basic Server Setup

```rust
use atproto_oauth_axum::{
    handle_complete::handle_oauth_callback,
    handle_jwks::handle_oauth_jwks,
    handler_metadata::handle_oauth_metadata,
    state::OAuthClientConfig,
};
use axum::{routing::get, Router};

let oauth_config = OAuthClientConfig {
    client_uri: "https://your-app.com".to_string(),
    client_id: "https://your-app.com/oauth/client-metadata.json".to_string(),
    redirect_uris: "https://your-app.com/oauth/callback".to_string(),
    jwks_uri: "https://your-app.com/.well-known/jwks.json".to_string(),
    signing_keys: vec![identify_key("did:key:zQ3sh...")?],
};

let app = Router::new()
    .route("/oauth/client-metadata.json", get(handle_oauth_metadata))
    .route("/.well-known/jwks.json", get(handle_oauth_jwks))
    .route("/oauth/callback", get(handle_oauth_callback))
    .with_state(oauth_config);
```

### OAuth Handlers

The library provides ready-to-use handlers for:

- **Client Metadata**: Generates RFC 7591 compliant metadata
- **JWKS Endpoint**: Serves JSON Web Key Sets for signature verification
- **Callback Processing**: Handles OAuth authorization callbacks with token exchange

## Command Line Examples

```bash
# Start OAuth login flow for a handle
cargo run --bin atproto-oauth-tool login did:key:zQ3sh... alice.bsky.social

# Start OAuth login flow for a DID
cargo run --bin atproto-oauth-tool login did:key:zQ3sh... did:plc:user123
```

The tool provides a complete OAuth client implementation with:
- Subject resolution and DID document retrieval
- PDS and authorization server discovery
- PKCE and DPoP parameter generation
- Local web server for callback handling
- Complete token exchange flow

## License

MIT License