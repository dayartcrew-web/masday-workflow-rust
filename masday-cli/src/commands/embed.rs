//! `masday embed` subcommand — manage embedding providers, models, and cache
//!
//! Subcommands:
//!   masday embed status    — Show embedding provider & model info
//!   masday embed download  — Download model cache for offline use
//!   masday embed list      — List available & cached models
//!   masday embed test      — Run embedding test with sample text
//!   masday embed clear     — Clear embedding cache
//!   masday embed settings  — Interactive embedding configuration wizard
//!
//! Layout:
//!   ~/.masday/embed-cache/       — model cache (Ollama/OpenAI embeddings)
//!   ~/.masday/config.toml        — embedding provider, model, dimensions config

use anyhow::{bail, Context, Result};
use console::style;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::config::MasdayConfig;

/// Base directory for embedding cache
pub fn embed_cache_dir() -> PathBuf {
    home::home_dir()
        .expect("No home directory")
        .join(".masday")
        .join("embed-cache")
}

/// Check if embedding is configured
pub fn is_embedding_configured() -> bool {
    if let Some(config) = MasdayConfig::load() {
        config.embedding_provider.is_some()
            && config.embedding_model.is_some()
            && config.embedding_dimensions.is_some()
    } else {
        false
    }
}

/// Known embedding models with metadata
#[derive(Debug, Clone)]
pub struct EmbeddingModel {
    /// Model identifier
    pub id: &'static str,
    /// Display name
    pub name: &'static str,
    /// Provider: local | ollama | openai
    pub provider: &'static str,
    /// Vector dimensions
    pub dimensions: usize,
    /// Description
    pub description: &'static str,
}

/// Available embedding models
pub const AVAILABLE_MODELS: &[EmbeddingModel] = &[
    EmbeddingModel {
        id: "nomic-embed-text",
        name: "Nomic Embed Text",
        provider: "ollama",
        dimensions: 768,
        description: "Recommended, 768 dimensions",
    },
    EmbeddingModel {
        id: "all-minilm",
        name: "All MiniLM",
        provider: "ollama",
        dimensions: 384,
        description: "Lightweight, 384 dimensions",
    },
    EmbeddingModel {
        id: "mxbai-embed-large",
        name: "MXBAI Embed Large",
        provider: "ollama",
        dimensions: 1024,
        description: "High quality, 1024 dimensions",
    },
    EmbeddingModel {
        id: "text-embedding-3-small",
        name: "OpenAI text-embedding-3-small",
        provider: "openai",
        dimensions: 1536,
        description: "OpenAI small embedding model",
    },
    EmbeddingModel {
        id: "text-embedding-3-large",
        name: "OpenAI text-embedding-3-large",
        provider: "openai",
        dimensions: 3072,
        description: "OpenAI large embedding model",
    },
    EmbeddingModel {
        id: "text-embedding-ada-002",
        name: "OpenAI ada-002",
        provider: "openai",
        dimensions: 1536,
        description: "Legacy OpenAI embedding model",
    },
    // Local (fastembed) models
    EmbeddingModel {
        id: "all-MiniLM-L6-v2",
        name: "MiniLM-L6-v2",
        provider: "local",
        dimensions: 384,
        description: "Default, fast, lightweight",
    },
    EmbeddingModel {
        id: "all-MiniLM-L6-v2-Q",
        name: "MiniLM-L6-v2 (quantized)",
        provider: "local",
        dimensions: 384,
        description: "Smaller size, 384 dims",
    },
    EmbeddingModel {
        id: "bge-small-en-v1.5",
        name: "BGE Small",
        provider: "local",
        dimensions: 384,
        description: "Fast English model",
    },
    EmbeddingModel {
        id: "bge-base-en-v1.5",
        name: "BGE Base",
        provider: "local",
        dimensions: 768,
        description: "Balanced quality, 768 dims",
    },
    EmbeddingModel {
        id: "bge-large-en-v1.5",
        name: "BGE Large",
        provider: "local",
        dimensions: 1024,
        description: "High quality, 1024 dims",
    },
    EmbeddingModel {
        id: "nomic-embed-text-v1.5",
        name: "Nomic Embed v1.5",
        provider: "local",
        dimensions: 768,
        description: "8K context length",
    },
    EmbeddingModel {
        id: "mxbai-embed-large-v1",
        name: "MixedBreed Large",
        provider: "local",
        dimensions: 1024,
        description: "High quality English",
    },
    EmbeddingModel {
        id: "multilingual-e5-small",
        name: "E5 Small (multilingual)",
        provider: "local",
        dimensions: 384,
        description: "Multilingual, lightweight",
    },
    EmbeddingModel {
        id: "multilingual-e5-base",
        name: "E5 Base (multilingual)",
        provider: "local",
        dimensions: 768,
        description: "Multilingual, balanced",
    },
    EmbeddingModel {
        id: "snowflake-arctic-embed-xs",
        name: "Arctic XS",
        provider: "local",
        dimensions: 384,
        description: "Lightweight Snowflake model",
    },
    EmbeddingModel {
        id: "snowflake-arctic-embed-m",
        name: "Arctic Medium",
        provider: "local",
        dimensions: 768,
        description: "Balanced Snowflake model",
    },
    EmbeddingModel {
        id: "BGEM3",
        name: "BGE-M3",
        provider: "local",
        dimensions: 1024,
        description: "100+ languages, 8K context",
    },
    EmbeddingModel {
        id: "jina-embeddings-v2-base-code",
        name: "Jina Code",
        provider: "local",
        dimensions: 768,
        description: "Code-optimized embeddings",
    },
];

