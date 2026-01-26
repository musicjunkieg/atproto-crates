//! Command-line client for TAP services.
//!
//! This tool provides commands for consuming TAP events and managing tracked repositories.
//!
//! # Usage
//!
//! ```bash
//! # Stream events from a TAP service
//! cargo run --features cli --bin atproto-tap-client -- localhost:2480 read
//!
//! # Stream with authentication and filters
//! cargo run --features cli --bin atproto-tap-client -- localhost:2480 -p secret read --live-only
//!
//! # Add repositories to track
//! cargo run --features cli --bin atproto-tap-client -- localhost:2480 -p secret repos add did:plc:xyz did:plc:abc
//!
//! # Remove repositories from tracking
//! cargo run --features cli --bin atproto-tap-client -- localhost:2480 -p secret repos remove did:plc:xyz
//!
//! # Resolve a DID to its DID document
//! cargo run --features cli --bin atproto-tap-client -- localhost:2480 resolve did:plc:xyz
//!
//! # Resolve a DID and only output the handle
//! cargo run --features cli --bin atproto-tap-client -- localhost:2480 resolve did:plc:xyz --handle-only
//!
//! # Get repository tracking info
//! cargo run --features cli --bin atproto-tap-client -- localhost:2480 info did:plc:xyz
//! ```

use atproto_tap::{TapClient, TapConfig, TapEvent, connect};
use clap::{Parser, Subcommand};
use std::time::Duration;
use tokio_stream::StreamExt;

/// TAP service client for consuming events and managing repositories.
#[derive(Parser)]
#[command(
    name = "atproto-tap-client",
    version,
    about = "TAP service client for AT Protocol",
    long_about = "Connect to a TAP service to stream repository/identity events or manage tracked repositories.\n\n\
                  Events are printed to stdout as JSON, one per line.\n\
                  Use Ctrl+C to gracefully stop the consumer."
)]
struct Args {
    /// TAP service hostname (e.g., localhost:2480)
    hostname: String,

    /// Admin password for authentication
    #[arg(short, long, global = true)]
    password: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Connect to TAP and stream events as JSON
    Read {
        /// Disable acknowledgments
        #[arg(long)]
        no_acks: bool,

        /// Maximum reconnection attempts (0 = unlimited)
        #[arg(long, default_value = "0")]
        max_reconnects: u32,

        /// Print debug information to stderr
        #[arg(short, long)]
        debug: bool,

        /// Filter to specific collections (comma-separated)
        #[arg(long)]
        collections: Option<String>,

        /// Only show live events (skip backfill)
        #[arg(long)]
        live_only: bool,
    },

    /// Manage tracked repositories
    Repos {
        #[command(subcommand)]
        action: ReposAction,
    },

    /// Resolve a DID to its DID document
    Resolve {
        /// DID to resolve (e.g., did:plc:xyz123)
        did: String,

        /// Only output the handle (instead of full DID document)
        #[arg(long)]
        handle_only: bool,
    },

    /// Get tracking info for a repository
    Info {
        /// DID to get info for (e.g., did:plc:xyz123)
        did: String,
    },
}

#[derive(Subcommand)]
enum ReposAction {
    /// Add repositories to track
    Add {
        /// DIDs to add (e.g., did:plc:xyz123)
        #[arg(required = true)]
        dids: Vec<String>,
    },

    /// Remove repositories from tracking
    Remove {
        /// DIDs to remove
        #[arg(required = true)]
        dids: Vec<String>,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Command::Read {
            no_acks,
            max_reconnects,
            debug,
            collections,
            live_only,
        } => {
            run_read(
                &args.hostname,
                args.password,
                no_acks,
                max_reconnects,
                debug,
                collections,
                live_only,
            )
            .await;
        }
        Command::Repos { action } => {
            run_repos(&args.hostname, args.password, action).await;
        }
        Command::Resolve { did, handle_only } => {
            run_resolve(&args.hostname, args.password, &did, handle_only).await;
        }
        Command::Info { did } => {
            run_info(&args.hostname, args.password, &did).await;
        }
    }
}

