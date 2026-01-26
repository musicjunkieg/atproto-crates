//! CLI tool for AT Protocol cryptographic key operations.

use anyhow::Result;
use atproto_identity::key::{KeyType, generate_key, to_public};
use clap::{Parser, Subcommand};

/// AT Protocol Key Management Tool
#[derive(Parser)]
#[command(
    name = "atproto-identity-key",
    version,
    about = "AT Protocol cryptographic key management",
    long_about = "
This command-line tool provides cryptographic key management capabilities for AT Protocol.
It supports key generation and inspection operations for P-256, P-384, and K-256 elliptic curves.

KEY TYPES:
  p256    P-256 (secp256r1/ES256): NIST standard curve, commonly used in web standards
  p384    P-384 (secp384r1/ES384): NIST standard curve, providing higher security than P-256
  k256    K-256 (secp256k1/ES256K): Bitcoin curve, widely used in blockchain applications

SECURITY CONSIDERATIONS:
- Generated keys use cryptographically secure random number generation
- Private keys are generated using industry-standard elliptic curve implementations
- Key material is handled securely in memory

EXAMPLES:
  # Generate P-256 key pair:
  atproto-identity-key generate p256

  # Generate P-384 key pair:
  atproto-identity-key generate p384

  # Generate K-256 key pair:
  atproto-identity-key generate k256

  # Inspect key (placeholder):
  atproto-identity-key inspect
"
)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new key pair and display both private and public keys
    Generate {
        /// Key type to generate (p256, p384, or k256)
        #[arg(value_enum)]
        key_type: KeyTypeArg,
    },
    /// Inspect key information (placeholder)
    Inspect,
}

#[derive(clap::ValueEnum, Clone)]
enum KeyTypeArg {
    /// P-256 (secp256r1/ES256) key pair
    P256,
    /// P-384 (secp384r1/ES384) key pair
    P384,
    /// K-256 (secp256k1/ES256K) key pair
    K256,
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Generate { key_type } => {
            let (key_type_actual, key_type_str) = match key_type {
                KeyTypeArg::P256 => (KeyType::P256Private, "p256"),
                KeyTypeArg::P384 => (KeyType::P384Private, "p384"),
                KeyTypeArg::K256 => (KeyType::K256Private, "k256"),
            };

            let private_key = generate_key(key_type_actual)?;
            let public_key = to_public(&private_key)?;

            println!("{} private: {}", key_type_str, private_key);
            println!("{} public: {}", key_type_str, public_key);

            Ok(())
        }
        Commands::Inspect => {
            println!("hello world");
            Ok(())
        }
    }
}
