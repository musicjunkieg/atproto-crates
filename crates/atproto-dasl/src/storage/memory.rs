//! In-memory block storage with configurable limits.
//!
//! [`MemoryStorage`] provides fast block storage using a `HashMap`, with
//! configurable limits on memory usage, block size, and block count.

use super::BlockStorage;
use crate::car::LimitsConfig;
use crate::errors::StorageError;
use cid::Cid;
use std::collections::HashMap;

/// In-memory block storage with configurable limits.
///
/// This is the simplest and fastest storage backend, suitable for
/// processing CAR files that fit in memory.
///
/// # Example
///
/// ```rust,ignore
/// use atproto_dasl::storage::MemoryStorage;
/// use atproto_dasl::LimitsConfig;
///
/// // Default limits (100MB)
/// let storage = MemoryStorage::new();
///
/// // Custom limits for low-memory environments
/// let storage = MemoryStorage::with_limits(LimitsConfig::low_memory());
/// ```
pub struct MemoryStorage {
    blocks: HashMap<Cid, Vec<u8>>,
    memory_used: usize,
    limits: LimitsConfig,
}

impl MemoryStorage {
    /// Create new memory storage with default limits.
    #[must_use]
    pub fn new() -> Self {
        Self::with_limits(LimitsConfig::default())
    }

    /// Create with custom limits.
    #[must_use]
    pub fn with_limits(limits: LimitsConfig) -> Self {
        Self {
            blocks: HashMap::new(),
            memory_used: 0,
            limits,
        }
    }

    /// Create from existing blocks.
    ///
    /// # Errors
    ///
    /// Returns `StorageError` if the blocks exceed configured limits.
    pub fn from_blocks(
        blocks: HashMap<Cid, Vec<u8>>,
        limits: LimitsConfig,
    ) -> Result<Self, StorageError> {
        let mut total_memory = 0;

        for data in blocks.values() {
            if data.len() > limits.max_block_size {
                return Err(StorageError::BlockTooLarge {
                    size: data.len(),
                    max: limits.max_block_size,
                });
            }
            total_memory += data.len();
        }

        if blocks.len() > limits.max_block_count {
            return Err(StorageError::BlockCountExceeded {
                count: blocks.len(),
                max: limits.max_block_count,
            });
        }

        if total_memory > limits.max_memory_bytes {
            return Err(StorageError::CapacityExceeded {
                limit: limits.max_memory_bytes,
            });
        }

        Ok(Self {
            blocks,
            memory_used: total_memory,
            limits,
        })
    }

    /// Get the configured limits.
    #[must_use]
    pub fn limits(&self) -> &LimitsConfig {
        &self.limits
    }

    /// Get direct access to the underlying blocks.
    #[must_use]
    pub fn blocks(&self) -> &HashMap<Cid, Vec<u8>> {
        &self.blocks
    }

    /// Consume storage and return the blocks.
    #[must_use]
    pub fn into_blocks(self) -> HashMap<Cid, Vec<u8>> {
        self.blocks
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockStorage for MemoryStorage {
    async fn put(&mut self, cid: &Cid, data: Vec<u8>) -> Result<(), StorageError> {
        // Check block size limit
        if data.len() > self.limits.max_block_size {
            return Err(StorageError::BlockTooLarge {
                size: data.len(),
                max: self.limits.max_block_size,
            });
        }

        // Check if this is a duplicate (avoid double-counting memory)
        if let Some(existing) = self.blocks.get(cid) {
            // If same data, nothing to do
            if existing.len() == data.len() {
                return Ok(());
            }
            // Update memory accounting for replacement
            let old_size = existing.len();
            let new_usage = self.memory_used - old_size + data.len();

            if new_usage > self.limits.max_memory_bytes {
                return Err(StorageError::CapacityExceeded {
                    limit: self.limits.max_memory_bytes,
                });
            }

            self.memory_used = new_usage;
            self.blocks.insert(*cid, data);
            return Ok(());
        }

        // Check block count limit
        if self.blocks.len() >= self.limits.max_block_count {
            return Err(StorageError::BlockCountExceeded {
                count: self.blocks.len(),
                max: self.limits.max_block_count,
            });
        }

        // Check memory limit
        let new_usage = self.memory_used + data.len();
        if new_usage > self.limits.max_memory_bytes {
            return Err(StorageError::CapacityExceeded {
                limit: self.limits.max_memory_bytes,
            });
        }

        self.memory_used = new_usage;
        self.blocks.insert(*cid, data);
        Ok(())
    }

    async fn get(&self, cid: &Cid) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.blocks.get(cid).cloned())
    }

    async fn contains(&self, cid: &Cid) -> Result<bool, StorageError> {
        Ok(self.blocks.contains_key(cid))
    }

