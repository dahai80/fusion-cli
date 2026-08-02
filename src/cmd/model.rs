use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled};
use tracing::{error, info};

use crate::service::{ServiceUrls, get_client};

#[derive(Subcommand)]
pub enum ModelCommands {
    /// 列出本地所有 MLX 模型
    List,
    /// 拉取官方/社区 MLX 模型
    Pull {
        name: String,
        #[arg(long, default_value = "https://hf-mirror.com")]
        mirror: String,
    },
    /// 查看模型详细信息
    Info { name: String },
    /// 删除本地模型
    Delete { name: String },
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
    /// 提交模型任务
    Submit {
        task: String,
        #[arg(long)]
        model_id: Option<String>,
    },
}

pub async fn handle_model(action: ModelCommands) -> Result<()> {
    match action {
        ModelCommands::List => list_models().await,
        ModelCommands::Pull { name, mirror } => pull_model(&name, &mirror).await,
        ModelCommands::Info { name } => model_info(&name).await,
        ModelCommands::Delete { name } => delete_model(&name).await,
        ModelCommands::Clean => clean_models().await,
        ModelCommands::Convert { source, quant } => convert_model(&source, &quant).await,
        ModelCommands::Quant { name, target } => quantize_model(&name, &target).await,
        ModelCommands::Submit { task, model_id } => submit_task(&task, model_id).await,
    }
}

async fn list_models() -> Result<()> {
    println!();
    println!("{}", "📦 Local MLX Models".bold());

    let models_dir = get_models_dir();
    if !models_dir.exists() {
        println!(
            "  {} No models directory found at {}",
            "ℹ️".blue(),
            models_dir.display().to_string().cyan()
        );
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
                if path.join("model.safetensors").exists() {
                    "safetensors"
                } else {
                    "mlx"
                }
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
    println!("{}", table);
    println!();
    println!("  Total: {} models", entries.len().to_string().cyan());
    Ok(())
}

async fn pull_model(name: &str, mirror: &str) -> Result<()> {
    println!("{} Pulling model: {}", "📥".bold(), name.cyan());

    let models_dir = get_models_dir();
    let target_dir = models_dir.join(name);

    if target_dir.exists() {
        println!(
            "  {} Model already exists at {}",
            "⚠".yellow(),
            target_dir.display()
        );
        let confirm = dialoguer::Confirm::new()
            .with_prompt("Re-download and overwrite?")
            .default(false)
            .interact()?;
        if !confirm {
            println!("  {} Cancelled.", "ℹ️".blue());
            return Ok(());
        }
    }

    println!("  Mirror: {}", mirror.cyan());
    println!();

    let hub_alive = crate::service::modelhub::health_check()
        .await
        .unwrap_or(false);
    if hub_alive {
        info!(model = name, "Attempting ModelHub download");
        println!("  {} Downloading via Fusion-Model-Hub...", "⏳".blue());
        match crate::service::modelhub::download_model(name).await {
            Ok(path) => {
                println!("  {} Downloaded to: {}", "✅".green(), path.cyan());
                println!(
                    "  {} Use `fusion chat --model={}` to start chatting.",
                    "💡".yellow(),
                    name.cyan()
                );
                return Ok(());
            }
            Err(e) => {
                println!("  {} ModelHub download failed: {}", "⚠".yellow(), e);
                info!(error = %e, "ModelHub download failed, falling back to huggingface-cli");
            }
        }
    }

    info!(
        model = name,
        mirror = mirror,
        "Downloading via huggingface-cli"
    );
    println!(
        "  {} Downloading via huggingface-cli (mirror: {})...",
        "⏳".blue(),
        mirror
    );

    let model_id = name;
    let hf_mirror = if mirror.is_empty() {
        "https://hf-mirror.com".to_string()
    } else {
        mirror.to_string()
    };

    let output = std::process::Command::new("huggingface-cli")
        .env("HF_ENDPOINT", &hf_mirror)
        .args([
            "download",
            model_id,
            "--local-dir",
            &target_dir.to_string_lossy(),
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            println!(
                "  {} Downloaded to: {}",
                "✅".green(),
                target_dir.display().to_string().cyan()
            );
            println!(
                "  {} Use `fusion chat --model={}` to start chatting.",
                "💡".yellow(),
                name.cyan()
            );
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            error!(stderr = %stderr, "huggingface-cli download failed");
            anyhow::bail!(
                "Download failed: huggingface-cli exited with code {:?}\n{}",
                out.status.code(),
                stderr
            );
        }
        Err(e) => {
            error!(error = %e, "huggingface-cli not found");
            anyhow::bail!(
                "huggingface-cli not found. Install with: pip install huggingface_hub\n\
                 Or start Fusion-Model-Hub for API-based downloads."
            );
        }
    }

    Ok(())
}

async fn model_info(name: &str) -> Result<()> {
    let model_dir = get_models_dir().join(name);
    if !model_dir.exists() {
        anyhow::bail!("Model '{}' not found at {}", name, model_dir.display());
    }

    println!();
    println!("{} Model: {}", "📄".bold(), name.cyan());
    println!("  Path:     {}", model_dir.display().to_string().cyan());
    println!(
        "  Size:     {}",
        indicatif::HumanBytes(dir_size(&model_dir))
            .to_string()
            .cyan()
    );
    println!("  Format:   MLX (native)");

    let config_path = model_dir.join("config.json");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(model_type) = config.get("model_type").and_then(|v| v.as_str()) {
                println!("  Type:     {}", model_type.cyan());
            }
            if let Some(num_params) = config.get("num_parameters").and_then(|v| v.as_u64()) {
                println!(
                    "  Params:   {}B",
                    (num_params as f64 / 1_000_000_000.0).to_string().cyan()
                );
            }
            if let Some(ctx_len) = config
                .get("max_position_embeddings")
                .and_then(|v| v.as_u64())
            {
                println!("  Max Ctx:  {}", ctx_len.to_string().cyan());
            }
        }
    }

    println!();
    println!("{} Compatible with all fusion-mlx commands.", "✅".green());
    Ok(())
}

