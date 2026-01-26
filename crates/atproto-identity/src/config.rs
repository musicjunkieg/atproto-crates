//! Configuration management for AT Protocol identity operations.
//!
//! Environment variable utilities, DNS nameserver configuration, and TLS certificate
//! bundle management with structured error handling.

use crate::errors::ConfigError;
use anyhow::Result;

/// Certificate bundle paths for TLS verification.
/// Contains a collection of file paths to certificate bundles.
#[derive(Clone)]
pub struct CertificateBundles(Vec<String>);

/// DNS nameserver IP addresses for domain resolution.
/// Contains a collection of IP addresses for custom DNS resolution.
#[derive(Clone)]
pub struct DnsNameservers(Vec<std::net::IpAddr>);

/// Gets a required environment variable value.
/// Returns an error if the variable is not set.
pub fn require_env(name: &str) -> Result<String, ConfigError> {
    std::env::var(name).map_err(|_| ConfigError::MissingEnvironmentVariable {
        name: name.to_string(),
    })
}

/// Gets an optional environment variable value.
/// Returns empty string if the variable is not set.
pub fn optional_env(name: &str) -> String {
    std::env::var(name).unwrap_or("".to_string())
}

/// Gets an environment variable value with a fallback default.
/// Returns the default value if the variable is not set.
pub fn default_env(name: &str, default_value: &str) -> String {
    std::env::var(name).unwrap_or(default_value.to_string())
}

/// Gets the application version from git hash or package version.
/// Returns the git hash if available, otherwise falls back to cargo package version.
pub fn version() -> Result<String, ConfigError> {
    option_env!("GIT_HASH")
        .or(option_env!("CARGO_PKG_VERSION"))
        .map(|val| val.to_string())
        .ok_or(ConfigError::VersionNotAvailable)
}

impl TryFrom<String> for CertificateBundles {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self(
            value
                .split(';')
                .filter_map(|s| {
                    if s.is_empty() {
                        None
                    } else {
                        Some(s.to_string())
                    }
                })
                .collect::<Vec<String>>(),
        ))
    }
}
impl AsRef<Vec<String>> for CertificateBundles {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}

impl TryFrom<String> for DnsNameservers {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        // Allow empty value for default DNS configuration
        if value.is_empty() {
            return Ok(Self(Vec::new()));
        }

        let nameservers = value
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<std::net::IpAddr>().map_err(|_| {
                    anyhow::Error::from(ConfigError::InvalidNameserverIP {
                        value: s.to_string(),
                    })
                })
            })
            .collect::<Result<Vec<std::net::IpAddr>, _>>()?;

        Ok(Self(nameservers))
    }
}

impl AsRef<Vec<std::net::IpAddr>> for DnsNameservers {
    fn as_ref(&self) -> &Vec<std::net::IpAddr> {
        &self.0
    }
}
