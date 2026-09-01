use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled};
use tracing::{error, info, warn};

use crate::service::{ServiceUrls, get_client};
use crate::utils::output::is_json_mode;

// 带超时运行外部进程, 防止网络卡死/磁盘满导致 CLI 永久阻塞。
// 返回 std::process::Output 以兼容现有 match 逻辑。
async fn run_with_timeout(
    program: &str,
    args: &[&str],
    envs: &[(&str, String)],
    timeout_secs: u64,
    label: &str,
) -> Result<std::process::Output> {
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let child = cmd
        .spawn()
        .map_err(|e| anyhow::anyhow!("spawn {} failed: {}", program, e))?;
    match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        child.wait_with_output(),
    )
    .await
    {
        Ok(o) => Ok(o?),
        Err(_) => {
            warn!(
                program = program,
                label = label,
                timeout_secs = timeout_secs,
                "external command timed out, killing"
            );
            anyhow::bail!("{} timed out after {}s", label, timeout_secs)
        }
    }
}

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
    let entries = collect_local_models()?;

    if crate::utils::output::is_json_mode() {
        let payload = serde_json::json!({
            "models": entries,
            "total": entries.len(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!();
    println!("{}", "📦 Local MLX Models".bold());

    if entries.is_empty() {
        let models_dir = get_models_dir();
        if !models_dir.exists() {
            println!(
                "  {} No models directory found at {}",
                "ℹ️".blue(),
                models_dir.display().to_string().cyan()
            );
        } else {
            println!("  {} No models found.", "ℹ️".blue());
        }
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

fn collect_local_models() -> Result<Vec<ModelEntry>> {
    let models_dir = get_models_dir();
    if !models_dir.exists() {
        return Ok(Vec::new());
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
    Ok(entries)
}

async fn pull_model(name: &str, mirror: &str) -> Result<()> {
    // 离线模式下，ModelHub 回退也要走外部网络，直接拒绝。
    if std::env::var("FUSION_OFFLINE").unwrap_or_else(|_| "1".to_string()) == "1" {
        anyhow::bail!(
            "Offline mode is ON (--offline). Model pull requires external network (huggingface/ModelHub).\n\
             Re-run with --offline=false to allow download."
        );
    }
    // name 可能是 HF repo id (含 '/', 如 "mlx-community/Qwen2-7B")。
    // 不能直接 join(name) — 会产生嵌套目录并允许 '../' 路径穿越逃出 models_dir。
    // 派生安全本地名: 替换 '/' 为 '_', 再校验, 确保 target_dir 始终在 models_dir 之内。
    let safe_name = name.replace('/', "_");
    validate_model_name(&safe_name)?;
    let json_mode = is_json_mode();
    if !json_mode {
        println!("{} Pulling model: {}", "📥".bold(), name.cyan());
    }

    let models_dir = get_models_dir();
    let target_dir = models_dir.join(&safe_name);
    // 二次校验: canonicalize 后必须仍以 models_dir 为前缀, 防止符号链接绕过。
    if target_dir.exists() {
        let canonical_target = target_dir
            .canonicalize()
            .unwrap_or_else(|_| target_dir.clone());
        let canonical_models = models_dir
            .canonicalize()
            .unwrap_or_else(|_| models_dir.clone());
        if !canonical_target.starts_with(&canonical_models) {
            anyhow::bail!(
                "Refusing pull: resolved path '{}' escapes models directory '{}'",
                canonical_target.display(),
                canonical_models.display()
            );
        }
    }

    if target_dir.exists() {
        let confirm = if json_mode {
            true
        } else {
            println!(
                "  {} Model already exists at {}",
                "⚠".yellow(),
                target_dir.display()
            );
            dialoguer::Confirm::new()
                .with_prompt("Re-download and overwrite?")
                .default(false)
                .interact()?
        };
        if !confirm {
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "pulled": false, "model": name, "reason": "already exists, cancelled" })
                    )?
                );
            } else {
                println!("  {} Cancelled.", "ℹ️".blue());
            }
            return Ok(());
        }
    }

    if !json_mode {
        println!("  Mirror: {}", mirror.cyan());
        println!();
    }

    let hub_alive = crate::service::modelhub::health_check()
        .await
        .unwrap_or(false);
    if hub_alive {
        info!(model = name, "Attempting ModelHub download");
        if !json_mode {
            println!("  {} Downloading via Fusion-Model-Hub...", "⏳".blue());
        }
        match crate::service::modelhub::download_model(name).await {
            Ok(path) => {
                if json_mode {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &serde_json::json!({ "pulled": true, "model": name, "source": "modelhub", "path": path })
                        )?
                    );
                } else {
                    println!("  {} Downloaded to: {}", "✅".green(), path.cyan());
                    println!(
                        "  {} Use `fusion chat --model={}` to start chatting.",
                        "💡".yellow(),
                        name.cyan()
                    );
                }
                return Ok(());
            }
            Err(e) => {
                if !json_mode {
                    println!("  {} ModelHub download failed: {}", "⚠".yellow(), e);
                }
                info!(error = %e, "ModelHub download failed, falling back to huggingface-cli");
            }
        }
    }

    info!(
        model = name,
        mirror = mirror,
        "Downloading via huggingface-cli"
    );
    if !json_mode {
        println!(
            "  {} Downloading via huggingface-cli (mirror: {})...",
            "⏳".blue(),
            mirror
        );
    }

    let model_id = name;
    let hf_mirror = if mirror.is_empty() {
        "https://hf-mirror.com".to_string()
    } else {
        mirror.to_string()
    };

    let output = run_with_timeout(
        "huggingface-cli",
        &[
            "download",
            model_id,
            "--local-dir",
            &target_dir.to_string_lossy(),
        ],
        &[("HF_ENDPOINT", hf_mirror.clone())],
        3600,
        "huggingface-cli download",
    )
    .await;

    match output {
        Ok(out) if out.status.success() => {
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "pulled": true, "model": name, "source": "huggingface-cli", "path": target_dir.display().to_string() })
                    )?
                );
            } else {
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
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            error!(stderr = %stderr, "huggingface-cli download failed");
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "pulled": false, "model": name, "exit_code": out.status.code(), "stderr": stderr.trim() })
                    )?
                );
                return Ok(());
            }
            anyhow::bail!(
                "Download failed: huggingface-cli exited with code {:?}\n{}",
                out.status.code(),
                stderr
            );
        }
        Err(e) => {
            error!(error = %e, "huggingface-cli not found");
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "pulled": false, "model": name, "error": "huggingface-cli not found (pip install huggingface_hub)" })
                    )?
                );
                return Ok(());
            }
            anyhow::bail!(
                "huggingface-cli not found. Install with: pip install huggingface_hub\n\
                 Or start Fusion-Model-Hub for API-based downloads."
            );
        }
    }

    Ok(())
}

