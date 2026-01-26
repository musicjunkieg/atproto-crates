//! Web DID client for did:web resolution.
//!
//! Resolves did:web identifiers by converting them to HTTPS URLs and fetching
//! DID documents from well-known locations on web servers.
//! - **`query_hostname()`**: Direct document retrieval from hostname well-known endpoints
//!
//! ## URL Conversion
//!
//! Transforms DIDs like `did:web:example.com:path:subpath` into HTTPS URLs following
//! the did:web specification for well-known document locations.

use tracing::instrument;

use super::errors::WebDIDError;
use super::model::Document;

/// Converts a did:web DID to its corresponding HTTPS URL.
/// Transforms DID format to the expected well-known document location.
pub fn did_web_to_url(did: &str) -> Result<String, WebDIDError> {
    let parts = did
        .strip_prefix("did:web:")
        .ok_or(WebDIDError::InvalidDIDPrefix)?
        .split(':')
        .collect::<Vec<&str>>();

    let hostname = parts.first().ok_or(WebDIDError::MissingHostname)?;
    if hostname.is_empty() {
        return Err(WebDIDError::MissingHostname);
    }
    let path_parts = &parts[1..];

    let url = if path_parts.is_empty() {
        format!("https://{}/.well-known/did.json", hostname)
    } else {
        format!("https://{}/{}/did.json", hostname, path_parts.join("/"))
    };

    Ok(url)
}

/// Queries a did:web DID document from its hosting location.
/// Resolves the DID to HTTPS URL and fetches the JSON document.
#[instrument(skip(http_client), err)]
pub async fn query(http_client: &reqwest::Client, did: &str) -> Result<Document, WebDIDError> {
    let url = did_web_to_url(did)?;

    http_client
        .get(&url)
        .send()
        .await
        .map_err(|error| WebDIDError::HttpRequestFailed {
            url: url.clone(),
            error,
        })?
        .json::<Document>()
        .await
        .map_err(|error| WebDIDError::DocumentParseFailed { url, error })
}

/// Queries a DID document directly from a hostname's well-known location.
/// Fetches from https://{hostname}/.well-known/did.json
#[instrument(skip(http_client), err)]
pub async fn query_hostname(
    http_client: &reqwest::Client,
    hostname: &str,
) -> Result<Document, WebDIDError> {
    let url = format!("https://{}/.well-known/did.json", hostname);

    http_client
        .get(&url)
        .send()
        .await
        .map_err(|error| WebDIDError::HttpRequestFailed {
            url: url.clone(),
            error,
        })?
        .json::<Document>()
        .await
        .map_err(|error| WebDIDError::DocumentParseFailed { url, error })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_did_web_to_url_simple_hostname() {
        let result = did_web_to_url("did:web:example.com");
        assert_eq!(result.unwrap(), "https://example.com/.well-known/did.json");
    }

    #[test]
    fn test_did_web_to_url_with_path() {
        let result = did_web_to_url("did:web:example.com:path");
        assert_eq!(result.unwrap(), "https://example.com/path/did.json");
    }

    #[test]
    fn test_did_web_to_url_with_nested_path() {
        let result = did_web_to_url("did:web:example.com:path:subpath");
        assert_eq!(result.unwrap(), "https://example.com/path/subpath/did.json");
    }

    #[test]
    fn test_did_web_to_url_invalid_prefix() {
        let result = did_web_to_url("did:plc:example.com");
        assert!(matches!(result, Err(WebDIDError::InvalidDIDPrefix)));
    }

    #[test]
    fn test_did_web_to_url_missing_hostname() {
        let result = did_web_to_url("did:web:");
        assert!(matches!(result, Err(WebDIDError::MissingHostname)));
    }

    #[test]
    fn test_did_web_to_url_no_prefix() {
        let result = did_web_to_url("example.com");
        assert!(matches!(result, Err(WebDIDError::InvalidDIDPrefix)));
    }
}
