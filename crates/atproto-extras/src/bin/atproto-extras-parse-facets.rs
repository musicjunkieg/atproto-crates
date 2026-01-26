//! Command-line tool for generating AT Protocol facet arrays from text.
//!
//! This tool parses a string and outputs the facet array in JSON format.
//! Facets include mentions (@handle), URLs (https://...), and hashtags (#tag).
//!
//! By default, mentions are detected but output with placeholder DIDs. Use
//! `--resolve-mentions` to resolve handles to actual DIDs (requires network access).
//!
//! # Usage
//!
//! ```bash
//! # Parse facets without resolving mentions
//! cargo run --features clap,serde_json,tokio,hickory-dns --bin atproto-extras-parse-facets -- "Check out https://example.com and #rust"
//!
//! # Resolve mentions to DIDs
//! cargo run --features clap,serde_json,tokio,hickory-dns --bin atproto-extras-parse-facets -- --resolve-mentions "Hello @bsky.app!"
//! ```

use atproto_extras::{FacetLimits, parse_mentions, parse_tags, parse_urls};
use atproto_identity::resolve::{HickoryDnsResolver, InnerIdentityResolver};
use atproto_record::lexicon::app::bsky::richtext::facet::{
    ByteSlice, Facet, FacetFeature, Mention,
};
use clap::Parser;
use regex::bytes::Regex;
use std::sync::Arc;

/// Parse text and output AT Protocol facets as JSON.
#[derive(Parser)]
#[command(
    name = "atproto-extras-parse-facets",
    version,
    about = "Parse text and output AT Protocol facets as JSON",
    long_about = "This tool parses a string for mentions, URLs, and hashtags,\n\
                  then outputs the corresponding AT Protocol facet array in JSON format.\n\n\
                  By default, mentions are detected but output with placeholder DIDs.\n\
                  Use --resolve-mentions to resolve handles to actual DIDs (requires network)."
)]
struct Args {
    /// The text to parse for facets
    text: String,

    /// Resolve mention handles to DIDs (requires network access)
    #[arg(long)]
    resolve_mentions: bool,

    /// Show debug information on stderr
    #[arg(long, short = 'd')]
    debug: bool,
}

/// Parse mention spans from text without resolution (returns placeholder DIDs).
fn parse_mention_spans(text: &str) -> Vec<Facet> {
    let mut facets = Vec::new();

    // Get URL ranges to exclude mentions within URLs
    let url_facets = parse_urls(text);

    // Same regex pattern as parse_mentions
    let mention_regex = Regex::new(
        r"(?:^|[^\w])(@([a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)",
    )
    .expect("Invalid mention regex");

    let text_bytes = text.as_bytes();

    for capture in mention_regex.captures_iter(text_bytes) {
        if let Some(mention_match) = capture.get(1) {
            let start = mention_match.start();
            let end = mention_match.end();

            // Check if this mention overlaps with any URL
            let overlaps_url = url_facets.iter().any(|facet| {
                (start >= facet.index.byte_start && start < facet.index.byte_end)
                    || (end > facet.index.byte_start && end <= facet.index.byte_end)
            });

            if !overlaps_url {
                let handle = std::str::from_utf8(&mention_match.as_bytes()[1..])
                    .unwrap_or_default()
                    .to_string();

                facets.push(Facet {
                    index: ByteSlice {
                        byte_start: start,
                        byte_end: end,
                    },
                    features: vec![FacetFeature::Mention(Mention {
                        did: format!("did:plc:<unresolved:{}>", handle),
                    })],
                });
            }
        }
    }

    facets
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let text = &args.text;
    let mut facets: Vec<Facet> = Vec::new();
    let limits = FacetLimits::default();

    // Parse mentions (either resolved or with placeholders)
    if args.resolve_mentions {
        let http_client = reqwest::Client::new();
        let dns_resolver = HickoryDnsResolver::create_resolver(&[]);
        let resolver = InnerIdentityResolver {
            http_client,
            dns_resolver: Arc::new(dns_resolver),
            plc_hostname: "plc.directory".to_string(),
        };
        let mention_facets = parse_mentions(text, &resolver, &limits).await;
        facets.extend(mention_facets);
    } else {
        let mention_facets = parse_mention_spans(text);
        facets.extend(mention_facets);
    }

    // Parse URLs
    let url_facets = parse_urls(text);
    facets.extend(url_facets);

    // Parse hashtags
    let tag_facets = parse_tags(text);
    facets.extend(tag_facets);

    // Sort facets by byte_start for consistent output
    facets.sort_by_key(|f| f.index.byte_start);

    // Output as JSON
    if facets.is_empty() {
        println!("null");
    } else {
        match serde_json::to_string_pretty(&facets) {
            Ok(json) => println!("{}", json),
            Err(e) => {
                eprintln!(
                    "error-atproto-extras-parse-facets-1 Error serializing facets: {}",
                    e
                );
                std::process::exit(1);
            }
        }
    }

    // Show debug info if requested
    if args.debug {
        eprintln!();
        eprintln!("--- Debug Info ---");
        eprintln!("Input text: {:?}", text);
        eprintln!("Text length: {} bytes", text.len());
        eprintln!("Facets found: {}", facets.len());
        eprintln!("Mentions resolved: {}", args.resolve_mentions);

        // Show byte slice verification
        let text_bytes = text.as_bytes();
        for (i, facet) in facets.iter().enumerate() {
            let start = facet.index.byte_start;
            let end = facet.index.byte_end;
            let slice_text =
                std::str::from_utf8(&text_bytes[start..end]).unwrap_or("<invalid utf8>");
            let feature_type = match &facet.features[0] {
                FacetFeature::Mention(_) => "mention",
                FacetFeature::Link(_) => "link",
                FacetFeature::Tag(_) => "tag",
            };
            eprintln!(
                "  [{}] {} @ bytes {}..{}: {:?}",
                i, feature_type, start, end, slice_text
            );
        }
    }
}
