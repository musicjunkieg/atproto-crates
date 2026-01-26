//! Cryptographic signature operations and utilities for AT Protocol records.
//!
//! This library provides comprehensive functionality for working with AT Protocol records,
//! including cryptographic signature creation and verification, AT-URI parsing, and
//! datetime serialization utilities. Built on IPLD DAG-CBOR for deterministic encoding
//! with support for P-256, P-384, and K-256 elliptic curve cryptography.
//!
//! ## Main Features
//!
//! - **Signature Operations**: Create and verify cryptographic signatures following the
//!   community.lexicon.attestation.signature specification
//! - **AT-URI Support**: Parse and validate AT Protocol URIs for record identification
//! - **DateTime Utilities**: RFC 3339 datetime serialization with millisecond precision
//! - **Type-Safe Errors**: Structured error types following project conventions
//!
//! ## Example Usage
//!
//! ```ignore
//! use atproto_record::attestation;
//! use atproto_identity::key::{identify_key, sign, to_public};
//! use base64::engine::general_purpose::STANDARD;
//! use serde_json::json;
//!
//! let private_key = identify_key("did:key:zPrivate...")?;
//! let public_key = to_public(&private_key)?;
//! let key_reference = format!("{}", &public_key);
//!
//! let record = json!({
//!     "$type": "app.example.record",
//!     "text": "Hello from attestation helpers!"
//! });
//!
//! let sig_metadata = json!({
//!     "$type": "com.example.inlineSignature",
//!     "key": &key_reference,
//!     "purpose": "demo"
//! });
//!
//! let signing_record = attestation::prepare_signing_record(&record, &sig_metadata)?;
//! let cid = attestation::create_cid(&signing_record)?;
//! let signature_bytes = sign(&private_key, &cid.to_bytes())?;
//!
//! let inline_attestation = json!({
//!     "$type": "com.example.inlineSignature",
//!    "key": key_reference,
//!     "purpose": "demo",
//!     "signature": {"$bytes": STANDARD.encode(signature_bytes)}
//! });
//!
//! let signed = attestation::create_inline_attestation_reference(&record, &inline_attestation)?;
//! let reports = tokio_test::block_on(async {
//!     attestation::verify_all_signatures(&signed, None).await
//! })?;
//! assert!(matches!(reports[0].status, attestation::VerificationStatus::Valid { .. }));
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Structured error types for record operations.
///
/// Comprehensive error handling for signature verification, AT-URI parsing,
/// and CLI operations. All errors follow the project's standardized format:
/// `error-atproto-record-{domain}-{number} {message}: {details}`
pub mod errors;

/// AT-URI parsing and validation.
///
/// Parse and validate AT Protocol URIs (at://authority/collection/record_key)
/// for identifying records within repositories. Supports both did:plc and
/// did:web authority types with comprehensive validation.
pub mod aturi;

/// DateTime serialization utilities.
///
/// RFC 3339 datetime serialization modules for consistent timestamp handling
/// across AT Protocol records. Includes support for both required and optional
/// datetime fields with millisecond precision.
pub mod datetime;

/// Byte array serialization utilities.
///
/// Provides specialized serialization and deserialization for byte arrays
/// in AT Protocol records, handling base64 encoding with the `$bytes` field format.
pub mod bytes;

/// AT Protocol lexicon type definitions.
///
/// Contains structured type definitions for various AT Protocol lexicons including
/// badges, calendar events, RSVPs, locations, and attestations. These types follow
/// the AT Protocol lexicon specifications for data interchange.
pub mod lexicon;

/// Generic wrapper for handling lexicon types with `$type` fields.
///
/// Provides a flexible way to handle the `$type` discriminator field that appears
/// in many AT Protocol lexicon structures. The wrapper can automatically add type
/// fields during serialization and validate them during deserialization.
pub mod typed;

/// Timestamp Identifier (TID) generation and parsing.
///
/// TIDs are sortable, distributed identifiers combining microsecond timestamps
/// with random clock identifiers. They provide a collision-resistant, monotonically
/// increasing identifier scheme for AT Protocol records encoded as 13-character
/// base32-sortable strings.
pub mod tid;
