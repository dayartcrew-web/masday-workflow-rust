//! CLI command handlers
//!
//! Command implementations for the masday CLI.

pub mod install;
pub mod uninstall;
pub mod update;

pub use install::{run as install_run, InstallArgs};
pub use uninstall::{run as uninstall_run, UninstallArgs};
pub use update::run as update_run;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that modules are accessible
        let args = InstallArgs::default();
        assert!(args.remote.is_none());
        assert!(!args.skip_build);
    }
}