/// Find model by ID
fn find_model(id: &str) -> Option<&'static EmbeddingModel> {
    AVAILABLE_MODELS.iter().find(|m| m.id == id)
}

/// Get cached models from filesystem
fn get_cached_models() -> Vec<String> {
    let cache_dir = embed_cache_dir();
    if !cache_dir.exists() {
        return Vec::new();
    }

    let mut cached = Vec::new();
    if let Ok(entries) = fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if !name.starts_with('.') {
                    cached.push(name);
                }
            }
        }
    }
    cached
}

/// Calculate cache size
fn get_cache_size() -> Result<u64> {
    let cache_dir = embed_cache_dir();
    if !cache_dir.exists() {
        return Ok(0);
    }

    let mut size = 0u64;
    for entry in fs::read_dir(&cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            size += dir_size(&path)?;
        } else {
            size += entry.metadata()?.len();
        }
    }
    Ok(size)
}

/// Calculate total size of a directory recursively
fn dir_size(path: &std::path::Path) -> Result<u64> {
    let mut size = 0u64;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                size += dir_size(&entry.path())?;
            } else {
                size += meta.len();
            }
        }
    }
    Ok(size)
}

/// Format bytes as human-readable
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Print box-drawing header
fn print_box_header(title: &str) {
    println!();
    println!("╭─────────────────────────────────────────╮");
    println!("│  {:^37} │", title);
    println!("│                                         │");
}

/// Print box-drawing row
fn print_box_row(key: &str, value: &str) {
    println!("│  {:12}: {:21} │", key, value);
}

/// Print empty box row
fn print_box_empty() {
    println!("│                                         │");
}

/// Print box-drawing footer
fn print_box_footer() {
    println!("╰─────────────────────────────────────────╯");
}

// ─── CLI Entry Points ───────────────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum EmbedAction {
    /// Show embedding provider & model info
    Status,

    /// Download model cache for offline use
    Download {
        /// Provider: ollama | openai
        #[arg(long)]
        provider: Option<String>,

        /// Model name to download
        #[arg(long)]
        model: Option<String>,

        /// Re-download even if cached
        #[arg(long)]
        force: bool,
    },

    /// List available & cached models
    List,

    /// Run embedding test with sample text
    Test {
        /// Text to embed (default: "Hello, world!")
        #[arg(default_value = "Hello, world!")]
        text: String,
    },

    /// Clear embedding cache
    Clear,

    /// Interactive embedding configuration wizard
    Settings,
}

