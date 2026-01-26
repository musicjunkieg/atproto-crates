//! Rich text facet parsing for AT Protocol.
//!
//! This module provides functionality for extracting semantic annotations (facets)
//! from plain text. Facets include mentions, links (URLs), and hashtags.
//!
//! # Overview
//!
//! AT Protocol rich text uses "facets" to annotate specific byte ranges within text with
//! semantic meaning. This module handles:
//!
//! - **Parsing**: Extract mentions, URLs, and hashtags from plain text
//! - **Facet Creation**: Build proper AT Protocol facet structures with resolved DIDs
//!
//! # Byte Offset Calculation
//!
//! This implementation correctly uses UTF-8 byte offsets as required by AT Protocol.
//! The facets use "inclusive start and exclusive end" byte ranges. All parsing is done
//! using `regex::bytes::Regex` which operates on byte slices and returns byte positions,
//! ensuring correct handling of multi-byte UTF-8 characters (emojis, CJK, accented chars).
//!
//! # Example
//!
//! ```ignore
//! use atproto_extras::facets::{parse_urls, parse_tags, FacetLimits};
//! use atproto_record::lexicon::app::bsky::richtext::facet::FacetFeature;
//!
//! let text = "Check out https://example.com #rust";
//!
//! // Parse URLs and tags as Facet objects
//! let url_facets = parse_urls(text);
//! let tag_facets = parse_tags(text);
//!
//! // Access facet data directly
//! for facet in url_facets {
//!     if let Some(FacetFeature::Link(link)) = facet.features.first() {
//!         println!("URL at bytes {}..{}: {}",
//!             facet.index.byte_start, facet.index.byte_end, link.uri);
//!     }
//! }
//! ```

use atproto_identity::resolve::IdentityResolver;
use atproto_record::lexicon::app::bsky::richtext::facet::{
    ByteSlice, Facet, FacetFeature, Link, Mention, Tag,
};
use regex::bytes::Regex;

/// Configuration for facet parsing limits.
///
/// These limits protect against abuse by capping the number of facets
/// that will be processed. This is important for both performance and
/// security when handling user-generated content.
///
/// # Example
///
/// ```
/// use atproto_extras::FacetLimits;
///
/// // Use defaults
/// let limits = FacetLimits::default();
///
/// // Or customize
/// let custom = FacetLimits {
///     mentions_max: 10,
///     tags_max: 10,
///     links_max: 10,
///     max: 20,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct FacetLimits {
    /// Maximum number of mention facets to process (default: 5)
    pub mentions_max: usize,
    /// Maximum number of tag facets to process (default: 5)
    pub tags_max: usize,
    /// Maximum number of link facets to process (default: 5)
    pub links_max: usize,
    /// Maximum total number of facets to process (default: 10)
    pub max: usize,
}

impl Default for FacetLimits {
    fn default() -> Self {
        Self {
            mentions_max: 5,
            tags_max: 5,
            links_max: 5,
            max: 10,
        }
    }
}

