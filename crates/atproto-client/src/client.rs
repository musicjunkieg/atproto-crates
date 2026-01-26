//! HTTP client operations with DPoP authentication support.
//!
//! Authenticated and unauthenticated HTTP requests for JSON APIs
//! with DPoP (Demonstration of Proof-of-Possession) support.

use crate::errors::{ClientError, DPoPError};
use anyhow::Result;
use atproto_identity::key::KeyData;
use atproto_oauth::dpop::{DpopRetry, request_dpop};
use bytes::Bytes;
use reqwest::header::HeaderMap;
use reqwest_chain::ChainMiddleware;
use reqwest_middleware::ClientBuilder;
use tracing::Instrument;

/// DPoP authentication credentials for authenticated HTTP requests.
///
/// Contains the private key for DPoP proof generation and OAuth access token
/// for Authorization header.
#[derive(Clone)]
pub struct DPoPAuth {
    /// Private key data for generating DPoP proof tokens
    pub dpop_private_key_data: KeyData,
    /// OAuth access token for the Authorization header
    pub oauth_access_token: String,
}

/// App password authentication credentials for authenticated HTTP requests.
///
/// Contains the JWT access token for Bearer token authentication.
#[derive(Clone)]
pub struct AppPasswordAuth {
    /// JWT access token for the Authorization header
    pub access_token: String,
}

/// Authentication method for AT Protocol XRPC requests.
///
/// Supports multiple authentication schemes including unauthenticated requests,
/// DPoP (Demonstration of Proof-of-Possession) tokens, and app password bearer tokens.
#[derive(Clone)]
pub enum Auth {
    /// No authentication - for public endpoints that don't require authentication
    None,
    /// DPoP authentication with proof-of-possession tokens and OAuth access token
    DPoP(DPoPAuth),
    /// App password authentication using JWT bearer tokens
    AppPassword(AppPasswordAuth),
}

/// Performs an unauthenticated HTTP GET request and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `url` - The URL to request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or `ClientError::JsonParseFailed` if JSON parsing fails.
pub async fn get_json(http_client: &reqwest::Client, url: &str) -> Result<serde_json::Value> {
    let empty = HeaderMap::default();
    get_json_with_headers(http_client, url, &empty).await
}

/// Performs an unauthenticated HTTP GET request with additional headers and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `url` - The URL to request
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or `ClientError::JsonParseFailed` if JSON parsing fails.
pub async fn get_json_with_headers(
    http_client: &reqwest::Client,
    url: &str,
    additional_headers: &HeaderMap,
) -> Result<serde_json::Value> {
    let http_response = http_client
        .get(url)
        .headers(additional_headers.clone())
        .send()
        .await
        .map_err(|error| ClientError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;

    let value = http_response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| ClientError::JsonParseFailed {
            url: url.to_string(),
            error,
        })?;

    Ok(value)
}

/// Performs an unauthenticated HTTP GET request and returns the response as bytes.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `url` - The URL to request
///
/// # Returns
///
/// The response body as bytes
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or an error if streaming the response bytes fails.
pub async fn get_bytes(http_client: &reqwest::Client, url: &str) -> Result<Bytes> {
    let empty = HeaderMap::default();
    get_bytes_with_headers(http_client, url, &empty).await
}

