//! CLI tool for resolving AT Protocol lexicons.
//!
//! This tool resolves lexicon NSIDs to their schema definitions using the
//! AT Protocol lexicon resolution process, with support for recursive resolution.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use atproto_identity::{
    config::{CertificateBundles, DnsNameservers, default_env, optional_env, version},
    resolve::HickoryDnsResolver,
};
use atproto_lexicon::{
    resolve::{DefaultLexiconResolver, LexiconResolver},
    resolve_recursive::{RecursiveLexiconResolver, RecursiveResolverConfig},
};
use clap::Parser;
use serde_json::Value;

/// AT Protocol Lexicon Resolution CLI
#[derive(Parser)]
#[command(
    name = "atproto-lexicon-resolve",
    version,
    about = "Resolve AT Protocol lexicon NSIDs to their schema definitions",
    long_about = "
A command-line tool for resolving AT Protocol lexicon NSIDs (Namespace Identifiers) to their
schema definitions. The resolution process follows the AT Protocol specification:

1. Convert NSID to DNS name with '_lexicon' prefix
2. Perform DNS TXT lookup to get the authoritative DID
3. Resolve the DID to get the DID document
4. Extract PDS endpoint from the DID document
5. Make XRPC call to fetch the lexicon schema

Supports recursive resolution to automatically resolve all referenced lexicons.

ENVIRONMENT VARIABLES:
  PLC_HOSTNAME         PLC directory hostname (default: \"plc.directory\")
  USER_AGENT           HTTP user agent string (default: auto-generated)
  CERTIFICATE_BUNDLES  Colon-separated paths to additional CA certificates
  DNS_NAMESERVERS      Comma-separated DNS nameserver addresses

EXAMPLES:
  # Resolve a single lexicon:
  atproto-lexicon-resolve app.bsky.feed.post

  # Resolve multiple lexicons:
  atproto-lexicon-resolve app.bsky.feed.post app.bsky.actor.profile

  # Pretty print the JSON output:
  atproto-lexicon-resolve --pretty app.bsky.feed.post

  # Recursively resolve all referenced lexicons:
  atproto-lexicon-resolve --recursive app.bsky.feed.post

  # Resolve recursively with limited depth:
  atproto-lexicon-resolve --recursive --max-depth 3 app.bsky.feed.post

  # Show dependency graph:
  atproto-lexicon-resolve --recursive --show-deps app.bsky.feed.post

  # List only the NSIDs of referenced lexicons:
  atproto-lexicon-resolve --list-refs app.bsky.feed.post
"
)]
struct Args {
    /// One or more lexicon NSIDs to resolve
    nsids: Vec<String>,

    /// Pretty print the JSON output
    #[arg(long)]
    pretty: bool,

    /// Output only the schema without metadata
    #[arg(long)]
    schema_only: bool,

    /// Recursively resolve all referenced lexicons
    #[arg(long, short = 'r')]
    recursive: bool,

    /// Maximum depth for recursive resolution (default: 10)
    #[arg(long, default_value = "10")]
    max_depth: usize,

    /// Exclude the entry lexicon from recursive results
    #[arg(long)]
    exclude_entry: bool,

    /// Show dependency graph for recursive resolution
    #[arg(long)]
    show_deps: bool,

    /// List only the NSIDs that were resolved (no schemas)
    #[arg(long)]
    list_nsids: bool,

    /// List only the direct references of the lexicon
    #[arg(long)]
    list_refs: bool,

    /// Show failed resolutions when using recursive mode
    #[arg(long)]
    show_failed: bool,

    /// Output format: json (default), yaml, or compact
    #[arg(long, default_value = "json")]
    format: OutputFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Compact,
    Summary,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Configure environment variables
    let _plc_hostname = default_env("PLC_HOSTNAME", "plc.directory");
    let certificate_bundles: CertificateBundles = optional_env("CERTIFICATE_BUNDLES").try_into()?;
    let default_user_agent = format!(
        "atproto-lexicon-rs ({}; +https://tangled.sh/@smokesignal.events/atproto-identity-rs)",
        version()?
    );
    let user_agent = default_env("USER_AGENT", &default_user_agent);
    let dns_nameservers: DnsNameservers = optional_env("DNS_NAMESERVERS").try_into()?;

    // Build HTTP client with certificate bundles
    let mut client_builder = reqwest::Client::builder();
    for ca_certificate in certificate_bundles.as_ref() {
        let cert = std::fs::read(ca_certificate)?;
        let cert = reqwest::Certificate::from_pem(&cert)?;
        client_builder = client_builder.add_root_certificate(cert);
    }

    client_builder = client_builder.user_agent(user_agent);
    let http_client = client_builder.build()?;

    // Create DNS resolver
    let dns_resolver = HickoryDnsResolver::create_resolver(dns_nameservers.as_ref());

    // Create lexicon resolver
    let base_resolver = DefaultLexiconResolver::new(http_client, dns_resolver);

