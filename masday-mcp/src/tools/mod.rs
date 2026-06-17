//! MCP tool handlers

// Command execution utilities
pub mod cmd;

// Universal entry point
pub mod use_masday;

// Data tools (HTTP-based)
pub mod capability;
pub mod context;
pub mod graph;
pub mod memory;
pub mod policy;
pub mod reminder;
pub mod review;
pub mod session;
pub mod workflow;

// Local-only tools (Phase 3.2 - stubs)
pub mod cicd;
pub mod docker;
pub mod filesystem;
pub mod git;
pub mod github;
pub mod local;
pub mod npm;
pub mod project_rules;
pub mod tests;
