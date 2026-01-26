//! Helpers for resolving AT Protocol records referenced by URI.

use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use async_trait::async_trait;
use atproto_identity::traits::IdentityResolver;
use atproto_record::aturi::ATURI;

use crate::{
    client::Auth,
    com::atproto::repo::{GetRecordResponse, get_record},
};

/// Trait for resolving AT Protocol records by `at://` URI.
///
/// Implementations perform the network lookup and deserialize the response into
/// the requested type.
#[async_trait]
pub trait RecordResolver: Send + Sync {
    /// Resolve an AT URI to a typed record.
    async fn resolve<T>(&self, aturi: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned + Send;
}

/// Resolver that fetches records using public XRPC endpoints.
///
/// Uses an identity resolver to dynamically determine the PDS endpoint for each record.
#[derive(Clone)]
pub struct HttpRecordResolver {
    http_client: reqwest::Client,
    identity_resolver: Arc<dyn IdentityResolver>,
}

impl HttpRecordResolver {
    /// Create a new resolver using the provided HTTP client and identity resolver.
    ///
    /// The identity resolver is used to dynamically determine the PDS endpoint for each record
    /// based on the authority (DID or handle) in the AT URI.
    pub fn new(http_client: reqwest::Client, identity_resolver: Arc<dyn IdentityResolver>) -> Self {
        Self {
            http_client,
            identity_resolver,
        }
    }
}

#[async_trait]
impl RecordResolver for HttpRecordResolver {
    async fn resolve<T>(&self, aturi: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned + Send,
    {
        let parsed = ATURI::from_str(aturi).map_err(|error| anyhow!(error))?;

        // Resolve the authority (DID or handle) to get the DID document
        let document = self
            .identity_resolver
            .resolve(&parsed.authority)
            .await
            .map_err(|error| {
                anyhow!(
                    "Failed to resolve identity for {}: {}",
                    parsed.authority,
                    error
                )
            })?;

        // Extract PDS endpoint from the DID document
        let pds_endpoints = document.pds_endpoints();
        let base_url = pds_endpoints
            .first()
            .ok_or_else(|| anyhow!("No PDS endpoint found for {}", parsed.authority))?;

        let auth = Auth::None;

        let response = get_record(
            &self.http_client,
            &auth,
            base_url,
            &parsed.authority,
            &parsed.collection,
            &parsed.record_key,
            None,
        )
        .await?;

        match response {
            GetRecordResponse::Record { value, .. } => {
                serde_json::from_value(value).map_err(|error| anyhow!(error))
            }
            GetRecordResponse::Error(error) => {
                let message = error.error_message();
                if message.is_empty() {
                    bail!("Record resolution failed without additional error details");
                }

                bail!(message);
            }
        }
    }
}
