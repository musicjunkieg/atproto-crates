# atproto-identity

AT Protocol identity management for DID resolution, handle resolution, and cryptographic operations.

## Overview

Core identity functionality for AT Protocol applications including multi-method DID resolution (plc, web, key), DNS/HTTP handle resolution, and P-256/P-384/K-256 key operations.

## Features

- **Multi-method DID resolution**: Support for `did:plc`, `did:web`, and `did:key` methods
- **Handle resolution**: DNS TXT record and HTTP `.well-known` endpoint resolution with conflict detection
- **Cryptographic operations**: P-256, P-384, and K-256 elliptic curve key generation, signing, and validation
- **Identity validation**: Input validation for handles and DIDs following AT Protocol specifications
- **Document storage**: LRU cache-based DID document storage with pluggable backends
- **Configuration management**: Environment variable handling and DNS nameserver configuration

## CLI Tools

The following command-line tools are available when built with the `clap` and `hickory-dns` features:

- **`atproto-identity-resolve`**: Resolve AT Protocol handles and DIDs to canonical identifiers with optional DID document output
- **`atproto-identity-key`**: Generate cryptographic keys for P-256, P-384, and K-256 curves
- **`atproto-identity-sign`**: Create cryptographic signatures of JSON data using private keys
- **`atproto-identity-validate`**: Validate cryptographic signatures against public keys

## Library Usage

### Handle Resolution

```rust
use atproto_identity::resolve::{resolve_subject, create_resolver};

let http_client = reqwest::Client::new();
let dns_resolver = create_resolver(&[]);

let did = resolve_subject(&http_client, &dns_resolver, "alice.bsky.social").await?;
```

### Key Operations

```rust
use atproto_identity::key::{identify_key, generate_key, validate, KeyType};

// Generate a new key
let private_key = generate_key(KeyType::P256Private)?;

// Identify existing key
let key_data = identify_key("did:key:zQ3sh...")?;

// Validate signature
validate(&key_data, &signature, content)?;
```

## Command Line Usage

All CLI tools require the `clap` feature:

```bash
# Build with CLI support
cargo build --features clap,hickory-dns --bins

# Resolve a handle to DID
cargo run --features clap,hickory-dns --bin atproto-identity-resolve -- alice.bsky.social

# Generate a new P-256 key
cargo run --features clap --bin atproto-identity-key -- generate p256

# Sign JSON data
cargo run --features clap --bin atproto-identity-sign -- did:key:zQ3sh... data.json

# Verify a signature
cargo run --features clap --bin atproto-identity-validate -- did:key:zQ3sh... data.json signature
```

## License

MIT License