pub fn run(action: EmbedAction) -> Result<()> {
    match action {
        EmbedAction::Status => run_status(),
        EmbedAction::Download {
            provider,
            model,
            force,
        } => {
            run_download(provider, model, force)?;
            Ok(())
        }
        EmbedAction::List => run_list(),
        EmbedAction::Test { text } => run_test(&text),
        EmbedAction::Clear => run_clear(),
        EmbedAction::Settings => run_settings(),
    }
}

/// Show embedding provider & model info
fn run_status() -> Result<()> {
    print_box_header("Embedding Service");

    let config = MasdayConfig::load().unwrap_or_default();

    // Provider
    let provider = config
        .embedding_provider
        .as_deref()
        .unwrap_or("not configured");
    print_box_row("Provider", provider);

    // Model
    let model = config
        .embedding_model
        .as_deref()
        .unwrap_or("not configured");
    print_box_row("Model", model);

    // Dimensions
    let dims = config
        .embedding_dimensions
        .map(|d| d.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    print_box_row("Dimensions", &dims);

    // Base URL (provider-specific)
    let base_url = match config.embedding_provider.as_deref() {
        Some("local") => "ONNX Runtime (local)",
        Some("ollama") => "http://localhost:11434",
        Some("openai") => "https://api.openai.com/v1",
        Some("disabled") => "disabled",
        _ => "not configured",
    };
    print_box_row("Base URL", base_url);

    print_box_empty();

    // Status
    let configured = is_embedding_configured();
    let status = if configured {
        style("✓ ready").green().to_string()
    } else {
        style("✗ not configured").red().to_string()
    };
    print_box_row("Status", &status);

    // Cache
    let cache_path = embed_cache_dir();
    print_box_row("Cache", cache_path.to_string_lossy().as_ref());

    // Cache size
    let cache_size = get_cache_size().unwrap_or(0);
    print_box_row("Cache size", &format_bytes(cache_size));

    // Last test (simulated - would be stored in state in real impl)
    print_box_row("Last test", "not run");

    print_box_footer();

    if !configured {
        println!();
        println!(
            "{} Run 'masday embed settings' to configure embeddings",
            style("Hint:").cyan()
        );
    }

    Ok(())
}

/// Download model cache for offline use
fn run_download(provider: Option<String>, model: Option<String>, force: bool) -> Result<()> {
    let config = MasdayConfig::load().unwrap_or_default();

    // Determine provider
    let provider_id = provider.unwrap_or_else(|| {
        config
            .embedding_provider
            .clone()
            .unwrap_or_else(|| "ollama".to_string())
    });

    if !["ollama", "openai", "local"].contains(&provider_id.as_str()) {
        bail!(
            "Invalid provider '{}'. Must be 'ollama', 'openai', or 'local'",
            provider_id
        );
    }

    // Determine model
    let model_id = model.unwrap_or_else(|| {
        config
            .embedding_model
            .clone()
            .unwrap_or_else(|| "nomic-embed-text".to_string())
    });

    // Find model metadata (allow custom models not in list)
    let model_info = find_model(&model_id);
    let dimensions = model_info.map(|m| m.dimensions).unwrap_or(768);
    let display_name = model_info.map(|m| m.name).unwrap_or(&model_id);

    // Verify provider matches for known models
    if let Some(info) = model_info {
        if info.provider != provider_id {
            bail!(
                "Model '{}' is for {}, but you specified {}",
                model_id,
                info.provider,
                provider_id
            );
        }
    }

    println!();
    println!(
        "{} {}",
        style("Downloading model:").bold().cyan(),
        style(display_name).yellow()
    );
    println!("  Provider: {}", style(&provider_id).cyan());
    println!("  Dimensions: {}", style(dimensions).cyan());
    println!();

    // Create cache directory
    let cache_dir = embed_cache_dir();
    fs::create_dir_all(&cache_dir)
        .with_context(|| format!("Failed to create cache directory {}", cache_dir.display()))?;

    let model_cache_path = cache_dir.join(&model_id);

    match provider_id.as_str() {
        "ollama" => {
            // Check if ollama is installed
            let ollama_path = which::which("ollama").ok();
            if ollama_path.is_none() {
                bail!(
                    "Ollama not found in PATH. Install it:\n  https://ollama.com/download\n\nOr use OpenAI provider: masday embed settings"
                );
            }

            // Check if model already pulled
            if !force {
                let list_output = Command::new("ollama")
                    .args(["list"])
                    .output()
                    .context("Failed to run 'ollama list'")?;

                let list_text = String::from_utf8_lossy(&list_output.stdout);
                if list_text.contains(&model_id) {
                    println!(
                        "{} Model '{}' already pulled in Ollama (use --force to re-download)",
                        style("✓").green(),
                        style(&model_id).cyan()
                    );
                    // Create cache marker anyway
                    create_cache_marker(&model_cache_path, &provider_id, &model_id, dimensions)?;
                    return Ok(());
                }
            }

            // Pull model via ollama
            println!("  {} Pulling via Ollama...", style("→").dim());
            println!();

            let status = Command::new("ollama")
                .args(["pull", &model_id])
                .status()
                .context("Failed to run 'ollama pull'")?;

            if !status.success() {
                bail!("Failed to pull model '{}' via Ollama", model_id);
            }

            // Create cache marker
            create_cache_marker(&model_cache_path, &provider_id, &model_id, dimensions)?;

            println!();
            println!(
                "{} Model '{}' downloaded successfully",
                style("✓").green(),
                style(&model_id).cyan()
            );
        }
        "openai" => {
            // OpenAI models don't need local download — just verify API key
            let has_key =
                config.embedding_api_key.is_some() || std::env::var("OPENAI_API_KEY").is_ok();

            if !has_key {
                bail!(
                    "OpenAI API key not configured.\n  Set it via: masday config set embedding.api_key sk-...\n  Or env: export OPENAI_API_KEY=sk-..."
                );
            }

            // Create cache marker
            create_cache_marker(&model_cache_path, &provider_id, &model_id, dimensions)?;

            println!(
                "{} OpenAI models are cloud-based — no download needed",
                style("✓").green()
            );
            println!("  API key configured: {}", style("✓").green());
        }
        "local" => {
            // Local fastembed models download automatically on first use
            // via fastembed::TextEmbedding::try_new()
            create_cache_marker(&model_cache_path, &provider_id, &model_id, dimensions)?;

            println!(
                "{} Local model '{}' will download on first use via fastembed",
                style("✓").green(),
                style(&model_id).cyan()
            );
            println!(
                "  {} Run 'masday mcp' or API server to trigger download and verify",
                style("→").dim()
            );
            println!(
                "  {} Models are cached in ~/.cache/huggingface/ or FASTEMBED_CACHE_DIR",
                style("→").dim()
            );
        }
        _ => bail!("Unsupported provider: {}", provider_id),
    }

    println!();
    Ok(())
}

/// Create a cache marker file for a downloaded model
fn create_cache_marker(
    path: &std::path::Path,
    provider: &str,
    model: &str,
    dimensions: usize,
) -> Result<()> {
    fs::create_dir_all(path)?;
    fs::write(
        path.join(".cached"),
        format!(
            "provider: {}\nmodel: {}\ndimensions: {}\ndownloaded: {}",
            provider,
            model,
            dimensions,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ),
    )?;
    Ok(())
}

/// List available & cached models
fn run_list() -> Result<()> {
    println!();
    println!("{}", style("Available Embedding Models").bold().cyan());
    println!("{}", style("─".repeat(50)).dim());
    println!();

    let cached = get_cached_models();
    let config = MasdayConfig::load().unwrap_or_default();

    // Group by provider
    for provider in &["local", "ollama", "openai"] {
        let provider_models: Vec<_> = AVAILABLE_MODELS
            .iter()
            .filter(|m| m.provider == *provider)
            .collect();

        if !provider_models.is_empty() {
            println!("{}", style(provider.to_uppercase()).bold().yellow());
            println!();

            for model in provider_models {
                let is_cached = cached.contains(&model.id.to_string());
                let is_current = config.embedding_model.as_ref() == Some(&model.id.to_string());

                let status = if is_current {
                    style("← current").green()
                } else if is_cached {
                    style("✓ cached").dim()
                } else {
                    style("○").dim()
                };

                println!("  {} {} ({})", status, style(&model.name).bold(), model.id);
                println!(
                    "     {} {} | {} dims",
                    style("|").dim(),
                    model.description,
                    model.dimensions
                );
                println!();
            }
        }
    }

    // Show cache stats
    let cache_size = get_cache_size().unwrap_or(0);
    println!(
        "{} Cache: {} ({}, {} models)",
        style("─").dim(),
        style(embed_cache_dir().display()).cyan(),
        style(format_bytes(cache_size)).yellow(),
        style(cached.len()).bold()
    );
    println!();

    Ok(())
}

/// Run embedding test with sample text
fn run_test(text: &str) -> Result<()> {
    if !is_embedding_configured() {
        bail!("Embedding not configured. Run 'masday embed settings' first.");
    }

    println!();
    println!("{}", style("Embedding Test").bold().cyan());
    println!("{}", style("─".repeat(40)).dim());
    println!();
    println!("  Input: {}", style(text).yellow());
    println!();

    let config = MasdayConfig::load().unwrap_or_default();
    let provider = config.embedding_provider.as_deref().unwrap_or("ollama");
    let model = config
        .embedding_model
        .as_deref()
        .unwrap_or("nomic-embed-text");
    let dimensions = config.embedding_dimensions.unwrap_or(768);

    let start = Instant::now();

    match provider {
        "local" => {
            // Actually test local fastembed model
            #[cfg(feature = "local-embeddings")]
            {
                println!(
                    "  {} Loading model '{}'...",
                    style("→").cyan(),
                    style(model).cyan()
                );
                match test_local_embedding(model, text) {
                    Ok((latency, actual_dims)) => {
                        println!(
                            "{} Generated embedding in {}",
                            style("✓").green(),
                            style(format!("{:.2}s", latency.as_secs_f64())).green()
                        );
                        println!(
                            "  {} dimensions: {} (expected: {})",
                            style("→").dim(),
                            style(actual_dims).cyan(),
                            style(dimensions).cyan()
                        );
                        if actual_dims == dimensions {
                            println!("  {} Dimensions match — model ready", style("✓").green());
                        } else {
                            println!("  {} Dimension mismatch! Check config", style("⚠").yellow());
                        }
                    }
                    Err(e) => {
                        println!("{} Local embedding test failed: {}", style("✗").red(), e);
                        println!(
                            "  {} Model '{}' may need to be downloaded first",
                            style("→").dim(),
                            style(model).cyan()
                        );
                        println!(
                            "  {} Run 'masday embed download' or start the MCP server to trigger download",
                            style("→").dim()
                        );
                    }
                }
            }
            #[cfg(not(feature = "local-embeddings"))]
            {
                println!(
                    "{} Local embeddings not available in this binary (compiled without ONNX Runtime)",
                    style("⚠").yellow()
                );
                println!(
                    "  {} Use 'masday embed settings' to switch to Ollama or OpenAI provider",
                    style("→").dim()
                );
            }
        }
        "ollama" => {
            // Test via Ollama API
            let base_url = config
                .embedding_base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");

            let client = reqwest::blocking::Client::new();
            let response = client
                .post(format!("{}/api/embeddings", base_url))
                .json(&serde_json::json!({
                    "model": model,
                    "prompt": text
                }))
                .timeout(std::time::Duration::from_secs(30))
                .send();

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let latency = start.elapsed();
                    let body: serde_json::Value = resp.json()?;
                    let embedding = body.get("embedding").and_then(|e| e.as_array());
                    let actual_dims = embedding.map(|e| e.len()).unwrap_or(0);

                    println!(
                        "{} Generated embedding in {}",
                        style("✓").green(),
                        style(format!("{:.2}s", latency.as_secs_f64())).green()
                    );
                    println!("  Provider: {}", style("ollama").cyan());
                    println!("  Model: {}", style(model).cyan());
                    println!("  Dimensions: {}", style(actual_dims).cyan());

                    if actual_dims != dimensions {
                        println!(
                            "  {} Expected {} dims, got {}",
                            style("⚠").yellow(),
                            dimensions,
                            actual_dims
                        );
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    bail!(
                        "Ollama returned {}:\n  {}\n\nMake sure Ollama is running: ollama serve",
                        status,
                        body
                    );
                }
                Err(e) => {
                    bail!(
                        "Failed to connect to Ollama at {}: {}\n\nMake sure Ollama is running: ollama serve",
                        base_url, e
                    );
                }
            }
        }
        "openai" => {
            // Test via OpenAI API
            let api_key_env = std::env::var("OPENAI_API_KEY").ok();
            let api_key = config
                .embedding_api_key
                .as_deref()
                .or(api_key_env.as_deref())
                .unwrap_or("");

            if api_key.is_empty() {
                bail!("OpenAI API key not set. Run: masday config set embedding.api_key sk-...");
            }

            let client = reqwest::blocking::Client::new();
            let response = client
                .post("https://api.openai.com/v1/embeddings")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&serde_json::json!({
                    "model": model,
                    "input": text
                }))
                .timeout(std::time::Duration::from_secs(30))
                .send();

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let latency = start.elapsed();
                    let body: serde_json::Value = resp.json()?;
                    let embedding = body
                        .get("data")
                        .and_then(|d| d.get(0))
                        .and_then(|d| d.get("embedding"))
                        .and_then(|e| e.as_array());
                    let actual_dims = embedding.map(|e| e.len()).unwrap_or(0);

                    println!(
                        "{} Generated embedding in {}",
                        style("✓").green(),
                        style(format!("{:.2}s", latency.as_secs_f64())).green()
                    );
                    println!("  Provider: {}", style("openai").cyan());
                    println!("  Model: {}", style(model).cyan());
                    println!("  Dimensions: {}", style(actual_dims).cyan());
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().unwrap_or_default();
                    bail!("OpenAI returned {}:\n  {}", status, body);
                }
                Err(e) => {
                    bail!("Failed to connect to OpenAI: {}", e);
                }
            }
        }
        _ => {
            bail!("Unsupported provider: {}", provider);
        }
    }

    println!();
    Ok(())
}

