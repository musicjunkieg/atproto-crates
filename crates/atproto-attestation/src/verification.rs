//! Signature verification for AT Protocol attestations.
//!
//! This module provides verification functions for AT Protocol record attestations.

use crate::cid::create_attestation_cid;
use crate::errors::AttestationError;
use crate::input::AnyInput;
use crate::utils::BASE64;
use atproto_identity::key::{KeyResolver, validate};
use atproto_record::lexicon::com::atproto::repo::STRONG_REF_NSID;
use base64::Engine;
use serde::Serialize;
use serde_json::{Map, Value};
use std::convert::TryInto;

/// Helper function to extract and validate signatures array from a record
fn extract_signatures(record_object: &Map<String, Value>) -> Result<Vec<Value>, AttestationError> {
    match record_object.get("signatures") {
        Some(value) => value
            .as_array()
            .ok_or(AttestationError::SignaturesFieldInvalid)
            .cloned(),
        None => Ok(vec![]),
    }
}

/// Verify all signatures in a record with flexible input types.
///
/// This is a high-level verification function that accepts records in multiple formats
/// (String, Json, or TypedLexicon) and verifies all signatures with custom resolvers.
///
/// # Arguments
///
/// * `verify_input` - The record to verify (as AnyInput: String, Json, or TypedLexicon)
/// * `repository` - The repository DID to validate against (prevents replay attacks)
/// * `key_resolver` - Resolver for looking up verification keys from DIDs
/// * `record_resolver` - Resolver for fetching remote attestation proof records
///
/// # Returns
///
/// Returns `Ok(())` if all signatures are valid, or an error if any verification fails.
///
/// # Errors
///
/// Returns an error if:
/// - The input is not a valid record object
/// - Any signature verification fails
/// - Key or record resolution fails
///
/// # Type Parameters
///
/// * `R` - The record type (must implement Serialize + LexiconType + PartialEq + Clone)
/// * `RR` - The record resolver type (must implement RecordResolver)
/// * `KR` - The key resolver type (must implement KeyResolver)
pub async fn verify_record<R, RR, KR>(
    verify_input: AnyInput<R>,
    repository: &str,
    key_resolver: KR,
    record_resolver: RR,
) -> Result<(), AttestationError>
where
    R: Serialize + Clone,
    RR: atproto_client::record_resolver::RecordResolver,
    KR: KeyResolver,
{
    let record_object: Map<String, Value> = verify_input
        .clone()
        .try_into()
        .map_err(|_| AttestationError::RecordMustBeObject)?;

    let signatures = extract_signatures(&record_object)?;

    if signatures.is_empty() {
        return Ok(());
    }

    for signature in signatures {
        let signature_refernce_type = signature
            .get("$type")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or(AttestationError::SigMetadataMissingType)?;

        let metadata = if signature_refernce_type == STRONG_REF_NSID {
            let aturi = signature
                .get("uri")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or(AttestationError::SignatureMissingField {
                    field: "uri".to_string(),
                })?;

            record_resolver
                .resolve::<serde_json::Value>(aturi)
                .await
                .map_err(|error| AttestationError::RemoteAttestationFetchFailed {
                    uri: aturi.to_string(),
                    error,
                })?
        } else {
            signature.clone()
        };

        let computed_cid = create_attestation_cid(
            verify_input.clone(),
            AnyInput::Serialize(metadata.clone()),
            repository,
        )?;

        if signature_refernce_type == STRONG_REF_NSID {
            let attestation_cid = metadata
                .get("cid")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or(AttestationError::SignatureMissingField {
                    field: "cid".to_string(),
                })?;

            if computed_cid.to_string() != attestation_cid {
                return Err(AttestationError::RemoteAttestationCidMismatch {
                    expected: attestation_cid.to_string(),
                    actual: computed_cid.to_string(),
                });
            }
            continue;
        }

        let key = metadata
            .get("key")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or(AttestationError::SignatureMissingField {
                field: "key".to_string(),
            })?;
        let key_data = key_resolver.resolve(key).await.map_err(|error| {
            AttestationError::KeyResolutionFailed {
                key: key.to_string(),
                error,
            }
        })?;

        let signature_bytes = metadata
            .get("signature")
            .and_then(Value::as_object)
            .and_then(|object| object.get("$bytes"))
            .and_then(Value::as_str)
            .ok_or(AttestationError::SignatureBytesFormatInvalid)?;

        let signature_bytes = BASE64
            .decode(signature_bytes)
            .map_err(|error| AttestationError::SignatureDecodingFailed { error })?;

        let computed_cid_bytes = computed_cid.to_bytes();

        validate(&key_data, &signature_bytes, &computed_cid_bytes)
            .map_err(|error| AttestationError::SignatureValidationFailed { error })?;
    }

    Ok(())
}
