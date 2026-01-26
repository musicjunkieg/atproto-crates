# atproto-oauth

OAuth 2.0 implementation for AT Protocol.

## Overview

Comprehensive OAuth support with DPoP, PKCE, JWT operations, and secure storage abstractions for AT Protocol authentication.

## Features

- **JWT operations**: Token minting, verification, and validation with ES256/ES384/ES256K support
- **JWK management**: JSON Web Key generation and conversion for P-256, P-384, and K-256 curves
- **PKCE implementation**: RFC 7636 compliant Proof Key for Code Exchange for secure authorization flows
- **DPoP support**: RFC 9449 compliant Demonstration of Proof-of-Possession with automatic retry middleware
- **OAuth discovery**: Resource discovery and validation using RFC 8414 well-known endpoints
- **Storage abstractions**: Pluggable storage with LRU cache implementation for OAuth requests
- **Base64 encoding**: URL-safe base64 encoding/decoding utilities for JWT operations

## CLI Tools

The following command-line tool is available when built with the `clap` feature:

- **`atproto-oauth-service-token`**: OAuth service token management tool for AT Protocol authentication workflows

## Usage

### JWT Operations

```rust
use atproto_oauth::jwt::{mint, verify, Header, Claims, JoseClaims};
use atproto_identity::key::identify_key;

let key_data = identify_key("did:key:zQ3sh...")?;

let header = Header {
    algorithm: Some("ES256".to_string()),
    type_: Some("JWT".to_string()),
    ..Default::default()
};

let claims = Claims::new(JoseClaims {
    issuer: Some("did:plc:issuer123".to_string()),
    subject: Some("did:plc:subject456".to_string()),
    audience: Some("https://pds.example.com".to_string()),
    expiration: Some(chrono::Utc::now().timestamp() as u64 + 3600),
    ..Default::default()
});

let token = mint(&key_data, &header, &claims)?;
verify(&key_data, &token).await?;
```

### PKCE Flow

```rust
use atproto_oauth::pkce;

let (code_verifier, code_challenge) = pkce::generate();
// Use code_challenge in authorization URL
// Later use code_verifier for token exchange
```

### DPoP Proofs

```rust
use atproto_oauth::dpop::{auth_dpop, request_dpop};

let (dpop_token, header, claims) = auth_dpop(
    &key_data,
    "POST",
    "https://auth.example.com/oauth/token"
)?;
```

### OAuth Discovery

```rust
use atproto_oauth::resources::{discover_protected_resource, discover_authorization_server};

let protected_resource = discover_protected_resource(&client, pds_url).await?;
let auth_server = discover_authorization_server(&client, auth_server_url).await?;
```

## License

MIT License