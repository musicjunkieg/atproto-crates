//! Decoding tests for DAG-CBOR
//!
//! Tests basic decoding functionality against known test vectors.

use atproto_dasl::{from_slice, from_slice_non_strict};

#[test]
fn test_decode_integers() {
    // Test vectors from plan
    assert_eq!(from_slice::<u8>(&[0x00]).unwrap(), 0);
    assert_eq!(from_slice::<u8>(&[0x17]).unwrap(), 23);
    assert_eq!(from_slice::<u8>(&[0x18, 0x18]).unwrap(), 24);
    assert_eq!(from_slice::<u8>(&[0x18, 0xff]).unwrap(), 255);
    assert_eq!(from_slice::<u16>(&[0x19, 0x01, 0x00]).unwrap(), 256);
    assert_eq!(from_slice::<u16>(&[0x19, 0xff, 0xff]).unwrap(), 65535);
    assert_eq!(
        from_slice::<u32>(&[0x1a, 0x00, 0x01, 0x00, 0x00]).unwrap(),
        65536
    );
}

#[test]
fn test_decode_negative_integers() {
    assert_eq!(from_slice::<i8>(&[0x20]).unwrap(), -1);
    assert_eq!(from_slice::<i8>(&[0x37]).unwrap(), -24);
    assert_eq!(from_slice::<i16>(&[0x38, 0x18]).unwrap(), -25);
}

#[test]
fn test_decode_strings() {
    assert_eq!(from_slice::<String>(&[0x60]).unwrap(), "");
    assert_eq!(from_slice::<String>(&[0x61, 0x61]).unwrap(), "a");
    assert_eq!(
        from_slice::<String>(&[0x65, b'h', b'e', b'l', b'l', b'o']).unwrap(),
        "hello"
    );
}

#[test]
fn test_decode_arrays() {
    let empty: Vec<i32> = from_slice(&[0x80]).unwrap();
    assert!(empty.is_empty());

    let arr: Vec<u8> = from_slice(&[0x83, 0x01, 0x02, 0x03]).unwrap();
    assert_eq!(arr, vec![1, 2, 3]);
}

#[test]
fn test_decode_booleans() {
    assert!(!from_slice::<bool>(&[0xf4]).unwrap());
    assert!(from_slice::<bool>(&[0xf5]).unwrap());
}

#[test]
fn test_decode_null() {
    let none: Option<i32> = from_slice(&[0xf6]).unwrap();
    assert!(none.is_none());
}

#[test]
fn test_decode_float() {
    let value: f64 = from_slice(&[0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]).unwrap();
    assert!((value - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_decode_rejects_nan() {
    // 64-bit NaN
    let result = from_slice::<f64>(&[0xfb, 0x7f, 0xf8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    assert!(result.is_err());
}

#[test]
fn test_decode_rejects_infinity() {
    // 64-bit Infinity
    let result = from_slice::<f64>(&[0xfb, 0x7f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    assert!(result.is_err());

    // 64-bit -Infinity
    let result = from_slice::<f64>(&[0xfb, 0xff, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    assert!(result.is_err());
}

#[test]
fn test_decode_rejects_indefinite_length() {
    // Indefinite array
    assert!(from_slice::<Vec<i32>>(&[0x9f]).is_err());
    // Indefinite map
    assert!(from_slice::<std::collections::BTreeMap<String, i32>>(&[0xbf]).is_err());
    // Break code
    assert!(from_slice::<i32>(&[0xff]).is_err());
}

#[test]
fn test_decode_rejects_unsupported_tags() {
    // Tag 0 (datetime)
    assert!(from_slice::<String>(&[0xc0, 0x60]).is_err());
    // Tag 1 (epoch)
    assert!(from_slice::<i64>(&[0xc1, 0x00]).is_err());
}

#[test]
fn test_decode_rejects_undefined() {
    assert!(from_slice::<()>(&[0xf7]).is_err());
}

#[test]
fn test_decode_non_strict_accepts_non_canonical() {
    // Non-minimal: 0 encoded with 1 extra byte
    assert_eq!(from_slice_non_strict::<u8>(&[0x18, 0x00]).unwrap(), 0);

    // Non-minimal: 23 encoded with 2 extra bytes
    assert_eq!(
        from_slice_non_strict::<u8>(&[0x19, 0x00, 0x17]).unwrap(),
        23
    );
}

#[test]
fn test_decode_strict_rejects_non_canonical() {
    // Non-minimal: 0 encoded with 1 extra byte
    assert!(from_slice::<u8>(&[0x18, 0x00]).is_err());

    // Non-minimal: 23 encoded with 2 extra bytes
    assert!(from_slice::<u8>(&[0x19, 0x00, 0x17]).is_err());
}

#[test]
fn test_decode_strict_rejects_non_64bit_float() {
    // 16-bit float
    assert!(from_slice::<f64>(&[0xf9, 0x00, 0x00]).is_err());
    // 32-bit float
    assert!(from_slice::<f64>(&[0xfa, 0x00, 0x00, 0x00, 0x00]).is_err());
}

#[test]
fn test_decode_non_strict_accepts_non_64bit_float() {
    // 16-bit float 0.0
    let value: f64 = from_slice_non_strict(&[0xf9, 0x00, 0x00]).unwrap();
    assert!((value - 0.0).abs() < f64::EPSILON);

    // 32-bit float 0.0
    let value: f64 = from_slice_non_strict(&[0xfa, 0x00, 0x00, 0x00, 0x00]).unwrap();
    assert!((value - 0.0).abs() < f64::EPSILON);
}
