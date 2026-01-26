# atproto-dasl

[DASL (Data-Addressed Structures & Links)](https://dasl.ing/) implementation for AT Protocol. Provides content-addressed data serialization, storage, and retrieval used throughout the `atproto-identity-rs` workspace.

## Modules

| Module | Description |
|--------|-------------|
| `cid` | Content Identifiers with SHA-256 and BLAKE3 hashing. Three CID types: `Cid` (general), `DaslCid` (strictly validated), and `RawCid` (unvalidated). |
| `drisl` | Deterministic DAG-CBOR encoding and decoding with full serde integration. Enforces sorted map keys, shortest-form integers, and rejects NaN/Infinity. |
| `car` | CAR v1 (Content-Addressable aRchive) async streaming reader and writer with configurable size and depth limits. |
| `storage` | Block storage trait with `MemoryStorage` and `DiskStorage` backends. `DiskStorage` supports automatic memory-to-disk spillover. |
| `bdasl` | Big DASL for large files using BLAKE3 and BLAKE3-BAO hashing with streaming verification. |
| `masl` | Metadata for Arbitrary Structures & Links. CBOR metadata documents in single-resource or bundle mode. |
| `rasl` | Retrieval of Arbitrary Structures & Links. `rasl://` URL scheme with HTTP retrieval and verification. |
| `tiles` | Web Tiles validation for composable web documents with security constraints (CSP, COOP, CORP). |
| `varint` | Unsigned LEB128 variable-length integer encoding and decoding. |
| `value` | IPLD data model (`Ipld` enum) with Null, Bool, Integer, Float, String, Bytes, List, Map, and Link variants. |

## Usage

```rust
use atproto_dasl::{to_vec, from_slice, compute_cid_for};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Post {
    text: String,
    likes: u64,
}

let post = Post { text: "Hello!".into(), likes: 42 };

// Serialize to DAG-CBOR bytes
let bytes = to_vec(&post).unwrap();

// Deserialize back
let decoded: Post = from_slice(&bytes).unwrap();
assert_eq!(post, decoded);

// Compute a CID directly from a serializable value
let cid = compute_cid_for(&post).unwrap();
```

## Features

| Feature | Description |
|---------|-------------|
| `clap` | Enables the `atproto-dasl` CLI binary for JSON/DAG-CBOR conversion. |
| `reqwest` | Enables RASL HTTP fetch with CID verification (`fetch_verified`). |
| `axum` | Enables RASL axum handlers for serving content-addressed data. |

## CLI

Requires the `clap` feature:

```sh
cargo build --package atproto-dasl --features clap

# JSON to DAG-CBOR (hex)
echo '{"text":"hello","likes":1}' | atproto-dasl encode

# DAG-CBOR (hex) to JSON
echo 'a2656c696b6573016474657874656....' | atproto-dasl decode
```

## License

MIT