    async fn remove(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>, StorageError> {
        if let Some(data) = self.blocks.remove(cid) {
            self.memory_used -= data.len();
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    fn memory_usage(&self) -> usize {
        self.memory_used
    }

    fn block_count(&self) -> usize {
        self.blocks.len()
    }

    fn cids(&self) -> Box<dyn Iterator<Item = Cid> + '_> {
        Box::new(self.blocks.keys().copied())
    }

    async fn clear(&mut self) -> Result<(), StorageError> {
        self.blocks.clear();
        self.memory_used = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cid::compute_cid;

    #[tokio::test]
    async fn test_put_and_get() {
        let mut storage = MemoryStorage::new();

        let data1 = b"block one".to_vec();
        let cid1 = compute_cid(&data1);

        storage.put(&cid1, data1.clone()).await.unwrap();

        let retrieved = storage.get(&cid1).await.unwrap();
        assert_eq!(retrieved, Some(data1));
    }

    #[tokio::test]
    async fn test_block_size_limit() {
        let limits = LimitsConfig::default().with_max_block_size(100);
        let mut storage = MemoryStorage::with_limits(limits);

        let data = vec![0u8; 200]; // Exceeds limit
        let cid = compute_cid(&data);

        let result = storage.put(&cid, data).await;
        assert!(matches!(result, Err(StorageError::BlockTooLarge { .. })));
    }

    #[tokio::test]
    async fn test_memory_limit() {
        let limits = LimitsConfig::default().with_max_memory_bytes(100);
        let mut storage = MemoryStorage::with_limits(limits);

        let data = vec![0u8; 60];
        let cid1 = compute_cid(&data);
        storage.put(&cid1, data).await.unwrap();

        let data2 = vec![1u8; 60];
        let cid2 = compute_cid(&data2);
        let result = storage.put(&cid2, data2).await;
        assert!(matches!(result, Err(StorageError::CapacityExceeded { .. })));
    }

    #[tokio::test]
    async fn test_block_count_limit() {
        let limits = LimitsConfig::default().with_max_block_count(2);
        let mut storage = MemoryStorage::with_limits(limits);

        for i in 0..2 {
            let data = vec![i as u8; 10];
            let cid = compute_cid(&data);
            storage.put(&cid, data).await.unwrap();
        }

        let data = vec![99u8; 10];
        let cid = compute_cid(&data);
        let result = storage.put(&cid, data).await;
        assert!(matches!(
            result,
            Err(StorageError::BlockCountExceeded { .. })
        ));
    }

    #[tokio::test]
    async fn test_duplicate_put() {
        let mut storage = MemoryStorage::new();

        let data = b"duplicate test".to_vec();
        let cid = compute_cid(&data);

        storage.put(&cid, data.clone()).await.unwrap();
        let mem_before = storage.memory_usage();

        // Put same data again - should not increase memory
        storage.put(&cid, data).await.unwrap();
        assert_eq!(storage.memory_usage(), mem_before);
        assert_eq!(storage.block_count(), 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let mut storage = MemoryStorage::new();

        for i in 0..5 {
            let data = vec![i; 100];
            let cid = compute_cid(&data);
            storage.put(&cid, data).await.unwrap();
        }

        assert_eq!(storage.block_count(), 5);
        assert!(storage.memory_usage() > 0);

        storage.clear().await.unwrap();

        assert_eq!(storage.block_count(), 0);
        assert_eq!(storage.memory_usage(), 0);
    }

    #[tokio::test]
    async fn test_cids_iterator() {
        let mut storage = MemoryStorage::new();
        let mut expected_cids = Vec::new();

        for i in 0..3 {
            let data = vec![i; 10];
            let cid = compute_cid(&data);
            expected_cids.push(cid);
            storage.put(&cid, data).await.unwrap();
        }

        let stored_cids: Vec<Cid> = storage.cids().collect();
        assert_eq!(stored_cids.len(), 3);

        for cid in &expected_cids {
            assert!(stored_cids.contains(cid));
        }
    }

    #[tokio::test]
    async fn test_from_blocks() {
        let mut blocks = HashMap::new();
        for i in 0..3 {
            let data = vec![i; 10];
            let cid = compute_cid(&data);
            blocks.insert(cid, data);
        }

        let storage = MemoryStorage::from_blocks(blocks.clone(), LimitsConfig::default()).unwrap();
        assert_eq!(storage.block_count(), 3);
        assert_eq!(storage.memory_usage(), 30);

        for (cid, data) in blocks {
            assert_eq!(storage.get(&cid).await.unwrap(), Some(data));
        }
    }

    #[tokio::test]
    async fn test_from_blocks_exceeds_limits() {
        let mut blocks = HashMap::new();
        for i in 0..10 {
            let data = vec![i; 100];
            let cid = compute_cid(&data);
            blocks.insert(cid, data);
        }

        let limits = LimitsConfig::default().with_max_block_count(5);
        let result = MemoryStorage::from_blocks(blocks, limits);
        assert!(matches!(
            result,
            Err(StorageError::BlockCountExceeded { .. })
        ));
    }
}