async fn delete_model(name: &str) -> Result<()> {
    let model_dir = get_models_dir().join(name);
    if !model_dir.exists() {
        anyhow::bail!("Model '{}' not found", name);
    }

    let confirm = dialoguer::Confirm::new()
        .with_prompt(format!(
            "Delete model '{}' ({}). This cannot be undone!",
            name.cyan(),
            indicatif::HumanBytes(dir_size(&model_dir))
        ))
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
        println!(
            "  {} Freed: {}",
            "✅".green(),
            indicatif::HumanBytes(size).to_string().cyan()
        );
    } else {
        println!("  {} No cache to clean.", "ℹ️".blue());
    }
    Ok(())
}

async fn convert_model(source: &str, quant: &str) -> Result<()> {
    println!("{} Converting model: {}", "🔄".bold(), source.cyan());
    println!("  Target quantization: {}", quant.cyan());

    let mlx_path =
        std::path::PathBuf::from(std::env::var("FUSION_MLX_PATH").unwrap_or_else(|_| {
            dirs::home_dir()
                .map(|h| {
                    h.join("claude-home/fusion-mlx")
                        .to_string_lossy()
                        .to_string()
                })
                .unwrap_or_default()
        }));

    let convert_script = mlx_path.join("convert.py");
    if !convert_script.exists() {
        let fallback_script = mlx_path.join("scripts").join("convert.py");
        if !fallback_script.exists() {
            anyhow::bail!(
                "Conversion script not found at {} or {}.\n\
                 Set FUSION_MLX_PATH or install fusion-mlx.",
                convert_script.display(),
                fallback_script.display()
            );
        }
    }

    println!("  {} Running conversion via fusion-mlx...", "⏳".blue());
    info!(source = source, quant = quant, "Running model conversion");

    let output = std::process::Command::new("python3")
        .args([
            "-m",
            "mlx_lm.convert",
            "--hf-path",
            source,
            "--quantize",
            quant,
            "--mlx-path",
            &get_models_dir()
                .join(source.replace('/', "_"))
                .to_string_lossy(),
        ])
        .env(
            "HF_ENDPOINT",
            std::env::var("HF_ENDPOINT").unwrap_or_else(|_| "https://hf-mirror.com".to_string()),
        )
        .output();

    match output {
        Ok(out) if out.status.success() => {
            println!("  {} Conversion complete.", "✅".green());
            let stdout = String::from_utf8_lossy(&out.stdout);
            if !stdout.trim().is_empty() {
                println!("{}", stdout);
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            error!(stderr = %stderr, "Conversion failed");
            anyhow::bail!(
                "Conversion failed (exit {:?}):\n{}",
                out.status.code(),
                stderr
            );
        }
        Err(e) => {
            error!(error = %e, "Failed to run conversion");
            anyhow::bail!(
                "Failed to run mlx_lm.convert. Ensure mlx-lm is installed: pip install mlx-lm\n\
                 Error: {}",
                e
            );
        }
    }

    Ok(())
}

async fn quantize_model(name: &str, target: &str) -> Result<()> {
    println!(
        "{} Quantizing model: {} → {}",
        "🔧".bold(),
        name.cyan(),
        target.cyan()
    );

    let model_dir = get_models_dir().join(name);
    if !model_dir.exists() {
        anyhow::bail!(
            "Model '{}' not found at {}. Use `fusion model pull {}` first.",
            name,
            model_dir.display(),
            name
        );
    }

    let output_name = format!("{}-{}", name, target);
    let output_dir = get_models_dir().join(&output_name);

    println!("  {} Running quantization via mlx_lm...", "⏳".blue());
    info!(model = name, target = target, "Running model quantization");

    let output = std::process::Command::new("python3")
        .args([
            "-m",
            "mlx_lm.convert",
            "--hf-path",
            &model_dir.to_string_lossy(),
            "--quantize",
            target,
            "--mlx-path",
            &output_dir.to_string_lossy(),
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            println!(
                "  {} Quantized: {} → {}",
                "✅".green(),
                name.cyan(),
                output_name.cyan()
            );
            println!("  Output: {}", output_dir.display());
            let stdout = String::from_utf8_lossy(&out.stdout);
            if !stdout.trim().is_empty() {
                println!("{}", stdout);
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            error!(stderr = %stderr, "Quantization failed");
            anyhow::bail!(
                "Quantization failed (exit {:?}):\n{}",
                out.status.code(),
                stderr
            );
        }
        Err(e) => {
            error!(error = %e, "Failed to run quantization");
            anyhow::bail!(
                "Failed to run mlx_lm.convert. Ensure mlx-lm is installed: pip install mlx-lm\n\
                 Error: {}",
                e
            );
        }
    }

    Ok(())
}

async fn submit_task(task: &str, model_id: Option<String>) -> Result<()> {
    println!();
    println!("{}", "📤 Submit Model Task".bold());
    println!("  Task: {}", task.cyan());
    if let Some(ref mid) = model_id {
        println!("  Model ID: {}", mid.cyan());
    } else {
        println!("  Model ID: {} (default)", "auto".dimmed());
    }
    println!();

    let urls = ServiceUrls::from_config();
    let url = format!(
        "{}/api/models/tasks/submit",
        urls.modelhub.trim_end_matches('/')
    );
    info!(url = %url, task = %task, model_id = ?model_id, "Submitting model task");

    let mut payload = serde_json::json!({
        "task": task,
    });
    if let Some(mid) = &model_id {
        payload["model_id"] = serde_json::Value::String(mid.clone());
    }

    let client = get_client();
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await;

    match resp {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await?;
            info!(body = %body, "Task submitted successfully");
            println!("  {} Task submitted.", "✅".green());
            match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap_or(body)),
                Err(_) => println!("{}", body),
            }
        }
        Ok(resp) => {
            let status = resp.status();
            error!(status = %status, "Task submit failed");
            anyhow::bail!("Failed to submit task: HTTP {}", status);
        }
        Err(e) => {
            error!(error = %e, "Task submit connection error");
            anyhow::bail!("Failed to connect to fusion-mlx: {}", e);
        }
    }

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