async fn model_info(name: &str) -> Result<()> {
    validate_model_name(name)?;
    let model_dir = get_models_dir().join(name);
    if !model_dir.exists() {
        anyhow::bail!("Model '{}' not found at {}", name, model_dir.display());
    }

    let size = dir_size(&model_dir);
    let mut info = serde_json::json!({
        "name": name,
        "path": model_dir.display().to_string(),
        "size_bytes": size,
        "format": "MLX (native)",
    });

    let config_path = model_dir.join("config.json");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(model_type) = config.get("model_type").and_then(|v| v.as_str()) {
                info["model_type"] = serde_json::Value::String(model_type.to_string());
            }
            if let Some(num_params) = config.get("num_parameters").and_then(|v| v.as_u64()) {
                info["params_billions"] = serde_json::json!(num_params as f64 / 1_000_000_000.0);
            }
            if let Some(ctx_len) = config
                .get("max_position_embeddings")
                .and_then(|v| v.as_u64())
            {
                info["max_context"] = serde_json::json!(ctx_len);
            }
        }
    }

    if is_json_mode() {
        println!("{}", serde_json::to_string_pretty(&info)?);
        return Ok(());
    }

    println!();
    println!("{} Model: {}", "📄".bold(), name.cyan());
    println!("  Path:     {}", model_dir.display().to_string().cyan());
    println!(
        "  Size:     {}",
        indicatif::HumanBytes(size).to_string().cyan()
    );
    println!("  Format:   MLX (native)");
    if let Some(t) = info.get("model_type").and_then(|v| v.as_str()) {
        println!("  Type:     {}", t.cyan());
    }
    if let Some(p) = info.get("params_billions").and_then(|v| v.as_f64()) {
        println!("  Params:   {}B", p.to_string().cyan());
    }
    if let Some(c) = info.get("max_context").and_then(|v| v.as_u64()) {
        println!("  Max Ctx:  {}", c.to_string().cyan());
    }
    println!();
    println!("{} Compatible with all fusion-mlx commands.", "✅".green());
    Ok(())
}