/// Performs an unauthenticated HTTP GET request with additional headers and returns the response as bytes.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `url` - The URL to request
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The response body as bytes
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or an error if streaming the response bytes fails.
pub async fn get_bytes_with_headers(
    http_client: &reqwest::Client,
    url: &str,
    additional_headers: &HeaderMap,
) -> Result<Bytes> {
    let http_response = http_client
        .get(url)
        .headers(additional_headers.clone())
        .send()
        .await
        .map_err(|error| ClientError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;
    Ok(http_response
        .bytes()
        .await
        .map_err(|error| ClientError::ByteStreamFailed {
            url: url.to_string(),
            error,
        })?)
}

/// Performs a DPoP-authenticated HTTP GET request and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `dpop_auth` - DPoP authentication credentials
/// * `url` - The URL to request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `DPoPError::ProofGenerationFailed` if DPoP proof generation fails,
/// `DPoPError::HttpRequestFailed` if the HTTP request fails,
/// or `DPoPError::JsonParseFailed` if JSON parsing fails.
pub async fn get_dpop_json(
    http_client: &reqwest::Client,
    dpop_auth: &DPoPAuth,
    url: &str,
) -> Result<serde_json::Value> {
    let empty = HeaderMap::default();
    get_dpop_json_with_headers(http_client, dpop_auth, url, &empty).await
}

/// Performs a DPoP-authenticated HTTP GET request with additional headers and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `dpop_auth` - DPoP authentication credentials
/// * `url` - The URL to request
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `DPoPError::ProofGenerationFailed` if DPoP proof generation fails,
/// `DPoPError::HttpRequestFailed` if the HTTP request fails,
/// or `DPoPError::JsonParseFailed` if JSON parsing fails.
pub async fn get_dpop_json_with_headers(
    http_client: &reqwest::Client,
    dpop_auth: &DPoPAuth,
    url: &str,
    additional_headers: &HeaderMap,
) -> Result<serde_json::Value> {
    let (dpop_proof_token, dpop_proof_header, dpop_proof_claim) = request_dpop(
        &dpop_auth.dpop_private_key_data,
        "GET",
        url,
        &dpop_auth.oauth_access_token,
    )
    .map_err(|error| DPoPError::ProofGenerationFailed { error })?;

    let dpop_retry = DpopRetry::new(
        dpop_proof_header.clone(),
        dpop_proof_claim.clone(),
        dpop_auth.dpop_private_key_data.clone(),
        true,
    );

    let dpop_retry_client = ClientBuilder::new(http_client.clone())
        .with(ChainMiddleware::new(dpop_retry.clone()))
        .build();

    let http_response = dpop_retry_client
        .get(url)
        .headers(additional_headers.clone())
        .header(
            "Authorization",
            &format!("DPoP {}", dpop_auth.oauth_access_token),
        )
        .header("DPoP", &dpop_proof_token)
        .send()
        .await
        .map_err(|error| DPoPError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;

    let value = http_response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| DPoPError::JsonParseFailed {
            url: url.to_string(),
            error,
        })?;

    Ok(value)
}

/// Performs a DPoP-authenticated HTTP POST request with JSON body and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `dpop_auth` - DPoP authentication credentials
/// * `url` - The URL to request
/// * `record` - The JSON data to send in the request body
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `DPoPError::ProofGenerationFailed` if DPoP proof generation fails,
/// `DPoPError::HttpRequestFailed` if the HTTP request fails,
/// or `DPoPError::JsonParseFailed` if JSON parsing fails.
pub async fn post_dpop_json(
    http_client: &reqwest::Client,
    dpop_auth: &DPoPAuth,
    url: &str,
    record: serde_json::Value,
) -> Result<serde_json::Value> {
    let empty = HeaderMap::default();
    post_dpop_json_with_headers(http_client, dpop_auth, url, record, &empty).await
}

/// Performs a DPoP-authenticated HTTP POST request with JSON body and additional headers, and parses the response as JSON.
///
/// This function extends `post_dpop_json` by allowing custom headers to be included
/// in the request. Useful for adding custom content types, user agents, or other
/// protocol-specific headers while maintaining DPoP authentication.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `dpop_auth` - DPoP authentication credentials
/// * `url` - The URL to request
/// * `record` - The JSON data to send in the request body
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `DPoPError::ProofGenerationFailed` if DPoP proof generation fails,
/// `DPoPError::HttpRequestFailed` if the HTTP request fails,
/// or `DPoPError::JsonParseFailed` if JSON parsing fails.
///
/// # Example
///
/// ```no_run
/// use atproto_client::client::{DPoPAuth, post_dpop_json_with_headers};
/// use atproto_identity::key::identify_key;
/// use reqwest::{Client, header::{HeaderMap, USER_AGENT}};
/// use serde_json::json;
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = Client::new();
/// let dpop_auth = DPoPAuth {
///     dpop_private_key_data: identify_key("did:key:zQ3sh...")?,
///     oauth_access_token: "access_token".to_string(),
/// };
///
/// let mut headers = HeaderMap::new();
/// headers.insert(USER_AGENT, "my-app/1.0".parse()?);
///
/// let response = post_dpop_json_with_headers(
///     &client,
///     &dpop_auth,
///     "https://pds.example.com/xrpc/com.atproto.repo.createRecord",
///     json!({"$type": "app.bsky.feed.post", "text": "Hello!"}),
///     &headers
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn post_dpop_json_with_headers(
    http_client: &reqwest::Client,
    dpop_auth: &DPoPAuth,
    url: &str,
    record: serde_json::Value,
    additional_headers: &HeaderMap,
) -> Result<serde_json::Value> {
    let (dpop_proof_token, dpop_proof_header, dpop_proof_claim) = request_dpop(
        &dpop_auth.dpop_private_key_data,
        "POST",
        url,
        &dpop_auth.oauth_access_token,
    )
    .map_err(|error| DPoPError::ProofGenerationFailed { error })?;

    let dpop_retry = DpopRetry::new(
        dpop_proof_header.clone(),
        dpop_proof_claim.clone(),
        dpop_auth.dpop_private_key_data.clone(),
        true,
    );

    let dpop_retry_client = ClientBuilder::new(http_client.clone())
        .with(ChainMiddleware::new(dpop_retry.clone()))
        .build();

    let http_response = dpop_retry_client
        .post(url)
        .headers(additional_headers.clone())
        .header(
            "Authorization",
            &format!("DPoP {}", dpop_auth.oauth_access_token),
        )
        .header("DPoP", &dpop_proof_token)
        .json(&record)
        .send()
        .await
        .map_err(|error| DPoPError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;

    let value = http_response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| DPoPError::JsonParseFailed {
            url: url.to_string(),
            error,
        })?;

    Ok(value)
}

/// Performs a DPoP-authenticated HTTP POST request with raw bytes body and additional headers, and parses the response as JSON.
///
/// This function is similar to `post_dpop_json_with_headers` but accepts a raw bytes payload
/// instead of JSON. Useful for sending pre-serialized data or binary payloads while maintaining
/// DPoP authentication and custom headers.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `dpop_auth` - DPoP authentication credentials
/// * `url` - The URL to request
/// * `payload` - The raw bytes to send in the request body
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `DPoPError::ProofGenerationFailed` if DPoP proof generation fails,
/// `DPoPError::HttpRequestFailed` if the HTTP request fails,
/// or `DPoPError::JsonParseFailed` if JSON parsing fails.
///
/// # Example
///
/// ```no_run
/// use atproto_client::client::{DPoPAuth, post_dpop_bytes_with_headers};
/// use atproto_identity::key::identify_key;
/// use reqwest::{Client, header::{HeaderMap, CONTENT_TYPE}};
/// use bytes::Bytes;
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = Client::new();
/// let dpop_auth = DPoPAuth {
///     dpop_private_key_data: identify_key("did:key:zQ3sh...")?,
///     oauth_access_token: "access_token".to_string(),
/// };
///
/// let mut headers = HeaderMap::new();
/// headers.insert(CONTENT_TYPE, "application/json".parse()?);
///
/// let payload = Bytes::from(r#"{"text": "Hello!"}"#);
/// let response = post_dpop_bytes_with_headers(
///     &client,
///     &dpop_auth,
///     "https://pds.example.com/xrpc/com.atproto.repo.createRecord",
///     payload,
///     &headers
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn post_dpop_bytes_with_headers(
    http_client: &reqwest::Client,
    dpop_auth: &DPoPAuth,
    url: &str,
    payload: Bytes,
    additional_headers: &HeaderMap,
) -> Result<serde_json::Value> {
    let (dpop_proof_token, dpop_proof_header, dpop_proof_claim) = request_dpop(
        &dpop_auth.dpop_private_key_data,
        "POST",
        url,
        &dpop_auth.oauth_access_token,
    )
    .map_err(|error| DPoPError::ProofGenerationFailed { error })?;

    let dpop_retry = DpopRetry::new(
        dpop_proof_header.clone(),
        dpop_proof_claim.clone(),
        dpop_auth.dpop_private_key_data.clone(),
        true,
    );

    let dpop_retry_client = ClientBuilder::new(http_client.clone())
        .with(ChainMiddleware::new(dpop_retry.clone()))
        .build();

    let http_response = dpop_retry_client
        .post(url)
        .headers(additional_headers.clone())
        .header(
            "Authorization",
            &format!("DPoP {}", dpop_auth.oauth_access_token),
        )
        .header("DPoP", &dpop_proof_token)
        .body(payload)
        .send()
        .await
        .map_err(|error| DPoPError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;

    let value = http_response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| DPoPError::JsonParseFailed {
            url: url.to_string(),
            error,
        })?;

    Ok(value)
}

/// Performs an unauthenticated HTTP POST request with JSON body and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `url` - The URL to request
/// * `data` - The JSON data to send in the request body
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or `ClientError::JsonParseFailed` if JSON parsing fails.
pub async fn post_json(
    http_client: &reqwest::Client,
    url: &str,
    data: serde_json::Value,
) -> Result<serde_json::Value> {
    let empty = HeaderMap::default();
    post_json_with_headers(http_client, url, data, &empty).await
}

/// Performs an unauthenticated HTTP POST request with JSON body and additional headers, and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `url` - The URL to request
/// * `data` - The JSON data to send in the request body
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or `ClientError::JsonParseFailed` if JSON parsing fails.
pub async fn post_json_with_headers(
    http_client: &reqwest::Client,
    url: &str,
    data: serde_json::Value,
    additional_headers: &HeaderMap,
) -> Result<serde_json::Value> {
    let http_response = http_client
        .post(url)
        .headers(additional_headers.clone())
        .json(&data)
        .send()
        .await
        .map_err(|error| ClientError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;

    let value = http_response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| ClientError::JsonParseFailed {
            url: url.to_string(),
            error,
        })?;

    Ok(value)
}

/// Performs an app password-authenticated HTTP GET request and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `app_auth` - App password authentication credentials
/// * `url` - The URL to request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or `ClientError::JsonParseFailed` if JSON parsing fails.
pub async fn get_apppassword_json(
    http_client: &reqwest::Client,
    app_auth: &AppPasswordAuth,
    url: &str,
) -> Result<serde_json::Value> {
    let empty = HeaderMap::default();
    get_apppassword_json_with_headers(http_client, app_auth, url, &empty).await
}

/// Performs an app password-authenticated HTTP GET request with additional headers and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `app_auth` - App password authentication credentials
/// * `url` - The URL to request
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or `ClientError::JsonParseFailed` if JSON parsing fails.
pub async fn get_apppassword_json_with_headers(
    http_client: &reqwest::Client,
    app_auth: &AppPasswordAuth,
    url: &str,
    additional_headers: &HeaderMap,
) -> Result<serde_json::Value> {
    let mut headers = additional_headers.clone();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", app_auth.access_token))?,
    );

    let http_response = http_client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|error| ClientError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;

    let value = http_response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| ClientError::JsonParseFailed {
            url: url.to_string(),
            error,
        })?;

    Ok(value)
}

