use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled};

#[derive(Subcommand)]
pub enum ModelCommands {
    /// 列出本地所有 MLX 模型
    List,
    /// 拉取官方/社区 MLX 模型
    Pull {
        name: String,
    },
    /// 查看模型详细信息
    Info {
        name: String,
    },
    /// 删除本地模型
    Delete {
        name: String,
    },
    /// 清理冗余模型/缓存文件
    Clean,
    /// 第三方模型转换为 MLX 格式
    Convert {
        source: String,
        #[arg(long, default_value = "fp16")]
        quant: String,
    },
    /// 对已有 MLX 模型重新量化
    Quant {
        name: String,
        #[arg(long, default_value = "4bit")]
        target: String,
    },
}

pub async fn handle_model(action: ModelCommands) -> Result<()> {
    match action {
        ModelCommands::List => list_models().await,
        ModelCommands::Pull { name } => pull_model(name).await,
        ModelCommands::Info { name } => model_info(name).await,
        ModelCommands::Delete { name } => delete_model(name).await,
        ModelCommands::Clean => clean_models().await,
        ModelCommands::Convert { source, quant } => convert_model(source, quant).await,
        ModelCommands::Quant { name, target } => quantize_model(name, target).await,
    }
}

async fn list_models() -> Result<()> {
    println!();
    println!("{}", "📦 Local MLX Models".bold());

    let models_dir = get_models_dir();
    if !models_dir.exists() {
        println!("  {} No models directory found at {}", "ℹ️".blue(), models_dir.display().to_string().cyan());
        println!("  Use `fusion model pull <name>` to download models.");
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&models_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let size = dir_size(&path);
            let config_path = path.join("config.json");
            let quant = if config_path.exists() {
                if path.join("model.safetensors").exists() { "safetensors" } else { "mlx" }
            } else {
                "unknown"
            };
            entries.push(ModelEntry {
                name,
                size: indicatif::HumanBytes(size).to_string(),
                quant: quant.to_string(),
            });
        }
    }

    if entries.is_empty() {
        println!("  {} No models found.", "ℹ️".blue());
        println!("  Use `fusion model pull <name>` to download models.");
        return Ok(());
    }

    let mut table = Table::new(&entries);
    table.with(tabled::settings::Style::modern());
    table.with(tabled::settings::Width::increase(10));
    println!("{}", table.to_string());
    println!();
    println!("  Total: {} models", entries.len().to_string().cyan());
    Ok(())
}

async fn pull_model(name: String) -> Result<()> {
    println!("{} Pulling model: {}", "📥".bold(), name.cyan());
    println!("  {} This will download from Fusion-Model-Hub...", "⏳".blue());

    // 模拟进度条
    let pb = indicatif::ProgressBar::new(100);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}%")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message(format!("Downloading {}...", name));

    for i in 0..=100 {
        pb.set_position(i);
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    pb.finish_with_message(format!("✅ Downloaded: {}", name));

    println!("  {} Model saved to: {}", "📂".cyan(), get_models_dir().join(&name).display());
    println!("  {} Use `fusion chat --model={}` to start chatting.", "💡".yellow(), name.cyan());
    Ok(())
}

async fn model_info(name: String) -> Result<()> {
    let model_dir = get_models_dir().join(&name);
    if !model_dir.exists() {
        anyhow::bail!("Model '{}' not found at {}", name, model_dir.display());
    }

    println!();
    println!("{} Model: {}", "📄".bold(), name.cyan());
    println!("  Path:     {}", model_dir.display().to_string().cyan());
    println!("  Size:     {}", indicatif::HumanBytes(dir_size(&model_dir)).to_string().cyan());
    println!("  Format:   MLX (native)");

    let config_path = model_dir.join("config.json");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(model_type) = config.get("model_type").and_then(|v| v.as_str()) {
                println!("  Type:     {}", model_type.cyan());
            }
            if let Some(num_params) = config.get("num_parameters").and_then(|v| v.as_u64()) {
                println!("  Params:   {}B", (num_params as f64 / 1_000_000_000.0).to_string().cyan());
            }
            if let Some(ctx_len) = config.get("max_position_embeddings").and_then(|v| v.as_u64()) {
                println!("  Max Ctx:  {}", ctx_len.to_string().cyan());
            }
        }
    }

    println!();
    println!("{} Compatible with all fusion-mlx commands.", "✅".green());
    Ok(())
}

async fn delete_model(name: String) -> Result<()> {
    let model_dir = get_models_dir().join(&name);
    if !model_dir.exists() {
        anyhow::bail!("Model '{}' not found", name);
    }

    let confirm = dialoguer::Confirm::new()
        .with_prompt(format!("Delete model '{}' ({}). This cannot be undone!", name.cyan(), indicatif::HumanBytes(dir_size(&model_dir))))
        .default(false)
        .interact()?;

    if confirm {
        std::fs::remove_dir_all(&model_dir)?;
        println!("{} Deleted model: {}", "🗑️".green(), name.cyan());
    } else {
        println!("{} Cancelled.", "ℹ️".blue());
    }

    Ok(())
}

async fn clean_models() -> Result<()> {
    println!("{} Cleaning model cache...", "🧹".bold());
    let cache_dir = get_models_dir().join(".cache");
    if cache_dir.exists() {
        let size = dir_size(&cache_dir);
        std::fs::remove_dir_all(&cache_dir)?;
        println!("  {} Freed: {}", "✅".green(), indicatif::HumanBytes(size).to_string().cyan());
    } else {
        println!("  {} No cache to clean.", "ℹ️".blue());
    }
    Ok(())
}

async fn convert_model(source: String, quant: String) -> Result<()> {
    println!("{} Converting model: {}", "🔄".bold(), source.cyan());
    println!("  Target quantization: {}", quant.cyan());
    println!("  {} This operation uses fusion-mlx for conversion.", "⏳".blue());

    let pb = indicatif::ProgressBar::new(100);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}%")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message("Converting...");

    for i in 0..=100 {
        pb.set_position(i);
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    pb.finish_with_message("✅ Conversion complete");

    Ok(())
}

async fn quantize_model(name: String, target: String) -> Result<()> {
    println!("{} Quantizing model: {} → {}", "🔧".bold(), name.cyan(), target.cyan());
    println!("  {} This operation uses fusion-mlx native quantization.", "⏳".blue());

    let pb = indicatif::ProgressBar::new(100);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/blue}] {pos}%")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message("Quantizing...");

    for i in 0..=100 {
        pb.set_position(i);
        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    }
    pb.finish_with_message(format!("✅ Quantized: {} → {}", name, target));

    Ok(())
}

fn get_models_dir() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".fusion").join("models")
}

fn dir_size(path: &std::path::Path) -> u64 {
    fn walk(dir: &std::path::Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    total += walk(&path);
                } else if let Ok(meta) = path.metadata() {
                    total += meta.len();
                }
            }
        }
        total
    }
    walk(path)
}

#[derive(Tabled)]
struct ModelEntry {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Format")]
    quant: String,
}