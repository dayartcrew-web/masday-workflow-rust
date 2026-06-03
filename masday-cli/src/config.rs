//! Persistent configuration for the all-in-one masday binary.
//!
//! Config file: `~/.masday/config.toml`
//! Binary:      `~/.masday/bin/masday`
//!
//! Everything under `~/.masday/` — config, binary, data.
//! Written by `masday quickstart` / `masday setup`, read by `masday serve`, `masday mcp`, etc.

use anyhow::{Context, Result};
use masday_core::constants::ports;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasdayConfig {
    /// "local" | "remote" | "standalone"
    pub mode: String,
    /// API server URL (e.g. "http://localhost:30101" or "https://masday.example.com")
    pub api_url: String,
    /// API key for authentication
    pub api_key: String,
    /// PostgreSQL connection URL (local mode only)
    pub database_url: Option<String>,
    /// Embedding provider: "local" | "ollama" | "openai"
    pub embedding_provider: Option<String>,
    /// Embedding model name
    pub embedding_model: Option<String>,
    /// Embedding vector dimensions
    pub embedding_dimensions: Option<usize>,
    /// API server port (local mode)
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    /// PostgreSQL port (local mode)
    #[serde(default = "default_db_port")]
    pub db_port: u16,
    /// Redis port (local mode)
    #[serde(default = "default_redis_port")]
    pub redis_port: u16,
    /// Dashboard port (local mode)
    #[serde(default = "default_api_port")]
    pub dashboard_port: u16,
    /// Target AI platforms: ["claude-code", "gemini", "vscode", "opencode"]
    #[serde(default)]
    pub platforms: Vec<String>,
}

fn default_api_port() -> u16 { ports::API_PORT }
fn default_db_port() -> u16 { ports::POSTGRES_PORT }
fn default_redis_port() -> u16 { ports::REDIS_PORT }

impl Default for MasdayConfig {
    fn default() -> Self {
        Self {
            mode: "local".to_string(),
            api_url: ports::api_base_url(),
            api_key: "local-dev".to_string(),
            database_url: None,
            embedding_provider: None,
            embedding_model: None,
            embedding_dimensions: None,
            api_port: ports::API_PORT,
            db_port: ports::POSTGRES_PORT,
            redis_port: ports::REDIS_PORT,
            dashboard_port: ports::API_PORT,
            platforms: vec!["claude-code".to_string()],
        }
    }
}

impl MasdayConfig {
    /// Get the masday home directory: `~/.masday/`
    ///
    /// Layout:
    ///   ~/.masday/
    ///     config.toml       — configuration
    ///     bin/masday        — CLI binary
    ///     agents/           — global agents (optional)
    ///     skills/           — global skills (optional)
    pub fn masday_home() -> PathBuf {
        home::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".masday")
    }

    /// Get the config directory path (same as masday_home).
    pub fn config_dir() -> PathBuf {
        Self::masday_home()
    }

    /// Get the config file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Load config from disk. Returns None if no config exists.
    pub fn load() -> Option<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// Load config or return error with context
    pub fn load_or_err() -> Result<Self> {
        let path = Self::config_path();
        let content = std::fs::read_to_string(&path).with_context(|| {
            format!(
                "Config not found at {}. Run 'masday setup' first.",
                path.display()
            )
        })?;
        toml::from_str(&content).with_context(|| "Failed to parse config.toml".to_string())
    }

    /// Save config to disk, creating directories as needed
    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create config dir {}", dir.display()))?;

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        let path = Self::config_path();
        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;

        Ok(())
    }

    /// Check if a config file exists
    pub fn exists() -> bool {
        Self::config_path().exists()
    }

    /// Set environment variables from config values
    pub fn set_env_vars(&self) {
        if let Some(ref db_url) = self.database_url {
            std::env::set_var("DATABASE_URL", db_url);
        }
        if let Some(ref provider) = self.embedding_provider {
            std::env::set_var("EMBEDDING_PROVIDER", provider);
        }
        if let Some(ref model) = self.embedding_model {
            std::env::set_var("EMBEDDING_MODEL", model);
        }
        if let Some(dims) = self.embedding_dimensions {
            std::env::set_var("EMBEDDING_DIMENSIONS", dims.to_string());
        }
        // Port config → env vars (so hooks, docker, and sub-processes can read them)
        std::env::set_var("MASDAY_API_PORT", self.api_port.to_string());
        std::env::set_var("MASDAY_DB_PORT", self.db_port.to_string());
        std::env::set_var("MASDAY_REDIS_PORT", self.redis_port.to_string());
        std::env::set_var("MASDAY_DASHBOARD_PORT", self.dashboard_port.to_string());
    }

    /// Get config as a JSON object suitable for writing to a hook env file
    /// or passing to Node.js hooks.
    pub fn to_hook_env(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        map.insert("MASDAY_API_PORT".into(), self.api_port.to_string());
        map.insert("MASDAY_DB_PORT".into(), self.db_port.to_string());
        map.insert("MASDAY_REDIS_PORT".into(), self.redis_port.to_string());
        map.insert("MASDAY_DASHBOARD_PORT".into(), self.dashboard_port.to_string());
        if let Some(ref url) = self.database_url {
            map.insert("DATABASE_URL".into(), url.clone());
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MasdayConfig::default();
        assert_eq!(config.mode, "local");
        assert_eq!(config.api_port, ports::API_PORT);
        assert!(config.database_url.is_none());
    }

    #[test]
    fn test_config_roundtrip() {
        let config = MasdayConfig {
            mode: "remote".to_string(),
            api_url: "https://masday.example.com".to_string(),
            api_key: "test-key-123".to_string(),
            database_url: None,
            embedding_provider: Some("local".to_string()),
            embedding_model: Some("all-MiniLM-L6-v2".to_string()),
            embedding_dimensions: Some(384),
            api_port: ports::API_PORT,
            db_port: ports::POSTGRES_PORT,
            redis_port: ports::REDIS_PORT,
            dashboard_port: ports::API_PORT,
            platforms: vec!["claude-code".to_string(), "gemini".to_string()],
        };

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: MasdayConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.mode, "remote");
        assert_eq!(parsed.api_url, "https://masday.example.com");
        assert_eq!(parsed.api_key, "test-key-123");
        assert_eq!(parsed.embedding_provider, Some("local".to_string()));
        assert_eq!(parsed.embedding_dimensions, Some(384));
        assert_eq!(parsed.platforms, vec!["claude-code", "gemini"]);
    }

    #[test]
    fn test_config_path_contains_masday() {
        let path = MasdayConfig::config_path();
        assert!(path.to_string_lossy().contains("masday"));
    }

    #[test]
    fn test_set_env_vars() {
        let config = MasdayConfig {
            embedding_provider: Some("local".to_string()),
            embedding_model: Some("test-model".to_string()),
            embedding_dimensions: Some(384),
            database_url: Some("postgresql://test:test@localhost/db".to_string()),
            ..MasdayConfig::default()
        };

        config.set_env_vars();

        assert_eq!(
            std::env::var("DATABASE_URL").ok(),
            Some("postgresql://test:test@localhost/db".to_string())
        );
        assert_eq!(
            std::env::var("EMBEDDING_PROVIDER").ok(),
            Some("local".to_string())
        );

        // Cleanup
        std::env::remove_var("DATABASE_URL");
        std::env::remove_var("EMBEDDING_PROVIDER");
        std::env::remove_var("EMBEDDING_MODEL");
        std::env::remove_var("EMBEDDING_DIMENSIONS");
    }
}