/// Performs an app password-authenticated HTTP POST request with JSON body and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `app_auth` - App password authentication credentials
/// * `url` - The URL to request
/// * `data` - The JSON data to send in the request body
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or `ClientError::JsonParseFailed` if JSON parsing fails.
pub async fn post_apppassword_json(
    http_client: &reqwest::Client,
    app_auth: &AppPasswordAuth,
    url: &str,
    data: serde_json::Value,
) -> Result<serde_json::Value> {
    let empty = HeaderMap::default();
    post_apppassword_json_with_headers(http_client, app_auth, url, data, &empty).await
}

/// Performs an app password-authenticated HTTP POST request with JSON body and additional headers, and parses the response as JSON.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `app_auth` - App password authentication credentials
/// * `url` - The URL to request
/// * `data` - The JSON data to send in the request body
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The parsed JSON response as a `serde_json::Value`
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or `ClientError::JsonParseFailed` if JSON parsing fails.
pub async fn post_apppassword_json_with_headers(
    http_client: &reqwest::Client,
    app_auth: &AppPasswordAuth,
    url: &str,
    data: serde_json::Value,
    additional_headers: &HeaderMap,
) -> Result<serde_json::Value> {
    let mut headers = additional_headers.clone();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", app_auth.access_token))?,
    );

    let http_response = http_client
        .post(url)
        .headers(headers)
        .json(&data)
        .send()
        .await
        .map_err(|error| ClientError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;

    let value = http_response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| ClientError::JsonParseFailed {
            url: url.to_string(),
            error,
        })?;

    Ok(value)
}

