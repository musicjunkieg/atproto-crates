//! # Structured Error Types for AT Protocol Jetstream
//!
//! Error handling for WebSocket-based event streaming operations using the AT Protocol Jetstream service.
//! All errors follow the project convention of prefixed error codes with descriptive messages.
//!
//! ## Error Categories
//!
//! - **`ConsumerError`** (consumer-1 to consumer-9): Errors related to WebSocket connections, message processing, and event handling
//!
//! ## Error Format
//!
//! All errors use the standardized format: `error-atproto-jetstream-{domain}-{number} {message}: {details}`

use thiserror::Error;

/// Errors that can occur when consuming events from the Jetstream service
#[derive(Error, Debug)]
pub enum ConsumerError {
    /// Occurs when WebSocket connection establishment or maintenance fails
    #[error("error-atproto-jetstream-consumer-1 WebSocket connection failed: {0}")]
    ConnectionFailed(String),

    /// Occurs when message decompression using zstd dictionary fails
    #[error("error-atproto-jetstream-consumer-2 Message decompression failed: {0}")]
    DecompressionFailed(String),

    /// Occurs when deserializing Jetstream events from JSON fails
    #[error("error-atproto-jetstream-consumer-3 Event deserialization failed: {0}")]
    DeserializationFailed(String),

    /// Occurs when attempting to register event handlers after consumer has started
    #[error("error-atproto-jetstream-consumer-4 Handler registration failed: {0}")]
    HandlerRegistrationFailed(String),

    /// Occurs when event sender channel is not properly initialized
    #[error("error-atproto-jetstream-consumer-5 Event sender not initialized: {0}")]
    EventSenderNotInitialized(String),

    /// Occurs when converting WebSocket messages to expected format fails
    #[error("error-atproto-jetstream-consumer-6 Message conversion failed: {0}")]
    MessageConversionFailed(String),

    /// Occurs when serializing update messages for transmission fails
    #[error("error-atproto-jetstream-consumer-7 Update serialization failed: {0}")]
    UpdateSerializationFailed(String),

    /// Occurs when sending updates through WebSocket connection fails
    #[error("error-atproto-jetstream-consumer-8 Update send failed: {0}")]
    UpdateSendFailed(String),

    /// Occurs when creating zstd decompressor with dictionary fails
    #[error("error-atproto-jetstream-consumer-9 Decompressor creation failed: {0}")]
    DecompressorCreationFailed(String),
}
