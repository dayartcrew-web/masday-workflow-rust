//! `masday embed` subcommand — manage local ONNX Runtime + embedding models
//!
//! Subcommands:
//!   masday embed setup    — Download ONNX Runtime + default model to ~/.masday/embeddings/
//!   masday embed status   — Show what's installed
//!   masday embed remove   — Delete cached embeddings
//!
//! Layout:
//!   ~/.masday/embeddings/
//!     onnxruntime.dll (Windows) / libonnxruntime.so (Linux) / libonnxruntime.dylib (macOS)
//!     models/
//!       BAAI__all-MiniLM-L6-v2/
//!         model.onnx, tokenizer.json, ...

use anyhow::{bail, Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;

/// Base directory for embedding artifacts
pub fn embeddings_dir() -> PathBuf {
    home::home_dir()
        .expect("No home directory")
        .join(".masday")
        .join("embeddings")
}

/// ONNX Runtime library filename per platform
fn ort_lib_filename() -> String {
    if cfg!(windows) {
        "onnxruntime.dll".to_string()
    } else if cfg!(target_os = "macos") {
        "libonnxruntime.dylib".to_string()
    } else {
        "libonnxruntime.so".to_string()
    }
}

/// Expected ONNX Runtime library path
pub fn ort_lib_path() -> PathBuf {
    embeddings_dir().join(ort_lib_filename())
}

/// Models cache directory
pub fn models_dir() -> PathBuf {
    embeddings_dir().join("models")
}

/// ONNX Runtime download URL (v1.21.0)
fn ort_download_url() -> String {
    let version = "1.21.0";
    if cfg!(windows) {
        format!("https://github.com/microsoft/onnxruntime/releases/download/v{}/onnxruntime-win-x64-{}.zip", version, version)
    } else if cfg!(target_os = "macos") {
        format!("https://github.com/microsoft/onnxruntime/releases/download/v{}/onnxruntime-osx-arm64-{}.tgz", version, version)
    } else {
        format!("https://github.com/microsoft/onnxruntime/releases/download/v{}/onnxruntime-linux-x64-{}.tgz", version, version)
    }
}

/// Check if ONNX Runtime is installed
pub fn is_ort_installed() -> bool {
    ort_lib_path().exists()
}

/// Check if models are cached
pub fn are_models_cached() -> bool {
    let models = models_dir();
    models.is_dir() && fs::read_dir(&models).is_ok_and(|mut d| d.next().is_some())
}

/// Download a file with progress bar
fn download_file(url: &str, dest: &std::path::Path, label: &str) -> Result<()> {
    let response = reqwest::blocking::Client::new()
        .get(url)
        .send()
        .with_context(|| format!("Failed to connect to {}", url))?;

    if !response.status().is_success() {
        bail!("Download failed: HTTP {}", response.status());
    }

    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(&format!(
            "{} {{msg}}\n  [{{elapsed_precise}}] [{{bar:40.cyan/blue}}] {{bytes}}/{{total_bytes}} ({{eta}})",
            style(label).cyan().bold()
        ))
        .unwrap()
        .progress_chars("#>-"),
    );

    let mut file =
        fs::File::create(dest).with_context(|| format!("Failed to create {}", dest.display()))?;

    // Stream the response body chunk-by-chunk to avoid loading the entire
    // file (ONNX Runtime archives are ~200 MB) into RAM at once.
    let mut downloaded: u64 = 0;
    let mut stream = response;
    let mut buf = [0u8; 8192];
    loop {
        use std::io::Read;
        let n = stream.read(&mut buf)?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n])?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message(format!("{} ✓ Downloaded", style(label).green()));
    Ok(())
}

/// Extract library from archive (zip on Windows, tgz on Linux/Mac)
fn extract_ort_from_archive(
    archive_path: &std::path::Path,
    dest_dir: &std::path::Path,
) -> Result<()> {
    let lib_name = ort_lib_filename();

    if cfg!(windows) {
        let file = fs::File::open(archive_path)?;
        let mut archive =
            zip::ZipArchive::new(file).with_context(|| "Failed to open zip archive")?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.ends_with(&lib_name) {
                let mut out = fs::File::create(dest_dir.join(&lib_name))?;
                std::io::copy(&mut entry, &mut out)?;
                println!("  {} Extracted {}", style("✓").green(), lib_name);
                return Ok(());
            }
        }
        bail!("{} not found in archive", lib_name);
    } else {
        let file = fs::File::open(archive_path)?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?.to_path_buf();
            let path_str = path.to_string_lossy();

            if path_str.ends_with(&lib_name) {
                let mut out = fs::File::create(dest_dir.join(&lib_name))?;
                std::io::copy(&mut entry, &mut out)?;
                println!("  {} Extracted {}", style("✓").green(), lib_name);
                return Ok(());
            }
        }
        bail!("{} not found in archive", lib_name);
    }
}