async fn delete_model(name: &str) -> Result<()> {
    validate_model_name(name)?;
    let model_dir = get_models_dir().join(name);
    if !model_dir.exists() {
        anyhow::bail!("Model '{}' not found", name);
    }

    let json_mode = is_json_mode();
    // json 模式为自动化场景，跳过交互确认直接删除（调用方应已自行确认）。
    let confirm = if json_mode {
        true
    } else {
        dialoguer::Confirm::new()
            .with_prompt(format!(
                "Delete model '{}' ({}). This cannot be undone!",
                name.cyan(),
                indicatif::HumanBytes(dir_size(&model_dir))
            ))
            .default(false)
            .interact()?
    };

    if confirm {
        std::fs::remove_dir_all(&model_dir)?;
        if json_mode {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({ "deleted": true, "model": name })
                )?
            );
        } else {
            println!("{} Deleted model: {}", "🗑️".green(), name.cyan());
        }
    } else if !json_mode {
        println!("{} Cancelled.", "ℹ️".blue());
    }

    Ok(())
}

async fn clean_models() -> Result<()> {
    let json_mode = is_json_mode();
    let cache_dir = get_models_dir().join(".cache");
    if cache_dir.exists() {
        let size = dir_size(&cache_dir);
        std::fs::remove_dir_all(&cache_dir)?;
        if json_mode {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({ "cleaned": true, "freed_bytes": size })
                )?
            );
        } else {
            println!("{} Cleaning model cache...", "🧹".bold());
            println!(
                "  {} Freed: {}",
                "✅".green(),
                indicatif::HumanBytes(size).to_string().cyan()
            );
        }
    } else if json_mode {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({ "cleaned": false, "freed_bytes": 0 })
            )?
        );
    } else {
        println!("{} Cleaning model cache...", "🧹".bold());
        println!("  {} No cache to clean.", "ℹ️".blue());
    }
    Ok(())
}

