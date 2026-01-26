//! Datetime syntax validation
//!
//! Validates datetime strings according to the AT Protocol specification,
//! which requires a subset of RFC 3339 / ISO 8601 datetime format.

use std::sync::LazyLock;

use regex::Regex;

use crate::validation::data_errors::DataValidationError;
use crate::validation::flags::ValidateFlags;

/// Strict RFC 3339 datetime regex
///
/// Format: YYYY-MM-DDTHH:MM:SS[.fractional]Z or YYYY-MM-DDTHH:MM:SS[.fractional]+HH:MM
static STRICT_DATETIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})$"
    ).expect("strict datetime regex should compile")
});

/// Lenient datetime regex that also accepts lowercase 't' and 'z', space separator, etc.
static LENIENT_DATETIME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^[0-9]{4}-[0-9]{2}-[0-9]{2}[T ][0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?(Z|[+-][0-9]{2}:?[0-9]{2})$"
    ).expect("lenient datetime regex should compile")
});

/// Validate a datetime string
///
/// With strict validation, requires RFC 3339 format with uppercase T and Z.
/// With lenient validation (via `ValidateFlags::ALLOW_LENIENT_DATETIME`),
/// accepts more datetime formats.
pub fn validate_datetime(value: &str, flags: ValidateFlags) -> Result<(), DataValidationError> {
    if value.is_empty() {
        return Err(DataValidationError::StringFormatInvalid {
            format: "datetime".to_string(),
            value: value.to_string(),
            reason: "datetime cannot be empty".to_string(),
        });
    }

    let is_valid = if flags.contains(ValidateFlags::ALLOW_LENIENT_DATETIME) {
        LENIENT_DATETIME_REGEX.is_match(value)
    } else {
        STRICT_DATETIME_REGEX.is_match(value)
    };

    if !is_valid {
        return Err(DataValidationError::StringFormatInvalid {
            format: "datetime".to_string(),
            value: value.to_string(),
            reason: "datetime must be a valid RFC 3339 datetime string".to_string(),
        });
    }

    // Validate date ranges
    let date_part = &value[..10];
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() == 3
        && let (Ok(month), Ok(day)) = (parts[1].parse::<u32>(), parts[2].parse::<u32>())
    {
        if !(1..=12).contains(&month) {
            return Err(DataValidationError::StringFormatInvalid {
                format: "datetime".to_string(),
                value: value.to_string(),
                reason: format!("invalid month: {}", month),
            });
        }
        if !(1..=31).contains(&day) {
            return Err(DataValidationError::StringFormatInvalid {
                format: "datetime".to_string(),
                value: value.to_string(),
                reason: format!("invalid day: {}", day),
            });
        }
    }

    // Validate time ranges
    let time_start = value.find('T').or_else(|| value.find('t')).unwrap_or(10) + 1;
    if time_start + 8 <= value.len() {
        let time_part = &value[time_start..time_start + 8];
        let time_parts: Vec<&str> = time_part.split(':').collect();
        if time_parts.len() == 3
            && let (Ok(hour), Ok(minute), Ok(second)) = (
                time_parts[0].parse::<u32>(),
                time_parts[1].parse::<u32>(),
                time_parts[2].parse::<u32>(),
            )
        {
            if hour > 23 {
                return Err(DataValidationError::StringFormatInvalid {
                    format: "datetime".to_string(),
                    value: value.to_string(),
                    reason: format!("invalid hour: {}", hour),
                });
            }
            if minute > 59 {
                return Err(DataValidationError::StringFormatInvalid {
                    format: "datetime".to_string(),
                    value: value.to_string(),
                    reason: format!("invalid minute: {}", minute),
                });
            }
            if second > 60 {
                // 60 allowed for leap seconds
                return Err(DataValidationError::StringFormatInvalid {
                    format: "datetime".to_string(),
                    value: value.to_string(),
                    reason: format!("invalid second: {}", second),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_datetimes_strict() {
        let valid = [
            "2023-01-01T00:00:00Z",
            "2023-12-31T23:59:59Z",
            "2023-06-15T12:30:45.123Z",
            "2023-06-15T12:30:45.123456789Z",
            "2023-06-15T12:30:45+05:30",
            "2023-06-15T12:30:45-08:00",
            "2023-06-15T12:30:45.000Z",
        ];
        let flags = ValidateFlags::empty();
        for dt in valid {
            assert!(
                validate_datetime(dt, flags).is_ok(),
                "should be valid: {}",
                dt
            );
        }
    }

    #[test]
    fn test_invalid_datetimes_strict() {
        let invalid = [
            "",
            "2023-01-01",
            "not-a-datetime",
            "2023-13-01T00:00:00Z",
            "2023-01-32T00:00:00Z",
            "2023-01-01T25:00:00Z",
        ];
        let flags = ValidateFlags::empty();
        for dt in invalid {
            assert!(
                validate_datetime(dt, flags).is_err(),
                "should be invalid: {}",
                dt
            );
        }
    }

    #[test]
    fn test_lenient_datetimes() {
        let flags = ValidateFlags::ALLOW_LENIENT_DATETIME;
        assert!(validate_datetime("2023-01-01T00:00:00Z", flags).is_ok());
        assert!(validate_datetime("2023-01-01t00:00:00z", flags).is_ok());
        assert!(validate_datetime("2023-01-01T00:00:00+0000", flags).is_ok());
    }
}
