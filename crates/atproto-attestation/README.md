# atproto-attestation

Utilities for creating and verifying AT Protocol record attestations using the CID-first workflow.

## Overview

A Rust library implementing the CID-first attestation specification for AT Protocol records. This crate provides cryptographic signature creation and verification for records, supporting both inline attestations (signatures embedded directly in records) and remote attestations (separate proof records with strongRef references).

The attestation workflow ensures deterministic signing payloads and prevents replay attacks by:
1. Automatically preparing records with `$sig` metadata containing `$type` and `repository` fields
2. Generating content identifiers (CIDs) using DAG-CBOR serialization
3. Signing CID bytes with elliptic curve cryptography (for inline attestations)
4. Normalizing signatures to low-S form to prevent malleability attacks
5. Embedding signatures or creating proof records with strongRef references

**Critical Security Feature**: The `repository` field in `$sig` metadata binds attestations to specific repositories, preventing replay attacks where an attacker might attempt to clone records from one repository into their own.

## Features

- **Inline attestations**: Embed cryptographic signatures directly in record structures
- **Remote attestations**: Create separate proof records with CID-based strongRef references
- **CID-first workflow**: Deterministic signing based on content identifiers
- **Multi-curve support**: Full support for P-256, P-384, and K-256 elliptic curves
- **Signature normalization**: Automatic low-S normalization for ECDSA signatures to prevent malleability
- **Flexible input types**: Accept records as JSON strings, JSON values, or typed lexicons
- **Repository binding**: Automatic prevention of replay attacks

## CLI Tools

The following command-line tools are available when built with the `clap` and `tokio` features:

- **`atproto-attestation-sign`**: Sign AT Protocol records with inline or remote attestations
- **`atproto-attestation-verify`**: Verify cryptographic signatures on AT Protocol records

## Library Usage

### Creating Inline Attestations

Inline attestations embed the signature bytes directly in the record:

```rust
use atproto_identity::key::{generate_key, to_public, KeyType};
use atproto_attestation::{create_inline_attestation, input::{AnyInput, PhantomSignature}};
use serde_json::json;

fn main() -> anyhow::Result<()> {
    // Generate a signing key
    let private_key = generate_key(KeyType::K256Private)?;
    let public_key = to_public(&private_key)?;
    let key_reference = format!("{}", &public_key);

    // The record to sign
    let record = json!({
        "$type": "app.bsky.feed.post",
        "text": "Hello world!",
        "createdAt": "2024-01-01T00:00:00.000Z"
    });

    // Repository housing this record (for replay attack prevention)
    let repository_did = "did:plc:repo123";

    // Attestation metadata (required: $type and key for inline attestations)
    // Note: repository field is automatically added during CID generation
    let sig_metadata = json!({
        "$type": "com.example.inlineSignature",
        "key": &key_reference,
        "issuer": "did:plc:issuer123",
        "issuedAt": "2024-01-01T00:00:00.000Z"
    });

    // Create inline attestation (repository_did is bound into the CID)
    // Signature is automatically normalized to low-S form
    let signed_record = create_inline_attestation::<PhantomSignature, PhantomSignature>(
        AnyInput::Json(record),
        AnyInput::Json(sig_metadata),
        repository_did,
        &private_key
    )?;

    println!("{}", serde_json::to_string_pretty(&signed_record)?);

    Ok(())
}
```

The resulting record will have a `signatures` array:

```json
{
  "$type": "app.bsky.feed.post",
  "text": "Hello world!",
  "createdAt": "2024-01-01T00:00:00.000Z",
  "signatures": [
    {
      "$type": "com.example.inlineSignature",
      "key": "did:key:zQ3sh...",
      "issuer": "did:plc:issuer123",
      "issuedAt": "2024-01-01T00:00:00.000Z",
      "cid": "bafyrei...",
      "signature": {
        "$bytes": "base64-encoded-normalized-signature-bytes"
      }
    }
  ]
}
```

### Creating Remote Attestations

Remote attestations create a separate proof record that must be stored in a repository:

