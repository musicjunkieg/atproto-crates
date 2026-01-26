//! Unsigned LEB128 varint encoding for CAR format.
//!
//! The CAR format uses unsigned LEB128 (Little Endian Base 128) varints for
//! length prefixes. This is the same encoding used in Protocol Buffers and
//! WebAssembly.
//!
//! # Encoding Algorithm
//!
//! 1. Take the low 7 bits of the value
//! 2. If there are more bits to encode, set the high bit (0x80) as continuation flag
//! 3. Write the byte and shift value right by 7
//! 4. Repeat until value is zero
//!
//! # Decoding Algorithm
//!
//! 1. Read a byte
//! 2. Take low 7 bits and add to result at appropriate position
//! 3. If high bit is set, continue reading
//! 4. If more than 9 bytes read, error (overflow for u64)
//!
//! # Example
//!
//! ```rust
//! use atproto_dasl::varint::{encode_varint, decode_varint, varint_size};
//!
//! // Small values use 1 byte
//! let mut buf = [0u8; 10];
//! let written = encode_varint(&mut buf, 127).unwrap();
//! assert_eq!(written, 1);
//! assert_eq!(buf[0], 127);
//!
//! // Values >= 128 use multiple bytes
//! let written = encode_varint(&mut buf, 300).unwrap();
//! assert_eq!(written, 2);
//!
//! // Decode back
//! let (value, consumed) = decode_varint(&buf[..written]).unwrap();
//! assert_eq!(value, 300);
//! assert_eq!(consumed, 2);
//! ```

use crate::errors::VarintError;
use std::io::{Read, Write};

/// Maximum number of bytes for a u64 varint (ceil(64/7) = 10, but we limit to 9).
const MAX_VARINT_BYTES: usize = 9;

/// Encode a u64 to a writer using unsigned LEB128.
///
/// Returns the number of bytes written.
///
/// # Errors
///
/// Returns `std::io::Error` if writing fails.
pub fn write_varint<W: Write>(writer: &mut W, mut value: u64) -> std::io::Result<usize> {
    let mut bytes_written = 0;

    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;

        if value != 0 {
            byte |= 0x80; // Set continuation bit
        }

        writer.write_all(&[byte])?;
        bytes_written += 1;

        if value == 0 {
            break;
        }
    }

    Ok(bytes_written)
}

/// Decode a u64 from a reader using unsigned LEB128.
///
/// # Errors
///
/// Returns `VarintError::UnexpectedEof` if the reader ends before the varint is complete.
/// Returns `VarintError::Overflow` if the varint exceeds 9 bytes.
pub fn read_varint<R: Read>(reader: &mut R) -> Result<u64, VarintError> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    let mut byte_buf = [0u8; 1];

    loop {
        if reader.read_exact(&mut byte_buf).is_err() {
            return Err(VarintError::UnexpectedEof);
        }

        let byte = byte_buf[0];
        let value = (byte & 0x7F) as u64;

        // Check for overflow before shifting
        if shift >= 63 && value > 1 {
            return Err(VarintError::Overflow);
        }

        result |= value << shift;
        shift += 7;

        if byte & 0x80 == 0 {
            break;
        }

        if shift >= 64 {
            return Err(VarintError::Overflow);
        }
    }

    Ok(result)
}

/// Encode a u64 to a buffer using unsigned LEB128.
///
/// Returns the number of bytes written.
///
/// # Errors
///
/// Returns `VarintError::Overflow` if the buffer is too small.
pub fn encode_varint(buffer: &mut [u8], mut value: u64) -> Result<usize, VarintError> {
    let mut pos = 0;

    loop {
        if pos >= buffer.len() {
            return Err(VarintError::Overflow);
        }

        let mut byte = (value & 0x7F) as u8;
        value >>= 7;

        if value != 0 {
            byte |= 0x80;
        }

        buffer[pos] = byte;
        pos += 1;

        if value == 0 {
            break;
        }
    }

    Ok(pos)
}

