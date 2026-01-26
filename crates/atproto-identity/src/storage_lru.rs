//! LRU cache implementation for DID document storage.
//!
//! Thread-safe in-memory storage with automatic eviction of least recently used
//! DID documents when capacity is reached.

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use lru::LruCache;

use crate::errors::StorageError;
use crate::model::Document;
use crate::traits::DidDocumentStorage;

/// An LRU-based implementation of `DidDocumentStorage` that maintains a fixed-size cache of DID documents.
///
/// This storage implementation uses an LRU (Least Recently Used) cache to store DID documents
/// in memory with automatic eviction of the least recently accessed entries when the cache reaches
/// its capacity. This is ideal for scenarios where you want to cache frequently accessed DID documents
/// while keeping memory usage bounded.
///
/// ## Thread Safety
///
/// This implementation is thread-safe through the use of `Arc<Mutex<LruCache<String, Document>>>`.
/// All operations are protected by a mutex, ensuring safe concurrent access from multiple threads
/// or async tasks.
///
/// ## Cache Behavior
///
/// - **Get operations**: Move accessed entries to the front of the LRU order
/// - **Store operations**: Add new entries at the front, evicting the least recently used if at capacity
/// - **Delete operations**: Remove entries from the cache entirely
/// - **Capacity management**: Automatically evicts least recently used entries when capacity is exceeded
///
/// ## Use Cases
///
/// This implementation is particularly suitable for:
/// - Caching frequently accessed DID documents from PLC/web resolution
/// - Scenarios with bounded memory requirements
/// - Applications where some document lookup misses are acceptable
/// - High-performance applications requiring in-memory access
/// - Identity resolution caching layers
///
/// ## Limitations
///
/// - **Persistence**: Data is lost when the application restarts
/// - **Capacity**: Limited to the configured cache size
/// - **Cache misses**: Older entries may be evicted and need to be re-resolved
/// - **Memory usage**: All cached data is kept in memory
/// - **Document size**: Large documents consume more memory per entry
///
/// ## Examples
///
/// ```rust
/// use atproto_identity::storage_lru::LruDidDocumentStorage;
/// use atproto_identity::traits::DidDocumentStorage;
/// use atproto_identity::model::Document;
/// use std::num::NonZeroUsize;
/// use std::collections::HashMap;
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// // Create an LRU cache with capacity for 1000 documents
/// let storage = LruDidDocumentStorage::new(NonZeroUsize::new(1000).unwrap());
///
/// // Create a sample document
/// let document = Document {
///     context: vec![],
///     id: "did:plc:bv6ggog3tya2z3vxsub7hnal".to_string(),
///     also_known_as: vec!["at://alice.bsky.social".to_string()],
///     service: vec![], // simplified for example
///     verification_method: vec![], // simplified for example
///     extra: HashMap::new(),
/// };
///
/// // Store the document
/// storage.store_document(document.clone()).await?;
///
/// // Retrieve the document
/// let retrieved = storage.get_document_by_did("did:plc:bv6ggog3tya2z3vxsub7hnal").await?;
/// assert_eq!(retrieved.as_ref().map(|d| &d.id), Some(&document.id));
///
/// // Delete the document
/// storage.delete_document_by_did("did:plc:bv6ggog3tya2z3vxsub7hnal").await?;
/// let retrieved = storage.get_document_by_did("did:plc:bv6ggog3tya2z3vxsub7hnal").await?;
/// assert_eq!(retrieved, None);
/// # Ok::<(), anyhow::Error>(())
/// # }).unwrap();
/// ```
///
/// ## Capacity Planning
///
/// When choosing the cache capacity, consider:
/// - **Expected number of unique DIDs**: Size cache to hold frequently accessed documents
/// - **Memory constraints**: Each entry uses approximately (document size + DID length + overhead) bytes
/// - **Document complexity**: Documents with many services/keys use more memory
/// - **Access patterns**: Higher capacity reduces cache misses for varied access patterns
/// - **Performance requirements**: Larger caches may have slightly higher lookup times
///
/// ```rust
/// use atproto_identity::storage_lru::LruDidDocumentStorage;
/// use std::num::NonZeroUsize;
///
/// // Small cache for testing or low-memory environments
/// let small_cache = LruDidDocumentStorage::new(NonZeroUsize::new(100).unwrap());
///
/// // Medium cache for typical applications
/// let medium_cache = LruDidDocumentStorage::new(NonZeroUsize::new(10_000).unwrap());
///
/// // Large cache for high-traffic applications
/// let large_cache = LruDidDocumentStorage::new(NonZeroUsize::new(100_000).unwrap());
/// ```
#[derive(Clone)]
pub struct LruDidDocumentStorage {
    /// The LRU cache storing DID -> Document mappings, protected by a mutex for thread safety.
    ///
    /// We use DID as the key since the primary operation is looking up documents by DID.
    /// The cache is wrapped in Arc<Mutex<>> to ensure thread-safe access across multiple
    /// async tasks and threads.
    cache: Arc<Mutex<LruCache<String, Document>>>,
}

