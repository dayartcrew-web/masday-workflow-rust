//! masday-api server entry point

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("masday_api=debug,tower_http=debug")
        .init();

    tracing::info!("Starting masday-api server");

    // Load embedding config from ~/.masday/config.toml into EMBEDDING_* env vars.
    // EmbeddingService::cached() reads env (OnceLock), so this must run before any
    // route touches it. Pre-existing env vars take precedence. Without this, memory
    // and code pgvector search silently fall back to text search (provider unset).
    load_embedding_config_from_file();

    let pool = masday_db::pool::init_pool_with_retry(3)
        .await
        .expect("Failed to create database pool");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(masday_core::constants::ports::API_PORT);

    masday_api::run(pool, port)
        .await
        .expect("API server failed");
}

/// Read embedding_* keys from ~/.masday/config.toml and set the matching
/// `EMBEDDING_*` env vars (if not already set). This wires the API's
/// `EmbeddingService` (env-based) to the config.toml the MCP already reads,
/// so both processes agree on provider/model/dimensions.
fn load_embedding_config_from_file() {
    let config_path = match std::env::var("HOME") {
        Ok(h) => std::path::PathBuf::from(h)
            .join(".masday")
            .join("config.toml"),
        Err(_) => return,
    };
    let Ok(content) = std::fs::read_to_string(&config_path) else {
        tracing::info!("No ~/.masday/config.toml found — embedding config from env only");
        return;
    };

    // config.toml key -> env var. Underscore/hyphen variants both accepted.
    let mappings: &[(&str, &str)] = &[
        ("embedding_provider", "EMBEDDING_PROVIDER"),
        ("embedding_model", "EMBEDDING_MODEL"),
        ("embedding_base_url", "EMBEDDING_BASE_URL"),
        ("embedding_api_key", "EMBEDDING_API_KEY"),
        ("embedding_dimensions", "EMBEDDING_DIMENSIONS"),
    ];

    let mut applied = 0usize;
    for (cfg_key, env_key) in mappings {
        if std::env::var(env_key).is_ok() {
            continue; // explicit env wins
        }
        if let Some(value) = read_key(&content, cfg_key) {
            std::env::set_var(env_key, &value);
            applied += 1;
        }
    }
    tracing::info!(
        "Loaded {} embedding config keys from ~/.masday/config.toml",
        applied
    );
}

/// Line-based key=value reader (mirrors masday-mcp pg::read_config_value).
/// Matches `key` and `key-with-hyphens`, strips quotes. Returns None if absent.
fn read_key(content: &str, key: &str) -> Option<String> {
    let key_alt = key.replace('_', "-");
    for line in content.lines() {
        let trimmed = line.trim();
        let rest = trimmed
            .strip_prefix(key)
            .or_else(|| trimmed.strip_prefix(&key_alt))?;
        let after = rest.trim_start();
        if !after.starts_with('=') {
            continue;
        }
        let value = after
            .trim_start_matches('=')
            .split('#')
            .next()?
            .trim()
            .trim_matches('"')
            .trim_matches('\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}
