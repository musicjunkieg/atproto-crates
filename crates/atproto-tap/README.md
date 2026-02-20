# atproto-tap

TAP (Trusted Attestation Protocol) service consumer for AT Protocol.

## Overview

WebSocket streaming client for TAP services, which deliver filtered and verified AT Protocol repository events. Supports automatic reconnection with exponential backoff, at-least-once delivery via acknowledgments, and an HTTP management API for controlling which repositories are tracked.

## Features

- **Verified event stream**: Connects to a TAP service that provides MST-verified, signature-checked events
- **Automatic reconnection**: Exponential backoff with configurable limits
- **At-least-once delivery**: Acknowledgment protocol ensures no events are lost across reconnects
- **Backfill support**: Historical events delivered with `live: false` for catch-up
- **Management API**: HTTP client for adding/removing tracked DIDs, health checks, and DID resolution
- **Memory efficient**: `Arc`-wrapped events, `CompactString` for small strings, pre-allocated ack buffers

## CLI Tools

The following command-line tools are available when built with the `clap` feature:

- **`atproto-tap-client`**: Stream TAP events as JSON and manage tracked repositories
- **`atproto-tap-extras`**: Bulk-add followed accounts to TAP tracking from a DID's social graph

## Usage

### Quick Start

```rust
use atproto_tap::{connect_to, TapEvent};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let mut stream = connect_to("localhost:2480");

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => match event.as_ref() {
                TapEvent::Record { record, .. } => {
                    println!("{} {} {}", record.action, record.collection, record.did);
                }
                TapEvent::Identity { identity, .. } => {
                    println!("Identity: {} = {}", identity.did, identity.handle);
                }
            },
            Err(e) => eprintln!("Error: {}", e),
        }
    }
}
```

### Configuration

```rust
use atproto_tap::{connect, TapConfig};
use std::time::Duration;

let config = TapConfig::builder()
    .hostname("localhost:2480")
    .admin_password("secret")
    .send_acks(true)
    .max_reconnect_attempts(Some(10))
    .initial_reconnect_delay(Duration::from_secs(1))
    .max_reconnect_delay(Duration::from_secs(60))
    .channel_buffer_size(64)
    .build();

let mut stream = connect(config);
```

### Using with `tokio::select!`

```rust
use atproto_tap::{connect, TapConfig};
use tokio_stream::StreamExt;
use tokio::signal;

#[tokio::main]
async fn main() {
    let config = TapConfig::builder()
        .hostname("localhost:2480")
        .admin_password("secret")
        .build();

    let mut stream = connect(config);

    loop {
        tokio::select! {
            Some(result) = stream.next() => {
                // Process event
            }
            _ = signal::ctrl_c() => {
                break;
            }
        }
    }
}
```

### Management API

```rust
use atproto_tap::TapClient;

let client = TapClient::new("localhost:2480", Some("password".to_string()));

// Add repositories to track
client.add_repos(&["did:plc:xyz123", "did:plc:abc456"]).await?;

// Remove repositories
client.remove_repos(&["did:plc:xyz123"]).await?;

// Check service health
if client.health().await? {
    println!("TAP service is healthy");
}

// Resolve a DID to its document
let doc = client.resolve("did:plc:xyz123").await?;
```

## Command Line Examples

```bash
# Stream all events from a TAP service
cargo run --features clap --bin atproto-tap-client -- localhost:2480 read

# Stream with authentication and collection filtering
cargo run --features clap --bin atproto-tap-client -- localhost:2480 -p secret read \
    --collections app.bsky.feed.post,app.bsky.actor.profile

# Stream only live events (skip backfill)
cargo run --features clap --bin atproto-tap-client -- localhost:2480 read --live-only

# Add DIDs to tracking
cargo run --features clap --bin atproto-tap-client -- localhost:2480 -p secret \
    repos add did:plc:xyz123 did:plc:abc456

# Remove DIDs from tracking
cargo run --features clap --bin atproto-tap-client -- localhost:2480 -p secret \
    repos remove did:plc:xyz123

# Resolve a DID to its DID document
cargo run --features clap --bin atproto-tap-client -- localhost:2480 resolve did:plc:xyz123

# Get repository tracking info
cargo run --features clap --bin atproto-tap-client -- localhost:2480 info did:plc:xyz123

# Bulk-add all accounts followed by a DID
cargo run --features clap --bin atproto-tap-extras -- localhost:2480 -p secret \
    repos-add-followers did:plc:xyz123

# Dry run to preview which DIDs would be added
cargo run --features clap --bin atproto-tap-extras -- localhost:2480 -p secret \
    repos-add-followers did:plc:xyz123 --dry-run --limit 500
```

## License

MIT License
