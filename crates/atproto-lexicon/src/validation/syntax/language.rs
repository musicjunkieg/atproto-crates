//! Language tag syntax validation
//!
//! Validates BCP 47 / IETF language tag strings.

use std::sync::LazyLock;

use regex::Regex;

use crate::validation::data_errors::DataValidationError;

/// Regex for validating BCP 47 language tags
///
/// Accepts:
/// - Primary language subtag: 2-3 letter code (e.g., "en", "pt")
/// - Optional script subtag: 4 letter code (e.g., "Latn")
/// - Optional region subtag: 2 letter or 3 digit code (e.g., "US", "419")
/// - Optional variant and extension subtags
static LANGUAGE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z]{2,3}(-[a-zA-Z0-9]{1,8})*$").expect("language regex should compile")
});

/// Validate a BCP 47 language tag
///
/// Accepts language tags like "en", "en-US", "pt-BR", "zh-Hans-CN", etc.
pub fn validate_language(value: &str) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "language".to_string(),
            value: value.to_string(),
            reason: "language tag cannot be empty".to_string(),
        });
    }

    if !LANGUAGE_REGEX.is_match(value) {
        return Err(DataValidationError::StringFormatInvalid {
            format: "language".to_string(),
            value: value.to_string(),
            reason: "language tag must be a valid BCP 47 tag".to_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_language_tags() {
        let valid = [
            "en",
            "en-US",
            "pt-BR",
            "zh-Hans",
            "zh-Hans-CN",
            "de",
            "fr-FR",
            "ja",
            "ko",
        ];
        for lang in valid {
            assert!(validate_language(lang).is_ok(), "should be valid: {}", lang);
        }
    }

    #[test]
    fn test_invalid_language_tags() {
        let invalid = ["", "e", "123", "en_US", "toolongsubtag-exceeds"];
        for lang in invalid {
            assert!(
                validate_language(lang).is_err(),
                "should be invalid: {}",
                lang
            );
        }
    }
}
