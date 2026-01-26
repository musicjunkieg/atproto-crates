//! Format validators for ATProtocol string formats
//!
//! This module provides validation functions for all ATProtocol string formats
//! as defined in the Lexicon specification.

mod at_identifier;
mod at_uri;
mod cid;
mod datetime;
mod did;
mod handle;
mod language;
mod nsid;
mod record_key;
mod tid;
mod uri;

pub use at_identifier::validate_at_identifier;
pub use at_uri::validate_at_uri;
pub use cid::validate_cid;
pub use datetime::validate_datetime;
pub use did::validate_did;
pub use handle::validate_handle;
pub use language::validate_language;
pub use nsid::validate_nsid;
pub use record_key::validate_record_key;
pub use tid::validate_tid;
pub use uri::validate_uri;

use crate::validation::data_errors::DataValidationError;
use crate::validation::flags::ValidateFlags;

/// Validate a string value against a format
pub fn validate_string_format(
    format: &str,
    value: &str,
    flags: ValidateFlags,
) -> Result<(), DataValidationError> {
    match format {
        "at-identifier" => validate_at_identifier(value),
        "at-uri" => validate_at_uri(value),
        "cid" => validate_cid(value),
        "datetime" => validate_datetime(value, flags),
        "did" => validate_did(value),
        "handle" => validate_handle(value),
        "language" => validate_language(value),
        "nsid" => validate_nsid(value),
        "record-key" => validate_record_key(value),
        "tid" => validate_tid(value),
        "uri" => validate_uri(value),
        _ => Err(DataValidationError::InvalidStringFormat {
            format: format.to_string(),
        }),
    }
}

/// Check if a format string is valid
pub fn is_valid_format(format: &str) -> bool {
    matches!(
        format,
        "at-identifier"
            | "at-uri"
            | "cid"
            | "datetime"
            | "did"
            | "handle"
            | "language"
            | "nsid"
            | "record-key"
            | "tid"
            | "uri"
    )
}
