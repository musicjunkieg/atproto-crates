//! Low-level CBOR encoding and decoding.
//!
//! This module provides the core CBOR primitives following RFC 8949,
//! with DAG-CBOR canonical encoding requirements.

pub mod decode;
pub mod encode;
pub mod types;

pub use decode::{CborDecoder, CborItem};
pub use encode::CborEncoder;
