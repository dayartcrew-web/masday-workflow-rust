//! CLI command handlers
//!
//! Command implementations for the masday CLI.

pub mod config;
pub mod db;
pub mod doctor;
pub mod embed;
pub mod install;
pub mod mcp_cmd;
pub mod quickstart;
pub mod serve;
pub mod setup;
pub mod status;
pub mod uninstall;
pub mod update;

#[cfg(feature = "dev-mode")]
pub mod dev;

pub use install::{run as install_run, InstallArgs};
pub use uninstall::{run as uninstall_run, UninstallArgs};
pub use update::run as update_run;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        let args = InstallArgs::default();
        assert!(args.remote.is_none());
        assert!(!args.force);
        assert!(!args.local_only);
        assert!(!args.no_hooks);
        assert!(!args.no_mcp);
        #[cfg(feature = "dev-mode")]
        {
            assert!(!args.skip_build);
        }
    }
}
