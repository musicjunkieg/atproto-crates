//! Error types for AT Protocol repository operations.
//!
//! This module defines structured error types following the project convention:
//! `error-atproto-repo-<domain>-<number> <message>: <details>`
//!
//! CAR, varint, and storage error types have been moved to the `atproto-dasl`
//! crate and are re-exported from the crate root.

use thiserror::Error;

/// Errors during MST operations.
#[derive(Debug, Error)]
pub enum MstError {
    /// Key is empty.
    #[error("error-atproto-repo-mst-1 Key cannot be empty")]
    EmptyKey,

    /// Key contains invalid characters.
    #[error("error-atproto-repo-mst-2 Invalid key character at position {position}")]
    InvalidKeyCharacter {
        /// Position of the invalid character in the key.
        position: usize,
    },

    /// MST node structure is invalid.
    #[error("error-atproto-repo-mst-3 Invalid MST node: {reason}")]
    InvalidNode {
        /// Description of why the node is invalid.
        reason: String,
    },

    /// Tree entry prefix is invalid.
    #[error("error-atproto-repo-mst-4 Invalid tree entry prefix: {reason}")]
    InvalidPrefix {
        /// Description of why the prefix is invalid.
        reason: String,
    },

    /// Referenced node not found.
    #[error("error-atproto-repo-mst-5 Node not found: {cid}")]
    NodeNotFound {
        /// CID of the missing node.
        cid: String,
    },

    /// DAG-CBOR serialization error.
    #[error("error-atproto-repo-mst-6 Serialization error: {0}")]
    Serialization(#[from] atproto_dasl::EncodeError),

    /// DAG-CBOR deserialization error.
    #[error("error-atproto-repo-mst-7 Deserialization error: {0}")]
    Deserialization(#[from] atproto_dasl::DecodeError),

    /// Key height calculation overflow.
    #[error("error-atproto-repo-mst-8 Key height overflow")]
    HeightOverflow,

    /// Duplicate key in tree.
    #[error("error-atproto-repo-mst-9 Duplicate key: {key}")]
    DuplicateKey {
        /// The duplicate key that was encountered.
        key: String,
    },

    /// Tree is not properly balanced.
    #[error("error-atproto-repo-mst-10 Tree structure violation: {reason}")]
    StructureViolation {
        /// Description of the structure violation.
        reason: String,
    },

    /// Storage operation failed.
    #[error("error-atproto-repo-mst-11 Storage error: {0}")]
    Storage(#[from] atproto_dasl::StorageError),
}

/// Errors during repository operations.
#[derive(Debug, Error)]
pub enum RepoError {
    /// Commit object is malformed.
    #[error("error-atproto-repo-repo-1 Invalid commit: {reason}")]
    InvalidCommit {
        /// Description of why the commit is invalid.
        reason: String,
    },

    /// Commit signature is invalid.
    #[error("error-atproto-repo-repo-2 Invalid commit signature")]
    InvalidSignature,

    /// Commit version is not supported.
    #[error("error-atproto-repo-repo-3 Unsupported commit version: {version}")]
    UnsupportedCommitVersion {
        /// The unsupported version number.
        version: u64,
    },

    /// Record not found at path.
    #[error("error-atproto-repo-repo-4 Record not found: {collection}/{rkey}")]
    RecordNotFound {
        /// Collection NSID.
        collection: String,
        /// Record key.
        rkey: String,
    },

    /// CAR file error.
    #[error("error-atproto-repo-repo-5 CAR error: {0}")]
    Car(#[from] atproto_dasl::CarError),

    /// MST error.
    #[error("error-atproto-repo-repo-6 MST error: {0}")]
    Mst(#[from] MstError),

    /// Invalid DID in commit.
    #[error("error-atproto-repo-repo-7 Invalid DID: {did}")]
    InvalidDid {
        /// The invalid DID string.
        did: String,
    },

    /// Missing required field in commit.
    #[error("error-atproto-repo-repo-8 Missing commit field: {field}")]
    MissingCommitField {
        /// Name of the missing field.
        field: String,
    },

    /// Identity resolution error.
    #[error("error-atproto-repo-repo-9 Identity error: {0}")]
    Identity(String),

    /// Storage operation failed.
    #[error("error-atproto-repo-repo-10 Storage error: {0}")]
    Storage(#[from] atproto_dasl::StorageError),
}
