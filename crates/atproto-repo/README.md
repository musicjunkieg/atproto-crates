# atproto-repo

AT Protocol repository handling - CAR v1 serialization and Merkle Search Tree operations.

## Features

- **CAR v1 Support**: Full Content Addressable aRchive (CAR) v1 format support
  - Async streaming reader and writer
  - Block-level verification
  - Configurable memory limits

- **Merkle Search Tree (MST)**: Deterministic tree structure for AT Protocol repositories
  - Insert, get, delete, and list operations
  - Tree diffing for sync operations
  - Height calculation using SHA-256

- **Storage Abstraction**: Pluggable storage backends
  - In-memory storage with LRU eviction
  - Disk-backed storage with memory caching
  - Spillable buffer for streaming large files

- **Repository Structures**: AT Protocol commit and record types
  - Commit serialization and validation
  - Record path handling (collection/rkey)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
atproto-repo = "0.13"
```

## Usage

### Reading a CAR file

```rust
use atproto_repo::car::CarReader;
use tokio::fs::File;

async fn read_car() -> anyhow::Result<()> {
    let file = File::open("repository.car").await?;
    let mut reader = CarReader::new(file).await?;

    println!("Roots: {:?}", reader.roots());

    while let Some(block) = reader.next_block().await? {
        println!("CID: {}", block.cid);
        println!("Data size: {} bytes", block.data.len());
    }

    Ok(())
}
```

### Writing a CAR file

```rust
use atproto_repo::car::{CarWriter, CarBlock};
use tokio::fs::File;

async fn write_car() -> anyhow::Result<()> {
    let file = File::create("output.car").await?;

    let data = b"hello world".to_vec();
    let block = CarBlock::from_data(data);
    let root = block.cid;

    let mut writer = CarWriter::new(file, vec![root.into()]).await?;
    writer.write_block(&block).await?;
    writer.finish().await?;

    Ok(())
}
```

### Working with MST

```rust
use atproto_repo::mst::Mst;
use atproto_repo::storage::MemoryStorage;
use atproto_repo::compute_cid;

async fn mst_operations() -> anyhow::Result<()> {
    let storage = MemoryStorage::new();
    let mut mst = Mst::new(storage);

    // Insert records
    let record_cid = compute_cid(b"record data").into();
    mst.insert("app.bsky.feed.post/abc123", record_cid).await?;

    // Lookup
    let cid = mst.get("app.bsky.feed.post/abc123").await?;
    println!("Found: {:?}", cid);

    // List all entries
    let entries = mst.entries().await?;
    for (key, cid) in entries {
        println!("{} -> {}", key, cid);
    }

    Ok(())
}
```

### Spillable Buffer for Large Files

```rust
use atproto_repo::storage::SpillableBuffer;

async fn stream_large_file() -> std::io::Result<()> {
    // Memory threshold: 1MB
    let mut buffer = SpillableBuffer::new(1024 * 1024);

    // Write data - spills to disk if exceeds threshold
    buffer.write_chunk(b"some data").await?;
    buffer.write_chunk(b"more data").await?;

    println!("On disk: {}", buffer.is_on_disk());

    // Convert to reader
    let reader = buffer.into_reader().await?;
    Ok(())
}
```

## CLI Tools

Two CLI tools are provided (requires `clap` feature):

### atproto-repo-car

Inspect and manipulate CAR files:

```bash
# List blocks
cargo run --features clap --bin atproto-repo-car -- ls repo.car

# Show roots
cargo run --features clap --bin atproto-repo-car -- roots repo.car

# Verify integrity
cargo run --features clap --bin atproto-repo-car -- verify repo.car

# Show statistics
cargo run --features clap --bin atproto-repo-car -- stats repo.car
```

### atproto-repo-mst

Inspect MST structures in repositories:

```bash
# List all keys
cargo run --features clap --bin atproto-repo-mst -- ls repo.car

# Get specific record
cargo run --features clap --bin atproto-repo-mst -- get repo.car app.bsky.feed.post/abc123

# Show tree structure
cargo run --features clap --bin atproto-repo-mst -- tree repo.car

# List collections
cargo run --features clap --bin atproto-repo-mst -- collections repo.car
```

## Configuration

```rust
use atproto_repo::config::{RepoConfig, LimitsConfig};

// Default configuration
let config = RepoConfig::default();

// No verification (faster but less safe)
let config = RepoConfig::no_verification();

// Custom limits
let config = RepoConfig::builder()
    .verify_cids(true)
    .limits(LimitsConfig::low_memory())
    .build();
```

## Error Handling

All error types follow the project convention:

```
error-atproto-repo-<domain>-<number> <message>: <details>
```

Example:
- `error-atproto-repo-car-3 CID mismatch: expected bafy..., got bafy...`
- `error-atproto-repo-mst-5 Node not found: bafy...`

## License

MIT OR Apache-2.0
