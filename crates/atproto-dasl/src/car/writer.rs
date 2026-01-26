//! Async streaming CAR v1 writer.
//!
//! The writer streams blocks to an async destination, computing CIDs
//! automatically if needed.

use super::CarConfig;
use super::block::async_io as block_io;
use super::header::async_io as header_io;
use super::{CarBlock, CarHeader};
use crate::Cid;
use crate::cid::compute_cid;
use crate::errors::CarError;
use crate::storage::BlockStorage;
use tokio::io::AsyncWrite;

/// Async streaming CAR v1 writer.
///
/// Writes blocks to an async destination with optional verification.
///
/// # Example
///
/// ```rust,ignore
/// use atproto_dasl::{CarWriter, CarBlock};
/// use tokio::fs::File;
///
/// async fn example() -> anyhow::Result<()> {
///     let file = File::create("output.car").await?;
///     let roots = vec![root_cid];
///     let mut writer = CarWriter::new(file, roots).await?;
///
///     let block = CarBlock::from_data(b"some data".to_vec());
///     writer.write_block(&block).await?;
///
///     writer.finish().await?;
///     Ok(())
/// }
/// ```
pub struct CarWriter<W> {
    writer: Option<W>,
    config: CarConfig,
    header_written: bool,
    blocks_written: usize,
    bytes_written: u64,
}

impl<W: AsyncWrite + Unpin> CarWriter<W> {
    /// Create a new writer with specified roots.
    ///
    /// Writes the header immediately.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if the header cannot be written.
    pub async fn new(writer: W, roots: Vec<Cid>) -> Result<Self, CarError> {
        Self::with_config(writer, roots, CarConfig::default()).await
    }

    /// Create with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if the header cannot be written.
    pub async fn with_config(
        mut writer: W,
        roots: Vec<Cid>,
        config: CarConfig,
    ) -> Result<Self, CarError> {
        let header = CarHeader::new(roots)?;
        let header_bytes = header_io::write_header(&mut writer, &header).await?;

        Ok(Self {
            writer: Some(writer),
            config,
            header_written: true,
            blocks_written: 0,
            bytes_written: header_bytes as u64,
        })
    }

    /// Write a block with explicit CID.
    ///
    /// # Verification
    ///
    /// If `config.verify_cids` is true, verifies that the CID matches the data
    /// before writing.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if verification fails or writing fails.
    pub async fn write_block(&mut self, block: &CarBlock) -> Result<usize, CarError> {
        let writer = self.writer.as_mut().ok_or_else(|| CarError::InvalidBlock {
            reason: "writer has been consumed".to_string(),
        })?;

        // Verify CID if configured
        if self.config.verify_cids {
            block.verify()?;
        }

        // Verify format if configured
        if self.config.strict_cid_format {
            block.verify_format()?;
        }

        let bytes_written = block_io::write_block(writer, block).await?;
        self.blocks_written += 1;
        self.bytes_written += bytes_written as u64;

        Ok(bytes_written)
    }

    /// Write data and compute CID automatically.
    ///
    /// Returns the computed CID.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if writing fails.
    pub async fn write_data(&mut self, data: &[u8]) -> Result<cid::Cid, CarError> {
        let cid = compute_cid(data);
        let block = CarBlock::new(cid, data.to_vec());
        self.write_block(&block).await?;
        Ok(cid)
    }

    /// Stream blocks from storage to writer.
    ///
    /// Writes all blocks from storage. Note: the order of blocks
    /// is determined by the storage's `cids()` iterator.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if storage access or writing fails.
    pub async fn write_from_storage<S: BlockStorage>(
        &mut self,
        storage: &S,
    ) -> Result<usize, CarError> {
        let cids: Vec<cid::Cid> = storage.cids().collect();
        let mut blocks_written = 0;

        for cid in cids {
            if let Some(data) = storage.get(&cid).await? {
                let block = CarBlock::new(cid, data);
                self.write_block(&block).await?;
                blocks_written += 1;
            }
        }

        Ok(blocks_written)
    }

    /// Finish writing and return the underlying writer.
    ///
    /// Flushes any pending data.
    ///
    /// # Errors
    ///
    /// Returns `CarError` if flushing fails.
    pub async fn finish(mut self) -> Result<W, CarError> {
        use tokio::io::AsyncWriteExt;

        let mut writer = self.writer.take().ok_or_else(|| CarError::InvalidBlock {
            reason: "writer has already been consumed".to_string(),
        })?;

        writer.flush().await?;
        Ok(writer)
    }

    /// Get count of blocks written.
    #[must_use]
    pub fn blocks_written(&self) -> usize {
        self.blocks_written
    }

