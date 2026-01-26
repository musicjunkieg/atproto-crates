//! Command-line tool for generating CIDs from JSON records.
//!
//! This tool reads JSON from stdin, serializes it using IPLD DAG-CBOR format,
//! and outputs the corresponding CID (Content Identifier) using CIDv1 with
//! SHA-256 hashing. This matches the AT Protocol specification for content
//! addressing of records.
//!
//! # AT Protocol CID Format
//!
//! The tool generates CIDs that follow the AT Protocol specification:
//! - **CID Version**: CIDv1
//! - **Codec**: DAG-CBOR (0x71)
//! - **Hash Function**: SHA-256 (0x12)
//! - **Encoding**: Base32 (default for CIDv1)
//!
//! # Example Usage
//!
//! ```bash
//! # Generate CID from a simple JSON object
//! echo '{"text":"Hello, AT Protocol!"}' | cargo run --features clap --bin atproto-record-cid
//!
//! # Generate CID from a file
//! cat post.json | cargo run --features clap --bin atproto-record-cid
//!
//! # Generate CID from a complex record
//! echo '{
//!   "$type": "app.bsky.feed.post",
//!   "text": "Hello world",
//!   "createdAt": "2025-01-19T10:00:00.000Z"
//! }' | cargo run --features clap --bin atproto-record-cid
//! ```
//!
//! # Output Format
//!
//! The tool outputs the CID as a single line string in the format:
//! ```text
//! bafyreibjzlvhtyxnhbvvzl3gj4qmg2ufl2jbhh5qr3gvvxlm7ksf3qwxqq
//! ```
//!
//! # Error Handling
//!
//! The tool will return an error if:
//! - Input is not valid JSON
//! - JSON cannot be serialized to DAG-CBOR
//! - CID generation fails
//!
//! # Technical Details
//!
//! The CID generation process:
//! 1. Read JSON from stdin
//! 2. Parse JSON into serde_json::Value
//! 3. Serialize to DAG-CBOR bytes using atproto_dasl
//! 4. Hash the bytes using SHA-256
//! 5. Create CIDv1 with DAG-CBOR codec
//! 6. Output the CID string

use anyhow::Result;
use atproto_record::errors::CliError;
use cid::Cid;
use clap::Parser;
use multihash::Multihash;
use sha2::{Digest, Sha256};
use std::io::{self, Read};

/// AT Protocol Record CID Generator
#[derive(Parser)]
#[command(
    name = "atproto-record-cid",
    version,
    about = "Generate CID for AT Protocol DAG-CBOR records from JSON",
    long_about = "
A command-line tool for generating Content Identifiers (CIDs) from JSON records
using the AT Protocol DAG-CBOR serialization format.

The tool reads JSON from stdin, serializes it using IPLD DAG-CBOR format, and
outputs the corresponding CID using CIDv1 with SHA-256 hashing. This matches
the AT Protocol specification for content addressing of records.

CID FORMAT:
  Version:  CIDv1
  Codec:    DAG-CBOR (0x71)
  Hash:     SHA-256 (0x12)
  Encoding: Base32 (default for CIDv1)

EXAMPLES:
  # Generate CID from stdin:
  echo '{\"text\":\"Hello!\"}' | atproto-record-cid

  # Generate CID from a file:
  cat post.json | atproto-record-cid

  # Complex record with AT Protocol fields:
  echo '{
    \"$type\": \"app.bsky.feed.post\",
    \"text\": \"Hello world\",
    \"createdAt\": \"2025-01-19T10:00:00.000Z\"
  }' | atproto-record-cid

OUTPUT:
  The tool outputs a single line containing the CID:
  bafyreibjzlvhtyxnhbvvzl3gj4qmg2ufl2jbhh5qr3gvvxlm7ksf3qwxqq

NOTES:
  - Input must be valid JSON
  - The same JSON input will always produce the same CID
  - Field order in JSON objects may affect the CID due to DAG-CBOR serialization
  - Special AT Protocol fields like $type, $sig, and $link are preserved
"
)]
struct Args {}

fn main() -> Result<()> {
    let _args = Args::parse();

    // Read JSON from stdin
    let mut stdin_content = String::new();
    io::stdin()
        .read_to_string(&mut stdin_content)
        .map_err(|_| CliError::StdinReadFailed)?;

    // Parse JSON
    let json_value: serde_json::Value =
        serde_json::from_str(&stdin_content).map_err(|_| CliError::StdinJsonParseFailed)?;

    // Serialize to DAG-CBOR
    let dag_cbor_bytes =
        atproto_dasl::to_vec(&json_value).map_err(|error| CliError::RecordSerializationFailed {
            error: error.to_string(),
        })?;

    // Hash the bytes using SHA-256
    // Code 0x12 is SHA-256, size 32 bytes
    let mut hasher = Sha256::new();
    hasher.update(&dag_cbor_bytes);
    let hash_result = hasher.finalize();

    let multihash =
        Multihash::wrap(0x12, &hash_result).map_err(|error| CliError::CidGenerationFailed {
            error: error.to_string(),
        })?;

    // Create CIDv1 with DAG-CBOR codec (0x71)
    let cid = Cid::new_v1(0x71, multihash);

    // Output the CID
    println!("{}", cid);

    Ok(())
}
