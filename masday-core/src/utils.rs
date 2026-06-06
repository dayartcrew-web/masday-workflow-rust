//! Utility functions for validation and formatting

use uuid::Uuid;

/// Validate if a string is a valid UUID.
///
/// This function checks if the input string can be parsed as a UUID.
/// Returns true if valid, false otherwise.
///
/// # Arguments
/// * `id` - The ID string to validate
///
/// # Returns
/// * `bool` - true if valid UUID format, false otherwise
///
/// # Examples
/// ```
/// use masday_core::utils::validate_uuid;
///
/// assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000"));
/// assert!(!validate_uuid("not-a-uuid"));
/// assert!(!validate_uuid(""));
/// ```
pub fn validate_uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_uuid_valid() {
        // Valid UUID v4
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000"));

        // Valid UUID with uppercase
        assert!(validate_uuid("550E8400-E29B-41D4-A716-446655440000"));

        // Valid UUID with mixed case
        assert!(validate_uuid("550e8400-E29b-41d4-a716-446655440000"));
    }

    #[test]
    fn test_validate_uuid_invalid() {
        // Invalid format
        assert!(!validate_uuid("not-a-uuid"));
        assert!(!validate_uuid("550e8400-e29b-41d4-a716")); // Too short

        // Empty string
        assert!(!validate_uuid(""));

        // Missing segments
        assert!(!validate_uuid("550e8400-e29b-41d4-a716"));

        // Invalid characters
        assert!(!validate_uuid("550e8400-e29b-41d4-a716-44665544000g"));

        // Too long
        assert!(!validate_uuid("550e8400-e29b-41d4-a716-446655440000-extra"));
    }

    #[test]
    fn test_validate_uuid_nil() {
        // Nil UUID is valid
        assert!(validate_uuid("00000000-0000-0000-0000-000000000000"));
    }
}