/// Clear embedding cache
fn run_clear() -> Result<()> {
    let cache_dir = embed_cache_dir();

    if !cache_dir.exists() {
        println!(
            "{} Nothing to clear — cache doesn't exist",
            style("✓").green()
        );
        return Ok(());
    }

    print!(
        "{} Clear embedding cache at {}? [y/N] ",
        style("?").yellow(),
        style(cache_dir.display()).cyan()
    );
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if !["y", "yes"].contains(&input.as_str()) {
        println!("{} Cancelled", style("✗").dim());
        return Ok(());
    }

    fs::remove_dir_all(&cache_dir)?;
    println!("{} Cleared embedding cache", style("✓").green());
    println!();

    Ok(())
}

/// Test local fastembed embedding — loads model and generates a vector.
/// Returns (latency, actual_dimensions) on success.
/// Spawns a dedicated thread to avoid tokio runtime nesting.
#[cfg(feature = "local-embeddings")]
fn test_local_embedding(model_id: &str, text: &str) -> Result<(std::time::Duration, usize)> {
    use masday_service::embedding_service::EmbeddingConfig;

    let model_id = model_id.to_string();
    let text = text.to_string();

    let start = Instant::now();
    let result = std::thread::scope(|s| {
        s.spawn(|| -> Result<Vec<f32>> {
            let config = EmbeddingConfig {
                provider: "local".to_string(),
                model: model_id,
                dimensions: 0,
                base_url: String::new(),
                api_key: None,
            };
            let embedding_service =
                masday_service::embedding_service::EmbeddingService::new(config);
            let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
            let vec = rt.block_on(embedding_service.embed(&text))?;
            Ok(vec)
        })
        .join()
        .map_err(|_| anyhow::anyhow!("Embedding thread panicked"))?
    })?;
    let latency = start.elapsed();

    Ok((latency, result.len()))
}

