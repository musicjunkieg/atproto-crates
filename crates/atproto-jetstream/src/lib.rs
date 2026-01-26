//! WebSocket consumer for AT Protocol Jetstream events.
//!
//! Real-time event streaming with Zstandard compression, automatic reconnection,
//! and configurable event filtering for AT Protocol repository changes.
//!
//! ## Command-Line Tools
//!
//! When built with the `clap` feature, provides Jetstream consumer tools:
//!
//! - **`atproto-jetstream-consumer`**: Connect to AT Protocol Jetstream and consume real-time events with Zstandard compression

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod consumer;
pub mod errors;

pub use consumer::{
    Consumer, ConsumerTaskConfig, EventHandler, JetstreamEvent, JetstreamEventCommit,
    JetstreamEventDelete, LoggingHandler,
};
pub use errors::ConsumerError;

// Re-export CancellationToken for convenience
pub use tokio_util::sync::CancellationToken;
