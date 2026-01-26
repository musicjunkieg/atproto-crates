//! WebSocket connection management for TAP streams.
//!
//! This module handles the low-level WebSocket connection to a TAP service,
//! including authentication and message sending/receiving.

use crate::config::TapConfig;
use crate::errors::TapError;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use futures::{SinkExt, StreamExt};
use http::Uri;
use std::str::FromStr;
use tokio::net::TcpStream;
use tokio_websockets::MaybeTlsStream;
use tokio_websockets::{ClientBuilder, Message, WebSocketStream};

/// WebSocket connection to a TAP service.
pub(crate) struct TapConnection {
    /// The underlying WebSocket stream.
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    /// Pre-allocated buffer for acknowledgment messages.
    ack_buffer: Vec<u8>,
}

impl TapConnection {
    /// Establish a new WebSocket connection to the TAP service.
    pub async fn connect(config: &TapConfig) -> Result<Self, TapError> {
        let uri =
            Uri::from_str(&config.ws_url()).map_err(|e| TapError::InvalidUrl(e.to_string()))?;

        let mut builder = ClientBuilder::from_uri(uri);

        // Add User-Agent header
        builder = builder
            .add_header(
                http::header::USER_AGENT,
                http::HeaderValue::from_str(&config.user_agent).map_err(|e| {
                    TapError::ConnectionFailed(format!("Invalid user agent: {}", e))
                })?,
            )
            .map_err(|e| TapError::ConnectionFailed(format!("Failed to add header: {}", e)))?;

        // Add Basic Auth header if password is configured
        if let Some(password) = &config.admin_password {
            let credentials = format!("admin:{}", password);
            let encoded = BASE64.encode(credentials.as_bytes());
            let auth_value = format!("Basic {}", encoded);

            builder = builder
                .add_header(
                    http::header::AUTHORIZATION,
                    http::HeaderValue::from_str(&auth_value).map_err(|e| {
                        TapError::ConnectionFailed(format!("Invalid auth header: {}", e))
                    })?,
                )
                .map_err(|e| {
                    TapError::ConnectionFailed(format!("Failed to add auth header: {}", e))
                })?;
        }

        // Connect
        let (ws, _response) = builder
            .connect()
            .await
            .map_err(|e| TapError::ConnectionFailed(e.to_string()))?;

        tracing::debug!(hostname = %config.hostname, "Connected to TAP service");

        Ok(Self {
            ws,
            ack_buffer: Vec::with_capacity(48), // {"type":"ack","id":18446744073709551615} is 40 bytes max
        })
    }

    /// Receive the next message from the WebSocket.
    ///
    /// Returns `None` if the connection was closed cleanly.
    pub async fn recv(&mut self) -> Result<Option<String>, TapError> {
        match self.ws.next().await {
            Some(Ok(msg)) => {
                if msg.is_text() {
                    msg.as_text().map(|s| Some(s.to_string())).ok_or_else(|| {
                        TapError::ParseError("Failed to get text from message".into())
                    })
                } else if msg.is_close() {
                    tracing::debug!("Received close frame from TAP service");
                    Ok(None)
                } else {
                    // Ignore ping/pong and binary messages
                    tracing::trace!("Received non-text message, ignoring");
                    // Recurse to get the next text message
                    Box::pin(self.recv()).await
                }
            }
            Some(Err(e)) => Err(TapError::ConnectionFailed(e.to_string())),
            None => {
                tracing::debug!("WebSocket stream ended");
                Ok(None)
            }
        }
    }

    /// Send an acknowledgment for the given event ID.
    ///
    /// Uses a pre-allocated buffer and itoa for allocation-free formatting.
    /// Format: `{"type":"ack","id":12345}`
    pub async fn send_ack(&mut self, id: u64) -> Result<(), TapError> {
        self.ack_buffer.clear();
        self.ack_buffer
            .extend_from_slice(b"{\"type\":\"ack\",\"id\":");
        let mut itoa_buf = itoa::Buffer::new();
        self.ack_buffer
            .extend_from_slice(itoa_buf.format(id).as_bytes());
        self.ack_buffer.push(b'}');

        // All bytes are ASCII so this is always valid UTF-8
        let msg = std::str::from_utf8(&self.ack_buffer).expect("ack buffer contains only ASCII");

        self.ws
            .send(Message::text(msg.to_string()))
            .await
            .map_err(|e| TapError::AckFailed(e.to_string()))?;

        // Flush to ensure the ack is sent immediately
        self.ws
            .flush()
            .await
            .map_err(|e| TapError::AckFailed(format!("Failed to flush ack: {}", e)))?;

        tracing::trace!(id, "Sent ack");
        Ok(())
    }

    /// Close the WebSocket connection gracefully.
    pub async fn close(&mut self) -> Result<(), TapError> {
        self.ws
            .close()
            .await
            .map_err(|e| TapError::ConnectionFailed(format!("Failed to close: {}", e)))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_ack_buffer_format() {
        // Test that our manual JSON formatting is correct
        // Format: {"type":"ack","id":12345}
        let mut buffer = Vec::with_capacity(64);

        let id: u64 = 12345;
        buffer.clear();
        buffer.extend_from_slice(b"{\"type\":\"ack\",\"id\":");
        let mut itoa_buf = itoa::Buffer::new();
        buffer.extend_from_slice(itoa_buf.format(id).as_bytes());
        buffer.push(b'}');

        let result = std::str::from_utf8(&buffer).unwrap();
        assert_eq!(result, r#"{"type":"ack","id":12345}"#);

        // Test max u64
        let id: u64 = u64::MAX;
        buffer.clear();
        buffer.extend_from_slice(b"{\"type\":\"ack\",\"id\":");
        buffer.extend_from_slice(itoa_buf.format(id).as_bytes());
        buffer.push(b'}');

        let result = std::str::from_utf8(&buffer).unwrap();
        assert_eq!(result, r#"{"type":"ack","id":18446744073709551615}"#);
        assert!(buffer.len() <= 64); // Fits in our pre-allocated buffer
    }
}