/// Parse mentions from text and return them as Facet objects with resolved DIDs.
///
/// This function extracts AT Protocol handle mentions (e.g., `@alice.bsky.social`)
/// from text, resolves each handle to a DID using the provided identity resolver,
/// and returns AT Protocol Facet objects with Mention features.
///
/// Mentions that cannot be resolved to a valid DID are skipped. Mentions that
/// appear within URLs are also excluded to avoid false positives.
///
/// # Arguments
///
/// * `text` - The text to parse for mentions
/// * `identity_resolver` - Resolver for converting handles to DIDs
/// * `limits` - Configuration for maximum mentions to process
///
/// # Returns
///
/// A vector of Facet objects for successfully resolved mentions.
///
/// # Example
///
/// ```ignore
/// use atproto_extras::{parse_mentions, FacetLimits};
/// use atproto_record::lexicon::app::bsky::richtext::facet::FacetFeature;
///
/// let text = "Hello @alice.bsky.social!";
/// let limits = FacetLimits::default();
///
/// // Requires an async context and identity resolver
/// let facets = parse_mentions(text, &resolver, &limits).await;
///
/// for facet in facets {
///     if let Some(FacetFeature::Mention(mention)) = facet.features.first() {
///         println!("Mention {} resolved to {}",
///             &text[facet.index.byte_start..facet.index.byte_end],
///             mention.did);
///     }
/// }
/// ```
pub async fn parse_mentions(
    text: &str,
    identity_resolver: &dyn IdentityResolver,
    limits: &FacetLimits,
) -> Vec<Facet> {
    let mut facets = Vec::new();

    // First, parse all URLs to exclude mention matches within them
    let url_facets = parse_urls(text);

    // Regex based on: https://atproto.com/specs/handle#handle-identifier-syntax
    // Pattern: [$|\W](@([a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)
    let mention_regex = Regex::new(
        r"(?:^|[^\w])(@([a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)",
    )
    .unwrap();

    let text_bytes = text.as_bytes();
    let mut mention_count = 0;

    for capture in mention_regex.captures_iter(text_bytes) {
        if mention_count >= limits.mentions_max {
            break;
        }

        if let Some(mention_match) = capture.get(1) {
            let start = mention_match.start();
            let end = mention_match.end();

            // Check if this mention overlaps with any URL
            let overlaps_url = url_facets.iter().any(|facet| {
                // Check if mention is within or overlaps the URL span
                (start >= facet.index.byte_start && start < facet.index.byte_end)
                    || (end > facet.index.byte_start && end <= facet.index.byte_end)
            });

            // Only process the mention if it doesn't overlap with a URL
            if !overlaps_url {
                let handle = std::str::from_utf8(&mention_match.as_bytes()[1..])
                    .unwrap_or_default()
                    .to_string();

                // Try to resolve the handle to a DID
                // First try with at:// prefix, then without
                let at_uri = format!("at://{}", handle);
                let did_result = match identity_resolver.resolve(&at_uri).await {
                    Ok(doc) => Ok(doc),
                    Err(_) => identity_resolver.resolve(&handle).await,
                };

                // Only add the mention facet if we successfully resolved the DID
                if let Ok(did_doc) = did_result {
                    facets.push(Facet {
                        index: ByteSlice {
                            byte_start: start,
                            byte_end: end,
                        },
                        features: vec![FacetFeature::Mention(Mention {
                            did: did_doc.id.to_string(),
                        })],
                    });
                    mention_count += 1;
                }
            }
        }
    }

    facets
}

/// Parse URLs from text and return them as Facet objects.
///
/// This function extracts HTTP and HTTPS URLs from text with correct
/// byte position tracking for UTF-8 text, returning AT Protocol Facet objects
/// with Link features.
///
/// # Supported URL Patterns
///
/// - HTTP URLs: `http://example.com`
/// - HTTPS URLs: `https://example.com`
/// - URLs with paths, query strings, and fragments
/// - URLs with subdomains: `https://www.example.com`
///
/// # Example
///
/// ```
/// use atproto_extras::parse_urls;
/// use atproto_record::lexicon::app::bsky::richtext::facet::FacetFeature;
///
/// let text = "Visit https://example.com/path?query=1 for more info";
/// let facets = parse_urls(text);
///
/// assert_eq!(facets.len(), 1);
/// assert_eq!(facets[0].index.byte_start, 6);
/// assert_eq!(facets[0].index.byte_end, 38);
/// if let Some(FacetFeature::Link(link)) = facets[0].features.first() {
///     assert_eq!(link.uri, "https://example.com/path?query=1");
/// }
/// ```
///
/// # Multi-byte Character Handling
///
/// Byte positions are correctly calculated even with emojis and other
/// multi-byte UTF-8 characters:
///
/// ```
/// use atproto_extras::parse_urls;
/// use atproto_record::lexicon::app::bsky::richtext::facet::FacetFeature;
///
/// let text = "Check out https://example.com now!";
/// let facets = parse_urls(text);
/// let text_bytes = text.as_bytes();
///
/// // The byte slice matches the URL
/// let url_bytes = &text_bytes[facets[0].index.byte_start..facets[0].index.byte_end];
/// assert_eq!(std::str::from_utf8(url_bytes).unwrap(), "https://example.com");
/// ```
pub fn parse_urls(text: &str) -> Vec<Facet> {
    let mut facets = Vec::new();

    // Partial/naive URL regex based on: https://stackoverflow.com/a/3809435
    // Pattern: [$|\W](https?:\/\/(www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]+\b([-a-zA-Z0-9()@:%_\+.~#?&//=]*[-a-zA-Z0-9@%_\+~#//=])?)
    // Modified to use + instead of {1,6} to support longer TLDs and multi-level subdomains
    let url_regex = Regex::new(
        r"(?:^|[^\w])(https?://(?:www\.)?[-a-zA-Z0-9@:%._\+~#=]{1,256}\.[a-zA-Z0-9()]+\b(?:[-a-zA-Z0-9()@:%_\+.~#?&//=]*[-a-zA-Z0-9@%_\+~#//=])?)"
    ).unwrap();

    let text_bytes = text.as_bytes();
    for capture in url_regex.captures_iter(text_bytes) {
        if let Some(url_match) = capture.get(1) {
            let url = std::str::from_utf8(url_match.as_bytes())
                .unwrap_or_default()
                .to_string();

            facets.push(Facet {
                index: ByteSlice {
                    byte_start: url_match.start(),
                    byte_end: url_match.end(),
                },
                features: vec![FacetFeature::Link(Link { uri: url })],
            });
        }
    }

    facets
}

