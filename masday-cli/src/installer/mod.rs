//! Installer module for masday-cli
//!
//! Provides template embedding, platform detection, and installation
//! logic for multi-platform Masday workflow support.

mod agent_sync;
mod build;
mod env_setup;
mod hook_setup;
mod mcp_config;
mod platform;
mod remote;
mod settings;
mod skill_sync;
mod templates;

pub use agent_sync::{
    sync_agents_to_global, sync_agents_to_project, SyncReport as AgentSyncReport,
};
pub use build::{build_crates, find_api_binary, find_mcp_binary};
pub use env_setup::{check_prerequisites, ensure_env_file, load_env, Prerequisites};
pub use hook_setup::{
    install_global_hooks, install_project_hooks, register_hooks_in_settings,
    uninstall_global_hooks, uninstall_project_hooks, SyncReport as HookSyncReport,
};
pub use mcp_config::{generate_mcp_config, remove_mcp_config, McpConfig};
pub use platform::{all_platforms, detect_active_platforms, Platform};
pub use remote::{resolve_mcp_binary, verify_remote_url};
pub use settings::{
    remove_masday_entries, update_global_settings, update_json_config, McpServerConfig,
    SettingsUpdates,
};
pub use skill_sync::{
    sync_skills_to_global, sync_skills_to_project, SyncReport as SkillSyncReport,
};
pub use templates::{
    extract_agents, extract_global_hooks, extract_project_hooks, extract_skill_files,
    extract_skill_names, get_templates,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that module exports are accessible
        let platforms = all_platforms();
        assert!(!platforms.is_empty(), "Should have at least one platform");
    }
}
