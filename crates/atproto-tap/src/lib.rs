//! TAP (Trusted Attestation Protocol) service consumer for AT Protocol.
//!
//! This crate provides a client for consuming events from a TAP service,
//! which delivers filtered, verified AT Protocol repository events.
//!
//! # Overview
//!
//! TAP is a single-tenant service that subscribes to an AT Protocol Relay and
//! outputs filtered, verified events. Key features include:
//!
//! - **Verified Events**: MST integrity checks and signature verification
//! - **Automatic Backfill**: Historical events delivered with `live: false`
//! - **Repository Filtering**: Track specific DIDs or collections
//! - **Acknowledgment Protocol**: At-least-once delivery semantics
//!
//! # Quick Start
//!
//! ```ignore
//! use atproto_tap::{connect_to, TapEvent};
//! use tokio_stream::StreamExt;
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut stream = connect_to("localhost:2480");
//!
//!     while let Some(result) = stream.next().await {
//!         match result {
//!             Ok(event) => match event.as_ref() {
//!                 TapEvent::Record { record, .. } => {
//!                     println!("{} {} {}", record.action, record.collection, record.did);
//!                 }
//!                 TapEvent::Identity { identity, .. } => {
//!                     println!("Identity: {} = {}", identity.did, identity.handle);
//!                 }
//!             },
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//! }
//! ```
//!
//! # Using with `tokio::select!`
//!
//! The stream integrates naturally with Tokio's select macro:
//!
//! ```ignore
//! use atproto_tap::{connect, TapConfig};
//! use tokio_stream::StreamExt;
//! use tokio::signal;
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = TapConfig::builder()
//!         .hostname("localhost:2480")
//!         .admin_password("secret")
//!         .build();
//!
//!     let mut stream = connect(config);
//!
//!     loop {
//!         tokio::select! {
//!             Some(result) = stream.next() => {
//!                 // Process event
//!             }
//!             _ = signal::ctrl_c() => {
//!                 break;
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! # Management API
//!
//! Use [`TapClient`] to manage tracked repositories:
//!
//! ```ignore
//! use atproto_tap::TapClient;
//!
//! let client = TapClient::new("localhost:2480", Some("password".to_string()));
//!
//! // Add repositories to track
//! client.add_repos(&["did:plc:xyz123"]).await?;
//!
//! // Check service health
//! if client.health().await? {
//!     println!("TAP service is healthy");
//! }
//! ```
//!
//! # Memory Efficiency
//!
//! This crate is optimized for high-throughput event processing:
//!
//! - **Arc-wrapped events**: Events are shared via `Arc` for zero-cost sharing
//! - **CompactString**: Small strings use inline storage (no heap allocation)
//! - **`Box<str>`**: Immutable strings without capacity overhead
//! - **RawValue**: Record payloads are lazily parsed on demand
//! - **Pre-allocated buffers**: Ack messages avoid per-message allocations

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod client;
mod config;
mod connection;
mod errors;
mod events;
mod stream;

// Re-export public types
pub use atproto_identity::model::{Document, Service, VerificationMethod};
#[allow(deprecated)]
pub use client::RepoStatus;
pub use client::{RepoInfo, RepoState, TapClient};
pub use config::{TapConfig, TapConfigBuilder};
pub use errors::TapError;
pub use events::{
    IdentityEvent, IdentityStatus, RecordAction, RecordEvent, TapEvent, extract_event_id,
};
pub use stream::{TapStream, connect, connect_to};
