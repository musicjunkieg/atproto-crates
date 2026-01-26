# atproto-oauth-aip

OAuth AIP (Identity Provider) implementation for AT Protocol.

## Overview

Complete OAuth 2.0 authorization code flow with PAR, PKCE, token exchange, and AT Protocol session management for identity providers.

## Features

- **OAuth 2.0 authorization code flow**: Complete implementation with PKCE support for secure authentication
- **Pushed Authorization Requests (PAR)**: Enhanced security through server-side request storage
- **Token exchange**: Secure OAuth token issuance and refresh capabilities
- **AT Protocol session management**: Convert OAuth tokens to AT Protocol sessions with DID resolution
- **Resource validation**: OAuth protected resource and authorization server configuration validation
- **Structured error handling**: Comprehensive error types following AT Protocol conventions

## CLI Tools

This crate does not provide standalone CLI tools. It serves as a library for OAuth AIP implementations.


## Usage

### Basic OAuth Flow

```rust
use atproto_oauth_aip::{OAuthClient, oauth_init, oauth_complete, session_exchange};
use atproto_oauth::storage::MemoryStorage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize OAuth client
    let client = OAuthClient::new(
        "https://your-app.com/client-id".to_string(),
        Some("your-client-secret".to_string()),
        "https://your-app.com/callback".to_string(),
    );
    
    // Initialize storage (use persistent storage in production)
    let storage = MemoryStorage::new();
    
    // Start OAuth flow
    let (authorization_url, state) = oauth_init(
        &client,
        "user@example.com",  // User identifier
        &storage,
    ).await?;
    
    // Redirect user to authorization_url
    println!("Please visit: {}", authorization_url);
    
    // After user authorizes and is redirected back with code and state...
    let authorization_code = "received-auth-code";
    let returned_state = "returned-state";
    
    // Complete OAuth flow
    let access_token = oauth_complete(
        &client,
        authorization_code,
        returned_state,
        &storage,
    ).await?;
    
    // Exchange for AT Protocol session
    let session = session_exchange(
        &client,
        &access_token,
        &storage,
    ).await?;
    
    println!("Authenticated as: {} ({})", session.handle, session.did);
    println!("PDS Endpoint: {}", session.pds_endpoint);
    
    Ok(())
}
```

### Fetching OAuth Metadata

```rust
use atproto_oauth_aip::resources::{oauth_protected_resource, oauth_authorization_server};

// Get OAuth protected resource configuration
let protected_resource = oauth_protected_resource("https://bsky.social").await?;

// Get OAuth authorization server metadata
let auth_server = oauth_authorization_server(&protected_resource).await?;
```

## API Documentation

### Core Types

- `OAuthClient`: OAuth client credentials and configuration
- `ATProtocolSession`: Authenticated session containing DID, handle, and PDS endpoint

### Main Functions

- `oauth_init()`: Initiates OAuth flow using PAR
- `oauth_complete()`: Exchanges authorization code for access token
- `session_exchange()`: Converts OAuth access token to AT Protocol session

### Resource Functions

- `oauth_protected_resource()`: Fetch OAuth protected resource metadata
- `oauth_authorization_server()`: Fetch OAuth authorization server metadata

## Error Handling

The crate uses typed errors following the AT Protocol error format:

```rust
use atproto_oauth_aip::OAuthWorkflowError;

match result {
    Err(OAuthWorkflowError::InvalidAuthorizationRequest(e)) => {
        // Handle PAR errors
    }
    Err(OAuthWorkflowError::TokenExchangeFailed(e)) => {
        // Handle token exchange errors
    }
    // ... other error types
}
```

## Storage Requirements

This crate requires an implementation of the `OAuthStorage` trait from `atproto-oauth`. For production use, implement persistent storage rather than using `MemoryStorage`.

## Security Considerations

- Always use HTTPS URLs for OAuth endpoints
- Implement proper state validation to prevent CSRF attacks
- Store client secrets securely
- Use persistent storage with appropriate security measures
- Validate DPoP keys when required by the authorization server

## Dependencies

This crate depends on:
- `atproto-oauth`: Core OAuth implementation
- `atproto-identity`: AT Protocol identity resolution
- `atproto-record`: Record handling
- `reqwest`: HTTP client
- `serde`: Serialization
- `tokio`: Async runtime

## License

MIT License