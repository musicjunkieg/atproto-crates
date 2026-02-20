//! AT Protocol client DPoP tool for making authenticated XRPC calls.
//!
//! This binary tool makes XRPC calls using DPoP authentication by resolving
//! subjects to DID documents and constructing authenticated requests.

use anyhow::Result;
use atproto_client::client::{DPoPAuth, get_dpop_json_with_headers, post_dpop_json_with_headers};
use atproto_client::errors::CliError;
use atproto_identity::{
    config::{CertificateBundles, DnsNameservers, default_env, optional_env, version},
    key::identify_key,
    plc,
    resolve::{HickoryDnsResolver, resolve_subject},
    web,
};
use clap::Parser;
use reqwest::header::HeaderMap;
use rpassword::read_password;
use secrecy::{ExposeSecret, SecretString};
use std::{
    collections::HashMap,
    env,
    io::{self, Write},
};

/// AT Protocol Client DPoP Tool
#[derive(Parser)]
#[command(
    name = "atproto-client-dpop",
    version,
    about = "Make authenticated XRPC calls using AT Protocol DPoP authentication",
    long_about = "
A command-line tool for making authenticated XRPC calls using DPoP (Demonstration
of Proof-of-Possession) authentication. Resolves subjects to DID documents and
constructs authenticated requests to AT Protocol services.

XRPC PATH FORMATS:
  query:<path>         For GET requests (default if no prefix)
  procedure:<path>     For POST requests (requires JSON file)
  <path>               Defaults to GET request

ADDITIONAL ARGUMENTS:
  key=value            Query parameters (for GET requests)
  header=name=value    Additional HTTP headers
  <file_path>          JSON file path (required for procedure calls)

EXAMPLES:
  # GET request:
  atproto-client-dpop alice.bsky.social dpop.pem token123 \\
    com.atproto.repo.listRecords repo=alice.bsky.social collection=app.bsky.feed.post

  # POST request:
  atproto-client-dpop alice.bsky.social dpop.pem token123 \\
    procedure:com.atproto.repo.createRecord data.json

ENVIRONMENT VARIABLES:
  PLC_HOSTNAME         PLC directory hostname (default: plc.directory)
  USER_AGENT           HTTP user agent string (default: auto-generated)
  CERTIFICATE_BUNDLES  Additional CA certificate bundles
  DNS_NAMESERVERS      Custom DNS nameserver addresses
"
)]
struct Args {
    /// Subject identifier to resolve (e.g., alice.bsky.social)
    subject: String,

    /// OAuth access token from environment variable
    #[arg(long, env = "ATPROTO_ACCESS_TOKEN")]
    access_token: Option<String>,

    /// XRPC path with optional prefix (query:/procedure:)
    xrpc_path: String,

    /// Additional arguments for query parameters, headers, and JSON file paths
    additional_args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let subject = &args.subject;

    // Secure DPoP key handling: from environment variable or secure prompt
    let private_dpop_key = if let Ok(key_content) = env::var("ATPROTO_DPOP_KEY") {
        key_content
    } else {
        print!("Enter DPoP private key content: ");
        io::stdout().flush()?;
        let key_content = read_password()?;
        if key_content.is_empty() {
            anyhow::bail!("DPoP key content cannot be empty");
        }
        key_content
    };

    // Secure access token handling: prefer environment variable, then secure prompt
    let access_token = if let Some(env_token) = args.access_token {
        SecretString::new(env_token.into())
    } else {
        // Use secure prompt with hidden input
        print!("Enter access token: ");
        io::stdout().flush()?;
        let token = read_password()?;
        if token.is_empty() {
            anyhow::bail!("Access token cannot be empty");
        }
        SecretString::new(token.into())
    };
    let xrpc_path_with_prefix = &args.xrpc_path;

    // Parse the xrpc_path prefix (optional, defaults to query:)
    let (is_procedure, xrpc_path) = if let Some(path) = xrpc_path_with_prefix.strip_prefix("query:")
    {
        (false, path)
    } else if let Some(path) = xrpc_path_with_prefix.strip_prefix("procedure:") {
        (true, path)
    } else {
        // Default to query if no prefix is provided
        (false, xrpc_path_with_prefix.as_str())
    };

    // Parse additional arguments based on request type
    let mut query_params = HashMap::new();
    let mut header_params = HashMap::new();
    let mut json_data: Option<serde_json::Value> = None;
    let mut arg_index = 0;

