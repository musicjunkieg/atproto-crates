//! # Structured Error Types  
//!
//! Comprehensive error handling for AT Protocol identity operations using structured error types
//! with the `thiserror` library. All errors follow the project convention of prefixed error codes
//! with descriptive messages.
//!
//! ## Error Categories
//!
//! - **`WebDIDError`** (web-1 to web-4): Errors specific to `did:web` operations including URL conversion and document fetching
//! - **`ConfigError`** (config-1 to config-3): Configuration and environment variable related errors
//! - **`ResolveError`** (resolve-1 to resolve-8): Handle and DID resolution errors including DNS/HTTP failures and conflicts
//! - **`PLCDIDError`** (plc-1 to plc-2): PLC directory communication and document parsing errors
//! - **`KeyError`** (key-1 to key-12): Cryptographic key operations including generation, parsing, signing, and validation
//! - **`StorageError`** (storage-1 to storage-3): Storage operations including cache lock failures and data access errors
//!
//! ## Error Format
//!
//! All errors use the standardized format: `error-atproto-identity-{domain}-{number} {message}: {details}`

use thiserror::Error;

/// Error types that can occur when working with Web DIDs
#[derive(Debug, Error)]
pub enum WebDIDError {
    /// Occurs when the DID is missing the 'did:web:' prefix
    #[error("error-atproto-identity-web-1 Invalid DID format: missing 'did:web:' prefix")]
    InvalidDIDPrefix,

    /// Occurs when the DID is missing a hostname component
    #[error("error-atproto-identity-web-2 Invalid DID format: missing hostname component")]
    MissingHostname,

    /// Occurs when the HTTP request to fetch the DID document fails
    #[error("error-atproto-identity-web-3 HTTP request failed: {url} {error}")]
    HttpRequestFailed {
        /// The URL that was requested
        url: String,
        /// The underlying HTTP error
        error: reqwest::Error,
    },

    /// Occurs when the DID document cannot be parsed from the HTTP response
    #[error("error-atproto-identity-web-4 Failed to parse DID document: {url} {error}")]
    DocumentParseFailed {
        /// The URL that was requested
        url: String,
        /// The underlying parse error
        error: reqwest::Error,
    },
}

/// Error types that can occur when working with configuration
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Occurs when a required environment variable is not set
    #[error("error-atproto-identity-config-1 Required environment variable not found: {name}")]
    MissingEnvironmentVariable {
        /// The name of the missing environment variable
        name: String,
    },

    /// Occurs when parsing an IP address from nameserver configuration fails
    #[error("error-atproto-identity-config-2 Unable to parse nameserver IP: {value}")]
    InvalidNameserverIP {
        /// The invalid IP address value that could not be parsed
        value: String,
    },

    /// Occurs when version information cannot be determined
    #[error(
        "error-atproto-identity-config-3 Version information not available: GIT_HASH or CARGO_PKG_VERSION must be set"
    )]
    VersionNotAvailable,
}

/// Error types that can occur when resolving AT Protocol identities
#[derive(Debug, Error)]
pub enum ResolveError {
    /// Occurs when multiple different DIDs are found via DNS TXT record lookup
    #[error(
        "error-atproto-identity-resolve-1 Multiple DIDs resolved for handle: expected single DID"
    )]
    MultipleDIDsFound,

    /// Occurs when no DIDs are found via either DNS or HTTP resolution methods
    #[error(
        "error-atproto-identity-resolve-2 No DIDs resolved for handle: no resolution methods succeeded"
    )]
    NoDIDsFound,

    /// Occurs when DNS and HTTP resolution return different DIDs for the same handle
    #[error(
        "error-atproto-identity-resolve-3 Conflicting DIDs found for handle: DNS and HTTP resolution returned different results"
    )]
    ConflictingDIDsFound,

    /// Occurs when DNS TXT record lookup fails
    #[cfg(feature = "hickory-dns")]
    #[error("error-atproto-identity-resolve-4 DNS resolution failed: {error:?}")]
    DNSResolutionFailed {
        /// The underlying DNS resolution error
        error: hickory_resolver::ResolveError,
    },

    /// Occurs when DNS TXT record lookup fails (generic version for when hickory-dns is not enabled)
    #[cfg(not(feature = "hickory-dns"))]
    #[error("error-atproto-identity-resolve-4 DNS resolution failed")]
    DNSResolutionFailed,

    /// Occurs when HTTP request to .well-known/atproto-did endpoint fails
    #[error("error-atproto-identity-resolve-5 HTTP resolution failed: {error:?}")]
    HTTPResolutionFailed {
        /// The underlying HTTP error
        error: reqwest::Error,
    },

    /// Occurs when HTTP response from .well-known/atproto-did doesn't start with "did:"
    #[error(
        "error-atproto-identity-resolve-6 Invalid HTTP resolution response: expected DID format"
    )]
    InvalidHTTPResolutionResponse,

    /// Occurs when input cannot be parsed as a valid handle or DID
    #[error("error-atproto-identity-resolve-7 Invalid input format: expected valid handle or DID")]
    InvalidInput,

    /// Occurs when subject resolution results in a handle instead of expected DID
    #[error("error-atproto-identity-resolve-8 Subject resolved to handle instead of DID")]
    SubjectResolvedToHandle,
}