```rust
use atproto_attestation::{create_remote_attestation, input::{AnyInput, PhantomSignature}};
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let record = json!({
        "$type": "app.bsky.feed.post",
        "text": "Hello world!"
    });

    // Repository housing the original record (for replay attack prevention)
    let repository_did = "did:plc:repo123";

    // DID of the entity creating the attestation (will store the proof record)
    let attestor_did = "did:plc:attestor456";

    let metadata = json!({
        "$type": "com.example.attestation",
        "issuer": "did:plc:issuer123",
        "purpose": "verification"
    });

    // Create both the attested record and proof record in one call
    // Returns: (attested_record_with_strongRef, proof_record)
    let (attested_record, proof_record) = create_remote_attestation::<PhantomSignature, PhantomSignature>(
        AnyInput::Json(record),
        AnyInput::Json(metadata),
        repository_did,  // Repository housing the original record
        attestor_did     // Repository that will store the proof record
    )?;

    // The proof_record should be stored in the attestor's repository
    // The attested_record contains the strongRef reference
    println!("Proof record:\n{}", serde_json::to_string_pretty(&proof_record)?);
    println!("Attested record:\n{}", serde_json::to_string_pretty(&attested_record)?);

    Ok(())
}
```

### Verifying Signatures

Verify all signatures in a record:

```rust
use atproto_attestation::{verify_record, input::AnyInput};
use atproto_identity::key::IdentityDocumentKeyResolver;
use atproto_client::record_resolver::HttpRecordResolver;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Signed record with signatures array
    let signed_record = /* ... */;

    // The repository DID where this record is stored
    // CRITICAL: This must match the repository used during signing to prevent replay attacks
    let repository_did = "did:plc:repo123";

    // Create resolvers for key and record fetching
    let key_resolver = /* ... */;  // IdentityDocumentKeyResolver
    let record_resolver = HttpRecordResolver::new(/* ... */);

    // Verify all signatures with repository validation
    verify_record(
        AnyInput::Json(signed_record),
        repository_did,
        key_resolver,
        record_resolver
    ).await?;

    println!("✓ All signatures verified successfully");

    Ok(())
}
```

## Command Line Usage

### Signing Records

#### Inline Attestation

```bash
# Sign with inline attestation (signature embedded in record)
cargo run --package atproto-attestation --features clap,tokio --bin atproto-attestation-sign -- \
  inline \
  record.json \
  did:key:zQ3shNzMp4oaaQ1gQRzCxMGXFrSW3NEM1M9T6KCY9eA7HhyEA \
  metadata.json

# Using JSON strings instead of files
cargo run --package atproto-attestation --features clap,tokio --bin atproto-attestation-sign -- \
  inline \
  '{"$type":"app.bsky.feed.post","text":"Hello!"}' \
  did:key:zQ3sh... \
  '{"$type":"com.example.sig","key":"did:key:zQ3sh..."}'

# Read record from stdin
cat record.json | cargo run --package atproto-attestation --features clap,tokio --bin atproto-attestation-sign -- \
  inline \
  - \
  did:key:zQ3sh... \
  metadata.json
```

#### Remote Attestation

```bash
# Create remote attestation (generates proof record + strongRef)
cargo run --package atproto-attestation --features clap,tokio --bin atproto-attestation-sign -- \
  remote \
  record.json \
  did:plc:repo123... \
  metadata.json

# This outputs TWO JSON objects:
# 1. Proof record (store this in the attestor's repository)
# 2. Source record with strongRef attestation
```

### Verifying Signatures

```bash
# Verify all signatures in a record from file
cargo run --package atproto-attestation --features clap,tokio --bin atproto-attestation-verify -- \
  ./signed_record.json \
  did:plc:repo123

# Verify from stdin
cat signed_record.json | cargo run --package atproto-attestation --features clap,tokio --bin atproto-attestation-verify -- \
  - \
  did:plc:repo123

# Verify from inline JSON
cargo run --package atproto-attestation --features clap,tokio --bin atproto-attestation-verify -- \
  '{"$type":"app.bsky.feed.post","text":"Hello","signatures":[...]}' \
  did:plc:repo123

# Verify specific attestation against record
cargo run --package atproto-attestation --features clap,tokio --bin atproto-attestation-verify -- \
  ./record.json \
  did:plc:repo123 \
  ./attestation.json
```

## Public API

The crate exposes the following public functions:

### Attestation Creation

- **`create_inline_attestation`**: Create a signed record with embedded signature
  - Automatically normalizes signatures to low-S form
  - Binds attestation to repository DID
  - Returns signed record with `signatures` array

- **`create_remote_attestation`**: Create separate proof record and strongRef
  - Returns tuple of (attested_record, proof_record)
  - Proof record must be stored in attestor's repository

### CID Generation

- **`create_cid`**: Generate CID for a record with `$sig` metadata
- **`create_dagbor_cid`**: Generate CID for any serializable data
- **`create_attestation_cid`**: High-level CID generation with automatic `$sig` preparation

