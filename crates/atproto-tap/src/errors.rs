//! Error types for TAP operations.
//!
//! This module defines the error types returned by TAP stream and client operations.

use thiserror::Error;

/// Errors that can occur during TAP operations.
#[derive(Debug, Error)]
pub enum TapError {
    /// WebSocket connection failed.
    #[error("error-atproto-tap-connection-1 WebSocket connection failed: {0}")]
    ConnectionFailed(String),

    /// Connection was closed unexpectedly.
    #[error("error-atproto-tap-connection-2 Connection closed unexpectedly")]
    ConnectionClosed,

    /// Maximum reconnection attempts exceeded.
    #[error(
        "error-atproto-tap-connection-3 Maximum reconnection attempts exceeded after {0} attempts"
    )]
    MaxReconnectAttemptsExceeded(u32),

    /// Authentication failed.
    #[error("error-atproto-tap-auth-1 Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Failed to parse a message from the server.
    #[error("error-atproto-tap-parse-1 Failed to parse message: {0}")]
    ParseError(String),

    /// Failed to send an acknowledgment.
    #[error("error-atproto-tap-ack-1 Failed to send acknowledgment: {0}")]
    AckFailed(String),

    /// HTTP request failed.
    #[error("error-atproto-tap-http-1 HTTP request failed: {0}")]
    HttpError(String),

    /// HTTP response indicated an error.
    #[error("error-atproto-tap-http-2 HTTP error response: {status} - {message}")]
    HttpResponseError {
        /// HTTP status code.
        status: u16,
        /// Error message from response.
        message: String,
    },

    /// Invalid URL.
    #[error("error-atproto-tap-url-1 Invalid URL: {0}")]
    InvalidUrl(String),

    /// I/O error.
    #[error("error-atproto-tap-io-1 I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("error-atproto-tap-json-1 JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Stream has been closed and cannot be used.
    #[error("error-atproto-tap-stream-1 Stream is closed")]
    StreamClosed,

    /// Operation timed out.
    #[error("error-atproto-tap-timeout-1 Operation timed out")]
    Timeout,
}

impl TapError {
    /// Returns true if this error indicates a connection issue that may be recoverable.
    pub fn is_connection_error(&self) -> bool {
        matches!(
            self,
            TapError::ConnectionFailed(_)
                | TapError::ConnectionClosed
                | TapError::IoError(_)
                | TapError::Timeout
        )
    }

    /// Returns true if this error is a parse error that doesn't affect connection state.
    pub fn is_parse_error(&self) -> bool {
        matches!(self, TapError::ParseError(_) | TapError::JsonError(_))
    }

    /// Returns true if this error is fatal and the stream should not attempt recovery.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            TapError::MaxReconnectAttemptsExceeded(_)
                | TapError::AuthenticationFailed(_)
                | TapError::StreamClosed
        )
    }
}

impl From<reqwest::Error> for TapError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            TapError::Timeout
        } else if err.is_connect() {
            TapError::ConnectionFailed(err.to_string())
        } else {
            TapError::HttpError(err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classification() {
        assert!(TapError::ConnectionFailed("test".into()).is_connection_error());
        assert!(TapError::ConnectionClosed.is_connection_error());
        assert!(TapError::Timeout.is_connection_error());

        assert!(TapError::ParseError("test".into()).is_parse_error());
        assert!(
            TapError::JsonError(serde_json::from_str::<()>("invalid").unwrap_err())
                .is_parse_error()
        );

        assert!(TapError::MaxReconnectAttemptsExceeded(5).is_fatal());
        assert!(TapError::AuthenticationFailed("test".into()).is_fatal());
        assert!(TapError::StreamClosed.is_fatal());

        // Non-fatal errors
        assert!(!TapError::ConnectionFailed("test".into()).is_fatal());
        assert!(!TapError::ParseError("test".into()).is_fatal());
    }

    #[test]
    fn test_error_display() {
        let err = TapError::ConnectionFailed("refused".to_string());
        assert!(err.to_string().contains("error-atproto-tap-connection-1"));
        assert!(err.to_string().contains("refused"));

        let err = TapError::HttpResponseError {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert!(err.to_string().contains("404"));
        assert!(err.to_string().contains("Not Found"));
    }
}
