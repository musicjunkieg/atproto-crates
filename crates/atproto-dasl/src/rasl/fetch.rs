//! RASL content fetching with parallel hint resolution and CID verification.
//!
//! This module provides the ability to fetch content-addressed resources
//! from RASL hints, with automatic CID verification of retrieved data.
//!
//! Requires the `reqwest` feature.

use crate::cid::{BLAKE3_CODE, CidCore, SHA256_CODE};
use crate::errors::RaslError;
use crate::rasl::{RaslUrl, WELL_KNOWN_PREFIX};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

/// Maximum number of redirects to follow per RASL fetch attempt.
const MAX_REDIRECTS: usize = 10;

/// Fetch the resource bytes from RASL hints, verifying the CID matches.
///
/// All hints are attempted in parallel. The first successful response
/// is used. After downloading, the CID is verified against the
/// received data.
///
/// Per the RASL spec, all HTTP redirects are treated as temporary (307).
/// Non-307 redirects are followed but not cached.
///
/// # Errors
///
/// Returns `RaslError::NoHints` if the URL has no hints.
/// Returns `RaslError::AllHintsFailed` if all hints fail.
/// Returns `RaslError::CidVerification` if the data doesn't match the CID.
pub async fn fetch_verified(url: &RaslUrl) -> Result<Vec<u8>, RaslError> {
    let client = build_rasl_client().map_err(|e| RaslError::HttpRequest {
        url: url.to_string(),
        reason: e.to_string(),
    })?;
    fetch_verified_with_client(url, &client).await
}

/// Build an HTTP client configured per the RASL spec.
///
/// The RASL spec requires treating all redirects as temporary (307):
/// follow them but never cache the redirect target.
fn build_rasl_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
        .build()
}

/// Fetch with a custom reqwest client.
///
/// See [`fetch_verified`] for details.
pub async fn fetch_verified_with_client(
    url: &RaslUrl,
    client: &reqwest::Client,
) -> Result<Vec<u8>, RaslError> {
    if url.hints().is_empty() {
        return Err(RaslError::NoHints);
    }

    let cid_core = *url.cid_core();
    let cid_str = url.cid().to_string();

    // Channel for first successful response
    let (tx, mut rx) = mpsc::channel::<Result<Vec<u8>, RaslError>>(1);

    for hint in url.hints() {
        let retrieval_url = format!("https://{}{}{}", hint, WELL_KNOWN_PREFIX, cid_str);
        let client = client.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            let result = attempt_fetch(&client, &retrieval_url).await;
            // Ignore send errors — receiver may have already gotten a result
            let _ = tx.send(result).await;
        });
    }

    // Drop our sender so the channel closes when all tasks complete
    drop(tx);

    // Collect first successful result or all errors
    let mut last_error = RaslError::AllHintsFailed;

    while let Some(result) = rx.recv().await {
        match result {
            Ok(data) => {
                verify_data(&cid_core, &data)?;
                return Ok(data);
            }
            Err(e) => {
                last_error = e;
            }
        }
    }

    Err(last_error)
}

/// Attempt to fetch data from a single URL.
///
/// Per the RASL spec: no cookies, credentials, or content negotiation.
/// The response media type is forced to `application/octet-stream`.
async fn attempt_fetch(client: &reqwest::Client, url: &str) -> Result<Vec<u8>, RaslError> {
    let response = client
        .get(url)
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|e| RaslError::HttpRequest {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    if !response.status().is_success() {
        return Err(RaslError::HttpRequest {
            url: url.to_string(),
            reason: format!("HTTP {}", response.status()),
        });
    }

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| RaslError::HttpRequest {
            url: url.to_string(),
            reason: e.to_string(),
        })
}

/// Verify that data matches the expected CID.
fn verify_data(cid: &CidCore, data: &[u8]) -> Result<(), RaslError> {
    let hash_code = cid.hash().code();
    let expected_digest = cid.hash().digest();

    let computed_digest = match hash_code {
        SHA256_CODE => {
            let hash = Sha256::digest(data);
            hash.to_vec()
        }
        BLAKE3_CODE => {
            let hash = blake3::hash(data);
            hash.as_bytes().to_vec()
        }
        _ => {
            return Err(RaslError::CidVerification {
                reason: format!("unsupported hash algorithm: 0x{:x}", hash_code),
            });
        }
    };

    if computed_digest.as_slice() != expected_digest {
        return Err(RaslError::CidVerification {
            reason: "data hash does not match CID digest".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cid::{Cid, compute_raw_cid};

    #[test]
    fn test_verify_data_sha256() {
        let data = b"hello world";
        let cid = compute_raw_cid(data);
        assert!(verify_data(&cid, data).is_ok());
    }

    #[test]
    fn test_verify_data_sha256_mismatch() {
        let data = b"hello world";
        let cid = compute_raw_cid(data);
        assert!(verify_data(&cid, b"wrong data").is_err());
    }

    #[test]
    fn test_verify_data_blake3() {
        let data = b"hello world";
        let cid = crate::cid::compute_raw_cid_blake3(data);
        assert!(verify_data(&cid, data).is_ok());
    }

    #[test]
    fn test_verify_data_blake3_mismatch() {
        let data = b"hello world";
        let cid = crate::cid::compute_raw_cid_blake3(data);
        assert!(verify_data(&cid, b"wrong data").is_err());
    }

    #[tokio::test]
    async fn test_fetch_no_hints() {
        let cid = Cid::new(compute_raw_cid(b"test"));
        let url = RaslUrl::new(cid);
        let result = fetch_verified(&url).await;
        assert!(matches!(result, Err(RaslError::NoHints)));
    }
}