### Signature Operations

- **`normalize_signature`**: Normalize raw signature bytes to low-S form
  - Prevents signature malleability attacks
  - Supports P-256, P-384, and K-256 curves

### Verification

- **`verify_record`**: Verify all signatures in a record
  - Validates repository binding
  - Supports both inline and remote attestations
  - Requires key and record resolvers

### Input Types

- **`AnyInput`**: Flexible input enum supporting:
  - `String`: JSON string to parse
  - `Json`: serde_json::Value
  - `TypedLexicon`: Strongly-typed lexicon records

## Attestation Specification

This crate implements the CID-first attestation specification, which ensures:

1. **Deterministic signing**: Records are serialized to DAG-CBOR with `$sig` metadata, producing consistent CIDs
2. **Content addressing**: Signatures are over CID bytes, not the full record
3. **Repository binding**: Every attestation is bound to a specific repository DID to prevent replay attacks
4. **Signature normalization**: ECDSA signatures are normalized to low-S form to prevent malleability
5. **Flexible metadata**: Custom fields in `$sig` are preserved and included in the CID calculation
6. **Multiple attestations**: Records can have multiple signatures in the `signatures` array

### Signature Structure

Inline attestation entry:
```json
{
  "$type": "com.example.signature",
  "key": "did:key:z...",
  "issuer": "did:plc:...",
  "cid": "bafyrei...",
  "signature": {
    "$bytes": "base64-normalized-signature"
  }
}
```

Remote attestation entry (strongRef):
```json
{
  "$type": "com.atproto.repo.strongRef",
  "uri": "at://did:plc:repo/com.example.attestation/tid",
  "cid": "bafyrei..."
}
```

## Error Handling

The crate provides structured error types via `AttestationError`:

- `RecordMustBeObject`: Input must be a JSON object
- `MetadataMustBeObject`: Attestation metadata must be a JSON object
- `SigMetadataMissing`: No `$sig` field found in prepared record
- `SignatureCreationFailed`: Key signing operation failed
- `SignatureValidationFailed`: Signature verification failed
- `SignatureNotNormalized`: ECDSA signature not in low-S form
- `SignatureLengthInvalid`: Signature bytes have incorrect length
- `KeyResolutionFailed`: Could not resolve verification key
- `UnsupportedKeyType`: Key type not supported for signing/verification
- `RemoteAttestationFetchFailed`: Failed to fetch remote proof record

## Security Considerations

### Repository Binding and Replay Attack Prevention

The most critical security feature of this attestation framework is **repository binding**. Every attestation includes the repository DID in the `$sig` metadata during CID generation, which:

- **Prevents replay attacks**: An attacker cannot copy a signed record from one repository to another because the signature is bound to the original repository DID
- **Ensures context integrity**: Attestations are only valid within their intended repository context
- **Automatic enforcement**: The library automatically adds the repository field during CID generation

**Important**: Always verify signatures with the correct repository DID. Verifying with a different repository DID will (correctly) fail validation, as this would indicate a potential replay attack.

### Signature Normalization

All ECDSA signatures are automatically normalized to low-S form to prevent signature malleability attacks:

- The library enforces low-S normalization during signature creation
- Verification accepts only normalized signatures
- This prevents attackers from creating alternate valid signatures for the same content

### Key Management Best Practices

- **Private keys**: Never log, transmit, or store private keys in plaintext
- **Key rotation**: Plan for key rotation by using verification method references that can be updated in DID documents
- **Key types**: The library supports P-256, P-384, and K-256 elliptic curves
- **did:key**: For testing and simple use cases, did:key identifiers provide self-contained key references

### CID Verification

- **Always verify against CIDs**: Signatures are over CID bytes, not the original record content
- **Deterministic generation**: The same record with the same `$sig` metadata always produces the same CID
- **Content integrity**: Any modification to the record will produce a different CID and invalidate signatures

### Metadata Validation

When creating attestations:

- The `$type` field is always required in metadata to scope the attestation
- The `repository` field is automatically added and must not be manually set
- Custom metadata fields are preserved and included in CID calculation
- The `cid` field is automatically added to inline attestation metadata

### Remote Attestation Considerations

- **Proof record storage**: Store proof records in the attestor's repository with appropriate access controls
- **CID matching**: Verify that the CID in the proof record matches the computed CID of the attested content
- **Record resolution**: Use trusted record resolvers when fetching remote proof records

## License

MIT License
