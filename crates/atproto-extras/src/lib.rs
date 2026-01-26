//! Extra utilities for AT Protocol applications.
//!
//! This crate provides additional utilities that complement the core AT Protocol
//! identity and record crates. Currently, it focuses on rich text facet parsing.
//!
//! ## Features
//!
//! - **Facet Parsing**: Extract mentions, URLs, and hashtags from plain text
//!   with correct UTF-8 byte offset calculation
//! - **Identity Integration**: Resolve mention handles to DIDs during parsing
//!
//! ## Example
//!
//! ```ignore
//! use atproto_extras::{parse_facets_from_text, FacetLimits};
//!
//! // Parse facets from text (requires an IdentityResolver)
//! let text = "Hello @alice.bsky.social! Check out https://example.com #rust";
//! let limits = FacetLimits::default();
//! let facets = parse_facets_from_text(text, &resolver, &limits).await;
//! ```
//!
//! ## Byte Offset Calculation
//!
//! This implementation correctly uses UTF-8 byte offsets as required by AT Protocol.
//! The facets use "inclusive start and exclusive end" byte ranges. All parsing is done
//! using `regex::bytes::Regex` which operates on byte slices and returns byte positions,
//! ensuring correct handling of multi-byte UTF-8 characters (emojis, CJK, accented chars).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Rich text facet parsing for AT Protocol.
///
/// This module provides functionality for extracting semantic annotations (facets)
/// from plain text. Facets include:
///
/// - **Mentions**: User handles prefixed with `@` (e.g., `@alice.bsky.social`)
/// - **Links**: HTTP/HTTPS URLs
/// - **Tags**: Hashtags prefixed with `#` or `＃` (e.g., `#rust`)
///
/// ## Byte Offsets
///
/// All facet indices use UTF-8 byte offsets, not character indices. This is
/// critical for correct handling of multi-byte characters like emojis or
/// non-ASCII text.
pub mod facets;

/// Re-export commonly used types for convenience.
pub use facets::{FacetLimits, parse_facets_from_text, parse_mentions, parse_tags, parse_urls};