async fn convert_model(source: &str, quant: &str) -> Result<()> {
    // convert 通过 mlx_lm.convert 从 HF repo 拉取源模型，需外部网络 → 离线模式拒绝。
    if std::env::var("FUSION_OFFLINE").unwrap_or_else(|_| "1".to_string()) == "1" {
        anyhow::bail!(
            "Offline mode is ON (--offline). Convert pulls source from huggingface.\n\
             Re-run with --offline=false to allow."
        );
    }
    let json_mode = is_json_mode();
    if !json_mode {
        println!("{} Converting model: {}", "🔄".bold(), source.cyan());
    }
    // source for convert is an HF repo id (may contain '/' like "org/model"), not a local
    // path joined into models_dir — but sanitize the local output dir name we derive from it.
    let output_name = source.replace('/', "_");
    validate_model_name(&output_name)?;
    if !json_mode {
        println!("  Target quantization: {}", quant.cyan());
    }

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

    // E11 修复: 之前只检查 convert.py 是否存在却从不使用, 随后直接 python3 -m mlx_lm.convert。
    // 现在若找到脚本则真正用它, 否则回退到 mlx_lm 模块, 不再做脱钩的空检查。
    let convert_script = mlx_path.join("convert.py");
    let fallback_script = mlx_path.join("scripts").join("convert.py");
    let script_path = if convert_script.exists() {
        Some(convert_script)
    } else if fallback_script.exists() {
        Some(fallback_script)
    } else {
        None
    };

    if !json_mode {
        println!("  {} Running conversion via fusion-mlx...", "⏳".blue());
    }
    info!(source = source, quant = quant, "Running model conversion");

    let hf_endpoint =
        std::env::var("HF_ENDPOINT").unwrap_or_else(|_| "https://hf-mirror.com".to_string());
    let output = match &script_path {
        Some(path) => {
            run_with_timeout(
                "python3",
                &[
                    path.to_string_lossy().as_ref(),
                    "--hf-path",
                    source,
                    "--quantize",
                    quant,
                    "--mlx-path",
                    &get_models_dir().join(&output_name).to_string_lossy(),
                ],
                &[("HF_ENDPOINT", hf_endpoint)],
                1800,
                "convert.py",
            )
            .await
        }
        None => {
            // 无脚本 → 回退到已安装的 mlx_lm 模块 (官方推荐路径)。
            run_with_timeout(
                "python3",
                &[
                    "-m",
                    "mlx_lm.convert",
                    "--hf-path",
                    source,
                    "--quantize",
                    quant,
                    "--mlx-path",
                    &get_models_dir().join(&output_name).to_string_lossy(),
                ],
                &[("HF_ENDPOINT", hf_endpoint)],
                1800,
                "mlx_lm.convert",
            )
            .await
        }
    };

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "converted": true, "source": source, "quant": quant, "output": output_name, "stdout": stdout.trim() })
                    )?
                );
            } else {
                println!("  {} Conversion complete.", "✅".green());
                if !stdout.trim().is_empty() {
                    println!("{}", stdout);
                }
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            error!(stderr = %stderr, "Conversion failed");
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "converted": false, "exit_code": out.status.code(), "stderr": stderr.trim() })
                    )?
                );
                return Ok(());
            }
            anyhow::bail!(
                "Conversion failed (exit {:?}):\n{}",
                out.status.code(),
                stderr
            );
        }
        Err(e) => {
            error!(error = %e, "Failed to run conversion");
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "converted": false, "error": e.to_string() })
                    )?
                );
                return Ok(());
            }
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
    validate_model_name(name)?;
    let json_mode = is_json_mode();
    if !json_mode {
        println!(
            "{} Quantizing model: {} → {}",
            "🔧".bold(),
            name.cyan(),
            target.cyan()
        );
    }

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

    if !json_mode {
        println!("  {} Running quantization via mlx_lm...", "⏳".blue());
    }
    info!(model = name, target = target, "Running model quantization");

    let output = run_with_timeout(
        "python3",
        &[
            "-m",
            "mlx_lm.convert",
            "--hf-path",
            &model_dir.to_string_lossy(),
            "--quantize",
            target,
            "--mlx-path",
            &output_dir.to_string_lossy(),
        ],
        &[],
        1800,
        "mlx_lm.quantize",
    )
    .await;

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "quantized": true, "model": name, "target": target, "output": output_name, "output_dir": output_dir.display().to_string(), "stdout": stdout.trim() })
                    )?
                );
            } else {
                println!(
                    "  {} Quantized: {} → {}",
                    "✅".green(),
                    name.cyan(),
                    output_name.cyan()
                );
                println!("  Output: {}", output_dir.display());
                if !stdout.trim().is_empty() {
                    println!("{}", stdout);
                }
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            error!(stderr = %stderr, "Quantization failed");
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "quantized": false, "exit_code": out.status.code(), "stderr": stderr.trim() })
                    )?
                );
                return Ok(());
            }
            anyhow::bail!(
                "Quantization failed (exit {:?}):\n{}",
                out.status.code(),
                stderr
            );
        }
        Err(e) => {
            error!(error = %e, "Failed to run quantization");
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "quantized": false, "error": e.to_string() })
                    )?
                );
                return Ok(());
            }
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
    let json_mode = is_json_mode();
    if !json_mode {
        println!();
        println!("{}", "📤 Submit Model Task".bold());
        println!("  Task: {}", task.cyan());
        if let Some(ref mid) = model_id {
            println!("  Model ID: {}", mid.cyan());
        } else {
            println!("  Model ID: {} (default)", "auto".dimmed());
        }
        println!();
    }

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
            if json_mode {
                match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(val) => println!("{}", serde_json::to_string_pretty(&val)?),
                    Err(_) => println!("{}", body),
                }
            } else {
                println!("  {} Task submitted.", "✅".green());
                match serde_json::from_str::<serde_json::Value>(&body) {
                    Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap_or(body)),
                    Err(_) => println!("{}", body),
                }
            }
        }
        Ok(resp) => {
            let status = resp.status();
            error!(status = %status, "Task submit failed");
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "submitted": false, "status": status.as_u16() })
                    )?
                );
                return Ok(());
            }
            anyhow::bail!("Failed to submit task: HTTP {}", status);
        }
        Err(e) => {
            error!(error = %e, "Task submit connection error");
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "submitted": false, "error": e.to_string() })
                    )?
                );
                return Ok(());
            }
            anyhow::bail!("Failed to connect to fusion-mlx: {}", e);
        }
    }

    Ok(())
}

