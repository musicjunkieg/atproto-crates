//! RASL (Retrieval of Arbitrary Structures & Links).
//!
//! A URL scheme used to identify content-addressed DASL resources along with
//! a simple HTTP-based retrieval method.
//!
//! RASL URL format: `rasl://[CID][/path]?hint=host1&hint=host2`
//!
//! Retrieval: `GET https://[hint]/.well-known/rasl/[cid]`
//!
//! See <https://dasl.ing/> for the specification.

#[cfg(feature = "reqwest")]
pub mod fetch;

#[cfg(feature = "axum")]
pub mod handler;

use crate::cid::{Cid, CidCore};
use crate::errors::RaslError;
use std::fmt;
use std::str::FromStr;
use url::Url;

/// The RASL URL scheme.
pub const RASL_SCHEME: &str = "rasl";

/// The well-known path prefix for RASL HTTP retrieval.
pub const WELL_KNOWN_PREFIX: &str = "/.well-known/rasl/";

/// A parsed RASL URL.
///
/// Contains the CID identifying the resource, an optional subpath, and
/// zero or more retrieval hints (domain names or IP addresses).
///
/// # Example
///
/// ```rust
/// use atproto_dasl::rasl::RaslUrl;
///
/// let url: RaslUrl = "rasl://bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi?hint=example.com"
///     .parse()
///     .unwrap();
/// assert_eq!(url.hints(), &["example.com"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaslUrl {
    /// The CID of the resource.
    cid: Cid,
    /// Optional subpath within the resource.
    path: Option<String>,
    /// Retrieval hints (domain names or IP addresses).
    hints: Vec<String>,
}

impl RaslUrl {
    /// Create a new RASL URL with just a CID.
    pub fn new(cid: Cid) -> Self {
        Self {
            cid,
            path: None,
            hints: Vec::new(),
        }
    }

    /// Create a new RASL URL with a CID and hints.
    pub fn with_hints(cid: Cid, hints: Vec<String>) -> Self {
        Self {
            cid,
            path: None,
            hints,
        }
    }

    /// Set the subpath.
    pub fn with_path(mut self, path: String) -> Self {
        self.path = Some(path);
        self
    }

    /// Add a retrieval hint.
    pub fn add_hint(&mut self, hint: String) {
        self.hints.push(hint);
    }

    /// Get the CID.
    pub fn cid(&self) -> &Cid {
        &self.cid
    }

    /// Get the optional subpath.
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Get the retrieval hints.
    pub fn hints(&self) -> &[String] {
        &self.hints
    }

    /// Get the inner CidCore.
    pub fn cid_core(&self) -> &CidCore {
        self.cid.inner()
    }

    /// Generate the HTTP retrieval URLs for each hint.
    ///
    /// Returns URLs of the form `https://[hint]/.well-known/rasl/[cid]`.
    pub fn retrieval_urls(&self) -> Vec<String> {
        let cid_str = self.cid.to_string();
        self.hints
            .iter()
            .map(|hint| format!("https://{}{}{}", hint, WELL_KNOWN_PREFIX, cid_str))
            .collect()
    }

    /// Parse a RASL URL string.
    ///
    /// # Errors
    ///
    /// Returns `RaslError` if the URL is malformed or violates RASL constraints.
    pub fn parse(s: &str) -> Result<Self, RaslError> {
        let url = Url::parse(s)?;

        // Validate scheme
        if url.scheme() != RASL_SCHEME {
            return Err(RaslError::InvalidScheme {
                scheme: url.scheme().to_string(),
            });
        }

        // No credentials allowed
        if !url.username().is_empty() || url.password().is_some() {
            return Err(RaslError::CredentialsNotAllowed);
        }

        // No fragments allowed
        if url.fragment().is_some() {
            return Err(RaslError::FragmentNotAllowed);
        }

        // Extract CID from host
        let cid_str = url.host_str().ok_or(RaslError::MissingCid)?;
        if cid_str.is_empty() {
            return Err(RaslError::MissingCid);
        }

        let cid = Cid::from_str(cid_str).map_err(|e| RaslError::InvalidCid {
            reason: e.to_string(),
        })?;

        // Extract optional path (beyond the root "/")
        let path = url.path();
        let path = if path.is_empty() || path == "/" {
            None
        } else {
            Some(path.to_string())
        };

        // Extract hints from query parameters
        let hints: Vec<String> = url
            .query_pairs()
            .filter(|(key, _)| key == "hint")
            .map(|(_, value)| value.to_string())
            .filter(|h| !h.is_empty())
            .collect();

        Ok(Self { cid, path, hints })
    }
}

impl fmt::Display for RaslUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}://{}", RASL_SCHEME, self.cid)?;
        if let Some(path) = &self.path {
            write!(f, "{}", path)?;
        }
        for (i, hint) in self.hints.iter().enumerate() {
            if i == 0 {
                write!(f, "?hint={}", hint)?;
            } else {
                write!(f, "&hint={}", hint)?;
            }
        }
        Ok(())
    }
}

impl FromStr for RaslUrl {
    type Err = RaslError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cid::compute_cid;

    fn test_cid() -> Cid {
        Cid::new(compute_cid(b"test data"))
    }

    #[test]
    fn test_rasl_url_new() {
        let cid = test_cid();
        let url = RaslUrl::new(cid.clone());
        assert_eq!(url.cid(), &cid);
        assert!(url.path().is_none());
        assert!(url.hints().is_empty());
    }

    #[test]
    fn test_rasl_url_with_hints() {
        let cid = test_cid();
        let url = RaslUrl::with_hints(cid, vec!["example.com".into(), "cdn.example.com".into()]);
        assert_eq!(url.hints().len(), 2);
        assert_eq!(url.hints()[0], "example.com");
    }

    #[test]
    fn test_rasl_url_display_roundtrip() {
        let cid = test_cid();
        let url =
            RaslUrl::with_hints(cid, vec!["example.com".into()]).with_path("/some/path".into());

        let s = url.to_string();
        assert!(s.starts_with("rasl://"));
        assert!(s.contains("?hint=example.com"));
        assert!(s.contains("/some/path"));
    }

    #[test]
    fn test_rasl_url_retrieval_urls() {
        let cid = test_cid();
        let url = RaslUrl::with_hints(cid, vec!["example.com".into()]);
        let retrieval = url.retrieval_urls();
        assert_eq!(retrieval.len(), 1);
        assert!(retrieval[0].starts_with("https://example.com/.well-known/rasl/"));
    }

    #[test]
    fn test_rasl_url_parse_invalid_scheme() {
        let result = RaslUrl::parse("https://example.com");
        assert!(result.is_err());
    }
}
