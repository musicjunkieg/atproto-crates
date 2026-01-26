//! AT Protocol record attestation utilities based on the CID-first specification.
//!
//! This crate implements helpers for creating inline and remote attestations
//! and verifying signatures against DID verification methods. It follows the
//! requirements documented in `bluesky-attestation-tee/documentation/spec/attestation.md`.
//!
//! ## Inline Attestations
//!
//! Use `create_inline_attestation` to create a signed record with an embedded signature:
//!
//! ```no_run
//! use atproto_attestation::{create_inline_attestation, AnyInput};
//! use atproto_identity::key::{generate_key, KeyType};
//! use serde_json::json;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let key = generate_key(KeyType::P256Private)?;
//! let record = json!({"$type": "app.example.post", "text": "Hello!"});
//! let metadata = json!({"$type": "com.example.sig", "key": "did:key:..."});
//!
//! let signed = create_inline_attestation(
//!     AnyInput::Serialize(record),
//!     AnyInput::Serialize(metadata),
//!     "did:plc:repository",
//!     &key
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Remote Attestations
//!
//! Use `create_remote_attestation` to generate both the proof record and the
//! attested record with strongRef in a single call.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

// Public modules
pub mod cid;
pub mod errors;
pub mod input;

// Internal modules
mod attestation;
mod signature;
mod utils;
mod verification;

// Re-export error type
pub use errors::AttestationError;

// Re-export CID generation functions
pub use cid::create_dagbor_cid;

// Re-export signature normalization
pub use signature::normalize_signature;

// Re-export attestation functions
pub use attestation::{
    append_inline_attestation, append_remote_attestation, create_inline_attestation,
    create_remote_attestation, create_signature,
};

// Re-export input types
pub use input::{AnyInput, AnyInputError};

// Re-export verification functions
pub use verification::verify_record;

/// Resolver trait for retrieving remote attestation records by AT URI.
///
/// This trait is re-exported from atproto_client for convenience.
pub use atproto_client::record_resolver::RecordResolver;