    // For procedure calls, expect the next argument to be a file path
    if is_procedure {
        if arg_index >= args.additional_args.len() {
            anyhow::bail!("procedure: prefix requires a JSON file path as the next argument");
        }
        let file_path = &args.additional_args[arg_index];
        let file_content =
            std::fs::read_to_string(file_path).map_err(|_| CliError::FileReadFailed {
                path: file_path.clone(),
            })?;
        json_data = Some(serde_json::from_str(&file_content).map_err(|_| {
            CliError::JsonParseFromFileFailed {
                path: file_path.clone(),
            }
        })?);
        arg_index += 1;
    }

    // Parse remaining key=value arguments and header=name=value arguments
    for arg in &args.additional_args[arg_index..] {
        if let Some((key, value)) = arg.split_once('=') {
            if key == "header" {
                // Parse header=name=value format
                if let Some((header_name, header_value)) = value.split_once('=') {
                    header_params.insert(header_name.to_string(), header_value.to_string());
                } else {
                    eprintln!("Warning: Ignoring invalid header format: {}", arg);
                    eprintln!("Expected format: header=name=value");
                }
            } else if is_procedure {
                eprintln!(
                    "Warning: Query parameters are not supported for procedure calls. Ignoring: {}",
                    arg
                );
            } else {
                query_params.insert(key.to_string(), value.to_string());
            }
        } else {
            eprintln!("Warning: Ignoring invalid argument format: {}", arg);
            eprintln!("Expected format: key=value or header=name=value");
        }
    }

    println!("Making DPoP authenticated XRPC call");
    println!("Subject: {}", subject);
    println!(
        "Request Type: {}",
        if is_procedure {
            "POST (procedure)"
        } else {
            "GET (query)"
        }
    );
    println!("XRPC Path: {}", xrpc_path);
    if !query_params.is_empty() {
        println!("Query Parameters: {:?}", query_params);
    }
    if !header_params.is_empty() {
        println!("Additional Headers: {:?}", header_params);
    }
    if let Some(ref data) = json_data {
        println!("JSON Data: {}", serde_json::to_string_pretty(data)?);
    }

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

    println!("Resolving subject: {}", subject);

    // Resolve the subject to a DID
    let did = resolve_subject(&http_client, &dns_resolver, subject).await?;

    println!("Resolved DID: {}", did);

    // Get the DID document based on DID type
    let document = if did.starts_with("did:plc:") {
        plc::query(&http_client, &plc_hostname, &did).await?
    } else if did.starts_with("did:web:") {
        web::query(&http_client, &did).await?
    } else {
        anyhow::bail!("Unsupported DID method: {}", did);
    };

    println!("Retrieved DID document for: {}", document.id);

    // Get PDS endpoint from the DID document
    let pds_endpoints = document.pds_endpoints();
    let pds_endpoint = pds_endpoints
        .first()
        .ok_or_else(|| CliError::NoPdsEndpointFound { did: did.clone() })?;

    println!("Using PDS endpoint: {}", pds_endpoint);

    // Parse private keys
    let private_dpop_key_data = identify_key(&private_dpop_key)?;

    // Construct the URL
    let mut url = format!("{}/xrpc/{}", pds_endpoint, xrpc_path);

    // Add query parameters if any (only for GET requests)
    if !is_procedure && !query_params.is_empty() {
        let query_string: Vec<String> = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect();
        url.push('?');
        url.push_str(&query_string.join("&"));
    }

    println!("Request URL: {}", url);

    // Create DPoP auth
    let dpop_auth = DPoPAuth {
        dpop_private_key_data: private_dpop_key_data,
        oauth_access_token: access_token.expose_secret().to_string(),
    };

    // Create HeaderMap from header parameters
    let mut additional_headers = HeaderMap::new();
    for (name, value) in header_params {
        if let (Ok(header_name), Ok(header_value)) = (
            name.parse::<reqwest::header::HeaderName>(),
            value.parse::<reqwest::header::HeaderValue>(),
        ) {
            additional_headers.insert(header_name, header_value);
        } else {
            eprintln!("Warning: Invalid header name or value: {}={}", name, value);
        }
    }

    // Make the authenticated request
    println!("Making DPoP authenticated request...");

    let response = if is_procedure {
        let data = json_data.ok_or(CliError::NoJsonDataProvided)?;
        post_dpop_json_with_headers(&http_client, &dpop_auth, &url, data, &additional_headers)
            .await?
    } else {
        get_dpop_json_with_headers(&http_client, &dpop_auth, &url, &additional_headers).await?
    };

    // Print the response
    println!("Response:");
    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}
