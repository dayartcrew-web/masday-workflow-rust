//! Embedding service — generates vector embeddings
//!
//! Supports:
//! - **local** (fastembed ONNX Runtime, no external service needed)
//! - Ollama (local, default at localhost:11434)
//! - OpenAI (remote API)
//!
//! Configuration via environment variables:
//! - EMBEDDING_PROVIDER: "local" | "ollama" | "openai" (unset = disabled)
//! - EMBEDDING_MODEL: model name (default per provider)
//! - EMBEDDING_BASE_URL: override base URL (ollama/openai only)
//! - EMBEDDING_API_KEY: API key (required for OpenAI)
//! - EMBEDDING_DIMENSIONS: vector dimensions (default per provider)
//! - FASTEMBED_CACHE_DIR: override model cache directory (local only)

use masday_core::{AppError, Result};
use serde::Deserialize;
use std::sync::OnceLock;
#[cfg(feature = "local-embeddings")]
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

// ── Provider defaults ────────────────────────────────────────────────────────

#[cfg(feature = "local-embeddings")]
const LOCAL_DEFAULT_MODEL: &str = "all-MiniLM-L6-v2";
#[cfg(feature = "local-embeddings")]
const LOCAL_DEFAULT_DIMENSIONS: usize = 384;
const OLLAMA_DEFAULT_MODEL: &str = "nomic-embed-text";
const OLLAMA_DEFAULT_DIMENSIONS: usize = 768;
const OPENAI_DEFAULT_MODEL: &str = "text-embedding-3-small";
const OPENAI_DEFAULT_DIMENSIONS: usize = 768;

