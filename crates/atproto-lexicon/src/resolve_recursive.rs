//! Recursive lexicon resolution functionality for AT Protocol.
//!
//! This module provides recursive resolution of lexicons, following references
//! within lexicon schemas to resolve all dependent lexicons up to a specified depth.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use serde_json::Value;
use tracing::instrument;

use crate::errors::LexiconRecursiveError;
use crate::resolve::LexiconResolver;
use crate::validation::{absolute, extract_nsid_from_ref_object};

/// Configuration for recursive lexicon resolution.
#[derive(Debug, Clone)]
pub struct RecursiveResolverConfig {
    /// Maximum depth for recursive resolution (0 = only resolve the entry lexicon).
    pub max_depth: usize,
    /// Whether to include the entry lexicon in the results.
    pub include_entry: bool,
}

impl Default for RecursiveResolverConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            include_entry: true,
        }
    }
}

/// A lexicon resolver that recursively resolves referenced lexicons.
pub struct RecursiveLexiconResolver<R> {
    /// The underlying lexicon resolver.
    resolver: R,
    /// Configuration for recursive resolution.
    config: RecursiveResolverConfig,
}

impl<R> RecursiveLexiconResolver<R> {
    /// Create a new recursive lexicon resolver with default configuration.
    pub fn new(resolver: R) -> Self {
        Self {
            resolver,
            config: RecursiveResolverConfig::default(),
        }
    }

    /// Create a new recursive lexicon resolver with custom configuration.
    pub fn with_config(resolver: R, config: RecursiveResolverConfig) -> Self {
        Self { resolver, config }
    }

    /// Set the maximum depth for recursive resolution.
    pub fn set_max_depth(&mut self, max_depth: usize) {
        self.config.max_depth = max_depth;
    }

    /// Set whether to include the entry lexicon in the results.
    pub fn set_include_entry(&mut self, include_entry: bool) {
        self.config.include_entry = include_entry;
    }
}

impl<R> RecursiveLexiconResolver<R>
where
    R: LexiconResolver,
{
    /// Recursively resolve a lexicon and all its referenced lexicons.
    ///
    /// Returns a HashMap where keys are NSIDs and values are the resolved lexicon schemas.
    #[instrument(skip(self), err)]
    pub async fn resolve_recursive(&self, entry_nsid: &str) -> Result<HashMap<String, Value>> {
        let mut resolved = HashMap::new();
        let mut visited = HashSet::new();
        let mut to_resolve = HashSet::new();

        // Start with the entry lexicon
        to_resolve.insert(entry_nsid.to_string());

        // Resolve lexicons level by level
        for depth in 0..=self.config.max_depth {
            if to_resolve.is_empty() {
                break;
            }

            let current_batch = to_resolve.clone();
            to_resolve.clear();

            for nsid in current_batch {
                // Skip if already visited
                if visited.contains(&nsid) {
                    continue;
                }
                visited.insert(nsid.clone());

                // Skip the entry lexicon if configured to exclude it
                if !self.config.include_entry && nsid == entry_nsid && depth == 0 {
                    // Still need to extract references from it
                    match self.resolver.resolve(&nsid).await {
                        Ok(lexicon) => {
                            let refs = extract_lexicon_references(&lexicon);
                            to_resolve.extend(refs);
                        }
                        Err(e) => {
                            tracing::warn!(error = ?e, nsid = %nsid, "Failed to resolve lexicon");
                            continue;
                        }
                    }
                    continue;
                }

                // Resolve the lexicon
                match self.resolver.resolve(&nsid).await {
                    Ok(lexicon) => {
                        // Extract references for next level
                        if depth < self.config.max_depth {
                            let refs = extract_lexicon_references(&lexicon);
                            to_resolve.extend(refs);
                        }

                        // Store the resolved lexicon
                        resolved.insert(nsid.clone(), lexicon);
                    }
                    Err(e) => {
                        tracing::warn!(error = ?e, nsid = %nsid, "Failed to resolve lexicon");
                        continue;
                    }
                }
            }
        }

        if resolved.is_empty() && self.config.include_entry {
            return Err(LexiconRecursiveError::NoLexiconsResolved.into());
        }

        Ok(resolved)
    }

    /// Resolve a lexicon and return only its direct references.
    #[instrument(skip(self), err)]
    pub async fn get_direct_references(&self, nsid: &str) -> Result<HashSet<String>> {
        let lexicon = self.resolver.resolve(nsid).await?;
        Ok(extract_lexicon_references(&lexicon))
    }
}

