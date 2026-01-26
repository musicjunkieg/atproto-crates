//! CAR block structure and operations.
//!
//! Each block in a CAR file consists of a CID and the block data.

use crate::cid::{DAG_CBOR_CODEC, RAW_CODEC, SHA256_CODE, compute_cid};
use crate::errors::CarError;
use crate::varint;
use cid::Cid;
use std::io::{Read, Write};

/// Expected byte length of a DASL CID in binary form.
///
/// A DASL-conformant CID is always exactly 36 bytes:
/// 1 (version) + 1 (codec varint) + 1 (hash code) + 1 (digest length) + 32 (digest).
pub const DASL_CID_BYTE_LENGTH: usize = 36;

/// A single block in a CAR file.
///
/// # Format
///
/// ```text
/// varint(cid_bytes.len() + data.len()) + cid_bytes + data
/// ```
#[derive(Debug, Clone)]
pub struct CarBlock {
    /// Content identifier for this block.
    pub cid: Cid,
    /// Raw block data (typically DAG-CBOR encoded).
    pub data: Vec<u8>,
}

impl CarBlock {
    /// Create a new block from CID and data.
    #[must_use]
    pub fn new(cid: Cid, data: Vec<u8>) -> Self {
        Self { cid, data }
    }

    /// Create a block by computing CID from data.
    ///
    /// Uses CIDv1 with dag-cbor codec and sha2-256 hash.
    #[must_use]
    pub fn from_data(data: Vec<u8>) -> Self {
        let cid = compute_cid(&data);
        Self { cid, data }
    }

    /// Verify that the CID matches the data.
    ///
    /// # Errors
    ///
    /// Returns `CarError::CidMismatch` if the CID doesn't match.
    pub fn verify(&self) -> Result<(), CarError> {
        let computed = compute_cid(&self.data);
        if computed != self.cid {
            return Err(CarError::CidMismatch {
                expected: self.cid.to_string(),
                actual: computed.to_string(),
            });
        }
        Ok(())
    }

    /// Verify that the CID meets DASL format requirements.
    ///
    /// DASL requires CIDv1 with raw (0x55) or dag-cbor (0x71) codec and
    /// sha-256 (0x12) hash.
    ///
    /// # Errors
    ///
    /// Returns `CarError::InvalidCid` if the format is not DASL-compliant.
    pub fn verify_format(&self) -> Result<(), CarError> {
        if self.cid.version() != cid::Version::V1 {
            return Err(CarError::InvalidCid {
                reason: format!("expected CIDv1, got {:?}", self.cid.version()),
            });
        }

        let codec = self.cid.codec();
        if codec != DAG_CBOR_CODEC && codec != RAW_CODEC {
            return Err(CarError::InvalidCid {
                reason: format!(
                    "expected dag-cbor (0x71) or raw (0x55) codec, got 0x{:x}",
                    codec
                ),
            });
        }

        if self.cid.hash().code() != SHA256_CODE {
            return Err(CarError::InvalidCid {
                reason: format!(
                    "expected sha2-256 hash (0x12), got 0x{:x}",
                    self.cid.hash().code()
                ),
            });
        }

        let cid_len = self.cid.to_bytes().len();
        if cid_len != DASL_CID_BYTE_LENGTH {
            return Err(CarError::InvalidCid {
                reason: format!(
                    "DASL CID must be exactly {} bytes, got {}",
                    DASL_CID_BYTE_LENGTH, cid_len
                ),
            });
        }

        Ok(())
    }

    /// Get the raw CID bytes for encoding.
    #[must_use]
    pub fn cid_bytes(&self) -> Vec<u8> {
        self.cid.to_bytes()
    }

    /// Encode the block to bytes (with length prefix).
    ///
    /// # Errors
    ///
    /// Returns `CarError` if encoding fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, CarError> {
        let cid_bytes = self.cid_bytes();
        let total_len = cid_bytes.len() + self.data.len();

        let mut result = Vec::with_capacity(10 + total_len);

        // Write length prefix
        varint::write_varint(&mut result, total_len as u64)?;

        // Write CID bytes
        result.extend_from_slice(&cid_bytes);

        // Write data
        result.extend_from_slice(&self.data);

        Ok(result)
    }

    /// Write block to a writer (with length prefix).
    ///
    /// Returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if encoding or writing fails.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<usize, CarError> {
        let bytes = self.to_bytes()?;
        writer.write_all(&bytes)?;
        Ok(bytes.len())
    }

    /// Read a block from a reader.
    ///
    /// Returns `None` at EOF.
    ///
    /// # Errors
    ///
    /// Returns `CarError::InvalidBlock` if the block is malformed.
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Option<Self>, CarError> {
        // Try to read length prefix
        let length = match varint::read_varint(reader) {
            Ok(len) => len,
            Err(crate::errors::VarintError::UnexpectedEof) => return Ok(None),
            Err(e) => return Err(CarError::Varint(e)),
        };

        if length == 0 {
            return Err(CarError::InvalidBlock {
                reason: "block length is zero".to_string(),
            });
        }

        // Read all bytes
        let mut block_bytes = vec![0u8; length as usize];
        reader.read_exact(&mut block_bytes)?;

        // Parse CID from beginning of block
        let cid = Cid::try_from(&block_bytes[..]).map_err(|e| CarError::InvalidCid {
            reason: e.to_string(),
        })?;

        let cid_len = cid.to_bytes().len();
        if cid_len >= block_bytes.len() {
            return Err(CarError::InvalidBlock {
                reason: "block has no data after CID".to_string(),
            });
        }

        // Remaining bytes are the data
        let data = block_bytes[cid_len..].to_vec();

        Ok(Some(Self { cid, data }))
    }

    /// Decode a block from bytes (with length prefix).
    ///
    /// Returns `(block, bytes_consumed)`.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if decoding fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), CarError> {
        let mut cursor = std::io::Cursor::new(bytes);
        match Self::read_from(&mut cursor)? {
            Some(block) => Ok((block, cursor.position() as usize)),
            None => Err(CarError::InvalidBlock {
                reason: "unexpected end of input".to_string(),
            }),
        }
    }
}

