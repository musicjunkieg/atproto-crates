//! Fuzz target: decode arbitrary bytes as IPLD.
//!
//! Feeds random bytes to the strict DAG-CBOR decoder. The decoder must never
//! panic — it should return `Ok` or `Err` for any input.
//!
//! Run: `cargo fuzz run fuzz_decode`

#![no_main]

use atproto_dasl::{Ipld, from_slice};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = from_slice::<Ipld>(data);
});
