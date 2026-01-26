//! CLI tool for signing data with cryptographic keys.

use anyhow::{Context, Result};
use atproto_identity::key::{identify_key, sign};
use clap::Parser;
use std::process::ExitCode;
use std::{fs::File, io::BufReader};

/// AT Protocol Digital Signature CLI
#[derive(Parser)]
#[command(
    name = "atproto-identity-sign",
    version,
    about = "Create cryptographic signatures of JSON data using AT Protocol DID keys",
    long_about = "
A command-line tool for creating cryptographic signatures of JSON data using
AT Protocol DID keys. Takes a JSON file, serializes it using IPLD DAG-CBOR
format, and produces a multibase-encoded signature using the specified key.
Supports both P-256 and K-256 elliptic curve keys via did:key method.

PROCESS:
1. Parses and validates the provided DID key
2. Reads and parses the JSON file
3. Serializes the JSON data using IPLD DAG-CBOR encoding
4. Creates a cryptographic signature using the specified key
5. Encodes the signature using multibase Base64URL format
6. Outputs the encoded signature to stdout

SUPPORTED KEY TYPES:
- P-256 (secp256r1) - NIST standard elliptic curve
- K-256 (secp256k1) - Bitcoin/Ethereum compatible curve

EXAMPLES:
  # Create test data and sign it:
  echo '{\"message\": \"hello world\"}' > data.json
  atproto-identity-sign did:key:z42tv1pb3... data.json

  # Using jo to create JSON:
  jo message=\"hello world\" when=\"$(date)\" > message.json
  atproto-identity-sign did:key:z42tv1pb3... message.json
"
)]
struct Args {
    /// A did:key identifier containing the signing key
    did_key: String,

    /// Path to a JSON file containing the data to sign
    json_file: String,
}

async fn real_main() -> Result<()> {
    let args = Args::parse();

    let identified_key = identify_key(&args.did_key)?;

    let record: serde_json::Value = File::open(&args.json_file)
        .context("failed to open json file")
        .map(BufReader::new)
        .and_then(|value| serde_json::from_reader(value).context("failed to parse json file"))?;

    let serialized_record = atproto_dasl::to_vec(&record)?;

    let signature = sign(&identified_key, &serialized_record)?;
    let encoded_signature = multibase::encode(multibase::Base::Base64Url, &signature);

    println!("{encoded_signature}");

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(err) = real_main().await {
        eprintln!("{err}");
        return ExitCode::from(1);
    }
    ExitCode::from(0)
}
