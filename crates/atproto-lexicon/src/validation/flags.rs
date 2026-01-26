//! Validation flags for controlling validation behavior

use bitflags::bitflags;

bitflags! {
    /// Flags that control validation behavior
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ValidateFlags: u32 {
        /// Allow legacy blob format (without size field)
        const ALLOW_LEGACY_BLOB = 0b0001;
        /// Allow lenient datetime parsing (more formats)
        const ALLOW_LENIENT_DATETIME = 0b0010;
        /// Strict validation of recursive types in open unions
        const STRICT_RECURSIVE_VALIDATION = 0b0100;
        /// Skip validation of external references (treat them as valid)
        const SKIP_EXTERNAL_REFS = 0b1000;
    }
}

impl Default for ValidateFlags {
    fn default() -> Self {
        Self::empty()
    }
}