/// Parse hashtags from text and return them as Facet objects.
///
/// This function extracts hashtags (e.g., `#rust`, `#ATProto`) from text,
/// returning AT Protocol Facet objects with Tag features.
/// It supports both standard `#` and full-width `＃` (U+FF03) hash symbols.
///
/// # Tag Syntax
///
/// - Tags must start with `#` or `＃` (full-width)
/// - Tag content follows word character rules (`\w`)
/// - Purely numeric tags (e.g., `#123`) are excluded
///
/// # Example
///
/// ```
/// use atproto_extras::parse_tags;
/// use atproto_record::lexicon::app::bsky::richtext::facet::FacetFeature;
///
/// let text = "Learning #rust and #golang today! #100DaysOfCode";
/// let facets = parse_tags(text);
///
/// assert_eq!(facets.len(), 3);
/// if let Some(FacetFeature::Tag(tag)) = facets[0].features.first() {
///     assert_eq!(tag.tag, "rust");
/// }
/// if let Some(FacetFeature::Tag(tag)) = facets[1].features.first() {
///     assert_eq!(tag.tag, "golang");
/// }
/// if let Some(FacetFeature::Tag(tag)) = facets[2].features.first() {
///     assert_eq!(tag.tag, "100DaysOfCode");
/// }
/// ```
///
/// # Numeric Tags
///
/// Purely numeric tags are excluded:
///
/// ```
/// use atproto_extras::parse_tags;
///
/// let text = "Item #42 is special";
/// let facets = parse_tags(text);
///
/// // #42 is not extracted because it's purely numeric
/// assert_eq!(facets.len(), 0);
/// ```
pub fn parse_tags(text: &str) -> Vec<Facet> {
    let mut facets = Vec::new();

    // Regex based on: https://github.com/bluesky-social/atproto/blob/d91988fe79030b61b556dd6f16a46f0c3b9d0b44/packages/api/src/rich-text/util.ts
    // Simplified for Rust - matches hashtags at word boundaries
    // Pattern matches: start of string or non-word char, then # or ＃, then tag content
    let tag_regex = Regex::new(r"(?:^|[^\w])([#\xEF\xBC\x83])([\w]+(?:[\w]*)*)").unwrap();

    let text_bytes = text.as_bytes();

    // Work with bytes for proper position tracking
    for capture in tag_regex.captures_iter(text_bytes) {
        if let (Some(full_match), Some(hash_match), Some(tag_match)) =
            (capture.get(0), capture.get(1), capture.get(2))
        {
            // Calculate the absolute byte position of the hash symbol
            // The full match includes the preceding character (if any)
            // so we need to adjust for that
            let match_start = full_match.start();
            let hash_offset = hash_match.start() - full_match.start();
            let start = match_start + hash_offset;
            let end = match_start + hash_offset + hash_match.len() + tag_match.len();

            // Extract just the tag text (without the hash symbol)
            let tag = std::str::from_utf8(tag_match.as_bytes()).unwrap_or_default();

            // Only include tags that are not purely numeric
            if !tag.chars().all(|c| c.is_ascii_digit()) {
                facets.push(Facet {
                    index: ByteSlice {
                        byte_start: start,
                        byte_end: end,
                    },
                    features: vec![FacetFeature::Tag(Tag {
                        tag: tag.to_string(),
                    })],
                });
            }
        }
    }

    facets
}

