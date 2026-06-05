//! Build origin detection.
//!
//! Production binaries are downloaded from GitHub Releases by end-users.

/// The build origin string: "production" (always, since dev-mode feature removed).
pub const BUILD_ORIGIN: &str = "production";

/// Returns true if this binary was built for production.
pub const fn is_production() -> bool {
    true
}

/// Returns true if this binary was built for development.
pub const fn is_development() -> bool {
    false
}