/// Performs an app password-authenticated HTTP GET request and returns the response as bytes.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `app_auth` - App password authentication credentials
/// * `url` - The URL to request
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The response body as bytes
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or an error if streaming the response bytes fails.
pub async fn get_apppassword_bytes_with_headers(
    http_client: &reqwest::Client,
    app_auth: &AppPasswordAuth,
    url: &str,
    additional_headers: &HeaderMap,
) -> Result<Bytes> {
    let mut headers = additional_headers.clone();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", app_auth.access_token))?,
    );
    let http_response = http_client
        .get(url)
        .headers(headers)
        .send()
        .await
        .map_err(|error| ClientError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;
    Ok(http_response
        .bytes()
        .await
        .map_err(|error| ClientError::ByteStreamFailed {
            url: url.to_string(),
            error,
        })?)
}

/// Performs an app password-authenticated HTTP POST request with JSON body and returns the response as bytes.
///
/// This is useful when the server returns binary data such as images, CAR files,
/// or other non-JSON content in response to authenticated POST requests.
///
/// # Arguments
///
/// * `http_client` - The HTTP client to use for the request
/// * `app_auth` - App password authentication credentials
/// * `url` - The URL to request
/// * `record` - The JSON data to send in the request body
/// * `additional_headers` - Additional HTTP headers to include in the request
///
/// # Returns
///
/// The response body as bytes
///
/// # Errors
///
/// Returns `ClientError::HttpRequestFailed` if the HTTP request fails,
/// or an error if streaming the response bytes fails.
pub async fn post_apppassword_bytes_with_headers(
    http_client: &reqwest::Client,
    app_auth: &AppPasswordAuth,
    url: &str,
    payload: Bytes,
    additional_headers: &HeaderMap,
) -> Result<Bytes> {
    let mut headers = additional_headers.clone();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        reqwest::header::HeaderValue::from_str(&format!("Bearer {}", app_auth.access_token))?,
    );
    let http_response = http_client
        .post(url)
        .headers(headers)
        .body(payload)
        .send()
        .instrument(tracing::info_span!("post_apppassword_bytes_with_headers", url = %url))
        .await
        .map_err(|error| ClientError::HttpRequestFailed {
            url: url.to_string(),
            error,
        })?;
    Ok(http_response
        .bytes()
        .await
        .map_err(|error| ClientError::ByteStreamFailed {
            url: url.to_string(),
            error,
        })?)
}
