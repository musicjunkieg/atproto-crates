//! Configuration for CAR file operations.
//!
//! This module provides configuration types for controlling CID verification,
//! memory limits, and other operational parameters for CAR processing.

/// Configuration for CAR file operations with verification and limits.
///
/// # Example
///
/// ```rust
/// use atproto_dasl::car::CarConfig;
///
/// // Default configuration with verification enabled
/// let config = CarConfig::default();
/// assert!(config.verify_cids);
///
/// // For performance-critical processing
/// let fast_config = CarConfig::no_verification();
///
/// // For low-memory environments
/// let low_mem_config = CarConfig::low_memory();
/// ```
#[derive(Debug, Clone)]
pub struct CarConfig {
    /// Whether to verify CIDs match block content (default: true).
    pub verify_cids: bool,

    /// Whether to strictly validate CID format (CIDv1, dag-cbor, sha-256).
    pub strict_cid_format: bool,

    /// Memory limits configuration.
    pub limits: LimitsConfig,
}

/// Memory and resource limits for low-resource environments.
///
/// These limits help prevent denial-of-service attacks and enable
/// safe processing in shared/multi-tenant environments.
///
/// # Example
///
/// ```rust
/// use atproto_dasl::LimitsConfig;
///
/// // Default limits (100MB memory, 1MB blocks)
/// let limits = LimitsConfig::default();
///
/// // For embedded or low-memory systems
/// let low_mem = LimitsConfig::low_memory();
///
/// // For high-throughput batch processing
/// let high_perf = LimitsConfig::high_throughput();
/// ```
#[derive(Debug, Clone)]
pub struct LimitsConfig {
    /// Maximum total memory for block storage in bytes (default: 100MB).
    pub max_memory_bytes: usize,

    /// Maximum size of a single block in bytes (default: 1MB).
    pub max_block_size: usize,

    /// Maximum number of blocks to hold in memory (default: 100,000).
    pub max_block_count: usize,

    /// Maximum traversal depth (default: 64).
    pub max_depth: usize,

    /// Maximum CAR file size to process in bytes (default: 10GB).
    pub max_car_size: u64,

    /// Threshold for spilling to disk in bytes (default: 50MB).
    pub disk_spill_threshold: usize,
}

impl Default for CarConfig {
    fn default() -> Self {
        Self {
            verify_cids: true,
            strict_cid_format: true,
            limits: LimitsConfig::default(),
        }
    }
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_bytes: 100 * 1024 * 1024, // 100 MB
            max_block_size: 1024 * 1024,         // 1 MB
            max_block_count: 100_000,
            max_depth: 64,
            max_car_size: 10 * 1024 * 1024 * 1024,  // 10 GB
            disk_spill_threshold: 50 * 1024 * 1024, // 50 MB
        }
    }
}

impl LimitsConfig {
    /// Create limits optimized for low-memory environments.
    ///
    /// - Max memory: 10 MB
    /// - Max block size: 256 KB
    /// - Max block count: 10,000
    /// - Max depth: 32
    /// - Max CAR size: 1 GB
    /// - Disk spill threshold: 5 MB
    #[must_use]
    pub fn low_memory() -> Self {
        Self {
            max_memory_bytes: 10 * 1024 * 1024, // 10 MB
            max_block_size: 256 * 1024,         // 256 KB
            max_block_count: 10_000,
            max_depth: 32,
            max_car_size: 1024 * 1024 * 1024,      // 1 GB
            disk_spill_threshold: 5 * 1024 * 1024, // 5 MB
        }
    }

    /// Create limits for high-throughput processing.
    ///
    /// - Max memory: 1 GB
    /// - Max block size: 10 MB
    /// - Max block count: 1,000,000
    /// - Max depth: 128
    /// - Max CAR size: 100 GB
    /// - Disk spill threshold: 500 MB
    #[must_use]
    pub fn high_throughput() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024, // 1 GB
            max_block_size: 10 * 1024 * 1024,     // 10 MB
            max_block_count: 1_000_000,
            max_depth: 128,
            max_car_size: 100 * 1024 * 1024 * 1024,  // 100 GB
            disk_spill_threshold: 500 * 1024 * 1024, // 500 MB
        }
    }

    /// Disable all limits (use with caution).
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            max_memory_bytes: usize::MAX,
            max_block_size: usize::MAX,
            max_block_count: usize::MAX,
            max_depth: usize::MAX,
            max_car_size: u64::MAX,
            disk_spill_threshold: usize::MAX,
        }
    }

    /// Set the maximum memory bytes.
    #[must_use]
    pub fn with_max_memory_bytes(mut self, bytes: usize) -> Self {
        self.max_memory_bytes = bytes;
        self
    }

    /// Set the maximum block size.
    #[must_use]
    pub fn with_max_block_size(mut self, size: usize) -> Self {
        self.max_block_size = size;
        self
    }

    /// Set the maximum block count.
    #[must_use]
    pub fn with_max_block_count(mut self, count: usize) -> Self {
        self.max_block_count = count;
        self
    }

    /// Set the maximum traversal depth.
    #[must_use]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set the maximum CAR file size.
    #[must_use]
    pub fn with_max_car_size(mut self, size: u64) -> Self {
        self.max_car_size = size;
        self
    }

    /// Set the disk spill threshold.
    #[must_use]
    pub fn with_disk_spill_threshold(mut self, threshold: usize) -> Self {
        self.disk_spill_threshold = threshold;
        self
    }
}

impl CarConfig {
    /// Create config with verification disabled (faster, less safe).
    #[must_use]
    pub fn no_verification() -> Self {
        Self {
            verify_cids: false,
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

    /// Set whether to use strict CID format validation.
    #[must_use]
    pub fn with_strict_cid_format(mut self, strict: bool) -> Self {
        self.strict_cid_format = strict;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CarConfig::default();
        assert!(config.verify_cids);
        assert!(config.strict_cid_format);
    }

    #[test]
    fn test_no_verification_config() {
        let config = CarConfig::no_verification();
        assert!(!config.verify_cids);
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

        let config = CarConfig::default()
            .with_verify_cids(false)
            .with_limits(limits);

        assert!(!config.verify_cids);
        assert_eq!(config.limits.max_memory_bytes, 50 * 1024 * 1024);
    }
}
