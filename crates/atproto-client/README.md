# atproto-client

AT Protocol HTTP client with authentication support.

## Overview

HTTP client for AT Protocol services supporting DPoP, bearer tokens, and sessions with native XRPC protocol operations and repository management.

## Features

- **Multiple authentication methods**: DPoP, Bearer tokens, and session-based authentication
- **XRPC protocol support**: Native support for AT Protocol's XRPC communication protocol
- **Repository operations**: Complete client for AT Protocol repository CRUD operations
- **Session management**: Authentication session creation, refresh, and validation
- **URL utilities**: Helper functions for AT Protocol URL construction and validation

## CLI Tools

The following command-line tools are available when built with the `clap` feature:

- **`atproto-client-auth`**: Create and refresh authentication sessions using identifier and password
- **`atproto-client-app-password`**: Make authenticated XRPC calls using app password Bearer tokens
- **`atproto-client-dpop`**: Make authenticated XRPC calls using DPoP (Demonstration of Proof-of-Possession) tokens

## Usage

### Basic HTTP Operations

```rust
use atproto_client::client;
use reqwest::Client;

let http_client = Client::new();
let response = client::get_json(&http_client, "https://api.example.com/data").await?;
```

### DPoP Authentication

```rust
use atproto_client::client::{DPoPAuth, get_dpop_json};
use atproto_identity::key::identify_key;

let dpop_auth = DPoPAuth {
    dpop_private_key_data: identify_key("did:key:zQ3sh...")?,
    oauth_access_token: "your_access_token".to_string(),
};

let response = get_dpop_json(&http_client, &dpop_auth, "https://pds.example.com/xrpc/...").await?;
```

### Repository Operations

```rust
use atproto_client::com::atproto::repo::{create_record, CreateRecordRequest};

let create_request = CreateRecordRequest {
    repo: "did:plc:user123".to_string(),
    collection: "app.bsky.feed.post".to_string(),
    record: json!({"$type": "app.bsky.feed.post", "text": "Hello!"}),
    // ...
};

let response = create_record(&http_client, &dpop_auth, pds_url, create_request).await?;
```

## Command Line Examples

### Create and Manage Sessions

```bash
# Create an authentication session with username and password
cargo run --features clap --bin atproto-client-auth -- login alice.example.com my_password

# Use an existing app password for session creation
cargo run --features clap --bin atproto-client-auth -- session alice.example.com app_password_token
```

### App Password Authentication

```bash
# Make an XRPC call using app password Bearer token
cargo run --features clap --bin atproto-client-app-password -- \
  did:plc:user123 \
  your_access_token \
  com.atproto.repo.listRecords
```

### DPoP Authentication

```bash
# Make an authenticated XRPC call using DPoP tokens
cargo run --features clap --bin atproto-client-dpop -- \
  did:plc:user123 \
  did:key:zQ3sh... \
  your_access_token \
  com.atproto.repo.getRecord
```

## License

MIT License