/// Error types that can occur when working with PLC DIDs
#[derive(Debug, Error)]
pub enum PLCDIDError {
    /// Occurs when the HTTP request to the PLC directory fails
    #[error("error-atproto-identity-plc-1 HTTP request failed: {url} {error}")]
    HttpRequestFailed {
        /// The URL that was requested
        url: String,
        /// The underlying HTTP error
        error: reqwest::Error,
    },

    /// Occurs when the DID document cannot be parsed from the PLC directory response
    #[error("error-atproto-identity-plc-2 Failed to parse DID document: {url} {error}")]
    DocumentParseFailed {
        /// The URL that was requested
        url: String,
        /// The underlying parse error
        error: reqwest::Error,
    },

    /// Occurs when a DID string cannot be parsed as a valid did:plc identifier
    #[error("error-atproto-identity-plc-3 Invalid DID format: {details}")]
    InvalidDidFormat {
        /// Details about the format violation
        details: String,
    },

    /// Occurs when base32 decoding fails
    #[error("error-atproto-identity-plc-4 Invalid base32 encoding: {details}")]
    InvalidBase32 {
        /// Details about the decoding failure
        details: String,
    },

    /// Occurs when base64url decoding fails
    #[error("error-atproto-identity-plc-5 Invalid base64url encoding: {details}")]
    InvalidBase64Url {
        /// Details about the decoding failure
        details: String,
    },

    /// Occurs when an operation exceeds the maximum allowed size
    #[error("error-atproto-identity-plc-6 Operation exceeds size limit: {size} bytes (max {max})")]
    OperationTooLarge {
        /// Actual size of the operation in bytes
        size: usize,
        /// Maximum allowed size in bytes
        max: usize,
    },

    /// Occurs when rotation keys fail validation
    #[error("error-atproto-identity-plc-7 Invalid rotation keys: {details}")]
    InvalidRotationKeys {
        /// Details about the validation failure
        details: String,
    },

    /// Occurs when verification methods fail validation
    #[error("error-atproto-identity-plc-8 Invalid verification methods: {details}")]
    InvalidVerificationMethods {
        /// Details about the validation failure
        details: String,
    },

    /// Occurs when a service endpoint fails validation
    #[error("error-atproto-identity-plc-9 Invalid service endpoint: {details}")]
    InvalidService {
        /// Details about the validation failure
        details: String,
    },

    /// Occurs when a field exceeds its maximum entry count
    #[error("error-atproto-identity-plc-10 Too many entries in {field}: max {max}, got {actual}")]
    TooManyEntries {
        /// The field that has too many entries
        field: String,
        /// Maximum allowed count
        max: usize,
        /// Actual count
        actual: usize,
    },

    /// Occurs when a duplicate value is found in a field
    #[error("error-atproto-identity-plc-11 Duplicate entry in {field}: {value}")]
    DuplicateEntry {
        /// The field containing the duplicate
        field: String,
        /// The duplicated value
        value: String,
    },

    /// Occurs when DAG-CBOR encoding fails
    #[error("error-atproto-identity-plc-12 DAG-CBOR encoding failed: {details}")]
    DagCborEncodeFailed {
        /// Details about the encoding failure
        details: String,
    },

    /// Occurs when DAG-CBOR decoding fails
    #[error("error-atproto-identity-plc-13 DAG-CBOR decoding failed: {details}")]
    DagCborDecodeFailed {
        /// Details about the decoding failure
        details: String,
    },

    /// Occurs when an operation's signature cannot be verified
    #[error("error-atproto-identity-plc-14 Signature verification failed")]
    SignatureVerificationFailed,

    /// Occurs when a CID is invalid or cannot be computed
    #[error("error-atproto-identity-plc-15 Invalid CID: {details}")]
    InvalidCid {
        /// Details about the CID error
        details: String,
    },

    /// Occurs when an operation has an unrecognized type
    #[error("error-atproto-identity-plc-16 Invalid operation type: {details}")]
    InvalidOperationType {
        /// Details about the invalid operation type
        details: String,
    },

    /// Occurs when a required field is missing from an operation
    #[error("error-atproto-identity-plc-17 Missing required field: {field}")]
    MissingField {
        /// The name of the missing field
        field: String,
    },

    /// Occurs when operation chain validation fails
    #[error("error-atproto-identity-plc-18 Chain validation failed: {details}")]
    ChainValidationFailed {
        /// Details about the validation failure
        details: String,
    },

    /// Occurs when an operation chain is empty
    #[error("error-atproto-identity-plc-19 Empty operation chain")]
    EmptyChain,

