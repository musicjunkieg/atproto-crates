//! Input types for attestation functions supporting multiple input formats.

use serde::Serialize;
use serde_json::{Map, Value};
use std::convert::TryFrom;
use std::str::FromStr;
use thiserror::Error;

/// Flexible input type for attestation functions.
///
/// Allows passing records and metadata as JSON strings or any serde serializable types.
#[derive(Clone)]
pub enum AnyInput<S: Serialize + Clone> {
    /// JSON string representation
    String(String),
    /// Serializable types
    Serialize(S),
}

/// Error types for AnyInput parsing and transformation operations.
///
/// This enum provides specific error types for various failure modes when working
/// with `AnyInput`, including JSON parsing errors, type conversion errors, and
/// serialization failures.
#[derive(Debug, Error)]
pub enum AnyInputError {
    /// Error when parsing JSON from a string fails.
    #[error("Failed to parse JSON from string: {0}")]
    JsonParseError(#[from] serde_json::Error),

    /// Error when the value is not a JSON object.
    #[error("Expected JSON object, but got {value_type}")]
    NotAnObject {
        /// The actual type of the value.
        value_type: String,
    },

    /// Error when the string contains invalid JSON.
    #[error("Invalid JSON string: {message}")]
    InvalidJson {
        /// Error message describing what's wrong with the JSON.
        message: String,
    },
}

impl AnyInputError {
    /// Creates a new `NotAnObject` error with the actual type information.
    pub fn not_an_object(value: &Value) -> Self {
        let value_type = match value {
            Value::Null => "null".to_string(),
            Value::Bool(_) => "boolean".to_string(),
            Value::Number(_) => "number".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Object(_) => "object".to_string(), // Should not happen
        };

        AnyInputError::NotAnObject { value_type }
    }
}

/// Implementation of `FromStr` for `AnyInput` that deserializes JSON strings.
///
/// This allows parsing JSON strings directly into `AnyInput<serde_json::Value>` using
/// the standard `FromStr` trait. The string is deserialized using `serde_json::from_str`
/// and wrapped in `AnyInput::Serialize`.
///
/// # Errors
///
/// Returns `AnyInputError::JsonParseError` if the string contains invalid JSON.
///
/// # Example
///
/// ```
/// use atproto_attestation::input::AnyInput;
/// use std::str::FromStr;
///
/// let input: AnyInput<serde_json::Value> = r#"{"type": "post", "text": "Hello"}"#.parse().unwrap();
/// ```
impl FromStr for AnyInput<serde_json::Value> {
    type Err = AnyInputError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = serde_json::from_str(s)?;
        Ok(AnyInput::Serialize(value))
    }
}

impl<S: Serialize + Clone> From<S> for AnyInput<S> {
    fn from(value: S) -> Self {
        AnyInput::Serialize(value)
    }
}

/// Implementation of `TryFrom` for converting `AnyInput` into a JSON object map.
///
/// This allows converting any `AnyInput` into a `serde_json::Map<String, Value>`, which
/// represents a JSON object. Both string and serializable inputs are converted to JSON
/// objects, with appropriate error handling for non-object values.
///
/// # Example
///
/// ```
/// use atproto_attestation::input::AnyInput;
/// use serde_json::{json, Map, Value};
/// use std::convert::TryInto;
///
/// let input = AnyInput::Serialize(json!({"type": "post", "text": "Hello"}));
/// let map: Map<String, Value> = input.try_into().unwrap();
/// assert_eq!(map.get("type").unwrap(), "post");
/// ```
impl<S: Serialize + Clone> TryFrom<AnyInput<S>> for Map<String, Value> {
    type Error = AnyInputError;

    fn try_from(input: AnyInput<S>) -> Result<Self, Self::Error> {
        match input {
            AnyInput::String(value) => {
                // Parse string as JSON
                let json_value = serde_json::from_str::<Value>(&value)?;

                // Extract as object
                json_value
                    .as_object()
                    .cloned()
                    .ok_or_else(|| AnyInputError::not_an_object(&json_value))
            }
            AnyInput::Serialize(value) => {
                // Convert to JSON value
                let json_value = serde_json::to_value(value)?;

                // Extract as object
                json_value
                    .as_object()
                    .cloned()
                    .ok_or_else(|| AnyInputError::not_an_object(&json_value))
            }
        }
    }
}

/// Default phantom type for AnyInput when no specific lexicon type is needed.
///
/// This type serves as the default generic parameter for `AnyInput`, allowing
/// for simpler usage when working with untyped JSON values.
#[derive(Serialize, PartialEq, Clone)]
pub struct PhantomSignature {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_valid_json() {
        let json_str = r#"{"type": "post", "text": "Hello", "count": 42}"#;
        let result: Result<AnyInput<serde_json::Value>, _> = json_str.parse();

        assert!(result.is_ok());

        let input = result.unwrap();
        match input {
            AnyInput::Serialize(value) => {
                assert_eq!(value["type"], "post");
                assert_eq!(value["text"], "Hello");
                assert_eq!(value["count"], 42);
            }
            _ => panic!("Expected AnyInput::Serialize variant"),
        }
    }

    #[test]
    fn test_from_str_invalid_json() {
        let invalid_json = r#"{"type": "post", "text": "Hello" invalid json"#;
        let result: Result<AnyInput<serde_json::Value>, _> = invalid_json.parse();

        assert!(result.is_err());
    }