/// Decode a u64 from a slice using unsigned LEB128.
///
/// Returns `(value, bytes_consumed)`.
///
/// # Errors
///
/// Returns `VarintError::UnexpectedEof` if the slice is too short.
/// Returns `VarintError::Overflow` if the varint exceeds 9 bytes.
pub fn decode_varint(bytes: &[u8]) -> Result<(u64, usize), VarintError> {
    if bytes.is_empty() {
        return Err(VarintError::UnexpectedEof);
    }

    let mut result: u64 = 0;
    let mut shift: u32 = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        if i >= MAX_VARINT_BYTES {
            return Err(VarintError::Overflow);
        }

        let value = (byte & 0x7F) as u64;

        // Check for overflow before shifting
        if shift >= 63 && value > 1 {
            return Err(VarintError::Overflow);
        }

        result |= value << shift;

        if byte & 0x80 == 0 {
            return Ok((result, i + 1));
        }

        shift += 7;
    }

    Err(VarintError::UnexpectedEof)
}

/// Decode a u64 from a slice with strict minimal encoding check.
///
/// Returns `(value, bytes_consumed)`.
///
/// # Errors
///
/// Returns `VarintError::NonMinimal` if the encoding uses more bytes than necessary.
pub fn decode_varint_strict(bytes: &[u8]) -> Result<(u64, usize), VarintError> {
    let (value, consumed) = decode_varint(bytes)?;

    // Check that the encoding is minimal
    let expected_size = varint_size(value);
    if consumed != expected_size {
        return Err(VarintError::NonMinimal);
    }

    Ok((value, consumed))
}

/// Calculate the number of bytes needed to encode a value as a varint.
#[must_use]
pub fn varint_size(value: u64) -> usize {
    if value == 0 {
        return 1;
    }

    // Number of bits needed, divided by 7 (rounded up)
    let bits = 64 - value.leading_zeros();
    bits.div_ceil(7) as usize
}

/// Async version of varint read for tokio.
pub mod async_io {
    use super::VarintError;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    /// Encode a u64 to an async writer using unsigned LEB128.
    pub async fn write_varint<W: AsyncWrite + Unpin>(
        writer: &mut W,
        mut value: u64,
    ) -> std::io::Result<usize> {
        let mut bytes_written = 0;

        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;

            if value != 0 {
                byte |= 0x80;
            }

            writer.write_all(&[byte]).await?;
            bytes_written += 1;

            if value == 0 {
                break;
            }
        }

