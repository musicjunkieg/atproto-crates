//! Fuzz target: roundtrip consistency.
//!
//! Decodes bytes with non-strict mode, re-encodes to canonical DAG-CBOR,
//! then decodes again and verifies the two decoded values are equal.
//!
//! Run: `cargo fuzz run fuzz_roundtrip`

#![no_main]

use atproto_dasl::{Ipld, from_slice, from_slice_non_strict, to_vec};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Decode with non-strict mode (accepts non-canonical CBOR)
    let decoded: Ipld = match from_slice_non_strict(data) {
        Ok(val) => val,
        Err(_) => return,
    };

    // Re-encode to canonical DAG-CBOR
    let encoded = match to_vec(&decoded) {
        Ok(bytes) => bytes,
        Err(_) => return,
    };

    // Decode the canonical output with strict mode
    let redecoded: Ipld = from_slice(&encoded)
        .expect("canonical DAG-CBOR should always decode successfully");

    // The two decoded values must be equal
    assert_eq!(decoded, redecoded, "roundtrip value mismatch");
});
