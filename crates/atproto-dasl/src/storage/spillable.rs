//! Spillable buffer for streaming I/O with automatic spill-to-disk.
//!
//! [`SpillableBuffer`] provides a write-once, read-many pattern for streaming
//! data that may exceed memory limits. It uses an enum-based state machine
//! to transition from memory to disk storage when a threshold is exceeded.
//!
//! # Key Features
//!
//! - Enum-based state machine (Memory → Disk transition)
//! - Async-first design using `tokio::io::AsyncWrite` and `AsyncRead`
//! - Automatic cleanup via `tempfile` crate
//! - Content-length hint for pre-emptive disk allocation
//! - Error recovery: memory buffer retained until disk write confirmed
//!
//! # Example
//!
//! ```rust,ignore
//! use atproto_dasl::storage::SpillableBuffer;
//!
//! async fn example() -> std::io::Result<()> {
//!     // Create buffer with 1MB memory threshold
//!     let mut buffer = SpillableBuffer::new(1024 * 1024);
//!
//!     // Write streaming data
//!     buffer.write_chunk(b"some data").await?;
//!     buffer.write_chunk(b"more data").await?;
//!
//!     // Convert to reader
//!     let mut reader = buffer.into_reader().await?;
//!
//!     Ok(())
//! }
//! ```

use std::io::{self, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};
use tempfile::NamedTempFile;
use tokio::io::{AsyncRead, AsyncSeek, AsyncSeekExt, AsyncWriteExt, ReadBuf};

/// Internal state for spillable buffer.
enum BufferState {
    /// Data fits in memory.
    Memory { buffer: Vec<u8>, threshold: usize },
    /// Data has spilled to disk.
    Disk {
        file: tokio::fs::File,
        /// Original temp file handle for cleanup.
        _temp: NamedTempFile,
        /// Total bytes written (for len tracking).
        bytes_written: u64,
    },
}

/// A buffer that writes to memory until a threshold, then spills to disk.
///
/// Implements async write for streaming data, then can be converted
/// to an async reader for consumption.
///
/// # Example
///
/// ```rust,ignore
/// use atproto_dasl::storage::SpillableBuffer;
///
/// async fn example() -> std::io::Result<()> {
///     // Create buffer with 1MB memory threshold
///     let mut buffer = SpillableBuffer::new(1024 * 1024);
///
///     // Write streaming data
///     buffer.write_chunk(b"some data").await?;
///     buffer.write_chunk(b"more data").await?;
///
///     // Convert to reader
///     let mut reader = buffer.into_reader().await?;
///
///     Ok(())
/// }
/// ```
pub struct SpillableBuffer {
    state: BufferState,
    bytes_written: u64,
}

impl SpillableBuffer {
    /// Create a new spillable buffer with the given memory threshold.
    ///
    /// Data will be held in memory until `threshold` bytes are written,
    /// then automatically spill to a temporary file on disk.
    #[must_use]
    pub fn new(threshold: usize) -> Self {
        Self {
            state: BufferState::Memory {
                // Don't pre-allocate full threshold, grow as needed
                buffer: Vec::with_capacity(threshold.min(8 * 1024)),
                threshold,
            },
            bytes_written: 0,
        }
    }

    /// Create a buffer that immediately uses disk storage.
    ///
    /// Use when you know the data will exceed memory limits (e.g., from Content-Length).
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if the temporary file cannot be created.
    pub async fn new_disk() -> io::Result<Self> {
        let temp = NamedTempFile::new()?;
        let file = tokio::fs::File::from_std(temp.reopen()?);
        Ok(Self {
            state: BufferState::Disk {
                file,
                _temp: temp,
                bytes_written: 0,
            },
            bytes_written: 0,
        })
    }

    /// Create buffer with hint about expected size.
    ///
    /// If `size_hint` exceeds threshold, immediately allocates disk storage.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if disk storage is needed but cannot be created.
    pub async fn with_size_hint(threshold: usize, size_hint: Option<u64>) -> io::Result<Self> {
        match size_hint {
            Some(size) if size as usize > threshold => Self::new_disk().await,
            _ => Ok(Self::new(threshold)),
        }
    }

