//! Serde deserialization for DAG-CBOR.
//!
//! This module provides the serde `Deserializer` implementation for converting
//! DAG-CBOR data to Rust types.

mod deserializer;

pub use deserializer::Deserializer;
