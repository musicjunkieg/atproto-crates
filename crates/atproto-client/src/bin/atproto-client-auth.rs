//! AT Protocol client authentication tool for app password session management.
//!
//! This binary tool provides commands for creating and refreshing app password
//! authentication sessions with AT Protocol servers.

use anyhow::Result;
use atproto_client::errors::CliError;
use atproto_identity::{
    config::{CertificateBundles, DnsNameservers, default_env, optional_env, version},
    plc,
    resolve::{HickoryDnsResolver, resolve_subject},
    web,
};
use clap::{Parser, Subcommand};
use rpassword::read_password;
use secrecy::{ExposeSecret, SecretString};
use serde_json::json;
use std::{
    env,
    io::{self, Write},
};

// Import from public module
use atproto_client::com::atproto::server::{create_session, refresh_session};

/// AT Protocol Client Authentication Tool
#[derive(Parser)]
#[command(
    name = "atproto-client-auth",
    version,
    about = "Create and refresh AT Protocol app password authentication sessions",
    long_about = "
A command-line tool for managing AT Protocol app password authentication sessions.
Provides commands for creating new sessions and refreshing existing ones.

ENVIRONMENT VARIABLES:
  CERTIFICATE_BUNDLES  Custom CA certificate bundles
  USER_AGENT          Custom user agent string
  DNS_NAMESERVERS     Custom DNS nameservers
  PDS_ENDPOINT        Override PDS endpoint (skips DID resolution)

EXAMPLES:
  # Login with handle and app password:
  atproto-client-auth login alice.bsky.social app-password-here

  # Login with email and 2FA:
  atproto-client-auth login alice@example.com password123 123456

  # Refresh session:
  atproto-client-auth refresh eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiJ9...
"
)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new app-password session
    Login {
        /// Handle or email for authentication
        identifier: String,
        /// App password or account password from environment variable
        #[arg(long, env = "ATPROTO_PASSWORD")]
        password: Option<String>,
        /// Optional 2FA token
        auth_factor_token: Option<String>,
    },
    /// Refresh an existing app-password session
    Refresh {
        /// JWT refresh token from previous session
        refresh_token: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

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

    match args.command {
        Commands::Login {
            identifier,
            password,
            auth_factor_token,
        } => {
            // Secure password handling: prefer environment variable, then secure prompt
            let password = if let Some(env_password) = password {
                SecretString::new(env_password.into())
            } else {
                // Use secure prompt with hidden input
                print!("Enter password: ");
                io::stdout().flush()?;
                let password = read_password()?;
                if password.is_empty() {
                    anyhow::bail!("Password cannot be empty");
                }
                SecretString::new(password.into())
            };
            println!("Creating app password session");
            println!("Identifier: {}", identifier);

            // Determine PDS endpoint
            let pds_endpoint = if let Ok(endpoint) = env::var("PDS_ENDPOINT") {
                println!("Using PDS endpoint from environment: {}", endpoint);
                endpoint
            } else {
                println!("Resolving identifier to find PDS endpoint...");

                // Resolve the identifier to a DID
                let did = resolve_subject(&http_client, &dns_resolver, &identifier).await?;
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

                println!("Found PDS endpoint: {}", pds_endpoint);
                pds_endpoint.to_string()
            };

            // Create session
            println!("Creating session...");
            let session = create_session(
                &http_client,
                &pds_endpoint,
                &identifier,
                password.expose_secret(),
                auth_factor_token.as_deref(),
            )
            .await?;

            println!("Session created successfully!");
            println!();
            println!("Session Details:");
            println!("  DID: {}", session.did);
            println!("  Handle: {}", session.handle);
            println!("  Email: {}", session.email);
            println!();
            println!("Tokens (save these for authenticated requests):");
            println!("  Access Token: {}", session.access_jwt);
            println!("  Refresh Token: {}", session.refresh_jwt);
            println!();
            println!("Session JSON:");
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "did": session.did,
                    "handle": session.handle,
                    "email": session.email,
                    "accessJwt": session.access_jwt,
                    "refreshJwt": session.refresh_jwt
                }))?
            );
        }

        Commands::Refresh { refresh_token } => {
            println!("Refreshing app password session");

            // Determine PDS endpoint
            let pds_endpoint = if let Ok(endpoint) = env::var("PDS_ENDPOINT") {
                println!("Using PDS endpoint from environment: {}", endpoint);
                endpoint
            } else {
                // For refresh, we don't have the identifier, so require PDS_ENDPOINT
                eprintln!("Error: PDS_ENDPOINT environment variable required for refresh command");
                eprintln!("Set PDS_ENDPOINT to your PDS URL (e.g., https://bsky.social)");
                return Ok(());
            };

            // Refresh session
            println!("Refreshing session...");
            let refreshed_session =
                refresh_session(&http_client, &pds_endpoint, &refresh_token).await?;

            println!("Session refreshed successfully!");
            println!();
            println!("Updated Session Details:");
            println!("  DID: {}", refreshed_session.did);
            println!("  Handle: {}", refreshed_session.handle);
            if let Some(active) = refreshed_session.active {
                println!("  Active: {}", active);
            }
            if let Some(ref status) = refreshed_session.status {
                println!("  Status: {}", status);
            }
            println!();
            println!("New Tokens (save these for authenticated requests):");
            println!("  Access Token: {}", refreshed_session.access_jwt);
            println!("  Refresh Token: {}", refreshed_session.refresh_jwt);
            println!();
            println!("Session JSON:");
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "did": refreshed_session.did,
                    "handle": refreshed_session.handle,
                    "accessJwt": refreshed_session.access_jwt,
                    "refreshJwt": refreshed_session.refresh_jwt,
                    "active": refreshed_session.active,
                    "status": refreshed_session.status
                }))?
            );
        }
    }

    Ok(())
}
