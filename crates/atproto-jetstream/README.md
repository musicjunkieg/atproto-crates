# atproto-jetstream

WebSocket consumer for AT Protocol Jetstream events.

## Overview

Real-time event streaming with Zstandard compression, automatic reconnection, and configurable event filtering for AT Protocol repository changes.

## Features

- **WebSocket streaming**: High-performance event consumption with automatic reconnection handling
- **Event filtering**: Configurable filtering by collections and DIDs for targeted event processing
- **Zstandard compression**: Built-in support for compressed event streams with custom dictionaries
- **Event handlers**: Flexible handler system supporting multiple custom event processors
- **Background processing**: Asynchronous event processing with graceful shutdown via cancellation tokens
- **Structured errors**: Comprehensive error handling with detailed error codes

## CLI Tools

The following command-line tool is available when built with the `clap` feature:

- **`atproto-jetstream-consumer`**: Real-time event stream consumer with filtering, compression, and background processing support

## Usage

### Basic Event Consumer

```rust
use atproto_jetstream::{Consumer, ConsumerTaskConfig, EventHandler, JetstreamEvent, CancellationToken};
use async_trait::async_trait;

struct MyEventHandler;

#[async_trait]
impl EventHandler for MyEventHandler {
    async fn handle_event(&self, event: JetstreamEvent) -> anyhow::Result<()> {
        println!("Received event: {:?}", event);
        Ok(())
    }

    fn handler_id(&self) -> String {
        "my-handler".to_string()
    }
}

let config = ConsumerTaskConfig {
    user_agent: "my-app/1.0".to_string(),
    compression: false,
    zstd_dictionary_location: String::new(),
    jetstream_hostname: "jetstream1.us-east.bsky.network".to_string(),
    collections: vec!["app.bsky.feed.post".to_string()],
};

let consumer = Consumer::new(config);
consumer.register_handler(std::sync::Arc::new(MyEventHandler)).await?;

let cancellation_token = CancellationToken::new();
consumer.run_background(cancellation_token).await?;
```

### With Compression

```rust
let config = ConsumerTaskConfig {
    compression: true,
    zstd_dictionary_location: "./data/zstd_dictionary".to_string(),
    // ... other config
};

// Download dictionary first:
// curl -o data/zstd_dictionary https://github.com/bluesky-social/jetstream/raw/refs/heads/main/pkg/models/zstd_dictionary
```

## Command Line Examples

```bash
# Basic streaming
cargo run --bin atproto-jetstream-consumer \
    --hostname jetstream1.us-east.bsky.network \
    --collections app.bsky.feed.post \
    --user-agent "my-consumer/1.0"

# With compression
cargo run --bin atproto-jetstream-consumer \
    --hostname jetstream1.us-east.bsky.network \
    --collections app.bsky.feed.post \
    --compression \
    --zstd-dictionary ./data/zstd_dictionary

# With filtering
cargo run --bin atproto-jetstream-consumer \
    --hostname jetstream1.us-east.bsky.network \
    --collections "app.bsky.feed.post,app.bsky.actor.profile" \
    --dids "did:plc:user123,did:plc:user456"
```

## License

MIT License