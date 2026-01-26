//! Shared types for repository operations.

use crate::errors::RepoError;

/// Record path within a repository.
///
/// Records are identified by a collection (NSID) and record key (rkey).
///
/// # Example
///
/// ```rust
/// use atproto_repo::RecordPath;
///
/// let path = RecordPath::new("app.bsky.feed.post", "3jui7kd2z2y2e");
/// assert_eq!(path.collection, "app.bsky.feed.post");
/// assert_eq!(path.rkey, "3jui7kd2z2y2e");
/// assert_eq!(path.to_mst_key(), "app.bsky.feed.post/3jui7kd2z2y2e");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordPath {
    /// Collection NSID (e.g., "app.bsky.feed.post").
    pub collection: String,
    /// Record key (e.g., "3jui7kd2z2y2e").
    pub rkey: String,
}

impl RecordPath {
    /// Create from collection and rkey.
    #[must_use]
    pub fn new(collection: impl Into<String>, rkey: impl Into<String>) -> Self {
        Self {
            collection: collection.into(),
            rkey: rkey.into(),
        }
    }

    /// Parse from MST key string.
    ///
    /// # Errors
    ///
    /// Returns `RepoError` if the key is not in `collection/rkey` format.
    pub fn from_mst_key(key: &str) -> Result<Self, RepoError> {
        let parts: Vec<&str> = key.splitn(2, '/').collect();

        if parts.len() != 2 {
            return Err(RepoError::InvalidCommit {
                reason: format!("invalid MST key format: {}", key),
            });
        }

        let collection = parts[0];
        let rkey = parts[1];

        if collection.is_empty() {
            return Err(RepoError::InvalidCommit {
                reason: "collection cannot be empty".to_string(),
            });
        }

        if rkey.is_empty() {
            return Err(RepoError::InvalidCommit {
                reason: "rkey cannot be empty".to_string(),
            });
        }

        Ok(Self {
            collection: collection.to_string(),
            rkey: rkey.to_string(),
        })
    }

    /// Convert to MST key string.
    #[must_use]
    pub fn to_mst_key(&self) -> String {
        format!("{}/{}", self.collection, self.rkey)
    }

    /// Convert to AT-URI string.
    ///
    /// Requires the repository DID.
    #[must_use]
    pub fn to_at_uri(&self, did: &str) -> String {
        format!("at://{}/{}/{}", did, self.collection, self.rkey)
    }
}

impl std::fmt::Display for RecordPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.collection, self.rkey)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_path_new() {
        let path = RecordPath::new("app.bsky.feed.post", "abc123");
        assert_eq!(path.collection, "app.bsky.feed.post");
        assert_eq!(path.rkey, "abc123");
    }

    #[test]
    fn test_record_path_to_mst_key() {
        let path = RecordPath::new("app.bsky.feed.post", "abc123");
        assert_eq!(path.to_mst_key(), "app.bsky.feed.post/abc123");
    }

    #[test]
    fn test_record_path_from_mst_key() {
        let path = RecordPath::from_mst_key("app.bsky.feed.post/abc123").unwrap();
        assert_eq!(path.collection, "app.bsky.feed.post");
        assert_eq!(path.rkey, "abc123");
    }

    #[test]
    fn test_record_path_from_mst_key_with_slashes() {
        // rkey can contain slashes (though unusual)
        let path = RecordPath::from_mst_key("com.example/a/b/c").unwrap();
        assert_eq!(path.collection, "com.example");
        assert_eq!(path.rkey, "a/b/c");
    }

    #[test]
    fn test_record_path_invalid_no_slash() {
        let result = RecordPath::from_mst_key("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_record_path_invalid_empty_collection() {
        let result = RecordPath::from_mst_key("/rkey");
        assert!(result.is_err());
    }

    #[test]
    fn test_record_path_invalid_empty_rkey() {
        let result = RecordPath::from_mst_key("collection/");
        assert!(result.is_err());
    }

    #[test]
    fn test_record_path_to_at_uri() {
        let path = RecordPath::new("app.bsky.feed.post", "abc123");
        let uri = path.to_at_uri("did:plc:test");
        assert_eq!(uri, "at://did:plc:test/app.bsky.feed.post/abc123");
    }

    #[test]
    fn test_record_path_display() {
        let path = RecordPath::new("app.bsky.feed.post", "abc123");
        assert_eq!(format!("{}", path), "app.bsky.feed.post/abc123");
    }

    #[test]
    fn test_record_path_equality() {
        let path1 = RecordPath::new("app.bsky.feed.post", "abc");
        let path2 = RecordPath::new("app.bsky.feed.post", "abc");
        let path3 = RecordPath::new("app.bsky.feed.post", "def");

        assert_eq!(path1, path2);
        assert_ne!(path1, path3);
    }

    #[test]
    fn test_record_path_hash() {
        use std::collections::HashSet;

        let path1 = RecordPath::new("app.bsky.feed.post", "abc");
        let path2 = RecordPath::new("app.bsky.feed.post", "abc");
        let path3 = RecordPath::new("app.bsky.feed.post", "def");

        let mut set = HashSet::new();
        set.insert(path1.clone());
        set.insert(path2);
        set.insert(path3);

        assert_eq!(set.len(), 2);
        assert!(set.contains(&path1));
    }
}
