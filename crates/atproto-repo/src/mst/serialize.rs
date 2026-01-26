//! DAG-CBOR serialization for MST nodes.

use super::MstNode;
use crate::compute_cid;
use crate::errors::MstError;
use cid::Cid;

impl MstNode {
    /// Serialize node to DAG-CBOR bytes.
    ///
    /// # Errors
    ///
    /// Returns `MstError::Serialization` if encoding fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, MstError> {
        atproto_dasl::to_vec(self).map_err(MstError::Serialization)
    }

    /// Deserialize node from DAG-CBOR bytes.
    ///
    /// # Errors
    ///
    /// Returns `MstError::Deserialization` if decoding fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MstError> {
        atproto_dasl::from_slice(bytes).map_err(MstError::Deserialization)
    }

    /// Compute the CID for this node.
    ///
    /// Uses CIDv1 with dag-cbor codec and sha2-256 hash.
    ///
    /// # Errors
    ///
    /// Returns `MstError::Serialization` if encoding fails.
    pub fn cid(&self) -> Result<Cid, MstError> {
        let bytes = self.to_bytes()?;
        Ok(compute_cid(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mst::TreeEntry;

    fn test_cid(data: &[u8]) -> atproto_dasl::Cid {
        compute_cid(data).into()
    }

    #[test]
    fn test_node_roundtrip() {
        let entries = vec![
            TreeEntry::first("app.bsky.feed.post/abc", test_cid(b"1")),
            TreeEntry::new(19, b"def".to_vec(), test_cid(b"2")),
        ];

        let node = MstNode::new(entries);
        let bytes = node.to_bytes().unwrap();
        let decoded = MstNode::from_bytes(&bytes).unwrap();

        assert_eq!(node, decoded);
    }

    #[test]
    fn test_node_with_left_roundtrip() {
        let left_cid = test_cid(b"left");
        let entries = vec![TreeEntry::first("key", test_cid(b"value"))];

        let node = MstNode::with_left(left_cid, entries);
        let bytes = node.to_bytes().unwrap();
        let decoded = MstNode::from_bytes(&bytes).unwrap();

        assert_eq!(node, decoded);
    }

    #[test]
    fn test_node_with_tree_roundtrip() {
        let tree_cid = test_cid(b"tree");
        let entries = vec![TreeEntry::with_tree(
            0,
            b"key".to_vec(),
            test_cid(b"value"),
            tree_cid,
        )];

        let node = MstNode::new(entries);
        let bytes = node.to_bytes().unwrap();
        let decoded = MstNode::from_bytes(&bytes).unwrap();

        assert_eq!(node, decoded);
    }

    #[test]
    fn test_empty_node_roundtrip() {
        let node = MstNode::empty();
        let bytes = node.to_bytes().unwrap();
        let decoded = MstNode::from_bytes(&bytes).unwrap();

        assert_eq!(node, decoded);
    }

    #[test]
    fn test_cid_deterministic() {
        let entries = vec![TreeEntry::first("key", test_cid(b"value"))];

        let node1 = MstNode::new(entries.clone());
        let node2 = MstNode::new(entries);

        let cid1 = node1.cid().unwrap();
        let cid2 = node2.cid().unwrap();

        assert_eq!(cid1, cid2);
    }

    #[test]
    fn test_different_nodes_different_cids() {
        let node1 = MstNode::new(vec![TreeEntry::first("key1", test_cid(b"1"))]);
        let node2 = MstNode::new(vec![TreeEntry::first("key2", test_cid(b"2"))]);

        let cid1 = node1.cid().unwrap();
        let cid2 = node2.cid().unwrap();

        assert_ne!(cid1, cid2);
    }
}
