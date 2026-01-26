//! HTTP client for TAP management API.
//!
//! This module provides [`TapClient`] for interacting with the TAP service's
//! HTTP management endpoints, including adding/removing tracked repositories.

use crate::errors::TapError;
use atproto_identity::model::Document;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

/// HTTP client for TAP management API.
///
/// Provides methods for managing which repositories the TAP service tracks,
/// checking service health, and querying repository status.
///
/// # Example
///
/// ```ignore
/// use atproto_tap::TapClient;
///
/// let client = TapClient::new("localhost:2480", Some("admin_password".to_string()));
///
/// // Add repositories to track
/// client.add_repos(&["did:plc:xyz123", "did:plc:abc456"]).await?;
///
/// // Check health
/// if client.health().await? {
///     println!("TAP service is healthy");
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TapClient {
    http_client: reqwest::Client,
    base_url: String,
    auth_header: Option<HeaderValue>,
}

impl TapClient {
    /// Create a new TAP management client.
    ///
    /// # Arguments
    ///
    /// * `hostname` - TAP service hostname (e.g., "localhost:2480")
    /// * `admin_password` - Optional admin password for authentication
    pub fn new(hostname: &str, admin_password: Option<String>) -> Self {
        let auth_header = admin_password.map(|password| {
            let credentials = format!("admin:{}", password);
            let encoded = BASE64.encode(credentials.as_bytes());
            HeaderValue::from_str(&format!("Basic {}", encoded)).expect("Invalid auth header value")
        });

        Self {
            http_client: reqwest::Client::new(),
            base_url: format!("http://{}", hostname),
            auth_header,
        }
    }

