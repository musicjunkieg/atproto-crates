//! CBOR type constants and utilities.
//!
//! Defines the CBOR major types and additional info values following RFC 8949.

/// CBOR major types (0-7), stored in the high 3 bits of the initial byte
pub mod major_type {
    /// Type 0: unsigned integer (0 to 2^64-1)
    pub const UNSIGNED: u8 = 0;
    /// Type 1: negative integer (-2^64 to -1)
    pub const NEGATIVE: u8 = 1;
    /// Type 2: byte string
    pub const BYTES: u8 = 2;
    /// Type 3: UTF-8 text string
    pub const TEXT: u8 = 3;
    /// Type 4: array of data items
    pub const ARRAY: u8 = 4;
    /// Type 5: map of key-value pairs
    pub const MAP: u8 = 5;
    /// Type 6: tagged data item
    pub const TAG: u8 = 6;
    /// Type 7: floating-point numbers and simple values
    pub const SIMPLE: u8 = 7;
}

/// Additional info values for major type 7 (simple values and floats)
pub mod simple_value {
    /// Boolean false
    pub const FALSE: u8 = 20;
    /// Boolean true
    pub const TRUE: u8 = 21;
    /// Null value
    pub const NULL: u8 = 22;
    /// Undefined value (not allowed in DAG-CBOR)
    pub const UNDEFINED: u8 = 23;
    /// 16-bit IEEE 754 half-precision float (not canonical in DAG-CBOR)
    pub const FLOAT16: u8 = 25;
    /// 32-bit IEEE 754 single-precision float (not canonical in DAG-CBOR)
    pub const FLOAT32: u8 = 26;
    /// 64-bit IEEE 754 double-precision float (canonical in DAG-CBOR)
    pub const FLOAT64: u8 = 27;
    /// Break code for indefinite-length items (not allowed in DAG-CBOR)
    pub const BREAK: u8 = 31;
}

/// CBOR tag values
pub mod tag {
    /// Tag 42 for CIDs (the only tag allowed in DAG-CBOR)
    pub const CID: u64 = 42;
}

/// Additional info values that indicate argument size
pub mod additional_info {
    /// Value 0-23: the argument is the value itself
    pub const DIRECT_MAX: u8 = 23;
    /// Value 24: argument is in the next 1 byte
    pub const ONE_BYTE: u8 = 24;
    /// Value 25: argument is in the next 2 bytes
    pub const TWO_BYTES: u8 = 25;
    /// Value 26: argument is in the next 4 bytes
    pub const FOUR_BYTES: u8 = 26;
    /// Value 27: argument is in the next 8 bytes
    pub const EIGHT_BYTES: u8 = 27;
    /// Values 28-30: reserved (not well-formed)
    pub const RESERVED_28: u8 = 28;
    /// Values 28-30: reserved (not well-formed)
    pub const RESERVED_29: u8 = 29;
    /// Values 28-30: reserved (not well-formed)
    pub const RESERVED_30: u8 = 30;
    /// Value 31: indefinite length (not allowed in DAG-CBOR)
    pub const INDEFINITE: u8 = 31;
}

/// Extract major type from the initial byte
#[inline]
pub fn major_type_from_byte(byte: u8) -> u8 {
    byte >> 5
}

/// Extract additional info from the initial byte
#[inline]
pub fn additional_info_from_byte(byte: u8) -> u8 {
    byte & 0x1f
}

/// Create an initial byte from major type and additional info
#[inline]
pub fn make_initial_byte(major: u8, additional: u8) -> u8 {
    (major << 5) | (additional & 0x1f)
}

/// Get the name of a major type for error messages
pub fn major_type_name(major: u8) -> &'static str {
    match major {
        major_type::UNSIGNED => "unsigned integer",
        major_type::NEGATIVE => "negative integer",
        major_type::BYTES => "byte string",
        major_type::TEXT => "text string",
        major_type::ARRAY => "array",
        major_type::MAP => "map",
        major_type::TAG => "tag",
        major_type::SIMPLE => "simple/float",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_extraction() {
        // Test unsigned integer 23 (0x17)
        assert_eq!(major_type_from_byte(0x17), major_type::UNSIGNED);
        assert_eq!(additional_info_from_byte(0x17), 23);

        // Test negative integer with 1-byte argument (0x38)
        assert_eq!(major_type_from_byte(0x38), major_type::NEGATIVE);
        assert_eq!(additional_info_from_byte(0x38), additional_info::ONE_BYTE);

        // Test text string with 2-byte length (0x79)
        assert_eq!(major_type_from_byte(0x79), major_type::TEXT);
        assert_eq!(additional_info_from_byte(0x79), additional_info::TWO_BYTES);

        // Test map with 4-byte length (0xba)
        assert_eq!(major_type_from_byte(0xba), major_type::MAP);
        assert_eq!(additional_info_from_byte(0xba), additional_info::FOUR_BYTES);

        // Test tag with 8-byte argument (0xdb)
        assert_eq!(major_type_from_byte(0xdb), major_type::TAG);
        assert_eq!(
            additional_info_from_byte(0xdb),
            additional_info::EIGHT_BYTES
        );

        // Test simple value false (0xf4)
        assert_eq!(major_type_from_byte(0xf4), major_type::SIMPLE);
        assert_eq!(additional_info_from_byte(0xf4), simple_value::FALSE);
    }

    #[test]
    fn test_make_initial_byte() {
        assert_eq!(make_initial_byte(major_type::UNSIGNED, 0), 0x00);
        assert_eq!(make_initial_byte(major_type::UNSIGNED, 23), 0x17);
        assert_eq!(
            make_initial_byte(major_type::UNSIGNED, additional_info::ONE_BYTE),
            0x18
        );
        assert_eq!(
            make_initial_byte(major_type::NEGATIVE, additional_info::TWO_BYTES),
            0x39
        );
        assert_eq!(
            make_initial_byte(major_type::SIMPLE, simple_value::FALSE),
            0xf4
        );
        assert_eq!(
            make_initial_byte(major_type::SIMPLE, simple_value::NULL),
            0xf6
        );
    }
}
