//! Persistent configuration for the all-in-one masday binary.
//!
//! Config file: `~/.local/share/masday/config.toml` (Linux)
//!              `~/Library/Application Support/masday/config.toml` (macOS)
//!              `%APPDATA%/masday/config.toml` (Windows)
//!
//! Written by `masday setup`, read by `masday serve`, `masday mcp`, etc.

use anyhow::{Context, Result};
use masday_core::constants::ports;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasdayConfig {
    /// "local" | "remote"
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
    #[serde(default = "default_port")]
    pub port: u16,
    /// Target AI platforms: ["claude-code", "gemini", "vscode", "opencode"]
    #[serde(default)]
    pub platforms: Vec<String>,
}

fn default_port() -> u16 {
    ports::api_port()
}

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
            port: ports::api_port(),
            platforms: vec!["claude-code".to_string()],
        }
    }
}

impl MasdayConfig {
    /// Get the config directory path.
    pub fn config_dir() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from(".masday"))
            .join("masday")
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MasdayConfig::default();
        assert_eq!(config.mode, "local");
        assert_eq!(config.port, ports::API_PORT);
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
            port: ports::API_PORT,
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
            database_url: Some("postgresql://USER:PASS@localhost/db".to_string()),
            ..MasdayConfig::default()
        };

        config.set_env_vars();

        assert_eq!(
            std::env::var("DATABASE_URL").ok(),
            Some("postgresql://USER:PASS@localhost/db".to_string())
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
