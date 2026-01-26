//! Hybrid memory/disk block storage with automatic spill-over.
//!
//! [`DiskStorage`] keeps frequently accessed blocks in memory and spills
//! to disk when memory limits are exceeded, enabling processing of large
//! CAR files in low-memory environments.

use super::BlockStorage;
use crate::car::LimitsConfig;
use crate::errors::StorageError;
use cid::Cid;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Hybrid storage that keeps hot data in memory and spills to disk.
///
/// Optimized for streaming large CAR files in low-resource environments.
/// Blocks are stored in memory until the threshold is exceeded, then
/// the oldest blocks are written to disk.
///
/// # Disk Layout
///
/// Blocks are stored as individual files named by their CID (base32 encoded)
/// in a temporary directory that is automatically cleaned up on drop.
///
/// # Example
///
/// ```rust,ignore
/// use atproto_dasl::storage::DiskStorage;
/// use atproto_dasl::LimitsConfig;
///
/// async fn example() -> anyhow::Result<()> {
///     // Create disk storage with low memory threshold
///     let limits = LimitsConfig::default()
///         .with_disk_spill_threshold(10 * 1024 * 1024); // 10MB
///
///     let mut storage = DiskStorage::new(limits).await?;
///
///     // Blocks will automatically spill to disk when threshold exceeded
///     storage.put(&cid, large_data).await?;
///
///     Ok(())
/// }
/// ```
pub struct DiskStorage {
    /// In-memory cache for recently accessed blocks.
    memory_cache: HashMap<Cid, Vec<u8>>,
    /// Access order for LRU eviction.
    access_order: VecDeque<Cid>,
    /// Index of CIDs stored on disk.
    disk_index: HashSet<Cid>,
    /// Temporary directory for disk storage.
    temp_dir: TempDir,
    /// Current memory usage.
    memory_used: usize,
    /// Configuration limits.
    limits: LimitsConfig,
}

impl DiskStorage {
    /// Create new disk storage with a temporary directory.
    ///
    /// # Errors
    ///
    /// Returns `StorageError::Io` if the temporary directory cannot be created.
    pub async fn new(limits: LimitsConfig) -> Result<Self, StorageError> {
        let temp_dir = TempDir::new()?;
        Ok(Self {
            memory_cache: HashMap::new(),
            access_order: VecDeque::new(),
            disk_index: HashSet::new(),
            temp_dir,
            memory_used: 0,
            limits,
        })
    }

    /// Create with a specific disk path instead of a temporary directory.
    ///
    /// Note: This path will NOT be automatically cleaned up on drop.
    /// Use `new()` for automatic cleanup.
    pub async fn with_path(path: PathBuf, limits: LimitsConfig) -> Result<Self, StorageError> {
        fs::create_dir_all(&path).await?;
        let temp_dir = TempDir::new_in(&path)?;
        Ok(Self {
            memory_cache: HashMap::new(),
            access_order: VecDeque::new(),
            disk_index: HashSet::new(),
            temp_dir,
            memory_used: 0,
            limits,
        })
    }