impl LruDidDocumentStorage {
    /// Creates a new `LruDidDocumentStorage` with the specified capacity.
    ///
    /// The capacity determines the maximum number of DID documents that can be stored
    /// in the cache. When the cache reaches this capacity, the least recently used
    /// entries will be automatically evicted to make room for new entries.
    ///
    /// # Arguments
    /// * `capacity` - The maximum number of DID documents to store. Must be greater than 0.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atproto_identity::storage_lru::LruDidDocumentStorage;
    /// use std::num::NonZeroUsize;
    ///
    /// // Create a cache that can hold up to 5000 DID documents
    /// let storage = LruDidDocumentStorage::new(NonZeroUsize::new(5000).unwrap());
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// - Larger capacities provide better cache hit rates but use more memory
    /// - The underlying LRU implementation has O(1) access time for all operations
    /// - Memory usage is approximately: capacity * (average_document_size + DID_size + overhead)
    /// - Document size varies based on number of services, verification methods, and aliases
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
        }
    }

    /// Returns the current number of entries in the cache.
    ///
    /// This method provides visibility into cache usage for monitoring and debugging purposes.
    /// The count represents the current number of DID documents stored in the cache.
    ///
    /// # Returns
    /// The number of entries currently stored in the cache.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atproto_identity::storage_lru::LruDidDocumentStorage;
    /// use atproto_identity::traits::DidDocumentStorage;
    /// use atproto_identity::model::Document;
    /// use std::num::NonZeroUsize;
    /// use std::collections::HashMap;
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let storage = LruDidDocumentStorage::new(NonZeroUsize::new(100).unwrap());
    /// assert_eq!(storage.len(), 0);
    ///
    /// let doc1 = Document {
    ///     context: vec![],
    ///     id: "did:plc:example1".to_string(),
    ///     also_known_as: vec![],
    ///     service: vec![],
    ///     verification_method: vec![],
    ///     extra: HashMap::new(),
    /// };
    /// storage.store_document(doc1).await?;
    /// assert_eq!(storage.len(), 1);
    ///
    /// let doc2 = Document {
    ///     context: vec![],
    ///     id: "did:plc:example2".to_string(),
    ///     also_known_as: vec![],
    ///     service: vec![],
    ///     verification_method: vec![],
    ///     extra: HashMap::new(),
    /// };
    /// storage.store_document(doc2).await?;
    /// assert_eq!(storage.len(), 2);
    /// # Ok::<(), anyhow::Error>(())
    /// # }).unwrap();
    /// ```
    pub fn len(&self) -> usize {
        self.cache.lock().unwrap().len()
    }

    /// Returns whether the cache is empty.
    ///
    /// # Returns
    /// `true` if the cache contains no entries, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atproto_identity::storage_lru::LruDidDocumentStorage;
    /// use std::num::NonZeroUsize;
    ///
    /// let storage = LruDidDocumentStorage::new(NonZeroUsize::new(100).unwrap());
    /// assert!(storage.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.cache.lock().unwrap().is_empty()
    }

    /// Returns the maximum capacity of the cache.
    ///
    /// This returns the capacity that was set when the cache was created and represents
    /// the maximum number of DID documents that can be stored before eviction occurs.
    ///
    /// # Returns
    /// The maximum capacity of the cache.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atproto_identity::storage_lru::LruDidDocumentStorage;
    /// use std::num::NonZeroUsize;
    ///
    /// let capacity = NonZeroUsize::new(500).unwrap();
    /// let storage = LruDidDocumentStorage::new(capacity);
    /// assert_eq!(storage.capacity().get(), 500);
    /// ```
    pub fn capacity(&self) -> NonZeroUsize {
        self.cache.lock().unwrap().cap()
    }

    /// Clears all entries from the cache.
    ///
    /// This method removes all DID documents from the cache, effectively resetting
    /// it to an empty state. This can be useful for testing or when you need to
    /// invalidate all cached data.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atproto_identity::storage_lru::LruDidDocumentStorage;
    /// use atproto_identity::traits::DidDocumentStorage;
    /// use atproto_identity::model::Document;
    /// use std::num::NonZeroUsize;
    /// use std::collections::HashMap;
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let storage = LruDidDocumentStorage::new(NonZeroUsize::new(100).unwrap());
    /// let document = Document {
    ///     context: vec![],
    ///     id: "did:plc:example".to_string(),
    ///     also_known_as: vec![],
    ///     service: vec![],
    ///     verification_method: vec![],
    ///     extra: HashMap::new(),
    /// };
    /// storage.store_document(document).await?;
    /// assert_eq!(storage.len(), 1);
    ///
    /// storage.clear();
    /// assert_eq!(storage.len(), 0);
    /// assert!(storage.is_empty());
    /// # Ok::<(), anyhow::Error>(())
    /// # }).unwrap();
    /// ```
    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
    }
}