    #[test]
    fn test_from_str_array() {
        let json_array = r#"[1, 2, 3, "four"]"#;
        let result: Result<AnyInput<serde_json::Value>, _> = json_array.parse();

        assert!(result.is_ok());

        let input = result.unwrap();
        match input {
            AnyInput::Serialize(value) => {
                assert!(value.is_array());
                let array = value.as_array().unwrap();
                assert_eq!(array.len(), 4);
                assert_eq!(array[0], 1);
                assert_eq!(array[3], "four");
            }
            _ => panic!("Expected AnyInput::Serialize variant"),
        }
    }

    #[test]
    fn test_from_str_null() {
        let null_str = "null";
        let result: Result<AnyInput<serde_json::Value>, _> = null_str.parse();

        assert!(result.is_ok());

        let input = result.unwrap();
        match input {
            AnyInput::Serialize(value) => {
                assert!(value.is_null());
            }
            _ => panic!("Expected AnyInput::Serialize variant"),
        }
    }

    #[test]
    fn test_from_str_with_use() {
        // Test using the parse method directly with type inference
        let input: AnyInput<serde_json::Value> = r#"{"$type": "app.bsky.feed.post"}"#
            .parse()
            .expect("Failed to parse JSON");

        match input {
            AnyInput::Serialize(value) => {
                assert_eq!(value["$type"], "app.bsky.feed.post");
            }
            _ => panic!("Expected AnyInput::Serialize variant"),
        }
    }

    #[test]
    fn test_try_into_from_string() {
        use std::convert::TryInto;

        let input = AnyInput::<Value>::String(r#"{"type": "post", "text": "Hello"}"#.to_string());
        let result: Result<Map<String, Value>, _> = input.try_into();

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("type").unwrap(), "post");
        assert_eq!(map.get("text").unwrap(), "Hello");
    }

    #[test]
    fn test_try_into_from_serialize() {
        use serde_json::json;
        use std::convert::TryInto;

        let input = AnyInput::Serialize(json!({"$type": "app.bsky.feed.post", "count": 42}));
        let result: Result<Map<String, Value>, _> = input.try_into();

        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("$type").unwrap(), "app.bsky.feed.post");
        assert_eq!(map.get("count").unwrap(), 42);
    }

    #[test]
    fn test_try_into_string_not_object() {
        use std::convert::TryInto;

        let input = AnyInput::<Value>::String(r#"["array", "not", "object"]"#.to_string());
        let result: Result<Map<String, Value>, AnyInputError> = input.try_into();

        assert!(result.is_err());
        match result.unwrap_err() {
            AnyInputError::NotAnObject { value_type } => {
                assert_eq!(value_type, "array");
            }
            _ => panic!("Expected NotAnObject error"),
        }
    }

    #[test]
    fn test_try_into_serialize_not_object() {
        use serde_json::json;
        use std::convert::TryInto;

        let input = AnyInput::Serialize(json!([1, 2, 3]));
        let result: Result<Map<String, Value>, AnyInputError> = input.try_into();

        assert!(result.is_err());
        match result.unwrap_err() {
            AnyInputError::NotAnObject { value_type } => {
                assert_eq!(value_type, "array");
            }
            _ => panic!("Expected NotAnObject error"),
        }
    }

    #[test]
    fn test_try_into_invalid_json_string() {
        use std::convert::TryInto;

        let input = AnyInput::<Value>::String("not valid json".to_string());
        let result: Result<Map<String, Value>, AnyInputError> = input.try_into();

        assert!(result.is_err());
        match result.unwrap_err() {
            AnyInputError::JsonParseError(_) => {}
            _ => panic!("Expected JsonParseError"),
        }
    }

    #[test]
    fn test_try_into_null() {
        use serde_json::json;
        use std::convert::TryInto;

        let input = AnyInput::Serialize(json!(null));
        let result: Result<Map<String, Value>, AnyInputError> = input.try_into();

        assert!(result.is_err());
        match result.unwrap_err() {
            AnyInputError::NotAnObject { value_type } => {
                assert_eq!(value_type, "null");
            }
            _ => panic!("Expected NotAnObject error"),
        }
    }

    #[test]
    fn test_any_input_error_not_an_object() {
        use serde_json::json;

        // Test null
        let err = AnyInputError::not_an_object(&json!(null));
        match err {
            AnyInputError::NotAnObject { value_type } => {
                assert_eq!(value_type, "null");
            }
            _ => panic!("Expected NotAnObject error"),
        }

        // Test boolean
        let err = AnyInputError::not_an_object(&json!(true));
        match err {
            AnyInputError::NotAnObject { value_type } => {
                assert_eq!(value_type, "boolean");
            }
            _ => panic!("Expected NotAnObject error"),
        }

        // Test number
        let err = AnyInputError::not_an_object(&json!(42));
        match err {
            AnyInputError::NotAnObject { value_type } => {
                assert_eq!(value_type, "number");
            }
            _ => panic!("Expected NotAnObject error"),
        }

        // Test string
        let err = AnyInputError::not_an_object(&json!("hello"));
        match err {
            AnyInputError::NotAnObject { value_type } => {
                assert_eq!(value_type, "string");
            }
            _ => panic!("Expected NotAnObject error"),
        }

        // Test array
        let err = AnyInputError::not_an_object(&json!([1, 2, 3]));
        match err {
            AnyInputError::NotAnObject { value_type } => {
                assert_eq!(value_type, "array");
            }
            _ => panic!("Expected NotAnObject error"),
        }
    }

    #[test]
    fn test_error_display() {
        use serde_json::json;

        // Test NotAnObject error display
        let err = AnyInputError::not_an_object(&json!(42));
        assert_eq!(err.to_string(), "Expected JSON object, but got number");

        // Test InvalidJson display
        let err = AnyInputError::InvalidJson {
            message: "unexpected token".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid JSON string: unexpected token");
    }
}