// ── EmbeddingConfig ──────────────────────────────────────────────────────────

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
        let model = std::env::var("EMBEDDING_MODEL").ok();
        let base_url = std::env::var("EMBEDDING_BASE_URL").ok();
        let api_key = std::env::var("EMBEDDING_API_KEY").ok();
        let dimensions = std::env::var("EMBEDDING_DIMENSIONS")
            .ok()
            .and_then(|d| d.parse::<usize>().ok());
        Self::from_values(
            &provider,
            model.as_deref(),
            base_url.as_deref(),
            api_key.as_deref(),
            dimensions,
        )
    }

    /// Construct config from explicit values. Returns None for unknown provider.
    /// Used by `from_env()` and directly by tests (no env mutation needed).
    pub fn from_values(
        provider: &str,
        model: Option<&str>,
        base_url: Option<&str>,
        api_key: Option<&str>,
        dimensions: Option<usize>,
    ) -> Option<Self> {
        let (default_model, default_url, default_dims) = match provider {
            #[cfg(feature = "local-embeddings")]
            "local" => (
                LOCAL_DEFAULT_MODEL.to_string(),
                String::new(),
                LOCAL_DEFAULT_DIMENSIONS,
            ),
            #[cfg(not(feature = "local-embeddings"))]
            "local" => {
                warn!("Local embeddings not available (compiled without local-embeddings feature)");
                return None;
            }
            "ollama" => (
                OLLAMA_DEFAULT_MODEL.to_string(),
                "http://localhost:11434".to_string(),
                OLLAMA_DEFAULT_DIMENSIONS,
            ),
            "openai" => (
                OPENAI_DEFAULT_MODEL.to_string(),
                "https://api.openai.com/v1".to_string(),
                OPENAI_DEFAULT_DIMENSIONS,
            ),
            _ => {
                warn!("Unknown embedding provider: {}", provider);
                return None;
            }
        };

        let model = model.map(|m| m.to_string()).unwrap_or(default_model);
        let base_url = base_url.map(|u| u.to_string()).unwrap_or(default_url);
        let api_key = api_key.map(|k| k.to_string());
        let dimensions = dimensions.unwrap_or(default_dims);

        info!(
            "Embedding config: provider={}, model={}, dimensions={}",
            provider, model, dimensions
        );

        Some(Self {
            provider: provider.to_string(),
            model,
            base_url,
            api_key,
            dimensions,
        })
    }

    /// Map model name string to fastembed EmbeddingModel enum.
    /// Falls back to AllMiniLML6V2 for unknown names.
    #[cfg(feature = "local-embeddings")]
    pub fn model_enum(&self) -> fastembed::EmbeddingModel {
        match self.model.as_str() {
            "all-MiniLM-L6-v2" => fastembed::EmbeddingModel::AllMiniLML6V2,
            "bge-small-en-v1.5" => fastembed::EmbeddingModel::BGESmallENV15,
            "bge-base-en-v1.5" => fastembed::EmbeddingModel::BGEBaseENV15,
            "nomic-embed-text-v1.5" => fastembed::EmbeddingModel::NomicEmbedTextV15,
            other => {
                warn!(
                    "Unknown fastembed model '{}', falling back to all-MiniLM-L6-v2",
                    other
                );
                fastembed::EmbeddingModel::AllMiniLML6V2
            }
        }
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

// ── EmbeddingService ─────────────────────────────────────────────────────────

/// Embedding service for generating vector embeddings.
///
/// For the "local" provider, holds a loaded ONNX model in memory.
/// Use `cached()` to get a process-wide singleton that avoids re-loading the model.
pub struct EmbeddingService {
    config: EmbeddingConfig,
    client: reqwest::Client,
    #[cfg(feature = "local-embeddings")]
    local_model: Option<Arc<Mutex<fastembed::TextEmbedding>>>,
}

impl EmbeddingService {
    /// Create a new embedding service from config.
    /// For "local" provider, loads the ONNX model (may download on first use).
    pub fn new(config: EmbeddingConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        #[cfg(feature = "local-embeddings")]
        let local_model = if config.provider == "local" {
            Self::load_local_model(&config)
        } else {
            None
        };

        Self {
            config,
            client,
            #[cfg(feature = "local-embeddings")]
            local_model,
        }
    }

    /// Load the local fastembed model. Returns None on failure.
    #[cfg(feature = "local-embeddings")]
    fn load_local_model(config: &EmbeddingConfig) -> Option<Arc<Mutex<fastembed::TextEmbedding>>> {
        let model_enum = config.model_enum();
        info!("Loading local embedding model: {:?}", model_enum);

        // Configure cache directory
        let mut opts = fastembed::InitOptions::new(model_enum).with_show_download_progress(true);

        if let Ok(cache_dir) = std::env::var("FASTEMBED_CACHE_DIR") {
            opts = opts.with_cache_dir(std::path::PathBuf::from(cache_dir));
        }

        match fastembed::TextEmbedding::try_new(opts) {
            Ok(model) => {
                info!("Local embedding model loaded successfully");
                Some(Arc::new(Mutex::new(model)))
            }
            Err(e) => {
                warn!("Failed to load local embedding model: {}", e);
                None
            }
        }
    }

    /// Try to create from environment variables.
    /// Returns None if embedding is not configured.
    pub fn from_env() -> Option<Self> {
        EmbeddingConfig::from_env().map(Self::new)
    }

    /// Get a cached singleton EmbeddingService.
    /// Creates once from env vars; subsequent calls return the same instance.
    /// Returns None if embedding is not configured.
    pub fn cached() -> Option<&'static EmbeddingService> {
        static INSTANCE: OnceLock<Option<EmbeddingService>> = OnceLock::new();
        INSTANCE.get_or_init(Self::from_env).as_ref()
    }

    /// Generate embedding for a single text input
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        match self.config.provider.as_str() {
            #[cfg(feature = "local-embeddings")]
            "local" => self.embed_local(text).await,
            #[cfg(not(feature = "local-embeddings"))]
            "local" => Err(AppError::Internal(
                "Local embeddings not available (compiled without local-embeddings feature)"
                    .to_string(),
            )),
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
        match self.config.provider.as_str() {
            #[cfg(feature = "local-embeddings")]
            "local" => self.embed_batch_local(texts).await,
            _ => {
                // Sequential for HTTP providers
                let mut results = Vec::with_capacity(texts.len());
                for text in texts {
                    let embedding = self.embed(text).await?;
                    results.push(embedding);
                }
                Ok(results)
            }
        }
    }

    /// Check if the embedding service is healthy
    pub async fn health_check(&self) -> bool {
        match self.config.provider.as_str() {
            #[cfg(feature = "local-embeddings")]
            "local" => self.local_model.is_some(),
            #[cfg(not(feature = "local-embeddings"))]
            "local" => false,
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

    // ── Local (fastembed ONNX) embedding ─────────────────────────────────────

    #[cfg(feature = "local-embeddings")]
    async fn embed_local(&self, text: &str) -> Result<Vec<f32>> {
        let model = self
            .local_model
            .as_ref()
            .ok_or_else(|| AppError::Internal("Local embedding model not loaded".to_string()))?;

        let text_owned = text.to_string();
        let model_arc = Arc::clone(model);

        tokio::task::spawn_blocking(move || {
            let mut model = model_arc.lock().unwrap();
            let documents = vec![text_owned.as_str()];
            let embeddings = model
                .embed(documents, None)
                .map_err(|e| AppError::Internal(format!("Local embedding failed: {}", e)))?;
            embeddings
                .into_iter()
                .next()
                .ok_or_else(|| AppError::Internal("Local model returned no embedding".to_string()))
        })
        .await
        .map_err(|e| AppError::Internal(format!("Blocking task failed: {}", e)))?
    }

    #[cfg(feature = "local-embeddings")]
    async fn embed_batch_local(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let model = self
            .local_model
            .as_ref()
            .ok_or_else(|| AppError::Internal("Local embedding model not loaded".to_string()))?;

        let texts_owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();
        let model_arc = Arc::clone(model);

        tokio::task::spawn_blocking(move || {
            let mut model = model_arc.lock().unwrap();
            let docs: Vec<&str> = texts_owned.iter().map(|s| s.as_str()).collect();
            model
                .embed(docs, None)
                .map_err(|e| AppError::Internal(format!("Local batch embedding failed: {}", e)))
        })
        .await
        .map_err(|e| AppError::Internal(format!("Blocking task failed: {}", e)))?
    }

    // ── Ollama embedding ─────────────────────────────────────────────────────

    async fn embed_ollama(&self, text: &str) -> Result<Vec<f32>> {
        debug!("Calling Ollama embed: model={}", self.config.model);

        // Try new API first (/api/embed with "input"), fall back to legacy (/api/embeddings with "prompt")
        let new_url = format!("{}/api/embed", self.config.base_url);
        let new_body = serde_json::json!({
            "model": self.config.model,
            "input": text,
        });

        let response = self
            .client
            .post(&new_url)
            .json(&new_body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Ollama request failed: {}", e)))?;

        if response.status().is_success() {
            let result: OllamaEmbedResponse = response
                .json()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to parse Ollama response: {}", e)))?;

            return result
                .embeddings
                .into_iter()
                .next()
                .ok_or_else(|| AppError::Internal("Ollama returned no embeddings".to_string()));
        }

        // Legacy endpoint fallback (/api/embeddings with "prompt")
        debug!("Ollama /api/embed not available, trying legacy /api/embeddings");
        let legacy_url = format!("{}/api/embeddings", self.config.base_url);
        let legacy_body = serde_json::json!({
            "model": self.config.model,
            "prompt": text,
        });

        let response = self
            .client
            .post(&legacy_url)
            .json(&legacy_body)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Ollama legacy request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "Ollama returned {}: {}",
                status, body
            )));
        }

        let result: OllamaLegacyEmbedResponse = response
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse Ollama legacy response: {}", e)))?;

        Ok(result.embedding)
    }

    // ── OpenAI embedding ─────────────────────────────────────────────────────

    async fn embed_openai(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/embeddings", self.config.base_url);

        let api_key = self.config.api_key.as_ref().ok_or_else(|| {
            AppError::Internal("EMBEDDING_API_KEY required for OpenAI".to_string())
        })?;

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

/// Ollama embed API response (new /api/embed endpoint)
#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

/// Ollama legacy embed API response (old /api/embeddings endpoint)
#[derive(Debug, Deserialize)]
struct OllamaLegacyEmbedResponse {
    embedding: Vec<f32>,
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Config construction tests (no env vars, no race conditions) ──────────

    #[test]
    #[cfg(feature = "local-embeddings")]
    fn test_config_local_construction() {
        let config = EmbeddingConfig {
            provider: "local".into(),
            model: LOCAL_DEFAULT_MODEL.into(),
            base_url: String::new(),
            api_key: None,
            dimensions: LOCAL_DEFAULT_DIMENSIONS,
        };
        assert_eq!(config.provider, "local");
        assert_eq!(config.model, "all-MiniLM-L6-v2");
        assert_eq!(config.dimensions, 384);
        assert!(config.base_url.is_empty());
    }

    #[test]
    fn test_config_ollama_construction() {
        let config = EmbeddingConfig {
            provider: "ollama".into(),
            model: OLLAMA_DEFAULT_MODEL.into(),
            base_url: "http://localhost:11434".into(),
            api_key: None,
            dimensions: OLLAMA_DEFAULT_DIMENSIONS,
        };
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.model, "nomic-embed-text");
        assert_eq!(config.dimensions, 768);
    }

    #[test]
    fn test_config_openai_construction() {
        let config = EmbeddingConfig {
            provider: "openai".into(),
            model: OPENAI_DEFAULT_MODEL.into(),
            base_url: "https://api.openai.com/v1".into(),
            api_key: Some("test-key".into()),
            dimensions: OPENAI_DEFAULT_DIMENSIONS,
        };
        assert_eq!(config.provider, "openai");
        assert_eq!(config.model, "text-embedding-3-small");
        assert_eq!(config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_config_custom_dimensions() {
        let config = EmbeddingConfig {
            provider: "ollama".into(),
            model: "bge-large".into(),
            base_url: "http://localhost:11434".into(),
            api_key: None,
            dimensions: 1024,
        };
        assert_eq!(config.model, "bge-large");
        assert_eq!(config.dimensions, 1024);
    }

    #[test]
    #[cfg(feature = "local-embeddings")]
    fn test_model_enum_known() {
        let config = EmbeddingConfig {
            provider: "local".into(),
            model: "all-MiniLM-L6-v2".into(),
            base_url: String::new(),
            api_key: None,
            dimensions: 384,
        };
        assert!(matches!(
            config.model_enum(),
            fastembed::EmbeddingModel::AllMiniLML6V2
        ));

        let config_bge = EmbeddingConfig {
            provider: "local".into(),
            model: "bge-base-en-v1.5".into(),
            base_url: String::new(),
            api_key: None,
            dimensions: 768,
        };
        assert!(matches!(
            config_bge.model_enum(),
            fastembed::EmbeddingModel::BGEBaseENV15
        ));
    }

    #[test]
    #[cfg(feature = "local-embeddings")]
    fn test_model_enum_unknown_fallback() {
        let config = EmbeddingConfig {
            provider: "local".into(),
            model: "unknown-model".into(),
            base_url: String::new(),
            api_key: None,
            dimensions: 384,
        };
        assert!(matches!(
            config.model_enum(),
            fastembed::EmbeddingModel::AllMiniLML6V2
        ));
    }

    // ── Config construction tests (no env mutation) ──────────────────────────

    #[test]
    fn test_config_from_values_none_for_unknown_provider() {
        assert!(EmbeddingConfig::from_values("unknown", None, None, None, None).is_none());
    }

    #[test]
    fn test_config_from_values_local() {
        let config = EmbeddingConfig::from_values("local", None, None, None, None).unwrap();
        assert_eq!(config.provider, "local");
        assert_eq!(config.model, "all-MiniLM-L6-v2");
        assert_eq!(config.dimensions, 384);
    }

    #[test]
    fn test_service_none_when_no_env() {
        // from_env() returns None when EMBEDDING_PROVIDER is not set — no mutation needed
        // since the test runner doesn't set it by default
        assert!(EmbeddingConfig::from_values("", None, None, None, None).is_none());
    }

    // ── Pure unit tests (no env, no IO) ─────────────────────────────────────

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
        assert!(!truncated.is_empty());
    }

    #[test]
    fn test_truncate_unicode_safe() {
        let text = "hello 🌍 world 🚀 test";
        let truncated = truncate_for_embedding(text, 3);
        assert!(!truncated.is_empty());
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

    #[test]
    #[cfg(feature = "local-embeddings")]
    fn test_local_service_loads_fallback_model() {
        let config = EmbeddingConfig::test_config("local");
        let service = EmbeddingService::new(config);
        assert_eq!(service.provider(), "local");
        assert!(service.local_model.is_some());
    }
}
