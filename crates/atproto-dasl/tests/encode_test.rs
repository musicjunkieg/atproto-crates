//! Encoding tests for DAG-CBOR
//!
//! Tests basic encoding functionality against known test vectors.

use atproto_dasl::to_vec;

#[test]
fn test_encode_integers() {
    // Test vectors from plan
    assert_eq!(to_vec(&0u8).unwrap(), vec![0x00]);
    assert_eq!(to_vec(&23u8).unwrap(), vec![0x17]);
    assert_eq!(to_vec(&24u8).unwrap(), vec![0x18, 0x18]);
    assert_eq!(to_vec(&255u8).unwrap(), vec![0x18, 0xff]);
    assert_eq!(to_vec(&256u16).unwrap(), vec![0x19, 0x01, 0x00]);
    assert_eq!(to_vec(&65535u16).unwrap(), vec![0x19, 0xff, 0xff]);
    assert_eq!(
        to_vec(&65536u32).unwrap(),
        vec![0x1a, 0x00, 0x01, 0x00, 0x00]
    );
}

#[test]
fn test_encode_negative_integers() {
    assert_eq!(to_vec(&(-1i8)).unwrap(), vec![0x20]);
    assert_eq!(to_vec(&(-24i8)).unwrap(), vec![0x37]);
    assert_eq!(to_vec(&(-25i8)).unwrap(), vec![0x38, 0x18]);
}

#[test]
fn test_encode_strings() {
    assert_eq!(to_vec(&"").unwrap(), vec![0x60]);
    assert_eq!(to_vec(&"a").unwrap(), vec![0x61, 0x61]);
    assert_eq!(
        to_vec(&"hello").unwrap(),
        vec![0x65, b'h', b'e', b'l', b'l', b'o']
    );
}

#[test]
fn test_encode_arrays() {
    let empty: Vec<i32> = vec![];
    assert_eq!(to_vec(&empty).unwrap(), vec![0x80]);

    let arr = vec![1u8, 2, 3];
    assert_eq!(to_vec(&arr).unwrap(), vec![0x83, 0x01, 0x02, 0x03]);
}

#[test]
fn test_encode_booleans() {
    assert_eq!(to_vec(&false).unwrap(), vec![0xf4]);
    assert_eq!(to_vec(&true).unwrap(), vec![0xf5]);
}

#[test]
fn test_encode_null() {
    let none: Option<i32> = None;
    assert_eq!(to_vec(&none).unwrap(), vec![0xf6]);
}

#[test]
fn test_encode_float() {
    // 1.0 in 64-bit float
    let bytes = to_vec(&1.0f64).unwrap();
    assert_eq!(
        bytes,
        vec![0xfb, 0x3f, 0xf0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn test_encode_rejects_nan() {
    assert!(to_vec(&f64::NAN).is_err());
}

#[test]
fn test_encode_rejects_infinity() {
    assert!(to_vec(&f64::INFINITY).is_err());
    assert!(to_vec(&f64::NEG_INFINITY).is_err());
}

#[test]
fn test_encode_bytes() {
    use serde::Serialize;

    #[derive(Serialize)]
    struct ByteWrapper(#[serde(with = "serde_bytes")] Vec<u8>);

    let wrapper = ByteWrapper(vec![0x01, 0x02, 0x03]);
    let bytes = to_vec(&wrapper).unwrap();
    assert_eq!(bytes, vec![0x43, 0x01, 0x02, 0x03]);
}
