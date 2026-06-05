//! Build origin detection — compile-time distinction between production and development builds.
//!
//! Production binaries are downloaded from GitHub Releases by end-users.
//! Development binaries are built from source by maintainers with `--features dev-mode`.
//!
//! This module provides constants and helpers to gate functionality at compile time.

/// The build origin string: "production" or "development".
#[cfg(feature = "dev-mode")]
pub const BUILD_ORIGIN: &str = "development";

#[cfg(not(feature = "dev-mode"))]
pub const BUILD_ORIGIN: &str = "production";

/// Returns true if this binary was built for production (downloaded from GitHub Releases).
pub const fn is_production() -> bool {
    !is_development()
}

/// Returns true if this binary was built for development (built from source with `--features dev-mode`).
pub const fn is_development() -> bool {
    cfg!(feature = "dev-mode")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_origin_is_consistent() {
        // BUILD_ORIGIN must be either "production" or "development"
        assert!(BUILD_ORIGIN == "production" || BUILD_ORIGIN == "development");
    }

    #[test]
    fn test_is_production_and_development_are_complementary() {
        assert_ne!(is_production(), is_development());
    }

    #[test]
    fn test_build_origin_matches_functions() {
        if is_production() {
            assert_eq!(BUILD_ORIGIN, "production");
        } else {
            assert_eq!(BUILD_ORIGIN, "development");
        }
    }
}
