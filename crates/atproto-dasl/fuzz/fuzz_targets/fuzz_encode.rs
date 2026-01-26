//! Fuzz target: encode idempotency.
//!
//! Decodes bytes with non-strict mode, re-encodes to canonical form, then
//! verifies that encoding the canonical form again produces identical bytes.
//!
//! Run: `cargo fuzz run fuzz_encode`

#![no_main]

use atproto_dasl::{Ipld, from_slice, from_slice_non_strict, to_vec};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Decode with non-strict mode
    let decoded: Ipld = match from_slice_non_strict(data) {
        Ok(val) => val,
        Err(_) => return,
    };

    // Encode to canonical DAG-CBOR
    let canonical = match to_vec(&decoded) {
        Ok(bytes) => bytes,
        Err(_) => return,
    };

    // Decode canonical bytes
    let redecoded: Ipld = from_slice(&canonical)
        .expect("canonical DAG-CBOR should always decode successfully");

    // Re-encode must produce identical bytes (encoding is deterministic)
    let reencoded = to_vec(&redecoded)
        .expect("re-encoding canonical value should always succeed");

    assert_eq!(
        canonical, reencoded,
        "encoding is not idempotent: first={} second={}",
        hex_encode(&canonical),
        hex_encode(&reencoded),
    );
});

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
