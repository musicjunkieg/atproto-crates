# atproto-extras

Extra utilities for AT Protocol applications, including rich text facet parsing.

## Features

- **Facet Parsing**: Extract mentions (`@handle`), URLs, and hashtags (`#tag`) from plain text with correct UTF-8 byte offset calculation
- **Identity Integration**: Resolve mention handles to DIDs during parsing

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
atproto-extras = "0.13"
```

## Usage

### Parsing Text for Facets

```rust
use atproto_extras::{parse_urls, parse_tags};
use atproto_record::lexicon::app::bsky::richtext::facet::FacetFeature;

let text = "Check out https://example.com #rust";

// Parse URLs and tags - returns Vec<Facet> directly
let url_facets = parse_urls(text);
let tag_facets = parse_tags(text);

// Each facet includes byte positions and typed features
for facet in url_facets {
    if let Some(FacetFeature::Link(link)) = facet.features.first() {
        println!("URL at bytes {}..{}: {}",
            facet.index.byte_start, facet.index.byte_end, link.uri);
    }
}

for facet in tag_facets {
    if let Some(FacetFeature::Tag(tag)) = facet.features.first() {
        println!("Tag at bytes {}..{}: #{}",
            facet.index.byte_start, facet.index.byte_end, tag.tag);
    }
}
```

### Parsing Mentions

Mention parsing requires an `IdentityResolver` to convert handles to DIDs:

```rust
use atproto_extras::{parse_mentions, FacetLimits};
use atproto_record::lexicon::app::bsky::richtext::facet::FacetFeature;

let text = "Hello @alice.bsky.social!";
let limits = FacetLimits::default();

// Requires an async context and IdentityResolver
let facets = parse_mentions(text, &resolver, &limits).await;

for facet in facets {
    if let Some(FacetFeature::Mention(mention)) = facet.features.first() {
        println!("Mention at bytes {}..{} resolved to {}",
            facet.index.byte_start, facet.index.byte_end, mention.did);
    }
}
```

Mentions that cannot be resolved to a valid DID are automatically skipped. Mentions appearing within URLs are also excluded.

### Creating AT Protocol Facets

```rust
use atproto_extras::{parse_facets_from_text, FacetLimits};

let text = "Hello @alice.bsky.social! Check https://rust-lang.org #rust";
let limits = FacetLimits::default();

// Requires an async context and IdentityResolver
let facets = parse_facets_from_text(text, &resolver, &limits).await;

if let Some(facets) = facets {
    for facet in &facets {
        println!("Facet at {}..{}", facet.index.byte_start, facet.index.byte_end);
    }
}
```

## Byte Offset Handling

AT Protocol facets use UTF-8 byte offsets, not character indices. This is critical for correct handling of multi-byte characters like emojis or non-ASCII text.

```rust
use atproto_extras::parse_urls;

// Text with emojis (multi-byte UTF-8 characters)
let text = "✨ Check https://example.com ✨";

let facets = parse_urls(text);
// Byte positions correctly account for the 4-byte emoji
assert_eq!(facets[0].index.byte_start, 11);  // After "✨ Check " (4 + 1 + 6 = 11 bytes)
```

## Facet Limits

Use `FacetLimits` to control the maximum number of facets processed:

```rust
use atproto_extras::FacetLimits;

// Default limits
let limits = FacetLimits::default();
// mentions_max: 5, tags_max: 5, links_max: 5, max: 10

// Custom limits
let custom = FacetLimits {
    mentions_max: 10,
    tags_max: 10,
    links_max: 10,
    max: 20,
};
```

## License

MIT
