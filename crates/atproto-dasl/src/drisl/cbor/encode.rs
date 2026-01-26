//! Low-level CBOR encoding following RFC 8949.
//!
//! Provides primitives for encoding CBOR major types with DAG-CBOR
//! canonical encoding requirements (shortest form, definite length).

use crate::drisl::cbor::types::{
    additional_info, major_type, make_initial_byte, simple_value, tag,
};
use crate::errors::EncodeError;
use std::io::Write;

/// Encoder for writing CBOR bytes with DAG-CBOR canonical encoding.
///
/// All encoding follows the DAG-CBOR specification:
/// - Integers use the shortest possible encoding
/// - Floats are always 64-bit
/// - Only definite-length items are supported
pub struct CborEncoder<W: Write> {
    writer: W,
}

impl<W: Write> CborEncoder<W> {
    /// Create a new CBOR encoder wrapping the given writer
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Get a mutable reference to the underlying writer
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Consume the encoder and return the underlying writer
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Encode a major type with argument using shortest encoding (DAG-CBOR requirement)
    fn encode_head(&mut self, major: u8, arg: u64) -> Result<(), EncodeError> {
        if arg < 24 {
            // Argument fits in additional info bits (0-23)
            self.writer
                .write_all(&[make_initial_byte(major, arg as u8)])?;
        } else if arg <= u8::MAX as u64 {
            // 1 additional byte
            self.writer.write_all(&[
                make_initial_byte(major, additional_info::ONE_BYTE),
                arg as u8,
            ])?;
        } else if arg <= u16::MAX as u64 {
            // 2 additional bytes (big-endian)
            self.writer
                .write_all(&[make_initial_byte(major, additional_info::TWO_BYTES)])?;
            self.writer.write_all(&(arg as u16).to_be_bytes())?;
        } else if arg <= u32::MAX as u64 {
            // 4 additional bytes (big-endian)
            self.writer
                .write_all(&[make_initial_byte(major, additional_info::FOUR_BYTES)])?;
            self.writer.write_all(&(arg as u32).to_be_bytes())?;
        } else {
            // 8 additional bytes (big-endian)
            self.writer
                .write_all(&[make_initial_byte(major, additional_info::EIGHT_BYTES)])?;
            self.writer.write_all(&arg.to_be_bytes())?;
        }

        Ok(())
    }

    /// Encode an unsigned integer (major type 0)
    pub fn encode_unsigned(&mut self, value: u64) -> Result<(), EncodeError> {
        self.encode_head(major_type::UNSIGNED, value)
    }

    /// Encode a negative integer (major type 1)
    ///
    /// CBOR encodes negative integers as -1-n, so -1 is encoded as argument 0,
    /// -2 as argument 1, etc.
    pub fn encode_negative(&mut self, value: i64) -> Result<(), EncodeError> {
        debug_assert!(value < 0, "encode_negative called with non-negative value");
        // For negative value v, we encode -1-v as the argument
        // So for v = -1, we encode 0; for v = -2, we encode 1, etc.
        let arg = (-1 - value).cast_unsigned();
        self.encode_head(major_type::NEGATIVE, arg)
    }

    /// Encode a signed integer, choosing the appropriate major type
    pub fn encode_integer(&mut self, value: i64) -> Result<(), EncodeError> {
        if value >= 0 {
            self.encode_unsigned(value.cast_unsigned())
        } else {
            self.encode_negative(value)
        }
    }

    /// Encode a 128-bit signed integer
    pub fn encode_i128(&mut self, value: i128) -> Result<(), EncodeError> {
        if value >= 0 {
            if value > u64::MAX as i128 {
                return Err(EncodeError::IntegerOverflow);
            }
            self.encode_unsigned(value as u64)
        } else {
            // For negative value v, CBOR encodes -1-v
            // The most negative value we can encode is -1 - u64::MAX = -18446744073709551616
            let arg = -1i128 - value;
            if arg > u64::MAX as i128 {
                return Err(EncodeError::IntegerOverflow);
            }
            self.encode_head(major_type::NEGATIVE, arg as u64)
        }
    }

    /// Encode a byte string (major type 2)
    pub fn encode_bytes(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.encode_head(major_type::BYTES, bytes.len() as u64)?;
        self.writer.write_all(bytes)?;
        Ok(())
    }

    /// Encode a UTF-8 text string (major type 3)
    pub fn encode_text(&mut self, text: &str) -> Result<(), EncodeError> {
        self.encode_head(major_type::TEXT, text.len() as u64)?;
        self.writer.write_all(text.as_bytes())?;
        Ok(())
    }

