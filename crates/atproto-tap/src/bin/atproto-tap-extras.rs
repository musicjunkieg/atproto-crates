//! Additional TAP client utilities for AT Protocol.
//!
//! This tool provides extra commands for managing TAP tracked repositories
//! based on social graph data.
//!
//! # Usage
//!
//! ```bash
//! # Add all accounts followed by a DID to TAP tracking
//! cargo run --features cli --bin atproto-tap-extras -- localhost:2480 repos-add-followers did:plc:xyz
//!
//! # With authentication
//! cargo run --features cli --bin atproto-tap-extras -- localhost:2480 -p secret repos-add-followers did:plc:xyz
//! ```

use atproto_client::client::Auth;
use atproto_client::com::atproto::repo::{ListRecordsParams, list_records};
use atproto_identity::plc::query as plc_query;
use atproto_tap::TapClient;
use clap::{Parser, Subcommand};
use serde::Deserialize;

/// TAP extras utility for managing tracked repositories.
#[derive(Parser)]
#[command(
    name = "atproto-tap-extras",
    version,
    about = "TAP extras utility for AT Protocol",
    long_about = "Additional utilities for managing TAP tracked repositories based on social graph data."
)]
struct Args {
    /// TAP service hostname (e.g., localhost:2480)
    hostname: String,

    /// Admin password for TAP authentication
    #[arg(short, long, global = true)]
    password: Option<String>,

    /// PLC directory hostname for DID resolution
    #[arg(long, default_value = "plc.directory", global = true)]
    plc_hostname: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add accounts followed by a DID to TAP tracking.
    ///
    /// Fetches all app.bsky.graph.follow records from the specified DID's repository
    /// and adds the followed DIDs to TAP for tracking.
    ReposAddFollowers {
        /// DID to read followers from (e.g., did:plc:xyz123)
        did: String,

        /// Batch size for adding repos to TAP
        #[arg(long, default_value = "100")]
        batch_size: usize,

        /// Dry run - print DIDs without adding to TAP
        #[arg(long)]
        dry_run: bool,

        /// Maximum number of followed DIDs to add to TAP
        #[arg(long)]
        limit: Option<usize>,
    },
}

/// Follow record structure from app.bsky.graph.follow.
#[derive(Debug, Deserialize)]
struct FollowRecord {
    /// The DID of the account being followed.
    subject: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Command::ReposAddFollowers {
            did,
            batch_size,
            dry_run,
            limit,
        } => {
            run_repos_add_followers(
                &args.hostname,
                args.password,
                &args.plc_hostname,
                &did,
                batch_size,
                dry_run,
                limit,
            )
            .await;
        }
    }
}

async fn run_repos_add_followers(
    tap_hostname: &str,
    tap_password: Option<String>,
    plc_hostname: &str,
    did: &str,
    batch_size: usize,
    dry_run: bool,
    limit: Option<usize>,
) {
    let http_client = reqwest::Client::new();

    // Resolve the DID to get the PDS endpoint
    eprintln!("Resolving DID: {}", did);
    let document = match plc_query(&http_client, plc_hostname, did).await {
        Ok(doc) => doc,
        Err(e) => {
            eprintln!("Failed to resolve DID: {}", e);
            std::process::exit(1);
        }
    };

    let pds_endpoints = document.pds_endpoints();
    if pds_endpoints.is_empty() {
        eprintln!("No PDS endpoint found in DID document");
        std::process::exit(1);
    }
    let pds_url = pds_endpoints[0];
    eprintln!("Using PDS: {}", pds_url);

    // Collect all followed DIDs
    let mut followed_dids: Vec<String> = Vec::new();
    let mut cursor: Option<String> = None;
    let collection = "app.bsky.graph.follow".to_string();

    eprintln!("Fetching follow records...");

    loop {
        let params = if let Some(c) = cursor.take() {
            ListRecordsParams::new().limit(100).cursor(c)
        } else {
            ListRecordsParams::new().limit(100)
        };

        let response = match list_records::<FollowRecord>(
            &http_client,
            &Auth::None,
            pds_url,
            did.to_string(),
            collection.clone(),
            params,
        )
        .await
        {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("Failed to list records: {}", e);
                std::process::exit(1);
            }
        };

        for record in &response.records {
            followed_dids.push(record.value.subject.clone());
        }

        eprintln!(
            "  Fetched {} records (total: {})",
            response.records.len(),
            followed_dids.len()
        );

        match response.cursor {
            Some(c) if !response.records.is_empty() => {
                cursor = Some(c);
            }
            _ => break,
        }
    }

    if followed_dids.is_empty() {
        eprintln!("No follow records found");
        return;
    }

    if let Some(limit) = limit {
        followed_dids.truncate(limit);
    }

    eprintln!("Found {} followed accounts", followed_dids.len());

    if dry_run {
        eprintln!("\nDry run - would add these DIDs to TAP:");
        for did in &followed_dids {
            println!("{}", did);
        }
        return;
    }

    // Add to TAP in batches
    let tap_client = TapClient::new(tap_hostname, tap_password);
    let mut added = 0;

    for chunk in followed_dids.chunks(batch_size) {
        let did_refs: Vec<&str> = chunk.iter().map(|s| s.as_str()).collect();

        match tap_client.add_repos(&did_refs).await {
            Ok(()) => {
                added += chunk.len();
                eprintln!("Added {} DIDs to TAP (total: {})", chunk.len(), added);
            }
            Err(e) => {
                eprintln!("Failed to add repos to TAP: {}", e);
                std::process::exit(1);
            }
        }
    }

    eprintln!("Successfully added {} DIDs to TAP", added);

    // Print all added DIDs
    for did in &followed_dids {
        println!("{}", did);
    }
}