    // Process each NSID
    for nsid in &args.nsids {
        if args.list_refs {
            // Just list direct references
            let recursive_resolver = RecursiveLexiconResolver::new(base_resolver.clone());
            match recursive_resolver.get_direct_references(nsid).await {
                Ok(refs) => {
                    if refs.is_empty() {
                        eprintln!("{}: no references", nsid);
                    } else {
                        println!("{}:", nsid);
                        let mut sorted_refs: Vec<_> = refs.into_iter().collect();
                        sorted_refs.sort();
                        for ref_nsid in sorted_refs {
                            println!("  - {}", ref_nsid);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error getting references for {}: {}", nsid, err);
                }
            }
        } else if args.recursive {
            // Recursive resolution
            let config = RecursiveResolverConfig {
                max_depth: args.max_depth,
                include_entry: !args.exclude_entry,
            };
            let recursive_resolver =
                RecursiveLexiconResolver::with_config(base_resolver.clone(), config);

            if args.show_deps || args.show_failed {
                // Use detailed resolution for dependency graph
                match recursive_resolver.resolve_with_details(nsid).await {
                    Ok(result) => {
                        if args.list_nsids {
                            // Just list the NSIDs
                            let mut nsids: Vec<_> = result.lexicons.keys().cloned().collect();
                            nsids.sort();
                            for nsid in nsids {
                                println!("{}", nsid);
                            }
                        } else if args.show_deps {
                            // Show dependency graph
                            println!("Dependency graph for {}:", nsid);
                            print_dependency_graph(&result.dependencies);

                            if args.show_failed && !result.failed.is_empty() {
                                println!("\nFailed to resolve:");
                                let mut failed: Vec<_> = result.failed.into_iter().collect();
                                failed.sort();
                                for nsid in failed {
                                    println!("  - {}", nsid);
                                }
                            }
                        } else {
                            // Output the resolved lexicons
                            output_lexicons(&result.lexicons, &args)?;
                        }
                    }
                    Err(err) => {
                        eprintln!("Error recursively resolving {}: {}", nsid, err);
                    }
                }
            } else {
                // Simple recursive resolution
                match recursive_resolver.resolve_recursive(nsid).await {
                    Ok(lexicons) => {
                        if args.list_nsids {
                            // Just list the NSIDs
                            let mut nsids: Vec<_> = lexicons.keys().cloned().collect();
                            nsids.sort();
                            for nsid in nsids {
                                println!("{}", nsid);
                            }
                        } else {
                            output_lexicons(&lexicons, &args)?;
                        }
                    }
                    Err(err) => {
                        eprintln!("Error recursively resolving {}: {}", nsid, err);
                    }
                }
            }
        } else {
            // Single lexicon resolution
            match base_resolver.resolve(nsid).await {
                Ok(lexicon) => {
                    let output = if args.schema_only {
                        // Extract just the schema portion if requested
                        lexicon.get("schema").unwrap_or(&lexicon).clone()
                    } else {
                        lexicon
                    };

                    match args.format {
                        OutputFormat::Json => {
                            if args.pretty {
                                println!("{}", serde_json::to_string_pretty(&output)?);
                            } else {
                                println!("{}", serde_json::to_string(&output)?);
                            }
                        }
                        OutputFormat::Compact => {
                            println!("{}", serde_json::to_string(&output)?);
                        }
                        OutputFormat::Summary => {
                            print_lexicon_summary(nsid, &output);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error resolving {}: {}", nsid, err);
                    continue;
                }
            }
        }
    }

    Ok(())
}

/// Output multiple lexicons according to the command-line arguments
fn output_lexicons(lexicons: &HashMap<String, Value>, args: &Args) -> Result<()> {
    match args.format {
        OutputFormat::Json => {
            // Create a single JSON object with all lexicons
            let output = if args.schema_only {
                let mut schemas = serde_json::Map::new();
                for (nsid, lexicon) in lexicons {
                    let schema = lexicon.get("schema").unwrap_or(lexicon).clone();
                    schemas.insert(nsid.clone(), schema);
                }
                Value::Object(schemas)
            } else {
                serde_json::to_value(lexicons)?
            };

            if args.pretty {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{}", serde_json::to_string(&output)?);
            }
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(lexicons)?);
        }
        OutputFormat::Summary => {
            println!("Resolved {} lexicons:", lexicons.len());
            let mut nsids: Vec<_> = lexicons.keys().cloned().collect();
            nsids.sort();
            for nsid in nsids {
                if let Some(lexicon) = lexicons.get(&nsid) {
                    print_lexicon_summary(&nsid, lexicon);
                    println!();
                }
            }
        }
    }
    Ok(())
}

/// Print a summary of a lexicon
fn print_lexicon_summary(nsid: &str, lexicon: &Value) {
    println!("NSID: {}", nsid);

    // Try to extract description
    if let Some(desc) = lexicon
        .get("defs")
        .and_then(|d| d.get("main"))
        .and_then(|m| m.get("description"))
        .and_then(|d| d.as_str())
    {
        println!("  Description: {}", desc);
    }

    // Count definitions
    if let Some(defs) = lexicon.get("defs").and_then(|d| d.as_object()) {
        println!("  Definitions: {}", defs.len());

        // List definition types
        let mut def_types = HashSet::new();
        for (_name, def) in defs {
            if let Some(type_str) = def.get("type").and_then(|t| t.as_str()) {
                def_types.insert(type_str);
            }
        }
        if !def_types.is_empty() {
            let mut types: Vec<_> = def_types.into_iter().collect();
            types.sort();
            println!("  Types: {}", types.join(", "));
        }
    }
}

/// Print the dependency graph
fn print_dependency_graph(deps: &HashMap<String, HashSet<String>>) {
    if deps.is_empty() {
        println!("No dependencies found.");
        return;
    }

    let mut sorted_deps: Vec<_> = deps.iter().collect();
    sorted_deps.sort_by_key(|(nsid, _)| *nsid);

    for (nsid, refs) in sorted_deps {
        println!("{}:", nsid);
        let mut sorted_refs: Vec<_> = refs.iter().cloned().collect();
        sorted_refs.sort();
        for ref_nsid in sorted_refs {
            println!("  → {}", ref_nsid);
        }
    }
}