    /// Write a chunk of data to the buffer.
    ///
    /// If the total data exceeds the threshold, the buffer will automatically
    /// spill to disk.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if writing to disk fails.
    pub async fn write_chunk(&mut self, chunk: &[u8]) -> io::Result<()> {
        // Check if we need to spill first
        let needs_spill = match &self.state {
            BufferState::Memory { buffer, threshold } => buffer.len() + chunk.len() > *threshold,
            BufferState::Disk { .. } => false,
        };

        if needs_spill && let BufferState::Memory { threshold, .. } = &self.state {
            let threshold_val = *threshold;
            self.spill_to_disk_internal(threshold_val).await?;
        }

        // Now write to the appropriate state (no recursion needed)
        match &mut self.state {
            BufferState::Memory { buffer, .. } => {
                buffer.extend_from_slice(chunk);
                self.bytes_written += chunk.len() as u64;
                Ok(())
            }
            BufferState::Disk {
                file,
                bytes_written,
                ..
            } => {
                file.write_all(chunk).await?;
                *bytes_written += chunk.len() as u64;
                self.bytes_written += chunk.len() as u64;
                Ok(())
            }
        }
    }

    /// Force spill to disk, even if under threshold.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if the temporary file cannot be created or written.
    pub async fn spill_to_disk(&mut self) -> io::Result<()> {
        if let BufferState::Memory { threshold, .. } = &self.state {
            let threshold_val = *threshold;
            self.spill_to_disk_internal(threshold_val).await?;
        }
        Ok(())
    }

    /// Internal spill implementation that takes threshold as parameter.
    async fn spill_to_disk_internal(&mut self, _threshold: usize) -> io::Result<()> {
        if let BufferState::Memory { buffer, .. } = &mut self.state {
            let temp = NamedTempFile::new()?;
            let mut file = tokio::fs::File::from_std(temp.reopen()?);

            // Write existing buffer to disk
            file.write_all(buffer).await?;
            file.flush().await?;

            let bytes_written = buffer.len() as u64;

            self.state = BufferState::Disk {
                file,
                _temp: temp,
                bytes_written,
            };
        }
        Ok(())
    }

    /// Check if data has spilled to disk.
    #[must_use]
    pub fn is_on_disk(&self) -> bool {
        matches!(self.state, BufferState::Disk { .. })
    }

    /// Get total bytes written.
    #[must_use]
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Get current memory usage (0 if on disk).
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        match &self.state {
            BufferState::Memory { buffer, .. } => buffer.len(),
            BufferState::Disk { .. } => 0,
        }
    }

    /// Convert to an async reader for consuming the data.
    ///
    /// After calling this, the buffer cannot be written to.
    ///
    /// # Errors
    ///
    /// Returns `io::Error` if seeking to the beginning of a disk file fails.
    pub async fn into_reader(self) -> io::Result<SpillableReader> {
        match self.state {
            BufferState::Memory { buffer, .. } => Ok(SpillableReader::Memory {
                cursor: std::io::Cursor::new(buffer),
            }),
            BufferState::Disk {
                mut file,
                _temp,
                bytes_written,
            } => {
                file.seek(SeekFrom::Start(0)).await?;
                Ok(SpillableReader::Disk {
                    file,
                    _temp,
                    len: bytes_written,
                })
            }
        }
    }
}

/// Reader for consuming data from a SpillableBuffer.
///
/// Implements `AsyncRead` and `AsyncSeek` for flexible consumption.
pub enum SpillableReader {
    /// Reading from memory.
    Memory {
        /// Cursor over the in-memory buffer.
        cursor: std::io::Cursor<Vec<u8>>,
    },
    /// Reading from disk.
    Disk {
        /// File handle for reading.
        file: tokio::fs::File,
        /// Retained for cleanup on drop.
        _temp: NamedTempFile,
        /// Total length of data.
        len: u64,
    },
}

impl SpillableReader {
    /// Check if reading from disk.
    #[must_use]
    pub fn is_on_disk(&self) -> bool {
        matches!(self, Self::Disk { .. })
    }

    /// Get total length of data.
    #[must_use]
    pub fn len(&self) -> u64 {
        match self {
            Self::Memory { cursor } => cursor.get_ref().len() as u64,
            Self::Disk { len, .. } => *len,
        }
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl AsyncRead for SpillableReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            Self::Memory { cursor } => {
                // Sync read into async context (memory is instant)
                let data = cursor.get_ref();
                let pos = cursor.position() as usize;
                let remaining = &data[pos..];
                let to_read = remaining.len().min(buf.remaining());
                buf.put_slice(&remaining[..to_read]);
                cursor.set_position((pos + to_read) as u64);
                Poll::Ready(Ok(()))
            }
            Self::Disk { file, .. } => Pin::new(file).poll_read(cx, buf),
        }
    }
}

