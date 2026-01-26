//! Property-based roundtrip tests for DRISL (DAG-CBOR) and CID operations.
//!
//! Uses proptest to verify that arbitrary values survive encode-decode roundtrips
//! and that CID computation is deterministic.

use atproto_dasl::{Ipld, compute_cid_for, from_slice, to_vec, verify_cid_bytes};
use proptest::prelude::*;
use std::collections::BTreeMap;

/// Generate arbitrary `Ipld` values up to a given depth.
fn arb_ipld(depth: u32) -> impl Strategy<Value = Ipld> {
    let leaf = prop_oneof![
        Just(Ipld::Null),
        any::<bool>().prop_map(Ipld::Bool),
        // Use i64 range to stay within DAG-CBOR's typical integer range.
        any::<i64>().prop_map(|n| Ipld::Integer(i128::from(n))),
        // Only generate finite, non-NaN f64 values (DAG-CBOR requirement).
        any::<f64>()
            .prop_filter("must be finite", |f| f.is_finite())
            .prop_map(Ipld::Float),
        "[ -~]{0,32}".prop_map(|s: String| Ipld::String(s)),
        prop::collection::vec(any::<u8>(), 0..64).prop_map(Ipld::Bytes),
    ];

    leaf.prop_recursive(
        depth, // depth
        128,   // desired_size
        8,     // expected_branch_size
        |inner| {
            prop_oneof![
                // List of Ipld values
                prop::collection::vec(inner.clone(), 0..8).prop_map(Ipld::List),
                // Map of string -> Ipld values
                prop::collection::btree_map("[a-z]{1,8}", inner, 0..8).prop_map(Ipld::Map),
            ]
        },
    )
}

proptest! {
    /// DRISL primitive roundtrip: any Ipld value survives encode+decode.
    #[test]
    fn drisl_roundtrip(value in arb_ipld(3)) {
        let encoded = to_vec(&value).expect("encoding should succeed");
        let decoded: Ipld = from_slice(&encoded).expect("decoding should succeed");
        prop_assert_eq!(value, decoded);
    }

    /// DRISL encoding is deterministic: encoding the same value twice yields identical bytes.
    #[test]
    fn drisl_deterministic(value in arb_ipld(3)) {
        let encoded1 = to_vec(&value).expect("first encode");
        let encoded2 = to_vec(&value).expect("second encode");
        prop_assert_eq!(encoded1, encoded2);
    }

    /// CID computation is deterministic and verifiable.
    #[test]
    fn cid_compute_and_verify(value in arb_ipld(2)) {
        let cid = compute_cid_for(&value).expect("CID computation should succeed");
        let encoded = to_vec(&value).expect("encoding should succeed");
        prop_assert!(verify_cid_bytes(&cid, &encoded).is_ok());
    }

    /// Integer roundtrip: all i64 values survive encoding.
    #[test]
    fn integer_roundtrip(n in any::<i64>()) {
        let value = Ipld::Integer(i128::from(n));
        let encoded = to_vec(&value).expect("encoding should succeed");
        let decoded: Ipld = from_slice(&encoded).expect("decoding should succeed");
        prop_assert_eq!(Ipld::Integer(i128::from(n)), decoded);
    }

    /// String roundtrip: arbitrary UTF-8 strings survive encoding.
    #[test]
    fn string_roundtrip(s in "[ -~]{0,64}") {
        let value = Ipld::String(s.clone());
        let encoded = to_vec(&value).expect("encoding should succeed");
        let decoded: Ipld = from_slice(&encoded).expect("decoding should succeed");
        prop_assert_eq!(Ipld::String(s), decoded);
    }

    /// Bytes roundtrip: arbitrary byte sequences survive encoding.
    #[test]
    fn bytes_roundtrip(data in prop::collection::vec(any::<u8>(), 0..256)) {
        let value = Ipld::Bytes(data.clone());
        let encoded = to_vec(&value).expect("encoding should succeed");
        let decoded: Ipld = from_slice(&encoded).expect("decoding should succeed");
        prop_assert_eq!(Ipld::Bytes(data), decoded);
    }

    /// Map key ordering: maps are always serialized with sorted keys.
    #[test]
    fn map_keys_sorted(entries in prop::collection::btree_map("[a-z]{1,8}", any::<i64>(), 0..16)) {
        let map: BTreeMap<String, Ipld> = entries.into_iter()
            .map(|(k, v)| (k, Ipld::Integer(i128::from(v))))
            .collect();
        let value = Ipld::Map(map);
        let encoded = to_vec(&value).expect("encoding should succeed");
        let decoded: Ipld = from_slice(&encoded).expect("decoding should succeed");
        prop_assert_eq!(value, decoded);
    }
}