    /// Create default headers for requests.
    fn default_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(auth) = &self.auth_header {
            headers.insert(AUTHORIZATION, auth.clone());
        }
        headers
    }

    /// Add repositories to track.
    ///
    /// Sends a POST request to `/repos/add` with the list of DIDs.
    ///
    /// # Arguments
    ///
    /// * `dids` - Slice of DID strings to track
    ///
    /// # Example
    ///
    /// ```ignore
    /// client.add_repos(&[
    ///     "did:plc:z72i7hdynmk6r22z27h6tvur",
    ///     "did:plc:ewvi7nxzyoun6zhxrhs64oiz",
    /// ]).await?;
    /// ```
    pub async fn add_repos(&self, dids: &[&str]) -> Result<(), TapError> {
        let url = format!("{}/repos/add", self.base_url);
        let body = AddReposRequest {
            dids: dids.iter().map(|s| s.to_string()).collect(),
        };

        let response = self
            .http_client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            tracing::debug!(count = dids.len(), "Added repositories to TAP");
            Ok(())
        } else {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            Err(TapError::HttpResponseError { status, message })
        }
    }

    /// Remove repositories from tracking.
    ///
    /// Sends a POST request to `/repos/remove` with the list of DIDs.
    ///
    /// # Arguments
    ///
    /// * `dids` - Slice of DID strings to stop tracking
    pub async fn remove_repos(&self, dids: &[&str]) -> Result<(), TapError> {
        let url = format!("{}/repos/remove", self.base_url);
        let body = AddReposRequest {
            dids: dids.iter().map(|s| s.to_string()).collect(),
        };

        let response = self
            .http_client
            .post(&url)
            .headers(self.default_headers())
            .json(&body)
            .send()
            .await?;

        if response.status().is_success() {
            tracing::debug!(count = dids.len(), "Removed repositories from TAP");
            Ok(())
        } else {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            Err(TapError::HttpResponseError { status, message })
        }
    }

    /// Check service health.
    ///
    /// Sends a GET request to `/health`.
    ///
    /// # Returns
    ///
    /// `true` if the service is healthy, `false` otherwise.
    pub async fn health(&self) -> Result<bool, TapError> {
        let url = format!("{}/health", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .headers(self.default_headers())
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Resolve a DID to its DID document.
    ///
    /// Sends a GET request to `/resolve/:did`.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID to resolve
    ///
    /// # Returns
    ///
    /// The DID document for the identity.
    pub async fn resolve(&self, did: &str) -> Result<Document, TapError> {
        let url = format!("{}/resolve/{}", self.base_url, did);

        let response = self
            .http_client
            .get(&url)
            .headers(self.default_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let doc: Document = response.json().await?;
            Ok(doc)
        } else {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            Err(TapError::HttpResponseError { status, message })
        }
    }

    /// Get info about a tracked repository.
    ///
    /// Sends a GET request to `/info/:did`.
    ///
    /// # Arguments
    ///
    /// * `did` - The DID to get info for
    ///
    /// # Returns
    ///
    /// Repository tracking information.
    pub async fn info(&self, did: &str) -> Result<RepoInfo, TapError> {
        let url = format!("{}/info/{}", self.base_url, did);

        let response = self
            .http_client
            .get(&url)
            .headers(self.default_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let info: RepoInfo = response.json().await?;
            Ok(info)
        } else {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            Err(TapError::HttpResponseError { status, message })
        }
    }
}

/// Request body for adding/removing repositories.
#[derive(Debug, Serialize)]
struct AddReposRequest {
    dids: Vec<String>,
}

/// Repository tracking information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    /// The repository DID.
    pub did: Box<str>,
    /// Current sync state.
    pub state: RepoState,
    /// The handle for the repository.
    #[serde(default)]
    pub handle: Option<Box<str>>,
    /// Number of records in the repository.
    #[serde(default)]
    pub records: u64,
    /// Current repository revision.
    #[serde(default)]
    pub rev: Option<Box<str>>,
    /// Number of retries for syncing.
    #[serde(default)]
    pub retries: u32,
    /// Error message if any.
    #[serde(default)]
    pub error: Option<Box<str>>,
    /// Additional fields may be present depending on TAP version.
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Repository sync state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoState {
    /// Repository is active and synced.
    Active,
    /// Repository is currently syncing.
    Syncing,
    /// Repository is fully synced.
    Synced,
    /// Sync failed for this repository.
    Failed,
    /// Repository is queued for sync.
    Queued,
    /// Unknown state.
    #[serde(other)]
    Unknown,
}

/// Deprecated alias for RepoState.
#[deprecated(since = "0.13.0", note = "Use RepoState instead")]
pub type RepoStatus = RepoState;

impl std::fmt::Display for RepoState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoState::Active => write!(f, "active"),
            RepoState::Syncing => write!(f, "syncing"),
            RepoState::Synced => write!(f, "synced"),
            RepoState::Failed => write!(f, "failed"),
            RepoState::Queued => write!(f, "queued"),
            RepoState::Unknown => write!(f, "unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TapClient::new("localhost:2480", None);
        assert_eq!(client.base_url, "http://localhost:2480");
        assert!(client.auth_header.is_none());

        let client = TapClient::new("localhost:2480", Some("secret".to_string()));
        assert!(client.auth_header.is_some());
    }

    #[test]
    fn test_repo_state_display() {
        assert_eq!(RepoState::Active.to_string(), "active");
        assert_eq!(RepoState::Syncing.to_string(), "syncing");
        assert_eq!(RepoState::Synced.to_string(), "synced");
        assert_eq!(RepoState::Failed.to_string(), "failed");
        assert_eq!(RepoState::Queued.to_string(), "queued");
        assert_eq!(RepoState::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_repo_state_deserialize() {
        let json = r#""active""#;
        let state: RepoState = serde_json::from_str(json).unwrap();
        assert_eq!(state, RepoState::Active);

        let json = r#""syncing""#;
        let state: RepoState = serde_json::from_str(json).unwrap();
        assert_eq!(state, RepoState::Syncing);

        let json = r#""some_new_state""#;
        let state: RepoState = serde_json::from_str(json).unwrap();
        assert_eq!(state, RepoState::Unknown);
    }

    #[test]
    fn test_repo_info_deserialize() {
        let json = r#"{"did":"did:plc:cbkjy5n7bk3ax2wplmtjofq2","error":"","handle":"ngerakines.me","records":21382,"retries":0,"rev":"3mam4aazabs2m","state":"active"}"#;
        let info: RepoInfo = serde_json::from_str(json).unwrap();
        assert_eq!(&*info.did, "did:plc:cbkjy5n7bk3ax2wplmtjofq2");
        assert_eq!(info.state, RepoState::Active);
        assert_eq!(info.handle.as_deref(), Some("ngerakines.me"));
        assert_eq!(info.records, 21382);
        assert_eq!(info.retries, 0);
        assert_eq!(info.rev.as_deref(), Some("3mam4aazabs2m"));
        // Empty string deserializes as Some("")
        assert_eq!(info.error.as_deref(), Some(""));
    }

    #[test]
    fn test_repo_info_deserialize_minimal() {
        // Test with only required fields
        let json = r#"{"did":"did:plc:test","state":"syncing"}"#;
        let info: RepoInfo = serde_json::from_str(json).unwrap();
        assert_eq!(&*info.did, "did:plc:test");
        assert_eq!(info.state, RepoState::Syncing);
        assert_eq!(info.handle, None);
        assert_eq!(info.records, 0);
        assert_eq!(info.retries, 0);
        assert_eq!(info.rev, None);
        assert_eq!(info.error, None);
    }

    #[test]
    fn test_add_repos_request_serialize() {
        let req = AddReposRequest {
            dids: vec!["did:plc:xyz".to_string(), "did:plc:abc".to_string()],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("dids"));
        assert!(json.contains("did:plc:xyz"));
    }
}