    /// Get the path to the disk storage directory.
    #[must_use]
    pub fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }

    /// Convert CID to filename.
    fn cid_to_filename(&self, cid: &Cid) -> PathBuf {
        // Use base32 encoding for filesystem-safe names
        let filename = cid.to_string();
        self.temp_dir.path().join(filename)
    }

    /// Spill oldest blocks to disk when memory threshold exceeded.
    async fn spill_to_disk(&mut self) -> Result<(), StorageError> {
        while self.memory_used > self.limits.disk_spill_threshold {
            // Find oldest block to evict
            if let Some(cid) = self.access_order.pop_front() {
                if let Some(data) = self.memory_cache.remove(&cid) {
                    self.memory_used -= data.len();
                    self.write_to_disk(&cid, &data).await?;
                    self.disk_index.insert(cid);
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Write a block to disk.
    async fn write_to_disk(&self, cid: &Cid, data: &[u8]) -> Result<(), StorageError> {
        let path = self.cid_to_filename(cid);
        let mut file = fs::File::create(&path).await?;
        file.write_all(data).await?;
        file.sync_all().await?;
        Ok(())
    }
}

impl BlockStorage for DiskStorage {
    async fn put(&mut self, cid: &Cid, data: Vec<u8>) -> Result<(), StorageError> {
        // Check block size
        if data.len() > self.limits.max_block_size {
            return Err(StorageError::BlockTooLarge {
                size: data.len(),
                max: self.limits.max_block_size,
            });
        }

        // Check block count limit
        let total_blocks = self.memory_cache.len() + self.disk_index.len();
        if !self.memory_cache.contains_key(cid)
            && !self.disk_index.contains(cid)
            && total_blocks >= self.limits.max_block_count
        {
            return Err(StorageError::BlockCountExceeded {
                count: total_blocks,
                max: self.limits.max_block_count,
            });
        }

        // Handle duplicates
        if self.memory_cache.contains_key(cid) || self.disk_index.contains(cid) {
            return Ok(());
        }

        // If adding would exceed memory threshold, spill to disk
        if self.memory_used + data.len() > self.limits.disk_spill_threshold {
            self.spill_to_disk().await?;
        }

        // If still over limit after spilling, write directly to disk
        if self.memory_used + data.len() > self.limits.disk_spill_threshold {
            self.write_to_disk(cid, &data).await?;
            self.disk_index.insert(*cid);
        } else {
            self.memory_used += data.len();
            self.memory_cache.insert(*cid, data);
            self.access_order.push_back(*cid);
        }

        Ok(())
    }

    async fn get(&self, cid: &Cid) -> Result<Option<Vec<u8>>, StorageError> {
        // Check memory first
        if let Some(data) = self.memory_cache.get(cid) {
            return Ok(Some(data.clone()));
        }

        // Check disk
        if self.disk_index.contains(cid) {
            let path = self.cid_to_filename(cid);
            let mut file = fs::File::open(&path).await?;
            let mut data = Vec::new();
            file.read_to_end(&mut data).await?;
            return Ok(Some(data));
        }

        Ok(None)
    }

    async fn contains(&self, cid: &Cid) -> Result<bool, StorageError> {
        Ok(self.memory_cache.contains_key(cid) || self.disk_index.contains(cid))
    }

    async fn remove(&mut self, cid: &Cid) -> Result<Option<Vec<u8>>, StorageError> {
        // Try memory first
        if let Some(data) = self.memory_cache.remove(cid) {
            self.memory_used -= data.len();
            self.access_order.retain(|c| c != cid);
            return Ok(Some(data));
        }

        // Try disk
        if self.disk_index.remove(cid) {
            let path = self.cid_to_filename(cid);

            // Read data before removing
            let data = if path.exists() {
                let mut file = fs::File::open(&path).await?;
                let mut data = Vec::new();
                file.read_to_end(&mut data).await?;
                fs::remove_file(&path).await?;
                Some(data)
            } else {
                None
            };

            return Ok(data);
        }

        Ok(None)
    }

    fn memory_usage(&self) -> usize {
        self.memory_used
    }

    fn block_count(&self) -> usize {
        self.memory_cache.len() + self.disk_index.len()
    }

    fn cids(&self) -> Box<dyn Iterator<Item = Cid> + '_> {
        let memory_cids = self.memory_cache.keys().copied();
        let disk_cids = self.disk_index.iter().copied();
        Box::new(memory_cids.chain(disk_cids))
    }

    async fn clear(&mut self) -> Result<(), StorageError> {
        self.memory_cache.clear();
        self.access_order.clear();
        self.memory_used = 0;

        // Collect CIDs first to avoid borrow conflict
        let cids: Vec<Cid> = self.disk_index.drain().collect();

        // Remove disk files
        for cid in cids {
            let path = self.cid_to_filename(&cid);
            if path.exists() {
                fs::remove_file(&path).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cid::compute_cid;

    #[tokio::test]
    async fn test_basic_operations() {
        let limits = LimitsConfig::default();
        let mut storage = DiskStorage::new(limits).await.unwrap();

        let data = b"test block".to_vec();
        let cid = compute_cid(&data);

        // Put and get
        storage.put(&cid, data.clone()).await.unwrap();
        assert!(storage.contains(&cid).await.unwrap());
        assert_eq!(storage.get(&cid).await.unwrap(), Some(data.clone()));

        // Remove
        let removed = storage.remove(&cid).await.unwrap();
        assert_eq!(removed, Some(data));
        assert!(!storage.contains(&cid).await.unwrap());
    }

    #[tokio::test]
    async fn test_spill_to_disk() {
        // Very low memory threshold to force spilling
        let limits = LimitsConfig::default()
            .with_disk_spill_threshold(100)
            .with_max_memory_bytes(1024 * 1024);

        let mut storage = DiskStorage::new(limits).await.unwrap();

        // Add blocks that exceed threshold
        let mut cids = Vec::new();
        for i in 0..5 {
            let data = vec![i as u8; 50];
            let cid = compute_cid(&data);
            cids.push(cid);
            storage.put(&cid, data).await.unwrap();
        }

        // Some blocks should have spilled to disk
        assert!(!storage.disk_index.is_empty());

        // All blocks should still be retrievable
        for (i, cid) in cids.iter().enumerate() {
            let data = storage.get(cid).await.unwrap();
            assert!(data.is_some());
            assert_eq!(data.unwrap()[0], i as u8);
        }
    }

    #[tokio::test]
    async fn test_block_size_limit() {
        let limits = LimitsConfig::default().with_max_block_size(100);
        let mut storage = DiskStorage::new(limits).await.unwrap();

        let data = vec![0u8; 200];
        let cid = compute_cid(&data);

        let result = storage.put(&cid, data).await;
        assert!(matches!(result, Err(StorageError::BlockTooLarge { .. })));
    }

    #[tokio::test]
    async fn test_clear() {
        let limits = LimitsConfig::default().with_disk_spill_threshold(50);
        let mut storage = DiskStorage::new(limits).await.unwrap();

        // Add some blocks
        for i in 0..5 {
            let data = vec![i; 30];
            let cid = compute_cid(&data);
            storage.put(&cid, data).await.unwrap();
        }

        assert!(storage.block_count() > 0);

        storage.clear().await.unwrap();

        assert_eq!(storage.block_count(), 0);
        assert_eq!(storage.memory_usage(), 0);
    }
}
