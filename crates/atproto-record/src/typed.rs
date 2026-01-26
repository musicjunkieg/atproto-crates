//! Generic wrapper type for AT Protocol lexicon structures with optional `$type` field handling.
//!
//! This module provides a flexible way to handle the `$type` discriminator field that
//! appears in many AT Protocol lexicon structures. The wrapper can be used when the
//! type field needs to be validated, automatically added during serialization, or
//! when it may not always be present.

use serde::de::{DeserializeOwned, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::marker::PhantomData;

/// A trait for types that have an associated lexicon type identifier.
pub trait LexiconType {
    /// Returns the lexicon type identifier (e.g., "community.lexicon.attestation.signature").
    fn lexicon_type() -> &'static str;

    /// Returns whether the type field is required for this lexicon type.
    /// Default is true, but types can override this for optional type fields.
    fn type_required() -> bool {
        true
    }
}

/// A wrapper type that handles the `$type` field for AT Protocol lexicon structures.
///
/// This wrapper provides flexibility in handling the type field:
/// - Conditionally adds the `$type` field during serialization based on `type_present`
/// - Validates the `$type` during deserialization if present
/// - Preserves the presence/absence of `$type` for round-trip compatibility
/// - Can handle cases where the `$type` field is optional
///
/// # Serialization Behavior
///
/// - When created with `TypedLexicon::new()`, the `$type` field will be included in serialization
/// - When created with `TypedLexicon::new_without_type()`, the `$type` field will be omitted
/// - When deserialized, the presence of `$type` is preserved for round-trip compatibility
///
/// # Example
///
/// ```ignore
/// use atproto_record::typed::{TypedLexicon, LexiconType};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Serialize, Deserialize)]
/// struct MyRecord {
///     name: String,
///     value: i32,
/// }
///
/// impl LexiconType for MyRecord {
///     fn lexicon_type() -> &'static str {
///         "com.example.myrecord"
///     }
/// }
///
/// // With type field
/// let record = MyRecord { name: "test".to_string(), value: 42 };
/// let typed = TypedLexicon::new(record);
/// let json = serde_json::to_string(&typed).unwrap();
/// assert!(json.contains("\"$type\":\"com.example.myrecord\""));
///
/// // Without type field
/// let record2 = MyRecord { name: "test2".to_string(), value: 43 };
/// let typed2 = TypedLexicon::new_without_type(record2);
/// let json2 = serde_json::to_string(&typed2).unwrap();
/// assert!(!json2.contains("\"$type\""));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TypedLexicon<T: LexiconType + PartialEq> {
    /// The inner value being wrapped
    pub inner: T,
    /// Whether the type field was explicitly present during deserialization
    type_present: bool,
}

impl<T: LexiconType + PartialEq> TypedLexicon<T> {
    /// Creates a new TypedLexicon wrapper that will include the `$type` field when serialized.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            type_present: true,
        }
    }

    /// Creates a new TypedLexicon wrapper that will NOT include the `$type` field when serialized.
    pub fn new_without_type(inner: T) -> Self {
        Self {
            inner,
            type_present: false,
        }
    }

    /// Returns whether the type field was present during deserialization.
    pub fn has_type_field(&self) -> bool {
        self.type_present
    }

    /// Validates that the type field is present if required.
    pub fn validate(&self) -> Result<(), String> {
        if T::type_required() && !self.type_present {
            return Err(format!(
                "Missing required $type field for {}",
                T::lexicon_type()
            ));
        }
        Ok(())
    }

    /// Consumes the wrapper and returns the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Serialize for TypedLexicon<T>
where
    T: LexiconType + Serialize + PartialEq,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Get the serialized form of the inner value
        let value = serde_json::to_value(&self.inner).map_err(serde::ser::Error::custom)?;

        if let serde_json::Value::Object(mut map) = value {
            // Only add the $type field if type_present is true
            if self.type_present {
                map.insert(
                    "$type".to_string(),
                    serde_json::Value::String(T::lexicon_type().to_string()),
                );
            }

            // Serialize the modified map
            let mut ser_map = serializer.serialize_map(Some(map.len()))?;
            for (k, v) in map {
                ser_map.serialize_entry(&k, &v)?;
            }
            ser_map.end()
        } else {
            // If it's not an object, just serialize as-is (shouldn't happen with proper structs)
            self.inner.serialize(serializer)
        }
    }
}

impl<'de, T> Deserialize<'de> for TypedLexicon<T>
where
    T: LexiconType + DeserializeOwned + PartialEq,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TypedLexiconVisitor<T: LexiconType + PartialEq>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for TypedLexiconVisitor<T>
        where
            T: LexiconType + DeserializeOwned + PartialEq,
        {
            type Value = TypedLexicon<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a lexicon object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut obj = serde_json::Map::new();
                let mut type_found = false;
                let expected_type = T::lexicon_type();

                while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                    if key == "$type" {
                        type_found = true;
                        // Validate the type matches
                        if let serde_json::Value::String(ref type_str) = value {
                            if type_str != expected_type {
                                return Err(serde::de::Error::custom(format!(
                                    "Invalid $type field: expected '{}', found '{}'",
                                    expected_type, type_str
                                )));
                            }
                        } else {
                            return Err(serde::de::Error::custom("$type field must be a string"));
                        }
                        // Don't include $type in the object we deserialize to T
                    } else {
                        obj.insert(key, value);
                    }
                }

                // Deserialize the inner value from the cleaned object
                let inner = serde_json::from_value(serde_json::Value::Object(obj))
                    .map_err(serde::de::Error::custom)?;

                Ok(TypedLexicon {
                    inner,
                    type_present: type_found,
                })
            }
        }

        deserializer.deserialize_map(TypedLexiconVisitor(PhantomData))
    }
}