/// Async versions of block I/O operations.
pub mod async_io {
    use super::*;
    use crate::varint::async_io as async_varint;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    /// Write block to an async writer.
    pub async fn write_block<W: AsyncWrite + Unpin>(
        writer: &mut W,
        block: &CarBlock,
    ) -> Result<usize, CarError> {
        let cid_bytes = block.cid_bytes();
        let total_len = cid_bytes.len() + block.data.len();

        // Write length prefix
        let prefix_len = async_varint::write_varint(writer, total_len as u64).await?;

        // Write CID bytes
        writer.write_all(&cid_bytes).await?;

        // Write data
        writer.write_all(&block.data).await?;

        Ok(prefix_len + total_len)
    }

    /// Read a block from an async reader.
    ///
    /// Returns `None` at EOF.
    pub async fn read_block<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<Option<CarBlock>, CarError> {
        // Try to read length prefix
        let length = match async_varint::read_varint(reader).await {
            Ok(len) => len,
            Err(crate::errors::VarintError::UnexpectedEof) => return Ok(None),
            Err(e) => return Err(CarError::Varint(e)),
        };

        if length == 0 {
            return Err(CarError::InvalidBlock {
                reason: "block length is zero".to_string(),
            });
        }

        // Read all bytes
        let mut block_bytes = vec![0u8; length as usize];
        reader.read_exact(&mut block_bytes).await?;

        // Parse CID from beginning of block
        let cid = Cid::try_from(&block_bytes[..]).map_err(|e| CarError::InvalidCid {
            reason: e.to_string(),
        })?;

        let cid_len = cid.to_bytes().len();
        if cid_len >= block_bytes.len() {
            return Err(CarError::InvalidBlock {
                reason: "block has no data after CID".to_string(),
            });
        }

        // Remaining bytes are the data
        let data = block_bytes[cid_len..].to_vec();

        Ok(Some(CarBlock { cid, data }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;

    #[test]
    fn test_block_from_data() {
        let data = b"test block data".to_vec();
        let block = CarBlock::from_data(data.clone());

        assert_eq!(block.data, data);
        assert!(block.verify().is_ok());
        assert!(block.verify_format().is_ok());
    }

    #[test]
    fn test_block_roundtrip() {
        let data = b"roundtrip test".to_vec();
        let block = CarBlock::from_data(data);

        let bytes = block.to_bytes().unwrap();
        let (decoded, consumed) = CarBlock::from_bytes(&bytes).unwrap();

        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded.cid, block.cid);
        assert_eq!(decoded.data, block.data);
    }

    #[test]
    fn test_block_read_write() {
        let data = b"read write test".to_vec();
        let block = CarBlock::from_data(data);

        let mut buffer = Vec::new();
        block.write_to(&mut buffer).unwrap();

        let mut cursor = std::io::Cursor::new(&buffer);
        let decoded = CarBlock::read_from(&mut cursor).unwrap().unwrap();

        assert_eq!(decoded.cid, block.cid);
        assert_eq!(decoded.data, block.data);
    }

    #[test]
    fn test_block_verify_mismatch() {
        let data = b"original data".to_vec();
        let mut block = CarBlock::from_data(data);

        // Corrupt the data
        block.data = b"corrupted data".to_vec();

        assert!(matches!(block.verify(), Err(CarError::CidMismatch { .. })));
    }

    #[test]
    fn test_block_eof() {
        let empty: &[u8] = &[];
        let mut cursor = std::io::Cursor::new(empty);
        let result = CarBlock::read_from(&mut cursor).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_verify_format_raw_codec() {
        let data = b"raw binary blob".to_vec();
        let cid = crate::cid::compute_raw_cid(&data);
        let block = CarBlock::new(cid, data);
        assert!(block.verify_format().is_ok());
    }

    #[test]
    fn test_verify_format_invalid_codec() {
        use multihash::Multihash;
        let data = b"test".to_vec();
        // Create a CID with an unsupported codec (0x50)
        let hash = sha2::Sha256::digest(&data);
        let mh = Multihash::<64>::wrap(SHA256_CODE, &hash).unwrap();
        let cid = Cid::new_v1(0x50, mh);
        let block = CarBlock::new(cid, data);
        assert!(matches!(
            block.verify_format(),
            Err(CarError::InvalidCid { .. })
        ));
    }

    #[tokio::test]
    async fn test_async_block_roundtrip() {
        let data = b"async roundtrip".to_vec();
        let block = CarBlock::from_data(data);

        let mut buffer = Vec::new();
        async_io::write_block(&mut buffer, &block).await.unwrap();

        let mut cursor = std::io::Cursor::new(&buffer);
        let decoded = async_io::read_block(&mut cursor).await.unwrap().unwrap();

        assert_eq!(decoded.cid, block.cid);
        assert_eq!(decoded.data, block.data);
    }

    #[tokio::test]
    async fn test_async_eof() {
        let empty: &[u8] = &[];
        let mut cursor = std::io::Cursor::new(empty);
        let result = async_io::read_block(&mut cursor).await.unwrap();
        assert!(result.is_none());
    }
}