fn get_models_dir() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".fusion").join("models")
}

fn validate_model_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Model name cannot be empty");
    }
    if name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
        || name.contains("..")
        || name.starts_with('.')
    {
        anyhow::bail!(
            "Invalid model name '{}': must not contain path separators or traversal sequences",
            name
        );
    }
    Ok(())
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

#[derive(Tabled, serde::Serialize)]
struct ModelEntry {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Format")]
    quant: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // E1 回归: validate_model_name 必须拦死路径穿越, 否则 join(name) 逃出 models_dir。
    #[test]
    fn test_validate_model_name_rejects_traversal() {
        assert!(validate_model_name("../etc/passwd").is_err());
        assert!(validate_model_name("org/../../sensitive").is_err());
        assert!(validate_model_name("..").is_err());
        assert!(validate_model_name(".").is_err());
        assert!(validate_model_name("").is_err());
        assert!(validate_model_name("/").is_err());
        assert!(validate_model_name("\\").is_err());
        assert!(validate_model_name(".hidden").is_err());
    }

    #[test]
    fn test_validate_model_name_accepts_safe_ids() {
        assert!(validate_model_name("Qwen2-7B").is_ok());
        // pull_model 已把 '/' 替换为 '_', 校验对象是 safe_name。
        assert!(validate_model_name("mlx-community_Qwen2-7B").is_ok());
        assert!(validate_model_name("org_model-7b-4bit").is_ok());
    }

    // E1: pull_model 派生 safe_name 必须消除 '/' 的穿越能力。
    #[test]
    fn test_pull_safe_name_neutralizes_slash_traversal() {
        // 模拟 pull_model 中 name.replace('/', "_") 的防御逻辑。
        let evil = "org/../../sensitive";
        let safe = evil.replace('/', "_");
        // 派生后不再含 '/' → join 不会嵌套, 校验应拒绝 '..' 残留。
        assert!(!safe.contains('/'));
        assert!(
            validate_model_name(&safe).is_err(),
            "must still reject '..' residue: {}",
            safe
        );
    }

    #[test]
    fn test_pull_safe_name_normal_repo_id() {
        let repo = "mlx-community/Qwen2-7B";
        let safe = repo.replace('/', "_");
        assert_eq!(safe, "mlx-community_Qwen2-7B");
        assert!(validate_model_name(&safe).is_ok());
    }
}