// Allow dereferencing to the inner type
impl<T: LexiconType + PartialEq> std::ops::Deref for TypedLexicon<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: LexiconType + PartialEq> std::ops::DerefMut for TypedLexicon<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct TestRecord {
        name: String,
        value: i32,
    }

    impl LexiconType for TestRecord {
        fn lexicon_type() -> &'static str {
            "test.lexicon.record"
        }
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
    struct OptionalTypeRecord {
        data: String,
    }

    impl LexiconType for OptionalTypeRecord {
        fn lexicon_type() -> &'static str {
            "test.lexicon.optional"
        }

        fn type_required() -> bool {
            false
        }
    }

    #[test]
    fn test_serialization_adds_type() {
        let record = TestRecord {
            name: "test".to_string(),
            value: 42,
        };
        let typed = TypedLexicon::new(record);

        let json = serde_json::to_value(&typed).unwrap();

        assert_eq!(json["$type"], "test.lexicon.record");
        assert_eq!(json["name"], "test");
        assert_eq!(json["value"], 42);
    }

    #[test]
    fn test_deserialization_validates_type() {
        let json = json!({
            "$type": "test.lexicon.record",
            "name": "test",
            "value": 42
        });

        let typed: TypedLexicon<TestRecord> = serde_json::from_value(json).unwrap();

        assert_eq!(typed.inner.name, "test");
        assert_eq!(typed.inner.value, 42);
        assert!(typed.has_type_field());
    }

    #[test]
    fn test_deserialization_without_type() {
        let json = json!({
            "name": "test",
            "value": 42
        });

        let typed: TypedLexicon<TestRecord> = serde_json::from_value(json).unwrap();

        assert_eq!(typed.inner.name, "test");
        assert_eq!(typed.inner.value, 42);
        assert!(!typed.has_type_field());
    }

    #[test]
    fn test_deserialization_wrong_type() {
        let json = json!({
            "$type": "wrong.type",
            "name": "test",
            "value": 42
        });

        let result: Result<TypedLexicon<TestRecord>, _> = serde_json::from_value(json);
        assert!(result.is_err());

        let err = result.unwrap_err().to_string();
        assert!(err.contains("expected 'test.lexicon.record'"));
        assert!(err.contains("found 'wrong.type'"));
    }

    #[test]
    fn test_validation_required_type() {
        let typed = TypedLexicon::new_without_type(TestRecord {
            name: "test".to_string(),
            value: 42,
        });

        let result = typed.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Missing required $type field"));
    }

    #[test]
    fn test_validation_optional_type() {
        let typed = TypedLexicon::new_without_type(OptionalTypeRecord {
            data: "test".to_string(),
        });

        let result = typed.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_round_trip() {
        let original = TestRecord {
            name: "round trip".to_string(),
            value: 123,
        };
        let typed = TypedLexicon::new(original.clone());

        let json = serde_json::to_string(&typed).unwrap();
        let deserialized: TypedLexicon<TestRecord> = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.inner, original);
        assert!(deserialized.has_type_field());
    }

    #[test]
    fn test_deref() {
        let typed = TypedLexicon::new(TestRecord {
            name: "deref test".to_string(),
            value: 456,
        });

        // Can access fields through deref
        assert_eq!(typed.name, "deref test");
        assert_eq!(typed.value, 456);
    }

    #[test]
    fn test_new_without_type_omits_type_field() {
        let record = TestRecord {
            name: "no type".to_string(),
            value: 99,
        };
        let typed = TypedLexicon::new_without_type(record);

        let json = serde_json::to_value(&typed).unwrap();

        // Should NOT have $type field
        assert!(!json.as_object().unwrap().contains_key("$type"));
        assert_eq!(json["name"], "no type");
        assert_eq!(json["value"], 99);
    }

    #[test]
    fn test_round_trip_preserves_type_absence() {
        // Deserialize without $type
        let json_without_type = json!({
            "name": "test",
            "value": 42
        });

        let typed: TypedLexicon<TestRecord> = serde_json::from_value(json_without_type).unwrap();
        assert!(!typed.has_type_field());

        // Re-serialize should NOT add $type
        let reserialized = serde_json::to_value(&typed).unwrap();
        assert!(!reserialized.as_object().unwrap().contains_key("$type"));
        assert_eq!(reserialized["name"], "test");
        assert_eq!(reserialized["value"], 42);
    }

    #[test]
    fn test_round_trip_preserves_type_presence() {
        // Deserialize with $type
        let json_with_type = json!({
            "$type": "test.lexicon.record",
            "name": "test",
            "value": 42
        });

        let typed: TypedLexicon<TestRecord> = serde_json::from_value(json_with_type).unwrap();
        assert!(typed.has_type_field());

        // Re-serialize should preserve $type
        let reserialized = serde_json::to_value(&typed).unwrap();
        assert_eq!(reserialized["$type"], "test.lexicon.record");
        assert_eq!(reserialized["name"], "test");
        assert_eq!(reserialized["value"], 42);
    }

    #[test]
    fn test_optional_type_record_behavior() {
        // Test with type_required() = false

        // new() should still add $type
        let with_type = TypedLexicon::new(OptionalTypeRecord {
            data: "with".to_string(),
        });
        let json = serde_json::to_value(&with_type).unwrap();
        assert_eq!(json["$type"], "test.lexicon.optional");

        // new_without_type() should omit $type
        let without_type = TypedLexicon::new_without_type(OptionalTypeRecord {
            data: "without".to_string(),
        });
        let json2 = serde_json::to_value(&without_type).unwrap();
        assert!(!json2.as_object().unwrap().contains_key("$type"));

        // Both should validate successfully since type is optional
        assert!(with_type.validate().is_ok());
        assert!(without_type.validate().is_ok());
    }
}