    /// Get total bytes written (including header).
    #[must_use]
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Check if header has been written.
    #[must_use]
    pub fn header_written(&self) -> bool {
        self.header_written
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::car::CarReader;
    use crate::storage::MemoryStorage;
    use std::io::Cursor;

    #[tokio::test]
    async fn test_write_single_block() {
        let data = b"test block".to_vec();
        let block = CarBlock::from_data(data.clone());
        let root = block.cid;

        let mut buffer = Vec::new();
        let mut writer = CarWriter::new(&mut buffer, vec![root.into()])
            .await
            .unwrap();
        writer.write_block(&block).await.unwrap();
        writer.finish().await.unwrap();

        // Verify by reading back
        let mut reader = CarReader::new(Cursor::new(&buffer)).await.unwrap();
        assert_eq!(reader.roots().len(), 1);

        let read_block = reader.next_block().await.unwrap().unwrap();
        assert_eq!(read_block.cid, block.cid);
        assert_eq!(read_block.data, data);
    }

    #[tokio::test]
    async fn test_write_multiple_blocks() {
        let blocks: Vec<CarBlock> = (0..5)
            .map(|i| CarBlock::from_data(format!("block {}", i).into_bytes()))
            .collect();

        let root = blocks[0].cid;

        let mut buffer = Vec::new();
        let mut writer = CarWriter::new(&mut buffer, vec![root.into()])
            .await
            .unwrap();

        for block in &blocks {
            writer.write_block(block).await.unwrap();
        }

        assert_eq!(writer.blocks_written(), 5);
        writer.finish().await.unwrap();

        // Verify by reading back
        let mut reader = CarReader::new(Cursor::new(&buffer)).await.unwrap();
        for expected in &blocks {
            let read_block = reader.next_block().await.unwrap().unwrap();
            assert_eq!(read_block.cid, expected.cid);
            assert_eq!(read_block.data, expected.data);
        }
    }

    #[tokio::test]
    async fn test_write_data_computes_cid() {
        let data = b"auto cid data";

        let mut buffer = Vec::new();
        let expected_cid = compute_cid(data);

        let mut writer = CarWriter::new(&mut buffer, vec![expected_cid.into()])
            .await
            .unwrap();
        let cid = writer.write_data(data).await.unwrap();
        writer.finish().await.unwrap();

        assert_eq!(cid, expected_cid);
    }

    #[tokio::test]
    async fn test_write_from_storage() {
        let mut storage = MemoryStorage::new();

        // Add blocks to storage
        let blocks: Vec<(cid::Cid, Vec<u8>)> = (0..3)
            .map(|i| {
                let data = format!("block {}", i).into_bytes();
                let cid = compute_cid(&data);
                (cid, data)
            })
            .collect();

        for (cid, data) in &blocks {
            storage.put(cid, data.clone()).await.unwrap();
        }

        // Write from storage
        let root = blocks[0].0;
        let mut buffer = Vec::new();
        let mut writer = CarWriter::new(&mut buffer, vec![root.into()])
            .await
            .unwrap();
        writer.write_from_storage(&storage).await.unwrap();
        writer.finish().await.unwrap();

        // Verify
        let mut reader = CarReader::new(Cursor::new(&buffer)).await.unwrap();
        let mut read_count = 0;
        while reader.next_block().await.unwrap().is_some() {
            read_count += 1;
        }
        assert_eq!(read_count, 3);
    }

    #[tokio::test]
    async fn test_verify_on_write() {
        let data = b"test data".to_vec();
        let block = CarBlock::from_data(data);
        let root = block.cid;

        // Create a block with mismatched CID
        let bad_block = CarBlock::new(block.cid, b"different data".to_vec());

        let mut buffer = Vec::new();
        let mut writer = CarWriter::new(&mut buffer, vec![root.into()])
            .await
            .unwrap();

        // This should fail verification
        let result = writer.write_block(&bad_block).await;
        assert!(matches!(result, Err(CarError::CidMismatch { .. })));
    }

    #[tokio::test]
    async fn test_no_verify() {
        let data = b"test data".to_vec();
        let block = CarBlock::from_data(data);
        let root = block.cid;

        // Create a block with mismatched CID
        let bad_block = CarBlock::new(block.cid, b"different data".to_vec());

        let config = CarConfig::no_verification();
        let mut buffer = Vec::new();
        let mut writer = CarWriter::with_config(&mut buffer, vec![root.into()], config)
            .await
            .unwrap();

        // This should succeed without verification
        writer.write_block(&bad_block).await.unwrap();
    }
}
