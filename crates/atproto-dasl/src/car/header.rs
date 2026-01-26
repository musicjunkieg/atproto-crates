//! CAR file header structure.
//!
//! The header contains the CAR version and root CIDs.

use crate::Cid;
use crate::errors::CarError;
use crate::varint;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// CAR v1 header structure.
///
/// The header is encoded as DAG-CBOR and prefixed with a varint length.
///
/// # Format
///
/// ```text
/// varint(header_length) + dag-cbor({version: 1, roots: [cid1, cid2, ...]})
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarHeader {
    /// CAR format version (must be 1).
    pub version: u64,
    /// Root CIDs pointing to the top-level blocks.
    pub roots: Vec<Cid>,
}

impl CarHeader {
    /// Create a new CAR header with version 1.
    ///
    /// # Errors
    ///
    /// Returns `CarError::NoRoots` if the roots array is empty.
    pub fn new(roots: Vec<Cid>) -> Result<Self, CarError> {
        if roots.is_empty() {
            return Err(CarError::NoRoots);
        }
        Ok(Self { version: 1, roots })
    }

    /// Create a header with a single root.
    pub fn with_root(root: Cid) -> Self {
        Self {
            version: 1,
            roots: vec![root],
        }
    }

    /// Encode the header to DAG-CBOR bytes (without length prefix).
    ///
    /// # Errors
    ///
    /// Returns `CarError::DagCborEncode` if encoding fails.
    pub fn to_cbor_bytes(&self) -> Result<Vec<u8>, CarError> {
        crate::to_vec(self).map_err(CarError::DagCborEncode)
    }

    /// Encode the header with varint length prefix.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if encoding fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, CarError> {
        let cbor = self.to_cbor_bytes()?;
        let mut result = Vec::with_capacity(10 + cbor.len());

        // Write length prefix
        varint::write_varint(&mut result, cbor.len() as u64)?;

        // Write CBOR data
        result.extend_from_slice(&cbor);

        Ok(result)
    }

    /// Write header to a writer (with length prefix).
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

    /// Read header from a reader.
    ///
    /// # Errors
    ///
    /// Returns `CarError::InvalidHeader` if the header is malformed.
    /// Returns `CarError::UnsupportedVersion` if version is not 1.
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, CarError> {
        // Read length prefix
        let length = varint::read_varint(reader)?;

        // Sanity check on length
        if length > 1024 * 1024 {
            // 1MB max header
            return Err(CarError::InvalidHeader {
                reason: format!("header too large: {} bytes", length),
            });
        }

        // Read header bytes
        let mut header_bytes = vec![0u8; length as usize];
        reader.read_exact(&mut header_bytes)?;

        // Decode DAG-CBOR
        let header: CarHeader =
            crate::from_slice(&header_bytes).map_err(CarError::DagCborDecode)?;

        // Validate version
        if header.version != 1 {
            return Err(CarError::UnsupportedVersion {
                version: header.version,
            });
        }

        // Validate roots
        if header.roots.is_empty() {
            return Err(CarError::NoRoots);
        }

        Ok(header)
    }

    /// Decode header from bytes (with length prefix).
    ///
    /// Returns `(header, bytes_consumed)`.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if decoding fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<(Self, usize), CarError> {
        let mut cursor = std::io::Cursor::new(bytes);
        let header = Self::read_from(&mut cursor)?;
        Ok((header, cursor.position() as usize))
    }
}

/// Async versions of header I/O operations.
pub mod async_io {
    use super::*;
    use crate::varint::async_io as async_varint;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    /// Write header to an async writer.
    pub async fn write_header<W: AsyncWrite + Unpin>(
        writer: &mut W,
        header: &CarHeader,
    ) -> Result<usize, CarError> {
        let cbor = header.to_cbor_bytes()?;

        // Write length prefix
        let prefix_len = async_varint::write_varint(writer, cbor.len() as u64).await?;

        // Write CBOR data
        writer.write_all(&cbor).await?;

        Ok(prefix_len + cbor.len())
    }

    /// Read header from an async reader.
    pub async fn read_header<R: AsyncRead + Unpin>(reader: &mut R) -> Result<CarHeader, CarError> {
        // Read length prefix
        let length = async_varint::read_varint(reader).await?;

        // Sanity check on length
        if length > 1024 * 1024 {
            return Err(CarError::InvalidHeader {
                reason: format!("header too large: {} bytes", length),
            });
        }

        // Read header bytes
        let mut header_bytes = vec![0u8; length as usize];
        reader.read_exact(&mut header_bytes).await?;

        // Decode DAG-CBOR
        let header: CarHeader =
            crate::from_slice(&header_bytes).map_err(CarError::DagCborDecode)?;

        // Validate version
        if header.version != 1 {
            return Err(CarError::UnsupportedVersion {
                version: header.version,
            });
        }

        // Validate roots
        if header.roots.is_empty() {
            return Err(CarError::NoRoots);
        }

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cid::compute_cid;

    fn test_cid() -> Cid {
        let data = b"test data for cid";
        compute_cid(data).into()
    }

    #[test]
    fn test_header_roundtrip() {
        let cid = test_cid();
        let header = CarHeader::new(vec![cid.clone()]).unwrap();

        let bytes = header.to_bytes().unwrap();
        let (decoded, consumed) = CarHeader::from_bytes(&bytes).unwrap();

        assert_eq!(consumed, bytes.len());
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.roots.len(), 1);
    }

    #[test]
    fn test_header_multiple_roots() {
        let cids: Vec<Cid> = (0..3)
            .map(|i| {
                let data = format!("data {}", i);
                compute_cid(data.as_bytes()).into()
            })
            .collect();

        let header = CarHeader::new(cids.clone()).unwrap();
        let bytes = header.to_bytes().unwrap();
        let (decoded, _) = CarHeader::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.roots.len(), 3);
    }

    #[test]
    fn test_header_no_roots() {
        let result = CarHeader::new(vec![]);
        assert!(matches!(result, Err(CarError::NoRoots)));
    }

    #[test]
    fn test_header_read_write() {
        let cid = test_cid();
        let header = CarHeader::new(vec![cid]).unwrap();

        let mut buffer = Vec::new();
        header.write_to(&mut buffer).unwrap();

        let mut cursor = std::io::Cursor::new(&buffer);
        let decoded = CarHeader::read_from(&mut cursor).unwrap();

        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.roots.len(), 1);
    }

    #[tokio::test]
    async fn test_async_header_roundtrip() {
        let cid = test_cid();
        let header = CarHeader::new(vec![cid]).unwrap();

        let mut buffer = Vec::new();
        async_io::write_header(&mut buffer, &header).await.unwrap();

        let mut cursor = std::io::Cursor::new(&buffer);
        let decoded = async_io::read_header(&mut cursor).await.unwrap();

        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.roots.len(), 1);
    }
}
