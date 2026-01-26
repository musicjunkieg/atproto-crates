//! Lexicon validation functionality for AT Protocol.
//!
//! This module provides validation of lexicon NSIDs, references, and schemas.

use std::fmt;

use anyhow::Result;
use serde_json::Value;

use crate::errors::{LexiconSchemaError, LexiconValidationError};

/// Components of a parsed NSID.
#[derive(Debug, Clone, PartialEq)]
pub struct NsidParts {
    /// The parts in original order (e.g., ["app", "bsky", "feed", "post"] for "app.bsky.feed.post")
    pub parts: Vec<String>,

    /// The optional fragment identifier (e.g., "uri" for "community.lexicon.calendar.event#uri")
    pub fragment: Option<String>,
}

impl NsidParts {}

impl fmt::Display for NsidParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let base = self.parts.join(".");
        match &self.fragment {
            Some(fragment) => write!(f, "{}#{}", base, fragment),
            None => write!(f, "{}", base),
        }
    }
}

/// Validates if a string is a valid NSID.
///
/// A valid NSID must:
/// - Contain at least one dot
/// - Have at least 3 parts when split by dots
/// - Not be empty
pub fn is_valid_nsid(nsid: &str) -> bool {
    if nsid.is_empty() {
        return false;
    }

    let parts: Vec<&str> = nsid.split('.').collect();
    parts.len() >= 3 && parts.iter().all(|p| !p.is_empty())
}

/// Validates if a string is a valid NSID reference.
///
/// This accepts:
/// - Regular NSIDs (e.g., "app.bsky.feed.post")
/// - NSIDs with fragment identifiers (e.g., "app.bsky.feed.post#uri")
///
/// This rejects:
/// - Fragment-only references (e.g., "#localref")
/// - Empty strings
/// - Invalid NSIDs without dots
pub fn is_valid_reference(reference: &str) -> bool {
    extract_nsid_from_reference(reference).is_some()
}

/// Extracts a clean NSID from a reference string.
///
/// Handles:
/// - Regular NSIDs (e.g., "app.bsky.feed.post")
/// - NSIDs with fragment identifiers (e.g., "app.bsky.feed.post#uri" -> "app.bsky.feed.post")
///
/// Returns None for:
/// - Fragment-only references (e.g., "#localref")
/// - Invalid NSIDs without dots
/// - Empty strings
pub fn extract_nsid_from_reference(reference: &str) -> Option<String> {
    if reference.is_empty() {
        return None;
    }

    // Fragment-only references (starting with #) are not NSIDs
    if reference.starts_with('#') {
        return None;
    }

    // Extract the NSID part (before any fragment identifier)
    let nsid = if let Some(hash_pos) = reference.find('#') {
        &reference[..hash_pos]
    } else {
        reference
    };

    // Validate the NSID part
    if nsid.is_empty() || !nsid.contains('.') {
        return None;
    }

    Some(nsid.to_string())
}

/// Converts a potentially relative NSID reference to an absolute one.
///
/// If the NSID starts with '#' (fragment-only), it concatenates the context with the NSID.
/// Otherwise, it returns the NSID as-is.
///
/// # Examples
/// ```
/// use atproto_lexicon::validation::absolute;
///
/// assert_eq!(absolute("app.bsky.feed.post", "#reply"), "app.bsky.feed.post#reply");
/// assert_eq!(absolute("app.bsky.feed.post", "com.example.other"), "com.example.other");
/// assert_eq!(absolute("app.bsky.feed.post", "#main"), "app.bsky.feed.post#main");
/// ```
pub fn absolute(context: &str, nsid: &str) -> String {
    if nsid.starts_with('#') {
        format!("{}{}", context, nsid)
    } else {
        nsid.to_string()
    }
}

