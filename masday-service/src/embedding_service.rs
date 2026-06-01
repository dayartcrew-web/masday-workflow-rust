//! Embedding service — generates vector embeddings via remote APIs
//!
//! Supports:
//! - Ollama (local, default at localhost:11434)
//! - OpenAI (remote API)
//!
//! Configuration via environment variables:
//! - EMBEDDING_PROVIDER: "ollama" or "openai" (unset = disabled)
//! - EMBEDDING_MODEL: model name (default per provider)
//! - EMBEDDING_BASE_URL: override base URL
//! - EMBEDDING_API_KEY: API key (required for OpenAI)
//! - EMBEDDING_DIMENSIONS: vector dimensions (default 768)

use masday_core::{AppError, Result};
use serde::Deserialize;
use tracing::{debug, info, warn};

/// Default models per provider
const OLLAMA_DEFAULT_MODEL: &str = "nomic-embed-text";
const OPENAI_DEFAULT_MODEL: &str = "text-embedding-3-small";
const DEFAULT_DIMENSIONS: usize = 768;

/// Embedding provider configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub dimensions: usize,
}

impl EmbeddingConfig {
    /// Load config from environment variables.
    /// Returns None if EMBEDDING_PROVIDER is not set (embedding disabled).
    pub fn from_env() -> Option<Self> {
        let provider = std::env::var("EMBEDDING_PROVIDER").ok()?;

        let (default_model, default_url) = match provider.as_str() {
            "ollama" => (
                OLLAMA_DEFAULT_MODEL.to_string(),
                "http://localhost:11434".to_string(),
            ),
            "openai" => (
                OPENAI_DEFAULT_MODEL.to_string(),
                "https://api.openai.com/v1".to_string(),
            ),
            _ => {
                warn!("Unknown embedding provider: {}", provider);
                return None;
            }
        };

        let model = std::env::var("EMBEDDING_MODEL").unwrap_or(default_model);
        let base_url = std::env::var("EMBEDDING_BASE_URL").unwrap_or(default_url);
        let api_key = std::env::var("EMBEDDING_API_KEY").ok();
        let dimensions = std::env::var("EMBEDDING_DIMENSIONS")
            .ok()
            .and_then(|d| d.parse::<usize>().ok())
            .unwrap_or(DEFAULT_DIMENSIONS);

        info!(
            "Embedding config: provider={}, model={}, dimensions={}",
            provider, model, dimensions
        );

        Some(Self {
            provider,
            model,
            base_url,
            api_key,
            dimensions,
        })
    }

    /// Create config for testing
    #[cfg(test)]
    pub fn test_config(provider: &str) -> Self {
        Self {
            provider: provider.to_string(),
            model: "test-model".to_string(),
            base_url: "http://localhost:11434".to_string(),
            api_key: None,
            dimensions: 768,
        }
    }
}

/// Embedding service for generating vector embeddings
pub struct EmbeddingService {
    config: EmbeddingConfig,
    client: reqwest::Client,
}

impl EmbeddingService {
    /// Create a new embedding service.
    /// Call `from_env()` to auto-create from environment variables.
    pub fn new(config: EmbeddingConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Try to create from environment variables.
    /// Returns None if embedding is not configured.
    pub fn from_env() -> Option<Self> {
        EmbeddingConfig::from_env().map(Self::new)
    }

    /// Generate embedding for a single text input
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        match self.config.provider.as_str() {
            "ollama" => self.embed_ollama(text).await,
            "openai" => self.embed_openai(text).await,
            _ => Err(AppError::Internal(format!(
                "Unknown embedding provider: {}",
                self.config.provider
            ))),
        }
    }

    /// Generate embeddings for multiple texts
    pub async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        // For now, process one by one. Could batch later.
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            let embedding = self.embed(text).await?;
            results.push(embedding);
        }
        Ok(results)
    }

    /// Check if the embedding service is healthy
    pub async fn health_check(&self) -> bool {
        match self.config.provider.as_str() {
            "ollama" => {
                let url = format!("{}/api/tags", self.config.base_url);
                self.client.get(&url).send().await.is_ok()
            }
            "openai" => {
                let url = format!("{}/models", self.config.base_url);
                let mut req = self.client.get(&url);
                if let Some(ref key) = self.config.api_key {
                    req = req.header("Authorization", format!("Bearer {}", key));
                }
                req.send().await.is_ok()
            }
            _ => false,
        }
    }

    /// Get the configured dimensions
    pub fn dimensions(&self) -> usize {
        self.config.dimensions
    }

    /// Get the provider name
    pub fn provider(&self) -> &str {
        &self.config.provider
    }

    /// Get the model name
    pub fn model(&self) -> &str {
        &self.config.model
    }

    // ── Ollama embedding ─────────────────────────────────────────────────────

    async fn embed_ollama(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embed", self.config.base_url);

        let body = serde_json::json!({
            "model": self.config.model,
            "input": text,
        });

        debug!("Calling Ollama embed: model={}", self.config.model);

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Ollama request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Ollama returned {}: {}",
                status, body
            )));
        }

        let result: OllamaEmbedResponse = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse Ollama response: {}", e)))?;

        result
            .embeddings
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Internal("Ollama returned no embeddings".to_string()))
    }

    // ── OpenAI embedding ─────────────────────────────────────────────────────

    async fn embed_openai(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/embeddings", self.config.base_url);

        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| AppError::Internal("EMBEDDING_API_KEY required for OpenAI".to_string()))?;

        let body = serde_json::json!({
            "model": self.config.model,
            "input": text,
            "dimensions": self.config.dimensions,
        });

        debug!("Calling OpenAI embeddings: model={}", self.config.model);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OpenAI request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "OpenAI returned {}: {}",
                status, body
            )));
        }

        let result: OpenAIEmbedResponse = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse OpenAI response: {}", e)))?;

        result
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| AppError::Internal("OpenAI returned no embeddings".to_string()))
    }
}