        Ok(bytes_written)
    }

    /// Decode a u64 from an async reader using unsigned LEB128.
    pub async fn read_varint<R: AsyncRead + Unpin>(reader: &mut R) -> Result<u64, VarintError> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        let mut byte_buf = [0u8; 1];

        loop {
            if reader.read_exact(&mut byte_buf).await.is_err() {
                return Err(VarintError::UnexpectedEof);
            }

            let byte = byte_buf[0];
            let value = (byte & 0x7F) as u64;

            if shift >= 63 && value > 1 {
                return Err(VarintError::Overflow);
            }

            result |= value << shift;
            shift += 7;

            if byte & 0x80 == 0 {
                break;
            }

            if shift >= 64 {
                return Err(VarintError::Overflow);
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_encode_decode_zero() {
        let mut buf = [0u8; 10];
        let written = encode_varint(&mut buf, 0).unwrap();
        assert_eq!(written, 1);
        assert_eq!(buf[0], 0);

        let (value, consumed) = decode_varint(&buf[..written]).unwrap();
        assert_eq!(value, 0);
        assert_eq!(consumed, 1);
    }

    #[test]
    fn test_encode_decode_single_byte() {
        for value in [1u64, 63, 127] {
            let mut buf = [0u8; 10];
            let written = encode_varint(&mut buf, value).unwrap();
            assert_eq!(written, 1);

            let (decoded, consumed) = decode_varint(&buf[..written]).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(consumed, 1);
        }
    }

    #[test]
    fn test_encode_decode_two_bytes() {
        for value in [128u64, 255, 300, 16383] {
            let mut buf = [0u8; 10];
            let written = encode_varint(&mut buf, value).unwrap();
            assert_eq!(written, 2, "value {} should take 2 bytes", value);

            let (decoded, consumed) = decode_varint(&buf[..written]).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(consumed, 2);
        }
    }

    #[test]
    fn test_encode_decode_large_values() {
        // Maximum value that fits in 9 bytes is (1 << 63) - 1
        let max_9_byte_value = (1u64 << 63) - 1;

        let test_values = [
            16384u64,         // 3 bytes
            2097151,          // 3 bytes
            268435455,        // 4 bytes
            u32::MAX as u64,  // 5 bytes
            max_9_byte_value, // 9 bytes
        ];

        for value in test_values {
            let mut buf = [0u8; 10];
            let written = encode_varint(&mut buf, value).unwrap();

            let (decoded, consumed) = decode_varint(&buf[..written]).unwrap();
            assert_eq!(decoded, value, "failed for value {}", value);
            assert_eq!(consumed, written);
        }
    }

    #[test]
    fn test_varint_size() {
        assert_eq!(varint_size(0), 1);
        assert_eq!(varint_size(1), 1);
        assert_eq!(varint_size(127), 1);
        assert_eq!(varint_size(128), 2);
        assert_eq!(varint_size(16383), 2);
        assert_eq!(varint_size(16384), 3);
        // 9 bytes can hold up to 63 bits, 10 bytes for full u64
        assert_eq!(varint_size((1u64 << 63) - 1), 9);
        assert_eq!(varint_size(u64::MAX), 10);
    }

    #[test]
    fn test_reader_writer() {
        let values = [0u64, 127, 128, 300, 16384, u64::MAX];

        for value in values {
            let mut buf = Vec::new();
            let written = write_varint(&mut buf, value).unwrap();

            let mut cursor = Cursor::new(&buf);
            let decoded = read_varint(&mut cursor).unwrap();

            assert_eq!(decoded, value);
            assert_eq!(buf.len(), written);
        }
    }

    #[test]
    fn test_unexpected_eof() {
        // Empty input
        let result = decode_varint(&[]);
        assert!(matches!(result, Err(VarintError::UnexpectedEof)));

        // Incomplete multi-byte varint (has continuation bit set)
        let result = decode_varint(&[0x80]);
        assert!(matches!(result, Err(VarintError::UnexpectedEof)));

        let result = decode_varint(&[0x80, 0x80]);
        assert!(matches!(result, Err(VarintError::UnexpectedEof)));
    }

    #[test]
    fn test_overflow() {
        // 10-byte varint (too long)
        let overflow_bytes = [0x80u8; 10];
        let result = decode_varint(&overflow_bytes);
        assert!(matches!(result, Err(VarintError::Overflow)));
    }

    #[test]
    fn test_non_minimal_detection() {
        // Non-minimal encoding of 0: 0x80 0x00 instead of just 0x00
        let non_minimal = [0x80, 0x00];
        let result = decode_varint_strict(&non_minimal);
        assert!(matches!(result, Err(VarintError::NonMinimal)));

        // Non-minimal encoding of 1: 0x81 0x00 instead of 0x01
        let non_minimal = [0x81, 0x00];
        let result = decode_varint_strict(&non_minimal);
        assert!(matches!(result, Err(VarintError::NonMinimal)));

        // Minimal encoding should succeed
        let (value, _) = decode_varint_strict(&[0x01]).unwrap();
        assert_eq!(value, 1);
    }

    #[tokio::test]
    async fn test_async_varint() {
        use std::io::Cursor;

        let values = [0u64, 127, 128, 300, 16384, u64::MAX];

        for value in values {
            let mut buf = Vec::new();
            async_io::write_varint(&mut buf, value).await.unwrap();

            let mut cursor = Cursor::new(&buf);
            let decoded = async_io::read_varint(&mut cursor).await.unwrap();

            assert_eq!(decoded, value);
        }
    }
}
