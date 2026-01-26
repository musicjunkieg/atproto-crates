//! MIME type matching for blob validation

/// Check if a MIME type matches an accepted pattern
///
/// Supports exact matches and wildcard patterns like "image/*"
pub fn mime_type_matches(actual: &str, pattern: &str) -> bool {
    // Exact match
    if actual == pattern {
        return true;
    }

    // Wildcard match (e.g., "image/*")
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 1]; // "image/"
        if actual.starts_with(prefix) {
            return true;
        }
    }

    // Handle case variations (MIME types are case-insensitive for type/subtype)
    if actual.eq_ignore_ascii_case(pattern) {
        return true;
    }

    // Wildcard with case insensitivity
    if pattern.ends_with("/*") {
        let prefix = pattern[..pattern.len() - 1].to_lowercase();
        if actual.to_lowercase().starts_with(&prefix) {
            return true;
        }
    }

    false
}

/// Check if a MIME type matches any of the accepted patterns
pub fn mime_type_matches_any(actual: &str, patterns: &[String]) -> bool {
    // Empty list means any type is accepted
    if patterns.is_empty() {
        return true;
    }

    patterns.iter().any(|p| mime_type_matches(actual, p))
}

/// Parse and normalize a MIME type
///
/// Removes parameters (e.g., "text/plain; charset=utf-8" -> "text/plain")
pub fn normalize_mime_type(mime: &str) -> &str {
    // Remove parameters
    mime.split(';').next().unwrap_or(mime).trim()
}

/// Validate that a string looks like a valid MIME type
pub fn is_valid_mime_type(mime: &str) -> bool {
    let normalized = normalize_mime_type(mime);

    // Must have exactly one slash
    let slash_count = normalized.chars().filter(|&c| c == '/').count();
    if slash_count != 1 {
        return false;
    }

    // Split into type and subtype
    let parts: Vec<&str> = normalized.split('/').collect();
    if parts.len() != 2 {
        return false;
    }

    let type_part = parts[0];
    let subtype_part = parts[1];

    // Type and subtype must not be empty
    if type_part.is_empty() || subtype_part.is_empty() {
        return false;
    }

    // Type and subtype must be valid tokens (alphanumeric, hyphen, dot, plus)
    let is_valid_token = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '+' || c == '_')
    };

    // Allow wildcard in subtype
    if subtype_part == "*" {
        return is_valid_token(type_part);
    }

    is_valid_token(type_part) && is_valid_token(subtype_part)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(mime_type_matches("image/png", "image/png"));
        assert!(mime_type_matches("text/plain", "text/plain"));
        assert!(!mime_type_matches("image/png", "image/jpeg"));
    }

    #[test]
    fn test_wildcard_match() {
        assert!(mime_type_matches("image/png", "image/*"));
        assert!(mime_type_matches("image/jpeg", "image/*"));
        assert!(mime_type_matches("image/gif", "image/*"));
        assert!(!mime_type_matches("video/mp4", "image/*"));
        assert!(!mime_type_matches("text/plain", "image/*"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(mime_type_matches("IMAGE/PNG", "image/png"));
        assert!(mime_type_matches("image/png", "IMAGE/PNG"));
        assert!(mime_type_matches("Image/Png", "image/*"));
    }

    #[test]
    fn test_matches_any() {
        let patterns = vec![
            "image/png".to_string(),
            "image/jpeg".to_string(),
            "image/gif".to_string(),
        ];
        assert!(mime_type_matches_any("image/png", &patterns));
        assert!(mime_type_matches_any("image/jpeg", &patterns));
        assert!(!mime_type_matches_any("image/webp", &patterns));
    }

    #[test]
    fn test_matches_any_wildcard() {
        let patterns = vec!["image/*".to_string(), "video/*".to_string()];
        assert!(mime_type_matches_any("image/png", &patterns));
        assert!(mime_type_matches_any("video/mp4", &patterns));
        assert!(!mime_type_matches_any("text/plain", &patterns));
    }

    #[test]
    fn test_matches_any_empty() {
        // Empty list means any type is accepted
        let patterns: Vec<String> = vec![];
        assert!(mime_type_matches_any("image/png", &patterns));
        assert!(mime_type_matches_any("application/json", &patterns));
    }

    #[test]
    fn test_normalize_mime_type() {
        assert_eq!(normalize_mime_type("text/plain"), "text/plain");
        assert_eq!(
            normalize_mime_type("text/plain; charset=utf-8"),
            "text/plain"
        );
        assert_eq!(
            normalize_mime_type("text/html; charset=utf-8; boundary=something"),
            "text/html"
        );
        assert_eq!(normalize_mime_type("  text/plain  "), "text/plain");
    }

    #[test]
    fn test_is_valid_mime_type() {
        // Valid
        assert!(is_valid_mime_type("text/plain"));
        assert!(is_valid_mime_type("image/png"));
        assert!(is_valid_mime_type("application/json"));
        assert!(is_valid_mime_type("application/vnd.api+json"));
        assert!(is_valid_mime_type("image/*"));
        assert!(is_valid_mime_type("application/x-custom"));

        // Invalid
        assert!(!is_valid_mime_type("text"));
        assert!(!is_valid_mime_type("text/"));
        assert!(!is_valid_mime_type("/plain"));
        assert!(!is_valid_mime_type("text/plain/extra"));
        assert!(!is_valid_mime_type(""));
        assert!(!is_valid_mime_type("invalid"));
    }

    #[test]
    fn test_common_mime_types() {
        // Common ATProtocol blob types
        let image_types = vec!["image/*".to_string()];
        assert!(mime_type_matches_any("image/png", &image_types));
        assert!(mime_type_matches_any("image/jpeg", &image_types));
        assert!(mime_type_matches_any("image/gif", &image_types));
        assert!(mime_type_matches_any("image/webp", &image_types));
        assert!(!mime_type_matches_any("video/mp4", &image_types));

        let video_types = vec!["video/*".to_string()];
        assert!(mime_type_matches_any("video/mp4", &video_types));
        assert!(mime_type_matches_any("video/webm", &video_types));
        assert!(!mime_type_matches_any("image/png", &video_types));
    }
}