#[async_trait::async_trait]
impl DidDocumentStorage for LruDidDocumentStorage {
    /// Retrieves a DID document associated with the given DID from the LRU cache.
    ///
    /// This method looks up the complete DID document that is currently cached for the provided
    /// DID. If the DID is found in the cache, the entry is moved to the front of the LRU
    /// order (marking it as recently used) and the document is returned.
    ///
    /// # Arguments
    /// * `did` - The DID to look up in the cache
    ///
    /// # Returns
    /// * `Ok(Some(document))` - If the DID is found in the cache
    /// * `Ok(None)` - If the DID is not found in the cache
    /// * `Err(error)` - If an error occurs (primarily mutex poisoning, which is very rare)
    ///
    /// # Cache Behavior
    ///
    /// When a document is successfully retrieved, it's marked as recently used in the LRU order,
    /// making it less likely to be evicted in future operations.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atproto_identity::storage_lru::LruDidDocumentStorage;
    /// use atproto_identity::traits::DidDocumentStorage;
    /// use atproto_identity::model::Document;
    /// use std::num::NonZeroUsize;
    /// use std::collections::HashMap;
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let storage = LruDidDocumentStorage::new(NonZeroUsize::new(100).unwrap());
    ///
    /// // Cache miss - DID not in cache
    /// let document = storage.get_document_by_did("did:plc:bv6ggog3tya2z3vxsub7hnal").await?;
    /// assert_eq!(document, None);
    ///
    /// // Add document to cache
    /// let doc = Document {
    ///     context: vec![],
    ///     id: "did:plc:bv6ggog3tya2z3vxsub7hnal".to_string(),
    ///     also_known_as: vec!["at://alice.bsky.social".to_string()],
    ///     service: vec![],
    ///     verification_method: vec![],
    ///     extra: HashMap::new(),
    /// };
    /// storage.store_document(doc.clone()).await?;
    ///
    /// // Cache hit - DID found in cache
    /// let document = storage.get_document_by_did("did:plc:bv6ggog3tya2z3vxsub7hnal").await?;
    /// assert_eq!(document.as_ref().map(|d| &d.id), Some(&doc.id));
    /// # Ok::<(), anyhow::Error>(())
    /// # }).unwrap();
    /// ```
    async fn get_document_by_did(&self, did: &str) -> Result<Option<Document>> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| StorageError::CacheLockFailedGet {
                details: e.to_string(),
            })?;

        Ok(cache.get(did).cloned())
    }

    /// Stores or updates a DID document in the LRU cache.
    ///
    /// This method stores a complete DID document in the cache. If the DID already exists
    /// in the cache, its document is updated and the entry is moved to the front of
    /// the LRU order. If the DID is new and the cache is at capacity, the least recently
    /// used entry is evicted to make room.
    ///
    /// # Arguments
    /// * `document` - The complete DID document to store. The document's `id` field
    ///               will be used as the storage key.
    ///
    /// # Returns
    /// * `Ok(())` - If the document was successfully stored
    /// * `Err(error)` - If an error occurs (primarily mutex poisoning, which is very rare)
    ///
    /// # Cache Behavior
    ///
    /// - If the cache is at capacity and this is a new DID, the least recently used entry is evicted
    /// - The new or updated entry is placed at the front of the LRU order
    /// - Existing entries with the same DID are updated in place
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atproto_identity::storage_lru::LruDidDocumentStorage;
    /// use atproto_identity::traits::DidDocumentStorage;
    /// use atproto_identity::model::Document;
    /// use std::num::NonZeroUsize;
    /// use std::collections::HashMap;
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let storage = LruDidDocumentStorage::new(NonZeroUsize::new(2).unwrap()); // Small cache for demo
    ///
    /// // Add first document
    /// let doc1 = Document {
    ///     context: vec![],
    ///     id: "did:plc:user1".to_string(),
    ///     also_known_as: vec!["at://alice.bsky.social".to_string()],
    ///     service: vec![],
    ///     verification_method: vec![],
    ///     extra: HashMap::new(),
    /// };
    /// storage.store_document(doc1).await?;
    /// assert_eq!(storage.len(), 1);
    ///
    /// // Add second document
    /// let doc2 = Document {
    ///     context: vec![],
    ///     id: "did:plc:user2".to_string(),
    ///     also_known_as: vec!["at://bob.bsky.social".to_string()],
    ///     service: vec![],
    ///     verification_method: vec![],
    ///     extra: HashMap::new(),
    /// };
    /// storage.store_document(doc2).await?;
    /// assert_eq!(storage.len(), 2);
    ///
    /// // Add third document - this will evict the least recently used entry (user1)
    /// let doc3 = Document {
    ///     context: vec![],
    ///     id: "did:plc:user3".to_string(),
    ///     also_known_as: vec!["at://charlie.bsky.social".to_string()],
    ///     service: vec![],
    ///     verification_method: vec![],
    ///     extra: HashMap::new(),
    /// };
    /// storage.store_document(doc3).await?;
    /// assert_eq!(storage.len(), 2); // Still at capacity
    ///
    /// // user1 should be evicted
    /// let document = storage.get_document_by_did("did:plc:user1").await?;
    /// assert_eq!(document, None);
    ///
    /// // user2 and user3 should still be present
    /// let doc2_retrieved = storage.get_document_by_did("did:plc:user2").await?;
    /// let doc3_retrieved = storage.get_document_by_did("did:plc:user3").await?;
    /// assert!(doc2_retrieved.is_some());
    /// assert!(doc3_retrieved.is_some());
    /// # Ok::<(), anyhow::Error>(())
    /// # }).unwrap();
    /// ```
    async fn store_document(&self, document: Document) -> Result<()> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| StorageError::CacheLockFailedStore {
                details: e.to_string(),
            })?;

        cache.put(document.id.clone(), document);
        Ok(())
    }

    /// Deletes a DID document from the LRU cache by DID.
    ///
    /// This method removes a DID document from the cache. If the DID exists in the cache,
    /// it is removed entirely, freeing up space for new entries.
    ///
    /// # Arguments
    /// * `did` - The DID identifying the document to delete
    ///
    /// # Returns
    /// * `Ok(())` - If the document was successfully deleted or didn't exist
    /// * `Err(error)` - If an error occurs (primarily mutex poisoning, which is very rare)
    ///
    /// # Cache Behavior
    ///
    /// - If the DID exists in the cache, it is removed completely
    /// - If the DID doesn't exist, the operation succeeds without error
    /// - Removing entries frees up capacity for new entries
    ///
    /// # Examples
    ///
    /// ```rust
    /// use atproto_identity::storage_lru::LruDidDocumentStorage;
    /// use atproto_identity::traits::DidDocumentStorage;
    /// use atproto_identity::model::Document;
    /// use std::num::NonZeroUsize;
    /// use std::collections::HashMap;
    ///
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let storage = LruDidDocumentStorage::new(NonZeroUsize::new(100).unwrap());
    ///
    /// // Add a document
    /// let document = Document {
    ///     context: vec![],
    ///     id: "did:plc:bv6ggog3tya2z3vxsub7hnal".to_string(),
    ///     also_known_as: vec!["at://alice.bsky.social".to_string()],
    ///     service: vec![],
    ///     verification_method: vec![],
    ///     extra: HashMap::new(),
    /// };
    /// storage.store_document(document).await?;
    /// let retrieved = storage.get_document_by_did("did:plc:bv6ggog3tya2z3vxsub7hnal").await?;
    /// assert!(retrieved.is_some());
    ///
    /// // Delete the document
    /// storage.delete_document_by_did("did:plc:bv6ggog3tya2z3vxsub7hnal").await?;
    /// let retrieved = storage.get_document_by_did("did:plc:bv6ggog3tya2z3vxsub7hnal").await?;
    /// assert_eq!(retrieved, None);
    ///
    /// // Deleting non-existent entry is safe
    /// storage.delete_document_by_did("did:plc:nonexistent").await?;
    /// # Ok::<(), anyhow::Error>(())
    /// # }).unwrap();
    /// ```
    async fn delete_document_by_did(&self, did: &str) -> Result<()> {
        let mut cache = self
            .cache
            .lock()
            .map_err(|e| StorageError::CacheLockFailedDelete {
                details: e.to_string(),
            })?;

        cache.pop(did);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::DidDocumentStorage;
    use std::collections::HashMap;
    use std::num::NonZeroUsize;

    fn create_test_document(did: &str, handle: &str) -> Document {
        Document {
            context: vec![],
            id: did.to_string(),
            also_known_as: vec![format!("at://{}", handle)],
            service: vec![],
            verification_method: vec![],
            extra: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_new_storage() {
        let storage = LruDidDocumentStorage::new(NonZeroUsize::new(100).unwrap());
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
        assert_eq!(storage.capacity().get(), 100);
    }

    #[tokio::test]
    async fn test_basic_operations() -> Result<()> {
        let storage = LruDidDocumentStorage::new(NonZeroUsize::new(10).unwrap());

        // Test get on empty cache
        let result = storage.get_document_by_did("did:plc:test").await?;
        assert_eq!(result, None);

        // Test store and get
        let document = create_test_document("did:plc:test", "test.handle");
        storage.store_document(document.clone()).await?;
        let result = storage.get_document_by_did("did:plc:test").await?;
        assert!(result.is_some());
        assert_eq!(result.as_ref().unwrap().id, document.id);
        assert_eq!(storage.len(), 1);

        // Test update existing
        let updated_document = create_test_document("did:plc:test", "updated.handle");
        storage.store_document(updated_document.clone()).await?;
        let result = storage.get_document_by_did("did:plc:test").await?;
        assert!(result.is_some());
        assert_eq!(
            result.as_ref().unwrap().also_known_as,
            updated_document.also_known_as
        );
        assert_eq!(storage.len(), 1); // Should still be 1

        // Test delete
        storage.delete_document_by_did("did:plc:test").await?;
        let result = storage.get_document_by_did("did:plc:test").await?;
        assert_eq!(result, None);
        assert_eq!(storage.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_lru_eviction() -> Result<()> {
        let storage = LruDidDocumentStorage::new(NonZeroUsize::new(2).unwrap());

        // Fill cache to capacity
        let doc1 = create_test_document("did:plc:user1", "user1.handle");
        let doc2 = create_test_document("did:plc:user2", "user2.handle");
        storage.store_document(doc1.clone()).await?;
        storage.store_document(doc2).await?;
        assert_eq!(storage.len(), 2);

        // Access user1 to make it recently used
        let _ = storage.get_document_by_did("did:plc:user1").await?;

        // Add user3, which should evict user2 (least recently used)
        let doc3 = create_test_document("did:plc:user3", "user3.handle");
        storage.store_document(doc3.clone()).await?;
        assert_eq!(storage.len(), 2);

        // user1 and user3 should be present, user2 should be evicted
        let result1 = storage.get_document_by_did("did:plc:user1").await?;
        assert!(result1.is_some());
        assert_eq!(result1.unwrap().also_known_as, doc1.also_known_as);

        let result3 = storage.get_document_by_did("did:plc:user3").await?;
        assert!(result3.is_some());
        assert_eq!(result3.unwrap().also_known_as, doc3.also_known_as);

        assert_eq!(storage.get_document_by_did("did:plc:user2").await?, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_clear() -> Result<()> {
        let storage = LruDidDocumentStorage::new(NonZeroUsize::new(10).unwrap());

        // Add some entries
        let doc1 = create_test_document("did:plc:user1", "user1.handle");
        let doc2 = create_test_document("did:plc:user2", "user2.handle");
        storage.store_document(doc1).await?;
        storage.store_document(doc2).await?;
        assert_eq!(storage.len(), 2);

        // Clear cache
        storage.clear();
        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());

        // Verify entries are gone
        assert_eq!(storage.get_document_by_did("did:plc:user1").await?, None);
        assert_eq!(storage.get_document_by_did("did:plc:user2").await?, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_thread_safety() -> Result<()> {
        let storage = Arc::new(LruDidDocumentStorage::new(NonZeroUsize::new(100).unwrap()));
        let mut handles = Vec::new();

        // Spawn multiple tasks that concurrently access the storage
        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let handle = tokio::spawn(async move {
                let did = format!("did:plc:user{}", i);
                let handle_name = format!("user{}.handle", i);
                let document = create_test_document(&did, &handle_name);

                // Store a document
                storage_clone.store_document(document.clone()).await?;

                // Get the document back
                let result = storage_clone.get_document_by_did(&did).await?;
                assert!(result.is_some());
                assert_eq!(result.unwrap().id, document.id);

                // Delete the document
                storage_clone.delete_document_by_did(&did).await?;
                let result = storage_clone.get_document_by_did(&did).await?;
                assert_eq!(result, None);

                Ok::<(), anyhow::Error>(())
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await??;
        }

        // Storage should be empty after all deletions
        assert_eq!(storage.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_delete_nonexistent() -> Result<()> {
        let storage = LruDidDocumentStorage::new(NonZeroUsize::new(10).unwrap());

        // Deleting non-existent entry should not error
        storage
            .delete_document_by_did("did:plc:nonexistent")
            .await?;
        assert_eq!(storage.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_document_content_preservation() -> Result<()> {
        let storage = LruDidDocumentStorage::new(NonZeroUsize::new(10).unwrap());

        // Create a document with complex content
        let mut complex_document = Document {
            context: vec![],
            id: "did:plc:complex".to_string(),
            also_known_as: vec![
                "at://alice.bsky.social".to_string(),
                "https://alice.example.com".to_string(),
            ],
            service: vec![],
            verification_method: vec![],
            extra: HashMap::new(),
        };
        complex_document.extra.insert(
            "custom_field".to_string(),
            serde_json::json!("custom_value"),
        );

        // Store and retrieve the document
        storage.store_document(complex_document.clone()).await?;
        let retrieved = storage.get_document_by_did("did:plc:complex").await?;

        // Verify all content is preserved
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, complex_document.id);
        assert_eq!(retrieved.also_known_as, complex_document.also_known_as);
        assert_eq!(retrieved.extra, complex_document.extra);

        Ok(())
    }
}