    /// Occurs when the first operation in a chain is not a genesis operation
    #[error("error-atproto-identity-plc-20 First operation must be genesis")]
    FirstOperationNotGenesis,

    /// Occurs when an operation references an invalid previous operation
    #[error("error-atproto-identity-plc-21 Invalid prev reference: {details}")]
    InvalidPrev {
        /// Details about the invalid reference
        details: String,
    },

    /// Occurs when fork resolution fails
    #[error("error-atproto-identity-plc-22 Fork resolution error: {details}")]
    ForkResolutionError {
        /// Details about the fork resolution failure
        details: String,
    },

    /// Occurs when an also-known-as URI is invalid
    #[error("error-atproto-identity-plc-23 Invalid also-known-as URI: {details}")]
    InvalidAlsoKnownAs {
        /// Details about the invalid URI
        details: String,
    },

    /// Occurs when a timestamp is invalid
    #[error("error-atproto-identity-plc-24 Invalid timestamp: {details}")]
    InvalidTimestamp {
        /// Details about the invalid timestamp
        details: String,
    },
}

/// Error types that can occur when working with cryptographic keys
#[derive(Debug, Error)]
pub enum KeyError {
    /// Occurs when multibase decoding of a key fails
    #[error("error-atproto-identity-key-1 Error decoding key: {error:?}")]
    DecodeError {
        /// The underlying multibase decode error
        error: multibase::Error,
    },

    /// Occurs when ECDSA signature parsing fails
    #[error("error-atproto-identity-key-2 Signature parsing failed: {error:?}")]
    SignatureError {
        /// The underlying signature parsing error
        error: ecdsa::signature::Error,
    },

    /// Occurs when P-256 key operations fail
    #[error("error-atproto-identity-key-3 P-256 key operation failed: {error:?}")]
    P256Error {
        /// The underlying P-256 key error
        error: p256::ecdsa::Error,
    },

    /// Occurs when P-384 key operations fail
    #[error("error-atproto-identity-key-4 P-384 key operation failed: {error:?}")]
    P384Error {
        /// The underlying P-384 key error
        error: p384::ecdsa::Error,
    },

    /// Occurs when K-256 key operations fail
    #[error("error-atproto-identity-key-5 K-256 key operation failed: {error:?}")]
    K256Error {
        /// The underlying K-256 key error
        error: k256::ecdsa::Error,
    },

    /// Occurs when ECDSA cryptographic operations fail
    #[error("error-atproto-identity-key-6 ECDSA operation failed: {error:?}")]
    ECDSAError {
        /// The underlying ECDSA error
        error: ecdsa::Error,
    },

    /// Occurs when secret key parsing or operations fail
    #[error("error-atproto-identity-key-7 Secret key operation failed: {error:?}")]
    SecretKeyError {
        /// The underlying secret key error
        error: ecdsa::elliptic_curve::Error,
    },

    /// Occurs when attempting to sign content with a public key instead of a private key
    #[error("error-atproto-identity-key-8 Private key required for signature")]
    PrivateKeyRequiredForSignature,

    /// Occurs when attempting to generate a public key directly
    #[error(
        "error-atproto-identity-key-9 Public key generation not supported: generate private key instead"
    )]
    PublicKeyGenerationNotSupported,

    /// Occurs when the decoded key data is too short to identify the key type
    #[error("error-atproto-identity-key-10 Unidentified key type: key data too short")]
    UnidentifiedKeyType,

    /// Occurs when the multibase key type prefix is not recognized
    #[error("error-atproto-identity-key-11 Invalid multibase key type: {prefix:?}")]
    InvalidMultibaseKeyType {
        /// The unrecognized key type prefix
        prefix: Vec<u8>,
    },

    /// Occurs when JWK format conversion fails for supported key types
    #[error("error-atproto-identity-key-12 JWK format conversion failed: {error}")]
    JWKConversionFailed {
        /// The underlying conversion error
        error: String,
    },
}

/// Error types that can occur when working with storage operations
#[derive(Debug, Error)]
pub enum StorageError {
    /// Occurs when cache lock acquisition fails during document retrieval operations
    #[error(
        "error-atproto-identity-storage-1 Cache lock acquisition failed for get operation: {details}"
    )]
    CacheLockFailedGet {
        /// Details about the lock failure
        details: String,
    },

    /// Occurs when cache lock acquisition fails during document storage operations
    #[error(
        "error-atproto-identity-storage-2 Cache lock acquisition failed for store operation: {details}"
    )]
    CacheLockFailedStore {
        /// Details about the lock failure
        details: String,
    },

    /// Occurs when cache lock acquisition fails during document deletion operations
    #[error(
        "error-atproto-identity-storage-3 Cache lock acquisition failed for delete operation: {details}"
    )]
    CacheLockFailedDelete {
        /// Details about the lock failure
        details: String,
    },
}