    /// Encode an array header (major type 4)
    ///
    /// This only writes the header; the caller must encode the items.
    pub fn encode_array_header(&mut self, len: usize) -> Result<(), EncodeError> {
        self.encode_head(major_type::ARRAY, len as u64)
    }

    /// Encode a map header (major type 5)
    ///
    /// This only writes the header; the caller must encode the key-value pairs.
    /// DAG-CBOR requires keys to be sorted in bytewise lexicographic order.
    pub fn encode_map_header(&mut self, len: usize) -> Result<(), EncodeError> {
        self.encode_head(major_type::MAP, len as u64)
    }

    /// Encode tag 42 for CID (must be exactly 0xd82a per DAG-CBOR spec)
    pub fn encode_cid_tag(&mut self) -> Result<(), EncodeError> {
        // Tag 42 must be encoded with 1-byte argument (0xd8 0x2a)
        // This is the canonical shortest form for value 42
        self.encode_head(major_type::TAG, tag::CID)
    }

    /// Encode a generic tag (only tag 42 is allowed in DAG-CBOR)
    pub fn encode_tag(&mut self, tag_value: u64) -> Result<(), EncodeError> {
        self.encode_head(major_type::TAG, tag_value)
    }

    /// Encode a boolean value (major type 7, simple values 20/21)
    pub fn encode_bool(&mut self, value: bool) -> Result<(), EncodeError> {
        let simple = if value {
            simple_value::TRUE
        } else {
            simple_value::FALSE
        };
        self.writer
            .write_all(&[make_initial_byte(major_type::SIMPLE, simple)])?;
        Ok(())
    }

    /// Encode a null value (major type 7, simple value 22)
    pub fn encode_null(&mut self) -> Result<(), EncodeError> {
        self.writer
            .write_all(&[make_initial_byte(major_type::SIMPLE, simple_value::NULL)])?;
        Ok(())
    }

    /// Encode a 64-bit float (major type 7, additional info 27)
    ///
    /// DAG-CBOR requires all floats to be 64-bit and rejects NaN/Infinity.
    pub fn encode_f64(&mut self, value: f64) -> Result<(), EncodeError> {
        if value.is_nan() {
            return Err(EncodeError::InvalidFloat {
                reason: "NaN is not allowed in DAG-CBOR".to_string(),
            });
        }
        if value.is_infinite() {
            return Err(EncodeError::InvalidFloat {
                reason: "Infinity is not allowed in DAG-CBOR".to_string(),
            });
        }

        self.writer
            .write_all(&[make_initial_byte(major_type::SIMPLE, simple_value::FLOAT64)])?;
        self.writer.write_all(&value.to_be_bytes())?;
        Ok(())
    }

