//! Configuration for TAP stream connections.
//!
//! This module provides the [`TapConfig`] struct for configuring TAP stream
//! connections, including hostname, authentication, and reconnection behavior.

use std::time::Duration;

/// Configuration for a TAP stream connection.
///
/// Use [`TapConfig::builder()`] for ergonomic construction with defaults.
///
/// # Example
///
/// ```
/// use atproto_tap::TapConfig;
/// use std::time::Duration;
///
/// let config = TapConfig::builder()
///     .hostname("localhost:2480")
///     .admin_password("secret")
///     .send_acks(true)
///     .max_reconnect_attempts(Some(10))
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct TapConfig {
    /// TAP service hostname (e.g., "localhost:2480").
    ///
    /// The WebSocket URL is constructed as `ws://{hostname}/channel`.
    pub hostname: String,

    /// Optional admin password for authentication.
    ///
    /// If set, HTTP Basic Auth is used with username "admin".
    pub admin_password: Option<String>,

    /// Whether to send acknowledgments for received messages.
    ///
    /// Default: `true`. Set to `false` if the TAP service has acks disabled.
    pub send_acks: bool,

    /// User-Agent header value for WebSocket connections.
    pub user_agent: String,

    /// Maximum reconnection attempts before giving up.
    ///
    /// `None` means unlimited reconnection attempts (default).
    pub max_reconnect_attempts: Option<u32>,

    /// Initial delay before first reconnection attempt.
    ///
    /// Default: 1 second.
    pub initial_reconnect_delay: Duration,

    /// Maximum delay between reconnection attempts.
    ///
    /// Default: 60 seconds.
    pub max_reconnect_delay: Duration,

    /// Multiplier for exponential backoff between reconnections.
    ///
    /// Default: 2.0 (doubles the delay each attempt).
    pub reconnect_backoff_multiplier: f64,

    /// Size of the internal channel buffer between the WebSocket reader and consumer.
    ///
    /// A larger buffer absorbs more burst latency when the consumer has occasional
    /// slow processing. Default: 32.
    pub channel_buffer_size: usize,
}

impl Default for TapConfig {
    fn default() -> Self {
        Self {
            hostname: "localhost:2480".to_string(),
            admin_password: None,
            send_acks: true,
            user_agent: format!("atproto-tap/{}", env!("CARGO_PKG_VERSION")),
            max_reconnect_attempts: None,
            initial_reconnect_delay: Duration::from_secs(1),
            max_reconnect_delay: Duration::from_secs(60),
            reconnect_backoff_multiplier: 2.0,
            channel_buffer_size: 32,
        }
    }
}

impl TapConfig {
    /// Create a new configuration builder with defaults.
    pub fn builder() -> TapConfigBuilder {
        TapConfigBuilder::default()
    }

    /// Create a minimal configuration for the given hostname.
    pub fn new(hostname: impl Into<String>) -> Self {
        Self {
            hostname: hostname.into(),
            ..Default::default()
        }
    }

    /// Returns the WebSocket URL for the TAP channel.
    pub fn ws_url(&self) -> String {
        format!("ws://{}/channel", self.hostname)
    }

    /// Returns the HTTP base URL for the TAP management API.
    pub fn http_base_url(&self) -> String {
        format!("http://{}", self.hostname)
    }
}

/// Builder for [`TapConfig`].
#[derive(Debug, Clone, Default)]
pub struct TapConfigBuilder {
    config: TapConfig,
}

impl TapConfigBuilder {
    /// Set the TAP service hostname.
    pub fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.config.hostname = hostname.into();
        self
    }

    /// Set the admin password for authentication.
    pub fn admin_password(mut self, password: impl Into<String>) -> Self {
        self.config.admin_password = Some(password.into());
        self
    }

    /// Set whether to send acknowledgments.
    pub fn send_acks(mut self, send_acks: bool) -> Self {
        self.config.send_acks = send_acks;
        self
    }

    /// Set the User-Agent header value.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.config.user_agent = user_agent.into();
        self
    }

    /// Set the maximum reconnection attempts.
    ///
    /// `None` means unlimited attempts.
    pub fn max_reconnect_attempts(mut self, max: Option<u32>) -> Self {
        self.config.max_reconnect_attempts = max;
        self
    }

    /// Set the initial reconnection delay.
    pub fn initial_reconnect_delay(mut self, delay: Duration) -> Self {
        self.config.initial_reconnect_delay = delay;
        self
    }

    /// Set the maximum reconnection delay.
    pub fn max_reconnect_delay(mut self, delay: Duration) -> Self {
        self.config.max_reconnect_delay = delay;
        self
    }

    /// Set the reconnection backoff multiplier.
    pub fn reconnect_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.config.reconnect_backoff_multiplier = multiplier;
        self
    }

    /// Set the internal channel buffer size.
    pub fn channel_buffer_size(mut self, size: usize) -> Self {
        self.config.channel_buffer_size = size;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> TapConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TapConfig::default();
        assert_eq!(config.hostname, "localhost:2480");
        assert!(config.admin_password.is_none());
        assert!(config.send_acks);
        assert!(config.max_reconnect_attempts.is_none());
        assert_eq!(config.initial_reconnect_delay, Duration::from_secs(1));
        assert_eq!(config.max_reconnect_delay, Duration::from_secs(60));
        assert!((config.reconnect_backoff_multiplier - 2.0).abs() < f64::EPSILON);
        assert_eq!(config.channel_buffer_size, 32);
    }

    #[test]
    fn test_builder() {
        let config = TapConfig::builder()
            .hostname("tap.example.com:2480")
            .admin_password("secret123")
            .send_acks(false)
            .max_reconnect_attempts(Some(5))
            .initial_reconnect_delay(Duration::from_millis(500))
            .max_reconnect_delay(Duration::from_secs(30))
            .reconnect_backoff_multiplier(1.5)
            .build();

        assert_eq!(config.hostname, "tap.example.com:2480");
        assert_eq!(config.admin_password, Some("secret123".to_string()));
        assert!(!config.send_acks);
        assert_eq!(config.max_reconnect_attempts, Some(5));
        assert_eq!(config.initial_reconnect_delay, Duration::from_millis(500));
        assert_eq!(config.max_reconnect_delay, Duration::from_secs(30));
        assert!((config.reconnect_backoff_multiplier - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_channel_buffer_size() {
        let config = TapConfig::builder().channel_buffer_size(128).build();
        assert_eq!(config.channel_buffer_size, 128);
    }

    #[test]
    fn test_ws_url() {
        let config = TapConfig::new("localhost:2480");
        assert_eq!(config.ws_url(), "ws://localhost:2480/channel");

        let config = TapConfig::new("tap.example.com:8080");
        assert_eq!(config.ws_url(), "ws://tap.example.com:8080/channel");
    }

    #[test]
    fn test_http_base_url() {
        let config = TapConfig::new("localhost:2480");
        assert_eq!(config.http_base_url(), "http://localhost:2480");
    }
}