/// Extract all lexicon references from a lexicon schema.
///
/// Looks for:
/// - Objects with `"type": "ref"` and extracts the `"ref"` field value
/// - Objects with `"type": "union"` and extracts NSIDs from the `"refs"` array
/// - Handles fragment-only references using the lexicon's `id` field as context
#[instrument(skip(value))]
pub fn extract_lexicon_references(value: &Value) -> HashSet<String> {
    // Extract the lexicon's ID to use as context for fragment-only references
    let context = value
        .as_object()
        .and_then(|obj| obj.get("id"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string());

    let mut references = HashSet::new();
    extract_references_recursive(value, &mut references, context.as_deref());
    references
}

/// Recursively extract references from a JSON value with optional context.
fn extract_references_recursive(
    value: &Value,
    references: &mut HashSet<String>,
    context: Option<&str>,
) {
    match value {
        Value::Object(map) => {
            // Check if this is a reference object
            if let Some(type_val) = map.get("type")
                && let Some(type_str) = type_val.as_str()
            {
                if type_str == "ref" {
                    // Handle ref objects with context for fragment-only refs
                    if let Some(ref_val) = map.get("ref").and_then(|v| v.as_str()) {
                        let absolute_ref = if let Some(ctx) = context {
                            absolute(ctx, ref_val)
                        } else {
                            ref_val.to_string()
                        };

                        // Now extract the NSID from the absolute reference
                        if let Some(nsid) = extract_nsid_from_ref_object(
                            serde_json::json!({
                                "type": "ref",
                                "ref": absolute_ref
                            })
                            .as_object()
                            .unwrap(),
                        ) {
                            references.insert(nsid);
                        }
                    }
                    return; // Don't recurse further into ref objects
                } else if type_str == "union" {
                    // Handle union objects with context for fragment-only refs
                    if let Some(refs_val) = map.get("refs")
                        && let Some(refs_array) = refs_val.as_array()
                    {
                        for ref_item in refs_array {
                            let ref_str = if let Some(s) = ref_item.as_str() {
                                s
                            } else if let Some(obj) = ref_item.as_object() {
                                if let Some(ref_val) = obj.get("ref").and_then(|v| v.as_str()) {
                                    ref_val
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            };

                            // Make fragment-only references absolute
                            let absolute_ref = if let Some(ctx) = context {
                                absolute(ctx, ref_str)
                            } else {
                                ref_str.to_string()
                            };

                            // Extract NSID from the absolute reference (stripping fragment)
                            let nsid = if let Some(hash_pos) = absolute_ref.find('#') {
                                &absolute_ref[..hash_pos]
                            } else {
                                &absolute_ref
                            };

                            // Validate it's a proper NSID
                            if nsid.contains('.') && !nsid.is_empty() {
                                references.insert(nsid.to_string());
                            }
                        }
                    }
                    return; // Don't recurse further into union objects
                }
            }

            // Otherwise, recursively check all values in the object
            for (_key, val) in map.iter() {
                extract_references_recursive(val, references, context);
            }
        }
        Value::Array(arr) => {
            // Recursively check all elements in the array
            for val in arr {
                extract_references_recursive(val, references, context);
            }
        }
        _ => {
            // Primitive values don't contain references
        }
    }
}

/// Result of recursive lexicon resolution.
#[derive(Debug, Clone)]
pub struct RecursiveResolutionResult {
    /// The resolved lexicons, keyed by NSID.
    pub lexicons: HashMap<String, Value>,
    /// NSIDs that were referenced but could not be resolved.
    pub failed: HashSet<String>,
    /// The dependency graph showing which lexicons reference which.
    pub dependencies: HashMap<String, HashSet<String>>,
}

impl<R> RecursiveLexiconResolver<R>
where
    R: LexiconResolver,
{
    /// Recursively resolve a lexicon with detailed results.
    ///
    /// This provides more information than `resolve_recursive`, including
    /// failed resolutions and the dependency graph.
    #[instrument(skip(self), err)]
    pub async fn resolve_with_details(
        &self,
        entry_nsid: &str,
    ) -> Result<RecursiveResolutionResult> {
        let mut lexicons = HashMap::new();
        let mut failed = HashSet::new();
        let mut dependencies = HashMap::new();
        let mut visited = HashSet::new();
        let mut to_resolve = HashSet::new();

        // Start with the entry lexicon
        to_resolve.insert(entry_nsid.to_string());

        // Resolve lexicons level by level
        for depth in 0..=self.config.max_depth {
            if to_resolve.is_empty() {
                break;
            }

            let current_batch = to_resolve.clone();
            to_resolve.clear();

            for nsid in current_batch {
                // Skip if already visited
                if visited.contains(&nsid) {
                    continue;
                }
                visited.insert(nsid.clone());

                // Resolve the lexicon
                match self.resolver.resolve(&nsid).await {
                    Ok(lexicon) => {
                        // Extract references
                        let refs = extract_lexicon_references(&lexicon);

                        // Record dependencies
                        if !refs.is_empty() {
                            dependencies.insert(nsid.clone(), refs.clone());
                        }

                        // Add references to resolve queue (if within depth limit)
                        if depth < self.config.max_depth {
                            to_resolve.extend(refs);
                        }

                        // Store the resolved lexicon (if configured to include it)
                        if self.config.include_entry || nsid != entry_nsid || depth > 0 {
                            lexicons.insert(nsid.clone(), lexicon);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = ?e, nsid = %nsid, "Failed to resolve lexicon");
                        failed.insert(nsid.clone());
                        continue;
                    }
                }
            }
        }

        Ok(RecursiveResolutionResult {
            lexicons,
            failed,
            dependencies,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_references() {
        let schema = serde_json::json!({
            "lexicon": 1,
            "id": "app.bsky.feed.post",
            "defs": {
                "main": {
                    "type": "record",
                    "record": {
                        "type": "object",
                        "properties": {
                            "text": {
                                "type": "string"
                            },
                            "embed": {
                                "type": "union",
                                "refs": [
                                    { "type": "ref", "ref": "app.bsky.embed.images" },
                                    { "type": "ref", "ref": "app.bsky.embed.external" },
                                    { "type": "ref", "ref": "#localref" }
                                ]
                            }
                        }
                    }
                }
            }
        });

        let refs = extract_lexicon_references(&schema);

        assert!(refs.contains("app.bsky.embed.images"));
        assert!(refs.contains("app.bsky.embed.external"));
        // Fragment-only reference #localref should be resolved to app.bsky.feed.post
        // (using the lexicon's id as context)
        assert!(refs.contains("app.bsky.feed.post"));
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_extract_nested_references() {
        let schema = serde_json::json!({
            "defs": {
                "main": {
                    "type": "object",
                    "properties": {
                        "nested": {
                            "type": "object",
                            "properties": {
                                "ref1": { "type": "ref", "ref": "com.example.schema1" },
                                "array": {
                                    "type": "array",
                                    "items": {
                                        "type": "union",
                                        "refs": [
                                            { "type": "ref", "ref": "#localref" },
                                            { "type": "ref", "ref": "com.example.schema3" }
                                        ]
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let refs = extract_lexicon_references(&schema);

        assert!(refs.contains("com.example.schema1"));
        assert!(refs.contains("com.example.schema3"));
        // Without an id field, fragment-only references cannot be resolved
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_fragment_only_with_context() {
        // Test that fragment-only references are properly resolved when lexicon has an ID
        let schema = serde_json::json!({
            "lexicon": 1,
            "id": "com.example.myschema",
            "defs": {
                "main": {
                    "type": "object",
                    "properties": {
                        "directRef": { "type": "ref", "ref": "#localDefinition" },
                        "unionRefs": {
                            "type": "union",
                            "refs": [
                                "#main",
                                "#otherDef",
                                "external.schema.type"
                            ]
                        },
                        "nestedRef": {
                            "type": "object",
                            "properties": {
                                "field": { "type": "ref", "ref": "#nested" }
                            }
                        }
                    }
                }
            }
        });

        let refs = extract_lexicon_references(&schema);

        // Fragment-only references should all resolve to com.example.myschema
        assert!(refs.contains("com.example.myschema"));
        assert!(refs.contains("external.schema.type"));
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_skip_invalid_references() {
        let schema = serde_json::json!({
            "defs": {
                "main": {
                    "refs": [
                        { "type": "ref", "ref": "valid.schema.name" },
                        { "type": "ref", "ref": "invalid" },  // No dots - should be skipped
                        { "type": "ref", "ref": "#localref" }, // Fragment-only, no ID context - should be skipped
                        { "type": "string", "ref": "not.a.ref" }, // Wrong type - should be skipped
                    ]
                }
            }
        });

        let refs = extract_lexicon_references(&schema);

        assert!(refs.contains("valid.schema.name"));
        // Only valid.schema.name should be extracted (no ID field, so #localref is skipped)
        assert_eq!(refs.len(), 1);
    }

    #[test]
    fn test_extract_union_references() {
        let schema = serde_json::json!({
            "defs": {
                "main": {
                    "type": "union",
                    "refs": [
                        "community.lexicon.calendar.event#uri",
                        "community.lexicon.location.address",
                        "community.lexicon.location.fsq",
                        "community.lexicon.location.geo",
                        "community.lexicon.location.hthree"
                    ]
                }
            }
        });

        let refs = extract_lexicon_references(&schema);

        // NSIDs should be extracted without fragment identifiers
        assert!(refs.contains("community.lexicon.calendar.event"));
        assert!(refs.contains("community.lexicon.location.address"));
        assert!(refs.contains("community.lexicon.location.fsq"));
        assert!(refs.contains("community.lexicon.location.geo"));
        assert!(refs.contains("community.lexicon.location.hthree"));
        assert_eq!(refs.len(), 5);
    }

    #[test]
    fn test_extract_mixed_union_references() {
        let schema = serde_json::json!({
            "defs": {
                "main": {
                    "type": "union",
                    "refs": [
                        "app.bsky.feed.post",
                        { "type": "ref", "ref": "app.bsky.actor.profile" },
                        "#app.bsky.graph.follow",  // Fragment-only, no ID context - should be skipped
                        "invalid",  // No dots - should be skipped
                    ]
                },
                "other": {
                    "type": "ref",
                    "ref": "app.bsky.embed.images"
                }
            }
        });

        let refs = extract_lexicon_references(&schema);

        assert!(refs.contains("app.bsky.feed.post"));
        assert!(refs.contains("app.bsky.actor.profile"));
        assert!(refs.contains("app.bsky.embed.images"));
        // #app.bsky.graph.follow is fragment-only with no ID context, should not be included
        assert!(!refs.contains("app.bsky.graph.follow"));
        assert!(!refs.contains("invalid"));
        assert_eq!(refs.len(), 3);
    }
}