/// Parses an NSID into its component parts, optionally with context.
///
/// # Parameters
/// - `nsid`: The NSID or fragment reference to parse
/// - `context`: Optional context NSID for resolving fragment-only references
///
/// # Behavior
/// - If `nsid` starts with "#" and no context: returns empty parts with fragment
/// - If `nsid` starts with "#" and context provided: uses context for parts, nsid (without #) for fragment
/// - Otherwise: splits on "#" to separate NSID from fragment, then splits NSID on "." for parts
/// - The special fragment "main" is treated as None
///
/// # Examples
/// ```
/// use atproto_lexicon::validation::parse_nsid;
///
/// // Regular NSID
/// let parts = parse_nsid("app.bsky.feed.post", None).unwrap();
/// assert_eq!(parts.parts, vec!["app", "bsky", "feed", "post"]);
/// assert_eq!(parts.fragment, None);
///
/// // NSID with fragment
/// let parts = parse_nsid("app.bsky.feed.post#uri", None).unwrap();
/// assert_eq!(parts.parts, vec!["app", "bsky", "feed", "post"]);
/// assert_eq!(parts.fragment, Some("uri".to_string()));
///
/// // Fragment-only without context
/// let parts = parse_nsid("#localref", None).unwrap();
/// assert_eq!(parts.parts, Vec::<String>::new());
/// assert_eq!(parts.fragment, Some("localref".to_string()));
///
/// // Fragment-only with context
/// let parts = parse_nsid("#localref", Some("app.bsky.feed.post".to_string())).unwrap();
/// assert_eq!(parts.parts, vec!["app", "bsky", "feed", "post"]);
/// assert_eq!(parts.fragment, Some("localref".to_string()));
///
/// // "main" fragment is treated as None
/// let parts = parse_nsid("app.bsky.feed.post#main", None).unwrap();
/// assert_eq!(parts.parts, vec!["app", "bsky", "feed", "post"]);
/// assert_eq!(parts.fragment, None);
/// ```
pub fn parse_nsid(nsid: &str, context: Option<String>) -> Result<NsidParts> {
    // Handle fragment-only references
    if let Some(fragment_str) = nsid.strip_prefix('#') {
        let fragment = if fragment_str == "main" || fragment_str.is_empty() {
            None
        } else {
            Some(fragment_str.to_string())
        };

        let parts = if let Some(ctx) = context {
            ctx.split('.').map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        return Ok(NsidParts { parts, fragment });
    }

    // Split on '#' to separate NSID from fragment
    let (nsid_part, fragment_part) = if let Some(hash_pos) = nsid.find('#') {
        (&nsid[..hash_pos], Some(&nsid[hash_pos + 1..]))
    } else {
        (nsid, None)
    };

    // Parse the NSID part
    if nsid_part.is_empty() {
        return Err(LexiconValidationError::EmptyNsid.into());
    }

    let parts: Vec<String> = nsid_part.split('.').map(|s| s.to_string()).collect();

    // Validate parts (at least 3 components for a valid NSID)
    if parts.len() < 3 {
        return Err(LexiconValidationError::InsufficientNsidParts {
            nsid: nsid.to_string(),
        }
        .into());
    }

    if parts.iter().any(|p| p.is_empty()) {
        return Err(LexiconValidationError::EmptyNsidParts {
            nsid: nsid.to_string(),
        }
        .into());
    }

    // Handle fragment
    let fragment = match fragment_part {
        Some("main") | Some("") => None,
        Some(frag) => Some(frag.to_string()),
        None => None,
    };

    Ok(NsidParts { parts, fragment })
}

/// Converts an NSID to a DNS name for lexicon resolution.
///
/// The conversion reverses the authority parts and prepends "_lexicon".
///
/// # Example
/// ```
/// use atproto_lexicon::validation::nsid_to_dns_name;
/// assert_eq!(
///     nsid_to_dns_name("app.bsky.feed.post").unwrap(),
///     "_lexicon.feed.bsky.app"
/// );
/// ```
pub fn nsid_to_dns_name(nsid: &str) -> Result<String> {
    let parsed = parse_nsid(nsid, None)?;

    // Need at least 3 parts for a valid NSID (authority + name + record_type)
    if parsed.parts.len() < 3 {
        return Err(LexiconValidationError::InsufficientNsidParts {
            nsid: nsid.to_string(),
        }
        .into());
    }

    // Build DNS name: _lexicon.<name>.<reversed-authority>
    let mut dns_parts = vec!["_lexicon".to_string()];

    // The name is the second-to-last part
    let name_idx = parsed.parts.len() - 2;
    dns_parts.push(parsed.parts[name_idx].clone());

    // Add authority parts in reverse order (all parts except the last two)
    for i in (0..name_idx).rev() {
        dns_parts.push(parsed.parts[i].clone());
    }

    Ok(dns_parts.join("."))
}

/// Checks if a JSON object represents a reference type.
///
/// A reference object has `"type": "ref"` and a `"ref"` field.
pub fn is_reference_object(obj: &serde_json::Map<String, Value>) -> bool {
    matches!(
        obj.get("type"),
        Some(Value::String(type_val)) if type_val == "ref"
    ) && obj.contains_key("ref")
}

/// Checks if a JSON object represents a union type.
///
/// A union object has `"type": "union"` and a `"refs"` array field.
pub fn is_union_object(obj: &serde_json::Map<String, Value>) -> bool {
    matches!(
        obj.get("type"),
        Some(Value::String(type_val)) if type_val == "union"
    ) && matches!(obj.get("refs"), Some(Value::Array(_)))
}

/// Extracts an NSID from a reference object.
///
/// Returns None if the object is not a valid reference or the NSID is invalid.
pub fn extract_nsid_from_ref_object(obj: &serde_json::Map<String, Value>) -> Option<String> {
    if !is_reference_object(obj) {
        return None;
    }

    if let Some(Value::String(ref_val)) = obj.get("ref") {
        extract_nsid_from_reference(ref_val)
    } else {
        None
    }
}

/// Extracts NSIDs from a union object's refs array.
///
/// Handles both direct string references and nested reference objects.
pub fn extract_nsids_from_union_object(obj: &serde_json::Map<String, Value>) -> Vec<String> {
    if !is_union_object(obj) {
        return Vec::new();
    }

    let mut nsids = Vec::new();

    if let Some(Value::Array(refs_array)) = obj.get("refs") {
        for ref_item in refs_array {
            match ref_item {
                Value::String(ref_str) => {
                    if let Some(nsid) = extract_nsid_from_reference(ref_str) {
                        nsids.push(nsid);
                    }
                }
                Value::Object(ref_obj) => {
                    if let Some(nsid) = extract_nsid_from_ref_object(ref_obj) {
                        nsids.push(nsid);
                    }
                }
                _ => {}
            }
        }
    }

    nsids
}

/// Validates a complete lexicon schema.
///
/// Checks for:
/// - Required fields (lexicon version, id, defs)
/// - Valid NSID in the id field
/// - Well-formed definitions
pub fn validate_lexicon_schema(schema: &Value) -> Result<()> {
    let obj = schema.as_object().ok_or(LexiconSchemaError::NotAnObject)?;

    // Check lexicon version
    if !obj.contains_key("lexicon") {
        return Err(LexiconSchemaError::MissingLexiconVersion.into());
    }

    // Check and validate ID
    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or(LexiconSchemaError::MissingOrInvalidId)?;

    if !is_valid_nsid(id) {
        return Err(LexiconValidationError::InvalidSchemaId { id: id.to_string() }.into());
    }

    // Check defs exists and is an object
    obj.get("defs")
        .and_then(|v| v.as_object())
        .ok_or(LexiconSchemaError::MissingOrInvalidDefs)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_nsid() {
        assert!(is_valid_nsid("app.bsky.feed.post"));
        assert!(is_valid_nsid("com.example.service.method"));
        assert!(is_valid_nsid("a.b.c"));

        assert!(!is_valid_nsid("app.bsky")); // Too few parts
        assert!(!is_valid_nsid("app")); // Too few parts
        assert!(!is_valid_nsid("")); // Empty
        assert!(!is_valid_nsid("app..feed.post")); // Empty part
    }

    #[test]
    fn test_extract_nsid_from_reference() {
        // Valid NSID
        assert_eq!(
            extract_nsid_from_reference("app.bsky.feed.post"),
            Some("app.bsky.feed.post".to_string())
        );

        // NSID with fragment identifier
        assert_eq!(
            extract_nsid_from_reference("app.bsky.feed.post#uri"),
            Some("app.bsky.feed.post".to_string())
        );

        // Fragment-only references should return None
        assert_eq!(extract_nsid_from_reference("#app.bsky.feed.post"), None);
        assert_eq!(extract_nsid_from_reference("#localref"), None);
        assert_eq!(extract_nsid_from_reference("#"), None);

        // Invalid formats
        assert_eq!(extract_nsid_from_reference("#app.bsky.feed.post#uri"), None); // Starts with #
        assert_eq!(extract_nsid_from_reference("invalid"), None); // No dots
        assert_eq!(extract_nsid_from_reference(""), None); // Empty
        assert_eq!(extract_nsid_from_reference("#com.example#foo"), None); // Multiple fragments
    }

    #[test]
    fn test_absolute() {
        // Fragment-only references should be made absolute with context
        assert_eq!(
            absolute("app.bsky.feed.post", "#reply"),
            "app.bsky.feed.post#reply"
        );
        assert_eq!(
            absolute("com.example.schema", "#main"),
            "com.example.schema#main"
        );
        assert_eq!(absolute("a.b.c", "#"), "a.b.c#");

        // Already absolute NSIDs should be returned as-is
        assert_eq!(
            absolute("app.bsky.feed.post", "com.example.other"),
            "com.example.other"
        );
        assert_eq!(
            absolute("app.bsky.feed.post", "app.bsky.actor.profile"),
            "app.bsky.actor.profile"
        );
        assert_eq!(
            absolute("ignored.context", "app.bsky.feed.post#uri"),
            "app.bsky.feed.post#uri"
        );
    }

    #[test]
    fn test_parse_nsid() {
        // Basic 4-part NSID
        let parts = parse_nsid("app.bsky.feed.post", None).unwrap();
        assert_eq!(parts.parts, vec!["app", "bsky", "feed", "post"]);
        assert_eq!(parts.fragment, None);

        // 4-part NSID with different authority
        let parts = parse_nsid("com.example.service.method", None).unwrap();
        assert_eq!(parts.parts, vec!["com", "example", "service", "method"]);
        assert_eq!(parts.fragment, None);

        // NSID with fragment
        let parts = parse_nsid("app.bsky.feed.post#reply", None).unwrap();
        assert_eq!(parts.parts, vec!["app", "bsky", "feed", "post"]);
        assert_eq!(parts.fragment, Some("reply".to_string()));

        // "main" fragment should be treated as None
        let parts = parse_nsid("app.bsky.feed.post#main", None).unwrap();
        assert_eq!(parts.parts, vec!["app", "bsky", "feed", "post"]);
        assert_eq!(parts.fragment, None);

        // Fragment-only without context
        let parts = parse_nsid("#reply", None).unwrap();
        assert_eq!(parts.parts, Vec::<String>::new());
        assert_eq!(parts.fragment, Some("reply".to_string()));

        // Fragment-only with context
        let parts = parse_nsid("#reply", Some("app.bsky.feed.post".to_string())).unwrap();
        assert_eq!(parts.parts, vec!["app", "bsky", "feed", "post"]);
        assert_eq!(parts.fragment, Some("reply".to_string()));

        // Too few parts
        assert!(parse_nsid("app.bsky", None).is_err());
        assert!(parse_nsid("", None).is_err());
    }

    #[test]
    fn test_nsid_parts_serialization() {
        // Basic NSID without fragment
        let parts = NsidParts {
            parts: vec![
                "app".to_string(),
                "bsky".to_string(),
                "feed".to_string(),
                "post".to_string(),
            ],
            fragment: None,
        };
        assert_eq!(parts.to_string(), "app.bsky.feed.post");
        assert_eq!(format!("{}", parts), "app.bsky.feed.post"); // Test Display trait

        // NSID with fragment
        let parts_with_fragment = NsidParts {
            parts: vec![
                "com".to_string(),
                "example".to_string(),
                "schema".to_string(),
                "type".to_string(),
            ],
            fragment: Some("reply".to_string()),
        };
        assert_eq!(
            parts_with_fragment.to_string(),
            "com.example.schema.type#reply"
        );
        assert_eq!(
            format!("{}", parts_with_fragment),
            "com.example.schema.type#reply"
        );

        // Empty parts with fragment (edge case from fragment-only parsing)
        let fragment_only = NsidParts {
            parts: vec![],
            fragment: Some("localref".to_string()),
        };
        assert_eq!(fragment_only.to_string(), "#localref");

        // Round-trip test: parse and serialize
        let original = "app.bsky.feed.post#main";
        let parsed = parse_nsid(original, None).unwrap();
        // Note: "main" is treated as None, so this won't round-trip exactly
        assert_eq!(parsed.to_string(), "app.bsky.feed.post");

        let original_with_fragment = "app.bsky.feed.post#reply";
        let parsed_with_fragment = parse_nsid(original_with_fragment, None).unwrap();
        assert_eq!(parsed_with_fragment.to_string(), original_with_fragment);
    }

    #[test]
    fn test_nsid_to_dns_name() {
        assert_eq!(
            nsid_to_dns_name("app.bsky.feed.post").unwrap(),
            "_lexicon.feed.bsky.app"
        );

        assert_eq!(
            nsid_to_dns_name("com.atproto.repo.getRecord").unwrap(),
            "_lexicon.repo.atproto.com"
        );

        assert_eq!(
            nsid_to_dns_name("org.example.deeply.nested.service.action").unwrap(),
            "_lexicon.service.nested.deeply.example.org"
        );

        // "main" fragment doesn't affect DNS name generation
        assert_eq!(
            nsid_to_dns_name("app.bsky.feed.main").unwrap(),
            "_lexicon.feed.bsky.app"
        );

        assert!(nsid_to_dns_name("app.bsky").is_err());
    }

    #[test]
    fn test_reference_object_detection() {
        let ref_obj = serde_json::json!({
            "type": "ref",
            "ref": "app.bsky.feed.post"
        });
        assert!(is_reference_object(ref_obj.as_object().unwrap()));

        let not_ref = serde_json::json!({
            "type": "string",
            "ref": "app.bsky.feed.post"
        });
        assert!(!is_reference_object(not_ref.as_object().unwrap()));

        let missing_ref = serde_json::json!({
            "type": "ref"
        });
        assert!(!is_reference_object(missing_ref.as_object().unwrap()));
    }

    #[test]
    fn test_union_object_detection() {
        let union_obj = serde_json::json!({
            "type": "union",
            "refs": ["app.bsky.feed.post", "app.bsky.actor.profile"]
        });
        assert!(is_union_object(union_obj.as_object().unwrap()));

        let not_union = serde_json::json!({
            "type": "ref",
            "refs": ["app.bsky.feed.post"]
        });
        assert!(!is_union_object(not_union.as_object().unwrap()));

        let invalid_refs = serde_json::json!({
            "type": "union",
            "refs": "not-an-array"
        });
        assert!(!is_union_object(invalid_refs.as_object().unwrap()));
    }

    #[test]
    fn test_extract_nsids_from_union() {
        let union_obj = serde_json::json!({
            "type": "union",
            "refs": [
                "app.bsky.feed.post",
                "#app.bsky.actor.profile",  // Fragment-only, should be skipped
                "app.bsky.graph.follow#uri",  // NSID with fragment
                { "type": "ref", "ref": "app.bsky.embed.images" },
                "invalid"  // No dots, should be skipped
            ]
        });

        let nsids = extract_nsids_from_union_object(union_obj.as_object().unwrap());
        assert_eq!(nsids.len(), 3);
        assert!(nsids.contains(&"app.bsky.feed.post".to_string()));
        assert!(nsids.contains(&"app.bsky.graph.follow".to_string())); // Fragment removed
        assert!(nsids.contains(&"app.bsky.embed.images".to_string()));
        // #app.bsky.actor.profile is fragment-only, so it's not included
        assert!(!nsids.contains(&"app.bsky.actor.profile".to_string()));
    }

    #[test]
    fn test_validate_lexicon_schema() {
        let valid_schema = serde_json::json!({
            "lexicon": 1,
            "id": "app.bsky.feed.post",
            "defs": {
                "main": {}
            }
        });
        assert!(validate_lexicon_schema(&valid_schema).is_ok());

        let missing_lexicon = serde_json::json!({
            "id": "app.bsky.feed.post",
            "defs": {}
        });
        assert!(validate_lexicon_schema(&missing_lexicon).is_err());

        let invalid_id = serde_json::json!({
            "lexicon": 1,
            "id": "invalid",
            "defs": {}
        });
        assert!(validate_lexicon_schema(&invalid_id).is_err());

        let missing_defs = serde_json::json!({
            "lexicon": 1,
            "id": "app.bsky.feed.post"
        });
        assert!(validate_lexicon_schema(&missing_defs).is_err());
    }
}