impl AsyncSeek for SpillableReader {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        match self.get_mut() {
            Self::Memory { cursor } => {
                use std::io::Seek;
                Seek::seek(cursor, position)?;
                Ok(())
            }
            Self::Disk { file, .. } => Pin::new(file).start_seek(position),
        }
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        match self.get_mut() {
            Self::Memory { cursor } => Poll::Ready(Ok(cursor.position())),
            Self::Disk { file, .. } => Pin::new(file).poll_complete(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_memory_only() {
        let mut buffer = SpillableBuffer::new(1024);

        buffer.write_chunk(b"hello ").await.unwrap();
        buffer.write_chunk(b"world").await.unwrap();

        assert!(!buffer.is_on_disk());
        assert_eq!(buffer.bytes_written(), 11);
        assert_eq!(buffer.memory_usage(), 11);

        let mut reader = buffer.into_reader().await.unwrap();
        assert!(!reader.is_on_disk());
        assert_eq!(reader.len(), 11);

        let mut result = String::new();
        reader.read_to_string(&mut result).await.unwrap();
        assert_eq!(result, "hello world");
    }

    #[tokio::test]
    async fn test_spill_to_disk() {
        let mut buffer = SpillableBuffer::new(10); // Very low threshold

        buffer.write_chunk(b"hello ").await.unwrap();
        assert!(!buffer.is_on_disk());

        buffer.write_chunk(b"world!").await.unwrap(); // Exceeds threshold
        assert!(buffer.is_on_disk());

        assert_eq!(buffer.bytes_written(), 12);
        assert_eq!(buffer.memory_usage(), 0);

        let mut reader = buffer.into_reader().await.unwrap();
        assert!(reader.is_on_disk());

        let mut result = String::new();
        reader.read_to_string(&mut result).await.unwrap();
        assert_eq!(result, "hello world!");
    }

    #[tokio::test]
    async fn test_new_disk() {
        let mut buffer = SpillableBuffer::new_disk().await.unwrap();
        assert!(buffer.is_on_disk());

        buffer.write_chunk(b"test data").await.unwrap();
        assert_eq!(buffer.bytes_written(), 9);

        let mut reader = buffer.into_reader().await.unwrap();
        let mut result = String::new();
        reader.read_to_string(&mut result).await.unwrap();
        assert_eq!(result, "test data");
    }

    #[tokio::test]
    async fn test_with_size_hint_large() {
        // Size hint larger than threshold should create disk buffer
        let buffer = SpillableBuffer::with_size_hint(1024, Some(2048))
            .await
            .unwrap();
        assert!(buffer.is_on_disk());
    }

    #[tokio::test]
    async fn test_with_size_hint_small() {
        // Size hint smaller than threshold should create memory buffer
        let buffer = SpillableBuffer::with_size_hint(1024, Some(512))
            .await
            .unwrap();
        assert!(!buffer.is_on_disk());
    }

    #[tokio::test]
    async fn test_with_size_hint_none() {
        // No size hint should create memory buffer
        let buffer = SpillableBuffer::with_size_hint(1024, None).await.unwrap();
        assert!(!buffer.is_on_disk());
    }

    #[tokio::test]
    async fn test_force_spill() {
        let mut buffer = SpillableBuffer::new(1024);

        buffer.write_chunk(b"small data").await.unwrap();
        assert!(!buffer.is_on_disk());

        buffer.spill_to_disk().await.unwrap();
        assert!(buffer.is_on_disk());

        // Can still read the data
        let mut reader = buffer.into_reader().await.unwrap();
        let mut result = String::new();
        reader.read_to_string(&mut result).await.unwrap();
        assert_eq!(result, "small data");
    }

    #[tokio::test]
    async fn test_large_data() {
        let mut buffer = SpillableBuffer::new(1024 * 1024); // 1MB threshold

        // Write 2MB of data
        let chunk = vec![b'x'; 1024];
        for _ in 0..2048 {
            buffer.write_chunk(&chunk).await.unwrap();
        }

        assert!(buffer.is_on_disk());
        assert_eq!(buffer.bytes_written(), 2 * 1024 * 1024);

        let reader = buffer.into_reader().await.unwrap();
        assert_eq!(reader.len(), 2 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_seek() {
        let mut buffer = SpillableBuffer::new(1024);
        buffer.write_chunk(b"0123456789").await.unwrap();

        let mut reader = buffer.into_reader().await.unwrap();

        // Read first 5 bytes
        let mut buf = [0u8; 5];
        reader.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"01234");

        // Seek back to start
        reader.seek(SeekFrom::Start(0)).await.unwrap();

        // Read again
        reader.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"01234");

        // Seek to position 7
        reader.seek(SeekFrom::Start(7)).await.unwrap();
        let mut buf = [0u8; 3];
        reader.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"789");
    }
}
