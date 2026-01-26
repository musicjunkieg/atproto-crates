//! Storage abstraction for block storage backends.
//!
//! This module provides a trait-based abstraction for storing and retrieving
//! blocks by CID, enabling both in-memory and disk-backed storage strategies.
//!
//! # Storage Backends
//!
//! - [`MemoryStorage`] - Fast in-memory storage with configurable limits
//! - [`DiskStorage`] - Hybrid memory/disk storage with automatic spill-over
//! - [`SpillableBuffer`] - Write-once buffer that spills to disk when threshold exceeded
//!
//! # Example
//!
//! ```rust,ignore
//! use atproto_dasl::storage::{BlockStorage, MemoryStorage};
//! use atproto_dasl::LimitsConfig;
//!
//! async fn example() -> anyhow::Result<()> {
//!     let mut storage = MemoryStorage::new();
//!
//!     // Store a block
//!     let data = b"hello world".to_vec();
//!     let cid = atproto_dasl::cid::compute_cid(&data);
//!     storage.put(&cid, data).await?;
//!
//!     // Retrieve it
//!     let retrieved = storage.get(&cid).await?;
//!     assert!(retrieved.is_some());
//!
//!     Ok(())
//! }
//! ```

mod disk;
mod memory;
mod spillable;

pub use disk::DiskStorage;
pub use memory::MemoryStorage;
pub use spillable::{SpillableBuffer, SpillableReader};

use crate::errors::StorageError;
use cid::Cid;

/// Trait for block storage backends.
///
/// Implementations can store blocks in memory, on disk, or use a hybrid
/// approach that spills to disk when memory limits are exceeded.
///
/// All operations are async to support both memory and disk-backed storage
/// with consistent interfaces.
#[allow(async_fn_in_trait)]
pub trait BlockStorage: Send + Sync {
    /// Store a block by CID.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::BlockTooLarge` if the block exceeds size limits.
    /// Returns `StorageError::CapacityExceeded` if storage is full.
    /// Returns `StorageError::BlockCountExceeded` if too many blocks.
    async fn put(&mut self, cid: &Cid, data: Vec<u8>) -> Result<(), StorageError>;

    /// Retrieve a block by CID.
    ///
    /// Returns `None` if the block is not found.
    async fn get(&self, cid: &Cid) -> Result<Option<Vec<u8>>, StorageError>;

    /// Check if a block exists.
    async fn contains(&self, cid: &Cid) -> Result<bool, StorageError>;

    /// Remove a block by CID.
    ///
    /// Returns the removed data if present.
    async fn remove(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>, StorageError>;

    /// Get current memory usage in bytes.
    fn memory_usage(&self) -> usize;

    /// Get total block count.
    fn block_count(&self) -> usize;

    /// Iterate over all CIDs in storage.
    fn cids(&self) -> Box<dyn Iterator<Item = Cid> + '_>;

    /// Clear all stored blocks.
    async fn clear(&mut self) -> Result<(), StorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cid::compute_cid;

    async fn test_storage_basic<S: BlockStorage>(storage: &mut S) {
        let data = b"test block data".to_vec();
        let cid = compute_cid(&data);

        // Initially empty
        assert!(!storage.contains(&cid).await.unwrap());
        assert!(storage.get(&cid).await.unwrap().is_none());

        // Put and get
        storage.put(&cid, data.clone()).await.unwrap();
        assert!(storage.contains(&cid).await.unwrap());
        assert_eq!(storage.get(&cid).await.unwrap(), Some(data.clone()));
        assert_eq!(storage.block_count(), 1);

        // Remove
        let removed = storage.remove(&cid).await.unwrap();
        assert_eq!(removed, Some(data));
        assert!(!storage.contains(&cid).await.unwrap());
        assert_eq!(storage.block_count(), 0);
    }

    #[tokio::test]
    async fn test_memory_storage_basic() {
        let mut storage = MemoryStorage::new();
        test_storage_basic(&mut storage).await;
    }
}