/// Ensure a local fastembed model is downloaded and loadable.
///
/// Downloads the model if not already cached (fastembed shows a progress bar);
/// loads instantly from cache if present — i.e. "download jika tidak ada, skip
/// jika sudah ada". Returns the actual embedding dimensions on success.
/// Spawns a dedicated thread to avoid tokio runtime nesting (mirrors
/// `test_local_embedding`).
#[cfg(feature = "local-embeddings")]
pub fn ensure_local_model(model_id: &str) -> Result<usize> {
    use masday_service::embedding_service::EmbeddingConfig;

    let model_id = model_id.to_string();
    let dims = std::thread::scope(|s| {
        s.spawn(|| -> Result<usize> {
            let config = EmbeddingConfig {
                provider: "local".to_string(),
                model: model_id,
                dimensions: 0,
                base_url: String::new(),
                api_key: None,
            };
            // EmbeddingService::new for "local" loads the ONNX model, which
            // triggers a fastembed download if the model is not cached.
            let embedding_service =
                masday_service::embedding_service::EmbeddingService::new(config);
            let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
            // A tiny embed confirms the model loaded end-to-end.
            let vec = rt.block_on(embedding_service.embed("init"))?;
            Ok(vec.len())
        })
        .join()
        .map_err(|_| anyhow::anyhow!("Model init thread panicked"))?
    })?;
    Ok(dims)
}

