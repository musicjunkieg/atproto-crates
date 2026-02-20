//! AT Protocol Jetstream consumer tool for streaming events.
//!
//! This binary tool connects to a Jetstream instance and streams AT Protocol
//! events from specified collections, with optional zstd compression support.

use anyhow::Result;
use atproto_identity::config::{CertificateBundles, default_env, optional_env, version};
use atproto_jetstream::{CancellationToken, Consumer, ConsumerTaskConfig, LoggingHandler};
use clap::Parser;
use std::sync::Arc;
use tokio::signal;

/// AT Protocol Jetstream Consumer Tool
#[derive(Parser)]
#[command(
    name = "atproto-jetstream-consumer",
    version,
    about = "Stream AT Protocol events from Jetstream instances",
    long_about = "
A command-line tool for connecting to AT Protocol Jetstream instances and
streaming events from specified collections. Supports optional zstd compression
for efficient data transfer.

COMPRESSION:
  Use 'none' as the zstd_dictionary to disable compression, or provide
  a path to a zstd dictionary file to enable compression.

COLLECTIONS:
  Specify zero or more AT Protocol collections to subscribe to. If no
  collections are specified, subscribes to all collections.

EXAMPLES:
  # Subscribe to feed posts with compression:
  atproto-jetstream-consumer jetstream1.us-east.bsky.network \\
    /path/to/dict.zstd app.bsky.feed.post

  # Subscribe to multiple collections without compression:
  atproto-jetstream-consumer jetstream1.us-east.bsky.network none \\
    app.bsky.feed.post app.bsky.feed.repost

  # Subscribe to all collections:
  atproto-jetstream-consumer jetstream1.us-east.bsky.network none

ENVIRONMENT VARIABLES:
  CERTIFICATE_BUNDLES  Additional CA certificate bundles
  USER_AGENT          Custom user agent string
"
)]
struct Args {
    /// Hostname of the Jetstream instance to connect to
    jetstream_hostname: String,

    /// Path to zstd dictionary file (use 'none' to disable compression)
    zstd_dictionary: String,

    /// Zero or more AT Protocol collections to subscribe to
    collections: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Parse command line arguments
    let args = Args::parse();

    let jetstream_hostname = &args.jetstream_hostname;
    let zstd_dictionary_path = &args.zstd_dictionary;
    let collections = args.collections;

    tracing::info!(
        hostname = %jetstream_hostname,
        dictionary = %zstd_dictionary_path,
        collections = ?collections,
        "Starting Jetstream consumer"
    );

    // Handle environment variables
    let _certificate_bundles: CertificateBundles =
        optional_env("CERTIFICATE_BUNDLES").try_into()?;
    let default_user_agent = format!(
        "atproto-jetstream-rs ({}; +https://tangled.org/ngerakines.me/atproto-crates)",
        version()?
    );
    let user_agent = default_env("USER_AGENT", &default_user_agent);

    tracing::info!(user_agent = %user_agent, "Configuration loaded");

    // Load zstd dictionary if specified
    let compression_enabled = zstd_dictionary_path != "none";

    // Create consumer configuration
    let config = ConsumerTaskConfig {
        user_agent,
        compression: compression_enabled,
        zstd_dictionary_location: if compression_enabled {
            zstd_dictionary_path.to_string()
        } else {
            String::new()
        },
        jetstream_hostname: jetstream_hostname.to_string(),
        collections: if collections.is_empty() {
            tracing::info!("No collections specified, subscribing to all");
            vec![]
        } else {
            collections
        },
        dids: vec![],                 // Default to all DIDs
        max_message_size_bytes: None, // Default to no limit
        cursor: None,                 // Default to live-tail
        require_hello: true,          // Default to true as requested
    };

    // Create consumer
    let consumer = Consumer::new(config);

    // Register logging handler
    let logging_handler = Arc::new(LoggingHandler::new("jetstream-logger".to_string()));
    consumer.register_handler(logging_handler).await?;

    tracing::info!("Jetstream consumer registered and ready");

    // Set up cancellation token for graceful shutdown
    let cancellation_token = CancellationToken::new();
    let cancellation_token_clone = cancellation_token.clone();

    // Set up signal handling for graceful shutdown
    let signal_handler = tokio::spawn(async move {
        #[cfg(unix)]
        {
            let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
                .expect("Failed to create SIGINT handler");
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("Failed to create SIGTERM handler");

            tokio::select! {
                _ = sigint.recv() => {
                    tracing::info!("Received SIGINT, initiating graceful shutdown");
                }
                _ = sigterm.recv() => {
                    tracing::info!("Received SIGTERM, initiating graceful shutdown");
                }
            }
        }

        #[cfg(windows)]
        {
            signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        }

        cancellation_token_clone.cancel();
    });

    // Run consumer
    let consumer_task = tokio::spawn(async move {
        if let Err(err) = consumer.run_background(cancellation_token).await {
            tracing::error!(error = ?err, "Consumer failed");
            return Err(err);
        }
        Ok(())
    });

    tracing::info!("Consumer started, press Ctrl+C to stop");

    // Wait for either the consumer to finish or signal handler
    tokio::select! {
        result = consumer_task => {
            match result {
                Ok(Ok(())) => tracing::info!("Consumer finished successfully"),
                Ok(Err(err)) => {
                    tracing::error!(error = ?err, "Consumer failed");
                    return Err(err);
                }
                Err(err) => {
                    tracing::error!(error = ?err, "Consumer task panicked");
                    return Err(err.into());
                }
            }
        }
        _ = signal_handler => {
            tracing::info!("Signal handler completed");
        }
    }

    tracing::info!("Jetstream consumer shutting down");
    Ok(())
}