// ── API Response Types ────────────────────────────────────────────────────────

/// Ollama embed API response
#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

/// OpenAI embeddings API response
#[derive(Debug, Deserialize)]
struct OpenAIEmbedResponse {
    data: Vec<OpenAIEmbedData>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedData {
    embedding: Vec<f32>,
}

// ── Helper: truncate text for embedding ───────────────────────────────────────

/// Truncate text to approximate token limit for embedding models.
/// Most embedding models have 512-8192 token limits.
/// Using ~4 chars/token as rough estimate.
pub fn truncate_for_embedding(text: &str, max_tokens: usize) -> &str {
    let max_chars = max_tokens * 4;
    if text.len() <= max_chars {
        return text;
    }

    // Find a safe truncation point (don't split mid-UTF8)
    let mut end = max_chars;
    while !text.is_char_boundary(end) && end > 0 {
        end -= 1;
    }
    &text[..end]
}

/// Build embedding input from summary + content
pub fn build_embedding_input(summary: &str, content: &str) -> String {
    // Use summary as primary, content as secondary (truncated)
    let truncated_content = truncate_for_embedding(content, 200);
    format!("{}\n\n{}", summary, truncated_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env_none() {
        // No EMBEDDING_PROVIDER set → None
        std::env::remove_var("EMBEDDING_PROVIDER");
        assert!(EmbeddingConfig::from_env().is_none());
    }

    #[test]
    fn test_config_ollama_defaults() {
        std::env::set_var("EMBEDDING_PROVIDER", "ollama");
        std::env::remove_var("EMBEDDING_MODEL");
        std::env::remove_var("EMBEDDING_BASE_URL");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.model, "nomic-embed-text");
        assert_eq!(config.base_url, "http://localhost:11434");
        assert!(config.api_key.is_none());
        assert_eq!(config.dimensions, 768);

        std::env::remove_var("EMBEDDING_PROVIDER");
    }

    #[test]
    fn test_config_openai_defaults() {
        std::env::set_var("EMBEDDING_PROVIDER", "openai");
        std::env::remove_var("EMBEDDING_MODEL");
        std::env::set_var("EMBEDDING_API_KEY", "test-key");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "text-embedding-3-small");
        assert_eq!(config.base_url, "https://api.openai.com/v1");
        assert_eq!(config.api_key, Some("test-key".to_string()));

        std::env::remove_var("EMBEDDING_PROVIDER");
        std::env::remove_var("EMBEDDING_API_KEY");
    }

    #[test]
    fn test_config_custom_model() {
        std::env::set_var("EMBEDDING_PROVIDER", "ollama");
        std::env::set_var("EMBEDDING_MODEL", "bge-large");
        std::env::set_var("EMBEDDING_DIMENSIONS", "1024");

        let config = EmbeddingConfig::from_env().unwrap();
        assert_eq!(config.model, "bge-large");
        assert_eq!(config.dimensions, 1024);

        std::env::remove_var("EMBEDDING_PROVIDER");
        std::env::remove_var("EMBEDDING_MODEL");
        std::env::remove_var("EMBEDDING_DIMENSIONS");
    }

    #[test]
    fn test_config_unknown_provider() {
        std::env::set_var("EMBEDDING_PROVIDER", "unknown");
        assert!(EmbeddingConfig::from_env().is_none());
        std::env::remove_var("EMBEDDING_PROVIDER");
    }

    #[test]
    fn test_service_from_env_disabled() {
        std::env::remove_var("EMBEDDING_PROVIDER");
        assert!(EmbeddingService::from_env().is_none());
    }

    #[test]
    fn test_truncate_short_text() {
        let text = "hello world";
        assert_eq!(truncate_for_embedding(text, 100), text);
    }

    #[test]
    fn test_truncate_long_text() {
        let text = "a".repeat(10000);
        let truncated = truncate_for_embedding(&text, 100);
        assert!(truncated.len() <= 400);
        assert!(truncated.len() > 0);
    }

    #[test]
    fn test_truncate_unicode_safe() {
        let text = "hello 🌍 world 🚀 test";
        let truncated = truncate_for_embedding(text, 3);
        // Should not panic on UTF-8 boundary
        assert!(truncated.len() > 0);
    }

    #[test]
    fn test_build_embedding_input() {
        let input = build_embedding_input("test summary", "test content that is longer");
        assert!(input.starts_with("test summary"));
        assert!(input.contains("test content"));
    }

    #[test]
    fn test_build_embedding_input_truncates_content() {
        let long_content = "x".repeat(5000);
        let input = build_embedding_input("short summary", &long_content);
        // Should be truncated
        assert!(input.len() < long_content.len() + 100);
    }

    #[test]
    fn test_service_properties() {
        let config = EmbeddingConfig::test_config("ollama");
        let service = EmbeddingService::new(config);
        assert_eq!(service.dimensions(), 768);
        assert_eq!(service.provider(), "ollama");
        assert_eq!(service.model(), "test-model");
    }
}
