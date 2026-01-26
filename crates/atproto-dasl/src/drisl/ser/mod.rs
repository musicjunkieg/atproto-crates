//! Serde serialization for DAG-CBOR.
//!
//! This module provides the serde `Serializer` implementation for converting
//! Rust types to DAG-CBOR format.

mod serializer;

pub use serializer::{
    MapSerializer, SeqSerializer, Serializer, StructSerializer, StructVariantSerializer,
};
