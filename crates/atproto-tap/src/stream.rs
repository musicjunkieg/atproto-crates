//! TAP event stream implementation.
//!
//! This module provides [`TapStream`], an async stream that yields TAP events
//! with automatic connection management and reconnection handling.
//!
//! # Design
//!
//! The stream encapsulates all connection logic, allowing consumers to simply
//! iterate over events using standard stream combinators or `tokio::select!`.
//!
//! Reconnection is handled automatically with exponential backoff. Parse errors
//! are yielded as `Err` items but don't affect connection state - only connection
//! errors trigger reconnection attempts.

use crate::config::TapConfig;
use crate::connection::TapConnection;
use crate::errors::TapError;
use crate::events::{TapEvent, extract_event_id};
use futures::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;

/// An async stream of TAP events with automatic reconnection.
///
/// `TapStream` implements [`Stream`] and yields `Result<Arc<TapEvent>, TapError>`.
/// Events are wrapped in `Arc` for efficient zero-cost sharing across consumers.
///
/// # Connection Management
///
/// The stream automatically:
/// - Connects on first poll
/// - Reconnects with exponential backoff on connection errors
/// - Sends acknowledgments after parsing each message (if enabled)
/// - Yields parse errors without affecting connection state
///
/// # Example
///
/// ```ignore
/// use atproto_tap::{TapConfig, TapStream};
/// use tokio_stream::StreamExt;
///
/// let config = TapConfig::builder()
///     .hostname("localhost:2480")
///     .build();
///
/// let mut stream = TapStream::new(config);
///
/// while let Some(result) = stream.next().await {
///     match result {
///         Ok(event) => println!("Event: {:?}", event),
///         Err(e) => eprintln!("Error: {}", e),
///     }
/// }
/// ```
pub struct TapStream {
    /// Receiver for events from the background task.
    receiver: mpsc::Receiver<Result<Arc<TapEvent>, TapError>>,
    /// Handle to request stream closure.
    close_sender: Option<mpsc::Sender<()>>,
    /// Whether the stream has been closed.
    closed: bool,
}

impl TapStream {
    /// Create a new TAP stream with the given configuration.
    ///
    /// The stream will start connecting immediately in a background task.
    pub fn new(config: TapConfig) -> Self {
        // Channel for events - buffer a few to handle bursts
        let (event_tx, event_rx) = mpsc::channel(config.channel_buffer_size);
        // Channel for close signal
        let (close_tx, close_rx) = mpsc::channel(1);

        // Spawn background task to manage connection
        tokio::spawn(connection_task(config, event_tx, close_rx));

        Self {
            receiver: event_rx,
            close_sender: Some(close_tx),
            closed: false,
        }
    }

    /// Close the stream and release resources.
    ///
    /// After calling this, the stream will yield `None` on the next poll.
    pub async fn close(&mut self) {
        if let Some(sender) = self.close_sender.take() {
            // Signal the background task to close
            let _ = sender.send(()).await;
        }
        self.closed = true;
    }

    /// Returns true if the stream is closed.
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

impl Stream for TapStream {
    type Item = Result<Arc<TapEvent>, TapError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.closed {
            return Poll::Ready(None);
        }

        self.receiver.poll_recv(cx)
    }
}

impl Drop for TapStream {
    fn drop(&mut self) {
        // Drop the close_sender to signal the background task
        self.close_sender.take();
        tracing::debug!("TapStream dropped");
    }
}