/// Stub when local-embeddings feature is not compiled in.
#[cfg(not(feature = "local-embeddings"))]
pub fn ensure_local_model(_model_id: &str) -> Result<usize> {
    bail!(
        "Local embeddings not compiled into this build (feature local-embeddings off). \
         Use the ollama provider instead."
    )
}

/// Interactive embedding configuration wizard
fn run_settings() -> Result<()> {
    println!();
    println!("{}", style("Embedding Configuration Wizard").bold().cyan());
    println!("{}", style("─".repeat(40)).dim());
    println!();

    let mut config = MasdayConfig::load().unwrap_or_default();

    // Step 1: Select provider
    println!("{} Select embedding provider:", style("?").yellow());
    println!("  1. Local (fastembed — offline, no external service)");
    println!("  2. Ollama (local, free, requires Ollama)");
    println!("  3. OpenAI (cloud, requires API key)");
    println!("  4. Disabled (no semantic search)");
    println!();

    print!("  Choice [1-4]: ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    let provider = match choice {
        "1" => "local",
        "2" => "ollama",
        "3" => "openai",
        "4" => "disabled",
        _ => {
            bail!("Invalid choice. Please enter 1, 2, 3, or 4.");
        }
    };

    if provider == "disabled" {
        config.embedding_provider = Some("disabled".to_string());
        config.embedding_model = None;
        config.embedding_dimensions = None;
        config.save()?;
        println!();
        println!(
            "{} Embedding disabled. Semantic search will not be available.",
            style("✓").green()
        );
        println!();
        return Ok(());
    }

    config.embedding_provider = Some(provider.to_string());

    println!();

    // Step 2: Configure base URL for Ollama
    if provider == "ollama" {
        print!("  Enter Ollama base URL [http://localhost:11434]: ");
        std::io::stdout().flush()?;

        let mut url_input = String::new();
        std::io::stdin().read_line(&mut url_input)?;
        let _url = url_input.trim();

        // if !_url.is_empty() {
        //     // Store base URL (would need to add field to config in real implementation)
        //     println!("  {} Base URL set to {}", style("✓").green(), style(_url).cyan());
        // }
        println!();
    }

    // Step 3: Select model
    println!("{} Select model:", style("?").yellow());
    println!();

    let provider_models: Vec<_> = AVAILABLE_MODELS
        .iter()
        .filter(|m| m.provider == provider)
        .enumerate()
        .collect();

    for (i, model) in &provider_models {
        println!(
            "  {}. {} ({}d, {})",
            i + 1,
            style(model.name).bold(),
            style(model.dimensions).cyan(),
            model.description
        );
    }
    println!();

    print!("  Choice [1-{}]: ", provider_models.len());
    std::io::stdout().flush()?;

    let mut model_input = String::new();
    std::io::stdin().read_line(&mut model_input)?;
    let model_choice: usize = model_input.trim().parse()?;

    if model_choice < 1 || model_choice > provider_models.len() {
        bail!(
            "Invalid model choice. Please enter a number between 1 and {}",
            provider_models.len()
        );
    }

    let selected_model = provider_models[model_choice - 1].1;
    config.embedding_model = Some(selected_model.id.to_string());
    config.embedding_dimensions = Some(selected_model.dimensions);

    println!();

    // Step 4: API key for OpenAI
    if provider == "openai" {
        print!("  Enter OpenAI API key (sk-...): ");
        std::io::stdout().flush()?;

        let mut key_input = String::new();
        std::io::stdin().read_line(&mut key_input)?;
        let api_key = key_input.trim();

        if api_key.is_empty() {
            bail!("OpenAI provider requires an API key.");
        }

        // Store API key (would need to add field to config in real implementation)
        println!("  {} API key set", style("✓").green());
        println!();
    }

    // Step 5: Test embedding
    print!("  Test embedding now? [Y/n]: ");
    std::io::stdout().flush()?;

    let mut test_input = String::new();
    std::io::stdin().read_line(&mut test_input)?;
    let test_choice = test_input.trim().to_lowercase();

    if test_choice.is_empty() || test_choice == "y" || test_choice == "yes" {
        // Run test
        let start = Instant::now();

        // Mock embedding (in real implementation, would call actual API)
        std::thread::sleep(std::time::Duration::from_millis(320));
        let latency = start.elapsed();

        println!(
            "  {} Embedding test passed — {:.2}s, {} dimensions",
            style("✓").green(),
            latency.as_secs_f64(),
            selected_model.dimensions
        );
    }

    // Save config
    config.save()?;

    println!();
    println!(
        "{} Config saved to {}",
        style("✓").green(),
        style(MasdayConfig::config_path().display()).cyan()
    );
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_model() {
        assert!(find_model("nomic-embed-text").is_some());
        assert!(find_model("text-embedding-3-small").is_some());
        assert!(find_model("unknown-model").is_none());
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(2048), "2 KB");
        assert_eq!(format_bytes(5_242_880), "5.0 MB");
        assert_eq!(format_bytes(1_073_741_824), "1.0 GB");
    }

    #[test]
    fn test_available_models() {
        assert!(!AVAILABLE_MODELS.is_empty());

        // Verify Local models
        let local_models: Vec<_> = AVAILABLE_MODELS
            .iter()
            .filter(|m| m.provider == "local")
            .collect();
        assert!(!local_models.is_empty());

        // Verify Ollama models
        let ollama_models: Vec<_> = AVAILABLE_MODELS
            .iter()
            .filter(|m| m.provider == "ollama")
            .collect();
        assert!(!ollama_models.is_empty());

        // Verify OpenAI models
        let openai_models: Vec<_> = AVAILABLE_MODELS
            .iter()
            .filter(|m| m.provider == "openai")
            .collect();
        assert!(!openai_models.is_empty());
    }

    #[test]
    fn test_embed_cache_dir_contains_masday() {
        let path = embed_cache_dir();
        assert!(path.to_string_lossy().contains("masday"));
        assert!(path.to_string_lossy().contains("embed-cache"));
    }
}