/// Download embedding model from HuggingFace using HTTP (no fastembed dep needed here)
fn download_model(model_name: &str, cache_dir: &std::path::Path) -> Result<()> {
    // HuggingFace model ID → URL mapping
    let (hf_id, files) = model_files(model_name);

    let model_dir = cache_dir.join(hf_id.replace('/', "__"));
    fs::create_dir_all(&model_dir)?;

    for (filename, url) in files {
        let dest = model_dir.join(filename);
        if dest.exists() {
            println!("  {} {} (cached)", style("●").dim(), filename);
            continue;
        }
        download_file(&url, &dest, &format!("Downloading {}", filename))?;
    }

    Ok(())
}

/// Standard files needed for a HuggingFace ONNX embedding model
const MODEL_FILES: &[&str] = &[
    "model.onnx",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
    "config.json",
];

/// Get model files list (name, download URL) for a given model
fn model_files(model_name: &str) -> (&'static str, Vec<(&'static str, String)>) {
    let base = "https://huggingface.co";
    let rev = "main";

    let hf_id = match model_name {
        "bge-small-en-v1.5" => "BAAI/bge-small-en-v1.5",
        "bge-base-en-v1.5" => "BAAI/bge-base-en-v1.5",
        // Default fallback for unknown names
        _ => "sentence-transformers/all-MiniLM-L6-v2",
    };

    let files: Vec<(&'static str, String)> = MODEL_FILES
        .iter()
        .map(|&f| (f, format!("{}/{}/resolve/{}/{}", base, hf_id, rev, f)))
        .collect();

    (hf_id, files)
}

// ─── CLI Entry Points ───────────────────────────────────────────────────

#[derive(clap::Subcommand)]
pub enum EmbedAction {
    /// Download ONNX Runtime + embedding model to ~/.masday/embeddings/
    Setup {
        /// Specific model to download (default: all-MiniLM-L6-v2)
        #[arg(long, default_value = "all-MiniLM-L6-v2")]
        model: String,
    },

    /// Show embedding setup status
    Status,

    /// Remove downloaded ONNX Runtime and models
    Remove {
        /// Remove only models, keep ONNX Runtime
        #[arg(long)]
        models_only: bool,
    },
}

pub fn run(action: EmbedAction) -> Result<()> {
    match action {
        EmbedAction::Setup { model } => run_setup(&model),
        EmbedAction::Status => run_status(),
        EmbedAction::Remove { models_only } => run_remove(models_only),
    }
}

fn run_setup(model: &str) -> Result<()> {
    let emb_dir = embeddings_dir();
    fs::create_dir_all(&emb_dir)
        .with_context(|| format!("Failed to create {}", emb_dir.display()))?;

    println!();
    println!("{}", style("Masday Local Embeddings Setup").bold().cyan());
    println!("{}", style("─".repeat(40)).dim());
    println!();

    // ── Step 1: Download ONNX Runtime ───────────────────────────────────
    if is_ort_installed() {
        println!("{} ONNX Runtime already installed", style("✓").green());
        println!("  {}", ort_lib_path().display());
    } else {
        println!("{}", style("Step 1: Downloading ONNX Runtime").bold());
        let url = ort_download_url();
        println!("  URL: {}", url);

        let ext = if cfg!(windows) { "zip" } else { "tgz" };
        let archive_path = emb_dir.join(format!("onnxruntime.{}", ext));

        let ort_result: Result<()> = (|| {
            download_file(&url, &archive_path, "Downloading ONNX Runtime")?;
            extract_ort_from_archive(&archive_path, &emb_dir)?;
            Ok(())
        })();

        // Always clean up archive, even on failure
        let _ = fs::remove_file(&archive_path);
        ort_result?;
    }

    println!();

    // ── Step 2: Download embedding model ────────────────────────────────
    let model_dir = models_dir();
    fs::create_dir_all(&model_dir)?;

    println!("{}", style("Step 2: Downloading embedding model").bold());
    println!("  Model: {}", style(model).yellow());
    println!("  Cache: {}", model_dir.display());
    println!();

    download_model(model, &model_dir)?;

    println!();

    // ── Done ────────────────────────────────────────────────────────────
    let emb_dir_str = emb_dir.to_string_lossy();
    let model_dir_str = model_dir.to_string_lossy();

    println!("{}", style("Setup complete!").bold().green());
    println!();
    println!("ONNX Runtime:  {}", ort_lib_path().display());
    println!("Models cache:  {}", model_dir.display());
    println!();

    // Print setup instructions
    println!(
        "{}",
        style("Add to your shell config (~/.bashrc / ~/.zshrc):").bold()
    );
    println!(
        "  {}{}",
        style("export FASTEMBED_CACHE_DIR=").cyan(),
        style(&*model_dir_str).yellow()
    );
    if cfg!(windows) {
        println!(
            "  {}{}",
            style("set PATH=").cyan(),
            style(format!("{};%PATH%", &*emb_dir_str)).yellow()
        );
    } else {
        println!(
            "  {}{}",
            style("export LD_LIBRARY_PATH=").cyan(),
            style(format!("{}:$LD_LIBRARY_PATH", &*emb_dir_str)).yellow()
        );
    }

    println!();
    println!("{}", style("Or update config.toml:").bold());
    println!("  {}", style("embedding_provider = \"local\"").cyan());
    println!(
        "  {}{}",
        style("embedding_model = \"").cyan(),
        style(format!("{}\"", model)).yellow()
    );
    println!();

    Ok(())
}

fn run_status() -> Result<()> {
    println!();
    println!("{}", style("Masday Local Embeddings Status").bold().cyan());
    println!("{}", style("─".repeat(40)).dim());
    println!();

    let emb_dir = embeddings_dir();
    println!("Embeddings dir: {}", emb_dir.display());

    if !emb_dir.exists() {
        println!(
            "  {} Not set up — run `masday embed setup`",
            style("✗").red()
        );
        return Ok(());
    }

    // Check ONNX Runtime
    let ort_path = ort_lib_path();
    if ort_path.exists() {
        let meta = fs::metadata(&ort_path)?;
        println!(
            "  {} ONNX Runtime: {} ({})",
            style("✓").green(),
            ort_path.display(),
            format_bytes(meta.len())
        );
    } else {
        println!("  {} ONNX Runtime: not installed", style("✗").red());
    }

    // Check models
    let mdir = models_dir();
    if mdir.is_dir() {
        let mut total_size: u64 = 0;
        let mut count = 0;
        if let Ok(entries) = fs::read_dir(&mdir) {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|t| t.is_dir()) {
                    count += 1;
                    if let Ok(size) = dir_size(&entry.path()) {
                        total_size += size;
                    }
                }
            }
        }
        if count > 0 {
            println!(
                "  {} Models: {} cached ({})",
                style("✓").green(),
                count,
                format_bytes(total_size)
            );
            if let Ok(entries) = fs::read_dir(&mdir) {
                for entry in entries.flatten() {
                    if entry.file_type().is_ok_and(|t| t.is_dir()) {
                        println!("    - {}", entry.file_name().to_string_lossy());
                    }
                }
            }
        } else {
            println!("  {} Models: none cached", style("⚠").yellow());
        }
    } else {
        println!("  {} Models: cache dir not found", style("✗").red());
    }

    // Check env
    if let Ok(cache) = std::env::var("FASTEMBED_CACHE_DIR") {
        println!("  {} FASTEMBED_CACHE_DIR={}", style("●").cyan(), cache);
    } else {
        println!("  {} FASTEMBED_CACHE_DIR not set", style("○").dim());
    }

    println!();
    Ok(())
}

fn run_remove(models_only: bool) -> Result<()> {
    let emb_dir = embeddings_dir();

    if !emb_dir.exists() {
        println!("Nothing to remove — {} doesn't exist", emb_dir.display());
        return Ok(());
    }

    if models_only {
        let mdir = models_dir();
        if mdir.exists() {
            fs::remove_dir_all(&mdir)?;
            println!("{} Removed model cache", style("✓").green());
        } else {
            println!("No model cache found");
        }
    } else {
        fs::remove_dir_all(&emb_dir)?;
        println!("{} Removed all embedding artifacts", style("✓").green());
    }

    Ok(())
}

// ─── Helpers ─────────────────────────────────────────────────────────────

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