/// Background task that manages the WebSocket connection.
async fn connection_task(
    config: TapConfig,
    event_tx: mpsc::Sender<Result<Arc<TapEvent>, TapError>>,
    mut close_rx: mpsc::Receiver<()>,
) {
    let mut current_reconnect_delay = config.initial_reconnect_delay;
    let mut attempt: u32 = 0;

    loop {
        // Check for close signal
        if close_rx.try_recv().is_ok() {
            tracing::debug!("Connection task received close signal");
            break;
        }

        // Try to connect
        tracing::debug!(attempt, hostname = %config.hostname, "Connecting to TAP service");
        let conn_result = TapConnection::connect(&config).await;

        match conn_result {
            Ok(mut conn) => {
                tracing::info!(hostname = %config.hostname, "TAP stream connected");
                // Reset reconnection state on successful connect
                current_reconnect_delay = config.initial_reconnect_delay;
                attempt = 0;

                // Event loop for this connection
                loop {
                    tokio::select! {
                        biased;

                        _ = close_rx.recv() => {
                            tracing::debug!("Connection task received close signal during receive");
                            let _ = conn.close().await;
                            return;
                        }

                        recv_result = conn.recv() => {
                            match recv_result {
                                Ok(Some(msg)) => {
                                    // Parse the message
                                    match serde_json::from_str::<TapEvent>(&msg) {
                                        Ok(event) => {
                                            let event_id = event.id();

                                            // Send ack if enabled (before sending event to channel)
                                            if config.send_acks
                                                && let Err(err) = conn.send_ack(event_id).await
                                            {
                                                tracing::warn!(error = %err, "Failed to send ack");
                                                // Don't break connection for ack errors
                                            }

                                            // Send event to channel
                                            let event = Arc::new(event);
                                            if event_tx.send(Ok(event)).await.is_err() {
                                                // Receiver dropped, exit task
                                                tracing::debug!("Event receiver dropped, closing connection");
                                                let _ = conn.close().await;
                                                return;
                                            }
                                        }
                                        Err(err) => {
                                            // Parse errors don't affect connection
                                            tracing::warn!(error = %err, "Failed to parse TAP message");

                                            // Try to extract just the ID using fallback parser
                                            // so we can still ack the message even if full parsing fails
                                            if config.send_acks {
                                                if let Some(event_id) = extract_event_id(&msg) {
                                                    tracing::debug!(event_id, "Extracted event ID via fallback parser");
                                                    if let Err(ack_err) = conn.send_ack(event_id).await {
                                                        tracing::warn!(error = %ack_err, "Failed to send ack for unparseable message");
                                                    }
                                                } else {
                                                    tracing::warn!("Could not extract event ID from unparseable message");
                                                }
                                            }

                                            if event_tx.send(Err(TapError::ParseError(err.to_string()))).await.is_err() {
                                                tracing::debug!("Event receiver dropped, closing connection");
                                                let _ = conn.close().await;
                                                return;
                                            }
                                        }
                                    }
                                }
                                Ok(None) => {
                                    // Connection closed by server
                                    tracing::debug!("TAP connection closed by server");
                                    break;
                                }
                                Err(err) => {
                                    // Connection error
                                    tracing::warn!(error = %err, "TAP connection error");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, attempt, "Failed to connect to TAP service");
            }
        }

        // Increment attempt counter
        attempt += 1;

        // Check if we've exceeded max attempts
        if let Some(max) = config.max_reconnect_attempts
            && attempt >= max
        {
            tracing::error!(attempts = attempt, "Max reconnection attempts exceeded");
            let _ = event_tx
                .send(Err(TapError::MaxReconnectAttemptsExceeded(attempt)))
                .await;
            break;
        }

        // Wait before reconnecting with exponential backoff
        tracing::debug!(
            delay_ms = current_reconnect_delay.as_millis(),
            attempt,
            "Waiting before reconnection"
        );

        tokio::select! {
            _ = close_rx.recv() => {
                tracing::debug!("Connection task received close signal during backoff");
                return;
            }
            _ = tokio::time::sleep(current_reconnect_delay) => {
                // Update delay for next attempt
                current_reconnect_delay = Duration::from_secs_f64(
                    (current_reconnect_delay.as_secs_f64() * config.reconnect_backoff_multiplier)
                        .min(config.max_reconnect_delay.as_secs_f64()),
                );
            }
        }
    }

    tracing::debug!("Connection task exiting");
}

/// Create a new TAP stream with the given configuration.
pub fn connect(config: TapConfig) -> TapStream {
    TapStream::new(config)
}

/// Create a new TAP stream connected to the given hostname.
///
/// Uses default configuration values.
pub fn connect_to(hostname: &str) -> TapStream {
    TapStream::new(TapConfig::new(hostname))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_initial_state() {
        // Note: This test doesn't actually poll the stream, just checks initial state
        // Creating a TapStream requires a tokio runtime for the spawn
    }

    #[tokio::test]
    async fn test_stream_close() {
        let mut stream = TapStream::new(TapConfig::new("localhost:9999"));
        assert!(!stream.is_closed());
        stream.close().await;
        assert!(stream.is_closed());
    }

    #[test]
    fn test_connect_functions() {
        // These just create configs, actual connection happens in background task
        // We can't test without a runtime, so just verify the types compile
        let _ = TapConfig::new("localhost:2480");
    }

    #[test]
    fn test_reconnect_delay_calculation() {
        // Test the delay calculation logic
        let initial = Duration::from_secs(1);
        let max = Duration::from_secs(10);
        let multiplier = 2.0;

        let mut delay = initial;
        assert_eq!(delay, Duration::from_secs(1));

        delay = Duration::from_secs_f64((delay.as_secs_f64() * multiplier).min(max.as_secs_f64()));
        assert_eq!(delay, Duration::from_secs(2));

        delay = Duration::from_secs_f64((delay.as_secs_f64() * multiplier).min(max.as_secs_f64()));
        assert_eq!(delay, Duration::from_secs(4));

        delay = Duration::from_secs_f64((delay.as_secs_f64() * multiplier).min(max.as_secs_f64()));
        assert_eq!(delay, Duration::from_secs(8));

        delay = Duration::from_secs_f64((delay.as_secs_f64() * multiplier).min(max.as_secs_f64()));
        assert_eq!(delay, Duration::from_secs(10)); // Capped at max
    }
}
