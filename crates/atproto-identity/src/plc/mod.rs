//! PLC directory client and did:plc operations.
//!
//! Core functionality for did:plc resolution from PLC directory servers, plus
//! full did:plc specification support including operation creation, signing,
//! chain validation, and DID creation.
//!
//! ## Modules
//!
//! - [`did`] - Strongly-typed PLC DID identifiers with parsing and validation
//! - [`encoding`] - Base32, base64url, and SHA-256 encoding utilities
//! - [`state`] - PLC state management and DID document conversion
//! - [`operations`] - PLC operation types (genesis, update, tombstone) with signing
//! - [`builder`] - Fluent API for creating new did:plc identities
//! - [`chain`] - Operation chain validation with fork resolution

use tracing::{Instrument, instrument};

use super::errors::PLCDIDError;
use super::model::Document;

pub mod builder;
pub mod chain;
pub mod did;
pub mod encoding;
pub mod operations;
pub mod state;

pub use builder::{BuilderKeys, DidBuilder};
pub use chain::OperationChainValidator;
pub use did::Did;
pub use operations::{Operation, UnsignedOperation};
pub use state::{PlcState, ServiceEndpoint};

/// Queries a PLC directory for a DID document.
/// Fetches the complete DID document from the specified PLC hostname.
#[instrument(skip(http_client), err)]
pub async fn query(
    http_client: &reqwest::Client,
    plc_hostname: &str,
    did: &str,
) -> Result<Document, PLCDIDError> {
    let url = format!("https://{}/{}", plc_hostname, did);

    http_client
        .get(&url)
        .send()
        .instrument(tracing::info_span!("http_client_get"))
        .await
        .map_err(|error| PLCDIDError::HttpRequestFailed {
            url: url.clone(),
            error,
        })?
        .json::<Document>()
        .await
        .map_err(|error| PLCDIDError::DocumentParseFailed { url, error })
}