async fn run_read(
    hostname: &str,
    password: Option<String>,
    no_acks: bool,
    max_reconnects: u32,
    debug: bool,
    collections: Option<String>,
    live_only: bool,
) {
    // Initialize tracing if debug mode
    if debug {
        tracing_subscriber::fmt()
            .with_env_filter("atproto_tap=debug")
            .with_writer(std::io::stderr)
            .init();
    }

    // Build configuration
    let mut config_builder = TapConfig::builder().hostname(hostname).send_acks(!no_acks);

    if let Some(password) = password {
        config_builder = config_builder.admin_password(password);
    }

    if max_reconnects > 0 {
        config_builder = config_builder.max_reconnect_attempts(Some(max_reconnects));
    }

    // Set reasonable defaults for CLI usage
    config_builder = config_builder
        .initial_reconnect_delay(Duration::from_secs(1))
        .max_reconnect_delay(Duration::from_secs(30));

    let config = config_builder.build();

    eprintln!("Connecting to TAP service at {}...", hostname);

    let mut stream = connect(config);

    // Parse collection filters
    let collection_filters: Vec<String> = collections
        .map(|c| c.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    // Handle Ctrl+C
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            Some(result) = stream.next() => {
                match result {
                    Ok(event) => {
                        // Apply filters
                        let should_print = match event.as_ref() {
                            TapEvent::Record { record, .. } => {
                                // Filter by live flag
                                if live_only && !record.live {
                                    false
                                }
                                // Filter by collection
                                else if !collection_filters.is_empty() {
                                    collection_filters.iter().any(|c| record.collection.as_ref() == c)
                                } else {
                                    true
                                }
                            }
                            TapEvent::Identity { .. } => !live_only, // Always show identity unless live_only
                        };

                        if should_print {
                            // Print as JSON to stdout
                            match serde_json::to_string(event.as_ref()) {
                                Ok(json) => println!("{}", json),
                                Err(e) => {
                                    eprintln!("Failed to serialize event: {}", e);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);

                        // Exit on fatal errors
                        if e.is_fatal() {
                            eprintln!("Fatal error, exiting");
                            std::process::exit(1);
                        }
                    }
                }
            }
            _ = &mut ctrl_c => {
                eprintln!("\nReceived Ctrl+C, shutting down...");
                stream.close().await;
                break;
            }
        }
    }

    eprintln!("Client stopped");
}

async fn run_repos(hostname: &str, password: Option<String>, action: ReposAction) {
    let client = TapClient::new(hostname, password);

    match action {
        ReposAction::Add { dids } => {
            let did_refs: Vec<&str> = dids.iter().map(|s| s.as_str()).collect();

            match client.add_repos(&did_refs).await {
                Ok(()) => {
                    eprintln!("Added {} repository(ies) to tracking", dids.len());
                    for did in &dids {
                        println!("{}", did);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to add repositories: {}", e);
                    std::process::exit(1);
                }
            }
        }
        ReposAction::Remove { dids } => {
            let did_refs: Vec<&str> = dids.iter().map(|s| s.as_str()).collect();

            match client.remove_repos(&did_refs).await {
                Ok(()) => {
                    eprintln!("Removed {} repository(ies) from tracking", dids.len());
                    for did in &dids {
                        println!("{}", did);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to remove repositories: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

async fn run_resolve(hostname: &str, password: Option<String>, did: &str, handle_only: bool) {
    let client = TapClient::new(hostname, password);

    match client.resolve(did).await {
        Ok(doc) => {
            if handle_only {
                // Use the handles() method from atproto_identity::model::Document
                match doc.handles() {
                    Some(handle) => println!("{}", handle),
                    None => {
                        eprintln!("No handle found in DID document");
                        std::process::exit(1);
                    }
                }
            } else {
                // Print full DID document as JSON
                match serde_json::to_string_pretty(&doc) {
                    Ok(json) => println!("{}", json),
                    Err(e) => {
                        eprintln!("Failed to serialize DID document: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to resolve DID: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_info(hostname: &str, password: Option<String>, did: &str) {
    let client = TapClient::new(hostname, password);

    match client.info(did).await {
        Ok(info) => {
            // Print as JSON for easy parsing
            match serde_json::to_string_pretty(&info) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("Failed to serialize info: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to get repository info: {}", e);
            std::process::exit(1);
        }
    }
}
