//! AT Protocol client tool for writing records to a repository.
//!
//! This binary tool creates or updates records in an AT Protocol repository
//! using app password authentication. It resolves the subject to a DID,
//! creates a session, and writes the record using the putRecord XRPC method.
//!
//! # Usage
//!
//! ```text
//! ATPROTO_PASSWORD=<password> atproto-client-put-record <subject> <record_key> <record_json>
//! ```
//!
//! # Environment Variables
//!
//! - `ATPROTO_PASSWORD` - Required. App password for authentication.
//! - `CERTIFICATE_BUNDLES` - Custom CA certificate bundles.
//! - `USER_AGENT` - Custom user agent string.
//! - `DNS_NAMESERVERS` - Custom DNS nameservers.
//! - `PLC_HOSTNAME` - Override PLC hostname (default: plc.directory).

use anyhow::Result;
use atproto_client::{
    client::{AppPasswordAuth, Auth},
    com::atproto::{
        repo::{PutRecordRequest, PutRecordResponse, put_record},
        server::create_session,
    },
    errors::CliError,
};
use atproto_identity::{
    config::{CertificateBundles, DnsNameservers, default_env, optional_env, version},
    plc,
    resolve::{HickoryDnsResolver, resolve_subject},
    web,
};
use std::env;

fn print_usage() {
    eprintln!("Usage: atproto-client-put-record <subject> <record_key> <record_json>");
    eprintln!();
    eprintln!("Arguments:");
    eprintln!("  <subject>      Handle or DID of the repository owner");
    eprintln!("  <record_key>   Record key (rkey) for the record");
    eprintln!("  <record_json>  JSON record data (must include $type field)");
    eprintln!();
    eprintln!("Environment Variables:");
    eprintln!("  ATPROTO_PASSWORD    Required. App password for authentication.");
    eprintln!("  CERTIFICATE_BUNDLES Custom CA certificate bundles.");
    eprintln!("  USER_AGENT          Custom user agent string.");
    eprintln!("  DNS_NAMESERVERS     Custom DNS nameservers.");
    eprintln!("  PLC_HOSTNAME        Override PLC hostname (default: plc.directory).");
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 {
        print_usage();
        std::process::exit(1);
    }

    let subject = &args[1];
    let record_key = &args[2];
    let record_json = &args[3];

    // Get password from environment variable
    let password = env::var("ATPROTO_PASSWORD")
        .map_err(|_| anyhow::anyhow!("ATPROTO_PASSWORD environment variable is required"))?;

    // Set up HTTP client configuration
    let certificate_bundles: CertificateBundles = optional_env("CERTIFICATE_BUNDLES").try_into()?;
    let default_user_agent = format!(
        "atproto-identity-rs ({}; +https://tangled.org/ngerakines.me/atproto-crates)",
        version()?
    );
    let user_agent = default_env("USER_AGENT", &default_user_agent);
    let dns_nameservers: DnsNameservers = optional_env("DNS_NAMESERVERS").try_into()?;
    let plc_hostname = default_env("PLC_HOSTNAME", "plc.directory");

    let mut client_builder = reqwest::Client::builder();
    for ca_certificate in certificate_bundles.as_ref() {
        let cert = std::fs::read(ca_certificate)?;
        let cert = reqwest::Certificate::from_pem(&cert)?;
        client_builder = client_builder.add_root_certificate(cert);
    }

    client_builder = client_builder.user_agent(user_agent);
    let http_client = client_builder.build()?;

    let dns_resolver = HickoryDnsResolver::create_resolver(dns_nameservers.as_ref());

    // Parse the record JSON
    let record: serde_json::Value = serde_json::from_str(record_json).map_err(|err| {
        tracing::error!(error = ?err, "Failed to parse record JSON");
        anyhow::anyhow!("Failed to parse record JSON: {}", err)
    })?;

    // Extract collection from $type field
    let collection = record
        .get("$type")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Record must contain a $type field for the collection"))?
        .to_string();

    // Resolve subject to DID
    let did = resolve_subject(&http_client, &dns_resolver, subject).await?;

    // Get DID document to find PDS endpoint
    let document = if did.starts_with("did:plc:") {
        plc::query(&http_client, &plc_hostname, &did).await?
    } else if did.starts_with("did:web:") {
        web::query(&http_client, &did).await?
    } else {
        anyhow::bail!("Unsupported DID method: {}", did);
    };

    // Get PDS endpoint from the DID document
    let pds_endpoints = document.pds_endpoints();
    let pds_endpoint = pds_endpoints
        .first()
        .ok_or_else(|| CliError::NoPdsEndpointFound { did: did.clone() })?;

    // Create session
    let session = create_session(&http_client, pds_endpoint, &did, &password, None).await?;

    // Set up app password authentication
    let auth = Auth::AppPassword(AppPasswordAuth {
        access_token: session.access_jwt.clone(),
    });

    // Create put record request
    let put_request = PutRecordRequest {
        repo: session.did.clone(),
        collection,
        record_key: record_key.clone(),
        validate: true,
        record,
        swap_commit: None,
        swap_record: None,
    };

    // Execute put record
    let response = put_record(&http_client, &auth, pds_endpoint, put_request).await?;

    match response {
        PutRecordResponse::StrongRef { uri, cid, .. } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "uri": uri,
                    "cid": cid
                }))?
            );
        }
        PutRecordResponse::Error(err) => {
            let error_message = err.error_message();
            tracing::error!(error = %error_message, "putRecord failed");
            anyhow::bail!("putRecord failed: {}", error_message);
        }
    }

    Ok(())
}
