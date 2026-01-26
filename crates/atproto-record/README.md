# atproto-record

Utilities for working with AT Protocol records.

## Overview

A Rust library for working with AT Protocol records, providing AT-URI parsing, TID generation, datetime formatting, and CID generation. Built on IPLD DAG-CBOR serialization for deterministic content addressing.

## Features

- **AT-URI parsing**: Parse and validate AT Protocol URIs (at://authority/collection/record_key) with robust error handling
- **TID generation**: Timestamp-based identifiers for AT Protocol records with microsecond precision
- **CID generation**: Content Identifier generation using DAG-CBOR serialization and SHA-256 hashing
- **DateTime utilities**: RFC 3339 datetime serialization with millisecond precision for consistent timestamp handling
- **Typed records**: Type-safe record handling with lexicon type validation
- **Bytes handling**: Base64 encoding/decoding for binary data in AT Protocol records
- **Structured errors**: Type-safe error handling following project conventions with detailed error messages

## CLI Tools

The following command-line tool is available when built with the `clap` feature:

- **`atproto-record-cid`**: Generate CID (Content Identifier) for AT Protocol records from JSON input

## Library Usage

### Generating CIDs

```rust
use serde_json::json;
use cid::Cid;
use sha2::{Digest, Sha256};
use multihash::Multihash;

// Serialize a record to DAG-CBOR and generate its CID
let record = json!({
    "$type": "app.bsky.feed.post",
    "text": "Hello world!",
    "createdAt": "2024-01-01T00:00:00.000Z"
});

let dag_cbor_bytes = atproto_dasl::to_vec(&record)?;
let hash = Sha256::digest(&dag_cbor_bytes);
let multihash = Multihash::wrap(0x12, &hash)?;
let cid = Cid::new_v1(0x71, multihash);

println!("Record CID: {}", cid);
```

### Generating TIDs

```rust
use atproto_record::tid::Tid;

// Generate a new timestamp-based identifier
let tid = Tid::new();
println!("TID: {}", tid); // e.g., "3l2k4j5h6g7f8d9s"

// TIDs are sortable by creation time
let tid1 = Tid::new();
std::thread::sleep(std::time::Duration::from_millis(1));
let tid2 = Tid::new();
assert!(tid1 < tid2);
```

### AT-URI Parsing

```rust
use atproto_record::aturi::ATURI;
use std::str::FromStr;

// Parse an AT-URI into its components
let aturi = ATURI::from_str("at://did:plc:abc123/app.bsky.feed.post/3k2k4j5h6g")?;

// Access the parsed components
println!("Authority: {}", aturi.authority);   // "did:plc:abc123"
println!("Collection: {}", aturi.collection);  // "app.bsky.feed.post"
println!("Record Key: {}", aturi.record_key);  // "3k2k4j5h6g"

// The Display trait formats back to a valid AT-URI
println!("Full URI: {}", aturi);  // "at://did:plc:abc123/app.bsky.feed.post/3k2k4j5h6g"
```

### DateTime Utilities

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// Use the datetime module for consistent RFC 3339 formatting
#[derive(Serialize, Deserialize)]
struct Record {
    #[serde(with = "atproto_record::datetime::format")]
    created_at: DateTime<Utc>,
    
    #[serde(with = "atproto_record::datetime::optional_format")]
    updated_at: Option<DateTime<Utc>>,
}
```

## Command Line Usage

The CLI tool requires the `clap` feature:

```bash
# Build with CLI support
cargo build --features clap --bins

# Generate CID from JSON file
cat record.json | cargo run --features clap --bin atproto-record-cid

# Generate CID from inline JSON
echo '{"$type":"app.bsky.feed.post","text":"Hello!"}' | cargo run --features clap --bin atproto-record-cid

# Example with a complete AT Protocol record
cat <<EOF | cargo run --features clap --bin atproto-record-cid
{
  "$type": "app.bsky.feed.post",
  "text": "Hello AT Protocol!",
  "createdAt": "2024-01-01T00:00:00.000Z"
}
EOF
```

The tool outputs the CID in base32 format:
```
bafyreibjzlvhtyxnhbvvzl3gj4qmg2ufl2jbhh5qr3gvvxlm7ksf3qwxqq
```

## License

MIT License
