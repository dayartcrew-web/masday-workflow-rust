//! Config command — view and manage ~/.masday/config.toml

use anyhow::{bail, Context, Result};
use console::style;
use std::env;
use std::io::Write;
use std::process::Command;

use crate::config::MasdayConfig;

/// Config subcommands
#[derive(Debug, clap::Subcommand)]
pub enum ConfigSubcommand {
    /// Print current config
    Show,
    /// Get a single config value
    Get {
        /// Config key (e.g., api_url, embedding.provider, ports.api_port)
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key (e.g., api_url, embedding.provider, ports.api_port)
        key: String,
        /// Config value
        value: String,
    },
    /// Open config in $EDITOR
    Edit,
    /// Reset to defaults (with confirmation)
    Reset,
    /// Print config file path
    Path,
}

/// Run the config command
pub async fn run(subcommand: ConfigSubcommand) -> Result<()> {
    match subcommand {
        ConfigSubcommand::Show => show_config()?,
        ConfigSubcommand::Get { key } => get_config(&key)?,
        ConfigSubcommand::Set { key, value } => set_config(&key, &value)?,
        ConfigSubcommand::Edit => edit_config()?,
        ConfigSubcommand::Reset => reset_config()?,
        ConfigSubcommand::Path => print_config_path()?,
    }
    Ok(())
}

/// Print the full config file contents
fn show_config() -> Result<()> {
    let path = MasdayConfig::config_path();

    if !path.exists() {
        bail!(
            "Config file not found: {}\nRun 'masday setup' to create it.",
            path.display()
        );
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config from {}", path.display()))?;

    println!("{}", content);
    Ok(())
}

/// Get a single config value by key
fn get_config(key: &str) -> Result<()> {
    if !MasdayConfig::exists() {
        bail!(
            "Config file not found. Run 'masday setup' to create it."
        );
    }

    let config = MasdayConfig::load_or_err()?;

    let value = match key {
        "mode" => config.mode,
        "api_url" => config.api_url,
        "api_key" => config.api_key,
        "database_url" => config.database_url.unwrap_or_else(|| "null".to_string()),
        "platforms" => config.platforms.join(", "),

        // Nested keys - embedding
        "embedding.provider" => config.embedding_provider.unwrap_or_else(|| "null".to_string()),
        "embedding.model" => config.embedding_model.unwrap_or_else(|| "null".to_string()),
        "embedding.dimensions" => config.embedding_dimensions.map(|d| d.to_string()).unwrap_or_else(|| "null".to_string()),
        "embedding.base_url" => config.embedding_base_url.unwrap_or_else(|| "null".to_string()),
        "embedding.api_key" => {
            // Mask API key for security
            match &config.embedding_api_key {
                Some(key) => {
                    if key.len() > 8 {
                        format!("{}...{}", &key[..4], &key[key.len()-4..])
                    } else {
                        "****".to_string()
                    }
                }
                None => "null".to_string(),
            }
        }

        // Nested keys - ports
        "ports.api_port" => config.api_port.to_string(),
        "ports.db_port" => config.db_port.to_string(),
        "ports.redis_port" => config.redis_port.to_string(),
        "ports.dashboard_port" => config.dashboard_port.to_string(),

        _ => bail!("Unknown config key: '{}'", key),
    };

    println!("{}", value);
    Ok(())
}

/// Set a config value by key
fn set_config(key: &str, value: &str) -> Result<()> {
    if !MasdayConfig::exists() {
        bail!(
            "Config file not found. Run 'masday setup' to create it."
        );
    }

    let mut config = MasdayConfig::load_or_err()?;

    match key {
        "mode" => {
            if !["local", "remote", "standalone"].contains(&value) {
                bail!("Invalid mode. Must be 'local', 'remote', or 'standalone'");
            }
            config.mode = value.to_string();
        }
        "api_url" => {
            config.api_url = value.to_string();
        }
        "api_key" => {
            config.api_key = value.to_string();
        }
        "database_url" => {
            config.database_url = if value == "null" {
                None
            } else {
                Some(value.to_string())
            };
        }
        "platforms" => {
            config.platforms = value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }

        // Nested keys - embedding
        "embedding.provider" => {
            config.embedding_provider = if value == "null" {
                None
            } else {
                Some(value.to_string())
            };
        }
        "embedding.model" => {
            config.embedding_model = if value == "null" {
                None
            } else {
                Some(value.to_string())
            };
        }
        "embedding.dimensions" => {
            config.embedding_dimensions = if value == "null" {
                None
            } else {
                Some(value.parse().context("Invalid dimensions value - must be a number")?)
            };
        }
        "embedding.base_url" => {
            config.embedding_base_url = if value == "null" {
                None
            } else {
                Some(value.to_string())
            };
        }
        "embedding.api_key" => {
            config.embedding_api_key = if value == "null" {
                None
            } else {
                Some(value.to_string())
            };
        }

        // Nested keys - ports
        "ports.api_port" => {
            config.api_port = value.parse().context("Invalid port value - must be a number")?;
        }
        "ports.db_port" => {
            config.db_port = value.parse().context("Invalid port value - must be a number")?;
        }
        "ports.redis_port" => {
            config.redis_port = value.parse().context("Invalid port value - must be a number")?;
        }
        "ports.dashboard_port" => {
            config.dashboard_port = value.parse().context("Invalid port value - must be a number")?;
        }

        _ => bail!("Unknown config key: '{}'", key),
    }

    config.save().with_context(|| format!("Failed to save config to {}", MasdayConfig::config_path().display()))?;
    println!("✓ Config updated: {} = {}", style(key).cyan(), style(value).green());
    Ok(())
}

/// Open config file in $EDITOR
fn edit_config() -> Result<()> {
    let path = MasdayConfig::config_path();

    if !path.exists() {
        bail!(
            "Config file not found: {}\nRun 'masday setup' to create it.",
            path.display()
        );
    }

    let editor = env::var("EDITOR").unwrap_or_else(|_| {
        if cfg!(windows) {
            "notepad".to_string()
        } else {
            "vi".to_string()
        }
    });

    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to open editor '{}'. Try: EDITOR=vim masday config edit", editor))?;

    if status.success() {
        println!("✓ Config file closed: {}", path.display());
    } else {
        bail!("Editor exited with non-zero status");
    }

    Ok(())
}

/// Reset config to defaults (with confirmation)
fn reset_config() -> Result<()> {
    if !MasdayConfig::exists() {
        bail!(
            "Config file not found. Run 'masday setup' to create it."
        );
    }

    print!("Are you sure you want to reset to defaults? [y/N] ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if !["y", "yes"].contains(&input.as_str()) {
        println!("Reset cancelled.");
        return Ok(());
    }

    let default_config = MasdayConfig::default();
    default_config.save().with_context(|| format!("Failed to save config to {}", MasdayConfig::config_path().display()))?;

    println!("✓ Config reset to defaults");
    Ok(())
}

/// Print the config file path
fn print_config_path() -> Result<()> {
    let path = MasdayConfig::config_path();
    println!("{}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_path_contains_masday() {
        let path = MasdayConfig::config_path();
        assert!(path.to_string_lossy().contains("masday"));
    }
}
