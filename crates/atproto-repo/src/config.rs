//! Configuration for AT Protocol repository operations.
//!
//! This module provides configuration types for controlling verification,
//! memory limits, and other operational parameters.
//!
//! `LimitsConfig` has been moved to the `atproto-dasl` crate and is
//! re-exported from the crate root.

use atproto_dasl::LimitsConfig;
use atproto_dasl::car::CarConfig;

/// Configuration for repository operations with verification and limits.
///
/// # Example
///
/// ```rust
/// use atproto_repo::RepoConfig;
///
/// // Default configuration with verification enabled
/// let config = RepoConfig::default();
/// assert!(config.verify_cids);
/// assert!(config.verify_signatures);
///
/// // For performance-critical processing
/// let fast_config = RepoConfig::no_verification();
///
/// // For low-memory environments
/// let low_mem_config = RepoConfig::low_memory();
/// ```
#[derive(Debug, Clone)]
pub struct RepoConfig {
    /// Whether to verify CIDs match block content (default: true).
    pub verify_cids: bool,

    /// Whether to verify commit signatures (default: true).
    pub verify_signatures: bool,

    /// Whether to strictly validate CID format (CIDv1, dag-cbor, sha-256).
    pub strict_cid_format: bool,

    /// Memory limits configuration.
    pub limits: LimitsConfig,
}

impl Default for RepoConfig {
    fn default() -> Self {
        Self {
            verify_cids: true,
            verify_signatures: true,
            strict_cid_format: true,
            limits: LimitsConfig::default(),
        }
    }
}

impl RepoConfig {
    /// Create config with verification disabled (faster, less safe).
    ///
    /// Use only with trusted inputs where verification is not needed.
    #[must_use]
    pub fn no_verification() -> Self {
        Self {
            verify_cids: false,
            verify_signatures: false,
            strict_cid_format: false,
            limits: LimitsConfig::default(),
        }
    }

    /// Create config optimized for low-memory environments.
    #[must_use]
    pub fn low_memory() -> Self {
        Self {
            limits: LimitsConfig::low_memory(),
            ..Default::default()
        }
    }

    /// Create config with custom limits.
    #[must_use]
    pub fn with_limits(mut self, limits: LimitsConfig) -> Self {
        self.limits = limits;
        self
    }

    /// Set whether to verify CIDs.
    #[must_use]
    pub fn with_verify_cids(mut self, verify: bool) -> Self {
        self.verify_cids = verify;
        self
    }

    /// Set whether to verify signatures.
    #[must_use]
    pub fn with_verify_signatures(mut self, verify: bool) -> Self {
        self.verify_signatures = verify;
        self
    }

    /// Set whether to use strict CID format validation.
    #[must_use]
    pub fn with_strict_cid_format(mut self, strict: bool) -> Self {
        self.strict_cid_format = strict;
        self
    }

    /// Convert to a `CarConfig` for use with the CAR reader/writer.
    #[must_use]
    pub fn car_config(&self) -> CarConfig {
        CarConfig {
            verify_cids: self.verify_cids,
            strict_cid_format: self.strict_cid_format,
            limits: self.limits.clone(),
        }
    }
}

impl From<RepoConfig> for CarConfig {
    fn from(config: RepoConfig) -> Self {
        config.car_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RepoConfig::default();
        assert!(config.verify_cids);
        assert!(config.verify_signatures);
        assert!(config.strict_cid_format);
    }

    #[test]
    fn test_no_verification_config() {
        let config = RepoConfig::no_verification();
        assert!(!config.verify_cids);
        assert!(!config.verify_signatures);
        assert!(!config.strict_cid_format);
    }

    #[test]
    fn test_low_memory_limits() {
        let limits = LimitsConfig::low_memory();
        assert_eq!(limits.max_memory_bytes, 10 * 1024 * 1024);
        assert_eq!(limits.max_block_size, 256 * 1024);
        assert_eq!(limits.max_block_count, 10_000);
    }

    #[test]
    fn test_high_throughput_limits() {
        let limits = LimitsConfig::high_throughput();
        assert_eq!(limits.max_memory_bytes, 1024 * 1024 * 1024);
        assert_eq!(limits.max_block_count, 1_000_000);
    }

    #[test]
    fn test_builder_pattern() {
        let limits = LimitsConfig::default()
            .with_max_memory_bytes(50 * 1024 * 1024)
            .with_max_block_size(512 * 1024);

        assert_eq!(limits.max_memory_bytes, 50 * 1024 * 1024);
        assert_eq!(limits.max_block_size, 512 * 1024);

        let config = RepoConfig::default()
            .with_verify_cids(false)
            .with_limits(limits);

        assert!(!config.verify_cids);
        assert_eq!(config.limits.max_memory_bytes, 50 * 1024 * 1024);
    }
}