    /// Write raw bytes directly to the output
    pub fn write_raw(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.writer.write_all(bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_to_vec<F>(f: F) -> Vec<u8>
    where
        F: FnOnce(&mut CborEncoder<Vec<u8>>) -> Result<(), EncodeError>,
    {
        let mut encoder = CborEncoder::new(Vec::new());
        f(&mut encoder).unwrap();
        encoder.into_inner()
    }

    #[test]
    fn test_encode_unsigned() {
        // 0 -> 0x00
        assert_eq!(encode_to_vec(|e| e.encode_unsigned(0)), vec![0x00]);
        // 1 -> 0x01
        assert_eq!(encode_to_vec(|e| e.encode_unsigned(1)), vec![0x01]);
        // 23 -> 0x17
        assert_eq!(encode_to_vec(|e| e.encode_unsigned(23)), vec![0x17]);
        // 24 -> 0x18 0x18
        assert_eq!(encode_to_vec(|e| e.encode_unsigned(24)), vec![0x18, 0x18]);
        // 255 -> 0x18 0xff
        assert_eq!(encode_to_vec(|e| e.encode_unsigned(255)), vec![0x18, 0xff]);
        // 256 -> 0x19 0x01 0x00
        assert_eq!(
            encode_to_vec(|e| e.encode_unsigned(256)),
            vec![0x19, 0x01, 0x00]
        );
        // 65535 -> 0x19 0xff 0xff
        assert_eq!(
            encode_to_vec(|e| e.encode_unsigned(65535)),
            vec![0x19, 0xff, 0xff]
        );
        // 65536 -> 0x1a 0x00 0x01 0x00 0x00
        assert_eq!(
            encode_to_vec(|e| e.encode_unsigned(65536)),
            vec![0x1a, 0x00, 0x01, 0x00, 0x00]
        );
        // 2^32-1 -> 0x1a 0xff 0xff 0xff 0xff
        assert_eq!(
            encode_to_vec(|e| e.encode_unsigned(0xffffffff)),
            vec![0x1a, 0xff, 0xff, 0xff, 0xff]
        );
        // 2^32 -> 0x1b 0x00 0x00 0x00 0x01 0x00 0x00 0x00 0x00
        assert_eq!(
            encode_to_vec(|e| e.encode_unsigned(0x100000000)),
            vec![0x1b, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_encode_negative() {
        // -1 -> 0x20
        assert_eq!(encode_to_vec(|e| e.encode_negative(-1)), vec![0x20]);
        // -24 -> 0x37
        assert_eq!(encode_to_vec(|e| e.encode_negative(-24)), vec![0x37]);
        // -25 -> 0x38 0x18
        assert_eq!(encode_to_vec(|e| e.encode_negative(-25)), vec![0x38, 0x18]);
        // -256 -> 0x38 0xff
        assert_eq!(encode_to_vec(|e| e.encode_negative(-256)), vec![0x38, 0xff]);
        // -257 -> 0x39 0x01 0x00
        assert_eq!(
            encode_to_vec(|e| e.encode_negative(-257)),
            vec![0x39, 0x01, 0x00]
        );
    }

    #[test]
    fn test_encode_bytes() {
        // Empty bytes -> 0x40
        assert_eq!(encode_to_vec(|e| e.encode_bytes(&[])), vec![0x40]);
        // Single byte -> 0x41 0xaa
        assert_eq!(encode_to_vec(|e| e.encode_bytes(&[0xaa])), vec![0x41, 0xaa]);
        // Multiple bytes
        assert_eq!(
            encode_to_vec(|e| e.encode_bytes(&[1, 2, 3])),
            vec![0x43, 0x01, 0x02, 0x03]
        );
    }

    #[test]
    fn test_encode_text() {
        // Empty string -> 0x60
        assert_eq!(encode_to_vec(|e| e.encode_text("")), vec![0x60]);
        // "a" -> 0x61 0x61
        assert_eq!(encode_to_vec(|e| e.encode_text("a")), vec![0x61, 0x61]);
        // "hello" -> 0x65 + "hello"
        assert_eq!(
            encode_to_vec(|e| e.encode_text("hello")),
            vec![0x65, b'h', b'e', b'l', b'l', b'o']
        );
    }

    #[test]
    fn test_encode_array_header() {
        // Empty array -> 0x80
        assert_eq!(encode_to_vec(|e| e.encode_array_header(0)), vec![0x80]);
        // Array of 3 -> 0x83
        assert_eq!(encode_to_vec(|e| e.encode_array_header(3)), vec![0x83]);
        // Array of 24 -> 0x98 0x18
        assert_eq!(
            encode_to_vec(|e| e.encode_array_header(24)),
            vec![0x98, 0x18]
        );
    }

    #[test]
    fn test_encode_map_header() {
        // Empty map -> 0xa0
        assert_eq!(encode_to_vec(|e| e.encode_map_header(0)), vec![0xa0]);
        // Map of 1 pair -> 0xa1
        assert_eq!(encode_to_vec(|e| e.encode_map_header(1)), vec![0xa1]);
    }

    #[test]
    fn test_encode_bool() {
        // false -> 0xf4
        assert_eq!(encode_to_vec(|e| e.encode_bool(false)), vec![0xf4]);
        // true -> 0xf5
        assert_eq!(encode_to_vec(|e| e.encode_bool(true)), vec![0xf5]);
    }

    #[test]
    fn test_encode_null() {
        // null -> 0xf6
        assert_eq!(encode_to_vec(|e| e.encode_null()), vec![0xf6]);
    }

    #[test]
    fn test_encode_f64() {
        // 1.0 -> 0xfb 0x3f 0xf0 0x00 0x00 0x00 0x00 0x00 0x00
        assert_eq!(
            encode_to_vec(|e| e.encode_f64(1.0)),
            vec![0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // 0.0
        assert_eq!(
            encode_to_vec(|e| e.encode_f64(0.0)),
            vec![0xfb, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        // -1.5
        assert_eq!(
            encode_to_vec(|e| e.encode_f64(-1.5)),
            vec![0xfb, 0xbf, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_encode_f64_rejects_nan() {
        let mut encoder = CborEncoder::new(Vec::new());
        let result = encoder.encode_f64(f64::NAN);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_f64_rejects_infinity() {
        let mut encoder = CborEncoder::new(Vec::new());
        let result = encoder.encode_f64(f64::INFINITY);
        assert!(result.is_err());

        let mut encoder = CborEncoder::new(Vec::new());
        let result = encoder.encode_f64(f64::NEG_INFINITY);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_cid_tag() {
        // Tag 42 -> 0xd8 0x2a
        assert_eq!(encode_to_vec(|e| e.encode_cid_tag()), vec![0xd8, 0x2a]);
    }
}
