//! CLI tool for validating cryptographic signatures.

use anyhow::{Context, Result};
use atproto_identity::key::{identify_key, validate};
use clap::Parser;
use std::{fs::File, io::BufReader, process::ExitCode};

/// AT Protocol Signature Validation CLI
#[derive(Parser)]
#[command(
    name = "atproto-identity-validate",
    version,
    about = "Validate cryptographic signatures of JSON data using AT Protocol DID keys",
    long_about = "
A command-line tool for verifying cryptographic signatures of JSON data using
AT Protocol DID keys. Takes a JSON file, a multibase-encoded signature, and
a DID key, then validates that the signature was created by the specified key
for the given data. Uses IPLD DAG-CBOR serialization format for verification.
Supports both P-256 and K-256 elliptic curve keys via did:key method.

EXAMPLES:
  # Basic signature validation:
  atproto-identity-validate did:key:z42tv1pb3... data.json uEiABC123...

  # Validate signature from external source:
  atproto-identity-validate \\
    did:key:z42tv1pb3Dzog28Q1udyieg1YJP3x1Un5vraE1bttXeCDSpW \\
    message.json \\
    uEiABC123defGHI456jklMNO789pqrSTU012vwxYZ345abc

SUPPORTED KEY TYPES:
  - P-256 (secp256r1) - NIST standard elliptic curve
  - K-256 (secp256k1) - Bitcoin/Ethereum compatible curve

ERROR HANDLING:
  - Invalid DID key format or unsupported key type
  - Missing or unreadable JSON file
  - Malformed JSON content
  - Invalid multibase-encoded signature
  - Signature validation failure

All errors are reported to stderr with descriptive messages and return exit code 1.
Successful validation prints \"OK\" to stdout and returns exit code 0.
"
)]
struct Args {
    /// DID key identifier containing the verification key (e.g., did:key:z42tv1pb3...)
    did_key: String,

    /// Path to a JSON file containing the original signed data
    json_file: String,

    /// Multibase-encoded signature to validate (typically Base64URL)
    signature: String,
}

async fn real_main() -> Result<()> {
    let args = Args::parse();

    let identified_key = identify_key(&args.did_key)?;

    let record: serde_json::Value = File::open(&args.json_file)
        .context("failed to open json file")
        .map(BufReader::new)
        .and_then(|value| serde_json::from_reader(value).context("failed to parse json file"))?;

    let serialized_record = atproto_dasl::to_vec(&record)?;

    let (_, signature_data) = multibase::decode(&args.signature)?;

    validate(&identified_key, &signature_data, &serialized_record)?;

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(err) = real_main().await {
        eprintln!("{err}");
        return ExitCode::from(1);
    }
    println!("OK");
    ExitCode::from(0)
}