/// Parse facets from text and return a vector of Facet objects.
///
/// This function extracts mentions, URLs, and hashtags from the provided text
/// and creates AT Protocol facets with proper byte indices.
///
/// Mentions are resolved to actual DIDs using the provided identity resolver.
/// If a handle cannot be resolved to a DID, the mention facet is skipped.
///
/// # Arguments
///
/// * `text` - The text to extract facets from
/// * `identity_resolver` - Resolver for converting handles to DIDs
/// * `limits` - Configuration for maximum facets per type and total
///
/// # Returns
///
/// Optional vector of facets. Returns `None` if no facets were found.
///
/// # Example
///
/// ```ignore
/// use atproto_extras::{parse_facets_from_text, FacetLimits};
///
/// let text = "Hello @alice.bsky.social! Check #rust at https://rust-lang.org";
/// let limits = FacetLimits::default();
///
/// // Requires an async context and identity resolver
/// let facets = parse_facets_from_text(text, &resolver, &limits).await;
///
/// if let Some(facets) = facets {
///     for facet in &facets {
///         println!("Facet at {}..{}", facet.index.byte_start, facet.index.byte_end);
///     }
/// }
/// ```
///
/// # Mention Resolution
///
/// Mentions are only included if the handle resolves to a valid DID:
///
/// ```ignore
/// let text = "@valid.handle.com and @invalid.handle.xyz";
/// let facets = parse_facets_from_text(text, &resolver, &limits).await;
///
/// // Only @valid.handle.com appears as a facet if @invalid.handle.xyz
/// // cannot be resolved to a DID
/// ```
pub async fn parse_facets_from_text(
    text: &str,
    identity_resolver: &dyn IdentityResolver,
    limits: &FacetLimits,
) -> Option<Vec<Facet>> {
    let mut facets = Vec::new();

    // Parse mentions (already limited by mentions_max in parse_mentions)
    let mention_facets = parse_mentions(text, identity_resolver, limits).await;
    facets.extend(mention_facets);

    // Parse URLs (limited by links_max)
    let url_facets = parse_urls(text);
    for (idx, facet) in url_facets.into_iter().enumerate() {
        if idx >= limits.links_max {
            break;
        }
        facets.push(facet);
    }

    // Parse hashtags (limited by tags_max)
    let tag_facets = parse_tags(text);
    for (idx, facet) in tag_facets.into_iter().enumerate() {
        if idx >= limits.tags_max {
            break;
        }
        facets.push(facet);
    }

    // Apply global facet limit (truncate if exceeds max)
    if facets.len() > limits.max {
        facets.truncate(limits.max);
    }

    // Only return facets if we found any
    if !facets.is_empty() {
        Some(facets)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use atproto_identity::model::Document;
    use std::collections::HashMap;

    use super::*;

    /// Mock identity resolver for testing
    struct MockIdentityResolver {
        handles_to_dids: HashMap<String, String>,
    }

    impl MockIdentityResolver {
        fn new() -> Self {
            let mut handles_to_dids = HashMap::new();
            handles_to_dids.insert(
                "alice.bsky.social".to_string(),
                "did:plc:alice123".to_string(),
            );
            handles_to_dids.insert(
                "at://alice.bsky.social".to_string(),
                "did:plc:alice123".to_string(),
            );
            Self { handles_to_dids }
        }

        fn add_identity(&mut self, handle: &str, did: &str) {
            self.handles_to_dids
                .insert(handle.to_string(), did.to_string());
            self.handles_to_dids
                .insert(format!("at://{}", handle), did.to_string());
        }
    }

    #[async_trait]
    impl IdentityResolver for MockIdentityResolver {
        async fn resolve(&self, handle: &str) -> anyhow::Result<Document> {
            let handle_key = handle.to_string();

            if let Some(did) = self.handles_to_dids.get(&handle_key) {
                Ok(Document {
                    context: vec![],
                    id: did.clone(),
                    also_known_as: vec![format!("at://{}", handle_key.trim_start_matches("at://"))],
                    verification_method: vec![],
                    service: vec![],
                    extra: HashMap::new(),
                })
            } else {
                Err(anyhow::anyhow!("Handle not found"))
            }
        }
    }

    #[tokio::test]
    async fn test_parse_facets_from_text_comprehensive() {
        let mut resolver = MockIdentityResolver::new();
        resolver.add_identity("bob.test.com", "did:plc:bob456");

        let limits = FacetLimits::default();
        let text = "Join @alice.bsky.social and @bob.test.com at https://example.com #rust #golang";
        let facets = parse_facets_from_text(text, &resolver, &limits).await;

        assert!(facets.is_some());
        let facets = facets.unwrap();
        assert_eq!(facets.len(), 5); // 2 mentions, 1 URL, 2 hashtags

        // Check first mention
        assert_eq!(facets[0].index.byte_start, 5);
        assert_eq!(facets[0].index.byte_end, 23);
        if let FacetFeature::Mention(ref mention) = facets[0].features[0] {
            assert_eq!(mention.did, "did:plc:alice123");
        } else {
            panic!("Expected Mention feature");
        }

        // Check second mention
        assert_eq!(facets[1].index.byte_start, 28);
        assert_eq!(facets[1].index.byte_end, 41);
        if let FacetFeature::Mention(mention) = &facets[1].features[0] {
            assert_eq!(mention.did, "did:plc:bob456");
        } else {
            panic!("Expected Mention feature");
        }

        // Check URL
        assert_eq!(facets[2].index.byte_start, 45);
        assert_eq!(facets[2].index.byte_end, 64);
        if let FacetFeature::Link(link) = &facets[2].features[0] {
            assert_eq!(link.uri, "https://example.com");
        } else {
            panic!("Expected Link feature");
        }

        // Check first hashtag
        assert_eq!(facets[3].index.byte_start, 65);
        assert_eq!(facets[3].index.byte_end, 70);
        if let FacetFeature::Tag(tag) = &facets[3].features[0] {
            assert_eq!(tag.tag, "rust");
        } else {
            panic!("Expected Tag feature");
        }

        // Check second hashtag
        assert_eq!(facets[4].index.byte_start, 71);
        assert_eq!(facets[4].index.byte_end, 78);
        if let FacetFeature::Tag(tag) = &facets[4].features[0] {
            assert_eq!(tag.tag, "golang");
        } else {
            panic!("Expected Tag feature");
        }
    }

    #[tokio::test]
    async fn test_parse_facets_from_text_with_unresolvable_mention() {
        let resolver = MockIdentityResolver::new();
        let limits = FacetLimits::default();

        // Only alice.bsky.social is in the resolver, not unknown.handle.com
        let text = "Contact @unknown.handle.com for details #rust";
        let facets = parse_facets_from_text(text, &resolver, &limits).await;

        assert!(facets.is_some());
        let facets = facets.unwrap();
        // Should only have 1 facet (the hashtag) since the mention couldn't be resolved
        assert_eq!(facets.len(), 1);

        // Check that it's the hashtag facet
        if let FacetFeature::Tag(tag) = &facets[0].features[0] {
            assert_eq!(tag.tag, "rust");
        } else {
            panic!("Expected Tag feature");
        }
    }

    #[tokio::test]
    async fn test_parse_facets_from_text_empty() {
        let resolver = MockIdentityResolver::new();
        let limits = FacetLimits::default();
        let text = "No mentions, URLs, or hashtags here";
        let facets = parse_facets_from_text(text, &resolver, &limits).await;
        assert!(facets.is_none());
    }

    #[tokio::test]
    async fn test_parse_facets_from_text_url_with_at_mention() {
        let resolver = MockIdentityResolver::new();
        let limits = FacetLimits::default();

        // URLs with @ should not create mention facets
        let text = "Tangled https://tangled.org/@smokesignal.events";
        let facets = parse_facets_from_text(text, &resolver, &limits).await;

        assert!(facets.is_some());
        let facets = facets.unwrap();

        // Should have exactly 1 facet (the URL), not 2 (URL + mention)
        assert_eq!(
            facets.len(),
            1,
            "Expected 1 facet (URL only), got {}",
            facets.len()
        );

        // Verify it's a link facet, not a mention
        if let FacetFeature::Link(link) = &facets[0].features[0] {
            assert_eq!(link.uri, "https://tangled.org/@smokesignal.events");
        } else {
            panic!("Expected Link feature, got Mention or Tag instead");
        }
    }

    #[tokio::test]
    async fn test_parse_facets_with_mention_limit() {
        let mut resolver = MockIdentityResolver::new();
        resolver.add_identity("bob.test.com", "did:plc:bob456");
        resolver.add_identity("charlie.test.com", "did:plc:charlie789");

        // Limit to 2 mentions
        let limits = FacetLimits {
            mentions_max: 2,
            tags_max: 5,
            links_max: 5,
            max: 10,
        };

        let text = "Join @alice.bsky.social @bob.test.com @charlie.test.com";
        let facets = parse_facets_from_text(text, &resolver, &limits).await;

        assert!(facets.is_some());
        let facets = facets.unwrap();
        // Should only have 2 mentions (alice and bob), charlie should be skipped
        assert_eq!(facets.len(), 2);

        // Verify they're both mentions
        for facet in &facets {
            assert!(matches!(facet.features[0], FacetFeature::Mention(_)));
        }
    }

    #[tokio::test]
    async fn test_parse_facets_with_global_limit() {
        let mut resolver = MockIdentityResolver::new();
        resolver.add_identity("bob.test.com", "did:plc:bob456");

        // Very restrictive global limit
        let limits = FacetLimits {
            mentions_max: 5,
            tags_max: 5,
            links_max: 5,
            max: 3, // Only allow 3 total facets
        };

        let text =
            "Join @alice.bsky.social @bob.test.com at https://example.com #rust #golang #python";
        let facets = parse_facets_from_text(text, &resolver, &limits).await;

        assert!(facets.is_some());
        let facets = facets.unwrap();
        // Should be truncated to 3 facets total
        assert_eq!(facets.len(), 3);
    }

    #[test]
    fn test_parse_urls_multiple_links() {
        let text = "IETF124 is happening in Montreal, Nov 1st to 7th https://www.ietf.org/meeting/124/\n\nWe're confirmed for two days of ATProto community sessions on Monday, Nov 3rd & Tuesday, Mov 4th at ECTO Co-Op. Many of us will also be participating in the free-to-attend IETF hackathon on Sunday, Nov 2nd.\n\nLatest updates and attendees in the forum https://discourse.atprotocol.community/t/update-on-timing-and-plan-for-montreal/164";

        let facets = parse_urls(text);

        // Should find both URLs
        assert_eq!(
            facets.len(),
            2,
            "Expected 2 URLs but found {}",
            facets.len()
        );

        // Check first URL
        if let Some(FacetFeature::Link(link)) = facets[0].features.first() {
            assert_eq!(link.uri, "https://www.ietf.org/meeting/124/");
        } else {
            panic!("Expected Link feature");
        }

        // Check second URL
        if let Some(FacetFeature::Link(link)) = facets[1].features.first() {
            assert_eq!(
                link.uri,
                "https://discourse.atprotocol.community/t/update-on-timing-and-plan-for-montreal/164"
            );
        } else {
            panic!("Expected Link feature");
        }
    }

    #[test]
    fn test_parse_urls_with_html_entity() {
        // Test with the HTML entity &amp; in the text
        let text = "IETF124 is happening in Montreal, Nov 1st to 7th https://www.ietf.org/meeting/124/\n\nWe're confirmed for two days of ATProto community sessions on Monday, Nov 3rd &amp; Tuesday, Mov 4th at ECTO Co-Op. Many of us will also be participating in the free-to-attend IETF hackathon on Sunday, Nov 2nd.\n\nLatest updates and attendees in the forum https://discourse.atprotocol.community/t/update-on-timing-and-plan-for-montreal/164";

        let facets = parse_urls(text);

        // Should find both URLs
        assert_eq!(
            facets.len(),
            2,
            "Expected 2 URLs but found {}",
            facets.len()
        );

        // Check first URL
        if let Some(FacetFeature::Link(link)) = facets[0].features.first() {
            assert_eq!(link.uri, "https://www.ietf.org/meeting/124/");
        } else {
            panic!("Expected Link feature");
        }

        // Check second URL
        if let Some(FacetFeature::Link(link)) = facets[1].features.first() {
            assert_eq!(
                link.uri,
                "https://discourse.atprotocol.community/t/update-on-timing-and-plan-for-montreal/164"
            );
        } else {
            panic!("Expected Link feature");
        }
    }

    #[test]
    fn test_byte_offset_with_html_entities() {
        // This test demonstrates that HTML entity escaping shifts byte positions.
        // The byte positions shift:
        // In original: '&' is at byte 8 (1 byte)
        // In escaped: '&amp;' starts at byte 8 (5 bytes)
        // This causes facet byte offsets to be misaligned if text is escaped before rendering.

        // If we have a URL after the ampersand in the original:
        let original_with_url = "Nov 3rd & Tuesday https://example.com";
        let escaped_with_url = "Nov 3rd &amp; Tuesday https://example.com";

        // Parse URLs from both versions
        let original_facets = parse_urls(original_with_url);
        let escaped_facets = parse_urls(escaped_with_url);

        // Both should find the URL, but at different byte positions
        assert_eq!(original_facets.len(), 1);
        assert_eq!(escaped_facets.len(), 1);

        // The byte positions will be different
        assert_eq!(original_facets[0].index.byte_start, 18); // After "Nov 3rd & Tuesday "
        assert_eq!(escaped_facets[0].index.byte_start, 22); // After "Nov 3rd &amp; Tuesday " (4 extra bytes for &amp;)
    }

    #[test]
    fn test_parse_urls_from_atproto_record_text() {
        // Test parsing URLs from real AT Protocol record description text.
        // This demonstrates the correct byte positions that should be used for facets.
        let text = "Dev, Power Users, and Generally inquisitive folks get a completely unprofessionally amateur interview. Just a yap sesh where chat is part of the call!\n\n✨the daniel✨ & I will be on a Zoom call and I will stream out to https://stream.place/psingletary.com\n\nSubscribe to the publications! https://atprotocalls.leaflet.pub/";

        let facets = parse_urls(text);

        assert_eq!(facets.len(), 2, "Should find 2 URLs");

        // First URL: https://stream.place/psingletary.com
        assert_eq!(facets[0].index.byte_start, 221);
        assert_eq!(facets[0].index.byte_end, 257);
        if let Some(FacetFeature::Link(link)) = facets[0].features.first() {
            assert_eq!(link.uri, "https://stream.place/psingletary.com");
        }

        // Second URL: https://atprotocalls.leaflet.pub/
        assert_eq!(facets[1].index.byte_start, 290);
        assert_eq!(facets[1].index.byte_end, 323);
        if let Some(FacetFeature::Link(link)) = facets[1].features.first() {
            assert_eq!(link.uri, "https://atprotocalls.leaflet.pub/");
        }

        // Verify the byte slices match the expected text
        let text_bytes = text.as_bytes();
        assert_eq!(
            std::str::from_utf8(&text_bytes[221..257]).unwrap(),
            "https://stream.place/psingletary.com"
        );
        assert_eq!(
            std::str::from_utf8(&text_bytes[290..323]).unwrap(),
            "https://atprotocalls.leaflet.pub/"
        );
    }

    #[tokio::test]
    async fn test_parse_mentions_basic() {
        let resolver = MockIdentityResolver::new();
        let limits = FacetLimits::default();
        let text = "Hello @alice.bsky.social!";
        let facets = parse_mentions(text, &resolver, &limits).await;

        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].index.byte_start, 6);
        assert_eq!(facets[0].index.byte_end, 24);
        if let Some(FacetFeature::Mention(mention)) = facets[0].features.first() {
            assert_eq!(mention.did, "did:plc:alice123");
        } else {
            panic!("Expected Mention feature");
        }
    }

    #[tokio::test]
    async fn test_parse_mentions_multiple() {
        let mut resolver = MockIdentityResolver::new();
        resolver.add_identity("bob.example.com", "did:plc:bob456");
        let limits = FacetLimits::default();
        let text = "CC @alice.bsky.social and @bob.example.com";
        let facets = parse_mentions(text, &resolver, &limits).await;

        assert_eq!(facets.len(), 2);
        if let Some(FacetFeature::Mention(mention)) = facets[0].features.first() {
            assert_eq!(mention.did, "did:plc:alice123");
        }
        if let Some(FacetFeature::Mention(mention)) = facets[1].features.first() {
            assert_eq!(mention.did, "did:plc:bob456");
        }
    }

    #[tokio::test]
    async fn test_parse_mentions_unresolvable() {
        let resolver = MockIdentityResolver::new();
        let limits = FacetLimits::default();
        // unknown.handle.com is not in the resolver
        let text = "Hello @unknown.handle.com!";
        let facets = parse_mentions(text, &resolver, &limits).await;

        // Should be empty since the handle can't be resolved
        assert_eq!(facets.len(), 0);
    }

    #[tokio::test]
    async fn test_parse_mentions_in_url_excluded() {
        let resolver = MockIdentityResolver::new();
        let limits = FacetLimits::default();
        // The @smokesignal.events is inside a URL and should not be parsed as a mention
        let text = "Check https://tangled.org/@smokesignal.events";
        let facets = parse_mentions(text, &resolver, &limits).await;

        // Should be empty since the mention is inside a URL
        assert_eq!(facets.len(), 0);
    }

    #[test]
    fn test_parse_tags_basic() {
        let text = "Learning #rust today!";
        let facets = parse_tags(text);

        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].index.byte_start, 9);
        assert_eq!(facets[0].index.byte_end, 14);
        if let Some(FacetFeature::Tag(tag)) = facets[0].features.first() {
            assert_eq!(tag.tag, "rust");
        } else {
            panic!("Expected Tag feature");
        }
    }

    #[test]
    fn test_parse_tags_multiple() {
        let text = "#rust #golang #python are great!";
        let facets = parse_tags(text);

        assert_eq!(facets.len(), 3);
        if let Some(FacetFeature::Tag(tag)) = facets[0].features.first() {
            assert_eq!(tag.tag, "rust");
        }
        if let Some(FacetFeature::Tag(tag)) = facets[1].features.first() {
            assert_eq!(tag.tag, "golang");
        }
        if let Some(FacetFeature::Tag(tag)) = facets[2].features.first() {
            assert_eq!(tag.tag, "python");
        }
    }

    #[test]
    fn test_parse_tags_excludes_numeric() {
        let text = "Item #42 is special #test123";
        let facets = parse_tags(text);

        // #42 should be excluded (purely numeric), #test123 should be included
        assert_eq!(facets.len(), 1);
        if let Some(FacetFeature::Tag(tag)) = facets[0].features.first() {
            assert_eq!(tag.tag, "test123");
        }
    }

    #[test]
    fn test_parse_urls_basic() {
        let text = "Visit https://example.com today!";
        let facets = parse_urls(text);

        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].index.byte_start, 6);
        assert_eq!(facets[0].index.byte_end, 25);
        if let Some(FacetFeature::Link(link)) = facets[0].features.first() {
            assert_eq!(link.uri, "https://example.com");
        }
    }

    #[test]
    fn test_parse_urls_with_path() {
        let text = "Check https://example.com/path/to/page?query=1#section";
        let facets = parse_urls(text);

        assert_eq!(facets.len(), 1);
        if let Some(FacetFeature::Link(link)) = facets[0].features.first() {
            assert_eq!(link.uri, "https://example.com/path/to/page?query=1#section");
        }
    }

    #[test]
    fn test_facet_limits_default() {
        let limits = FacetLimits::default();
        assert_eq!(limits.mentions_max, 5);
        assert_eq!(limits.tags_max, 5);
        assert_eq!(limits.links_max, 5);
        assert_eq!(limits.max, 10);
    }
}
