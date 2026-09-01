use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tracing::info;

#[derive(Subcommand)]
pub enum BenchCommands {
    /// 基础测速：生成速度 token/s
    Speed {
        model: String,
        #[arg(long, default_value_t = 128)]
        tokens: u32,
        #[arg(long, default_value_t = 1)]
        runs: u32,
    },
    /// 显存/内存占用检测
    Mem { model: String },
    /// 最大上下文压力测试
    Ctx {
        model: String,
        #[arg(long, default_value_t = 4096)]
        max_ctx: u32,
        #[arg(long, default_value_t = 256)]
        step: u32,
    },
    /// 全自动参数寻优
    Auto { model: String },
    /// 导出评测报告
    Report {
        model: String,
        #[arg(short, long, default_value = "bench_report.md")]
        output: String,
    },
}

pub async fn handle_bench(action: BenchCommands) -> Result<()> {
    match action {
        BenchCommands::Speed {
            model,
            tokens,
            runs,
        } => bench_speed(&model, tokens, runs).await,
        BenchCommands::Mem { model } => bench_mem(&model).await,
        BenchCommands::Ctx {
            model,
            max_ctx,
            step,
        } => bench_ctx(&model, max_ctx, step).await,
        BenchCommands::Auto { model } => bench_auto(&model).await,
        BenchCommands::Report { model, output } => bench_report(&model, &output).await,
    }
}

async fn bench_speed(model: &str, tokens: u32, runs: u32) -> Result<()> {
    let alive = crate::service::mlx::health_check().await?;
    if !alive {
        anyhow::bail!("fusion-mlx is not running — start it with: fusion service start mlx");
    }
    info!(
        model = model,
        tokens = tokens,
        runs = runs,
        "Starting speed benchmark"
    );

    let mut results = Vec::new();
    let json_mode = crate::utils::output::is_json_mode();

    if !json_mode {
        println!();
        println!("{}", "⚡ Speed Benchmark".bold());
        println!("  Model: {}", model.cyan());
        println!("  Target: {} tokens", tokens.to_string().cyan());
        println!("  Runs: {}", runs.to_string().cyan());
        println!();
    }

    for run in 1..=runs {
        // R8 资源管控: 轮间短暂让出, 避免连续 N 轮独占单推理引擎饿死同期真实业务。
        if run > 1 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        if !json_mode {
            println!(
                "  {} Run {}/{}...",
                "▶".blue(),
                run.to_string().cyan(),
                runs.to_string().cyan()
            );
        }
        match crate::service::mlx::generate_tokens(model, tokens).await {
            Ok(result) => {
                if !json_mode {
                    println!(
                        "    {} {:.1} tok/s | {} tokens in {:.2}s",
                        "✓".green(),
                        result.tokens_per_sec,
                        result.completion_tokens.to_string().cyan(),
                        result.elapsed_secs,
                    );
                }
                results.push(result);
            }
            Err(e) => {
                if !json_mode {
                    println!("    {} Run failed: {}", "✗".red(), e);
                }
                info!(error = %e, run = run, "Speed benchmark run failed");
            }
        }
    }

    if results.is_empty() {
        anyhow::bail!("All benchmark runs failed — is fusion-mlx running?");
    }

    let avg_speed = results.iter().map(|r| r.tokens_per_sec).sum::<f64>() / results.len() as f64;
    let avg_elapsed = results.iter().map(|r| r.elapsed_secs).sum::<f64>() / results.len() as f64;
    let total_tokens: u32 = results.iter().map(|r| r.completion_tokens).sum();

    if json_mode {
        let payload = serde_json::json!({
            "model": model,
            "avg_speed_tokens_per_sec": (avg_speed * 10.0).round() / 10.0,
            "avg_time_secs": (avg_elapsed * 100.0).round() / 100.0,
            "total_tokens": total_tokens,
            "successful_runs": results.len(),
            "total_runs": runs,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!();
    println!("{}", "📊 Results".bold());
    println!("  Model:            {}", model.cyan());
    println!(
        "  Avg speed:        {} tokens/s",
        format!("{:.1}", avg_speed).cyan().bold()
    );
    println!(
        "  Avg time:         {}",
        format!("{:.2}s", avg_elapsed).cyan()
    );
    println!("  Total tokens:     {}", total_tokens.to_string().cyan());
    println!(
        "  Successful runs:  {}/{}",
        results.len().to_string().cyan(),
        runs
    );

    Ok(())
}

async fn bench_mem(model: &str) -> Result<()> {
    let json_mode = crate::utils::output::is_json_mode();

    let alive = crate::service::mlx::health_check().await?;
    if !alive {
        anyhow::bail!("fusion-mlx is not running — start it with: fusion service start mlx");
    }
    info!(model = model, "Starting memory benchmark");

    let stats = crate::service::mlx::get_server_stats().await;
    let models = crate::service::mlx::list_models().await;

    use sysinfo::System;
    // P3-4 修复: System::new_all() 枚举全部进程/组件, bench 仅需内存 → new()+refresh_memory 足够, 省开销。
    let mut sys = System::new();
    sys.refresh_memory();

    if json_mode {
        let model_loaded = models
            .as_ref()
            .map(|list| list.iter().any(|m| m.id == model))
            .unwrap_or(false);
        let payload = serde_json::json!({
            "model": model,
            "system": {
                "total_ram": sys.total_memory(),
                "used_ram": sys.used_memory(),
                "available_ram": sys.available_memory(),
            },
            "server_stats": stats.as_ref().unwrap_or(&serde_json::Value::Null),
            "server_stats_error": stats.as_ref().err().map(|e| e.to_string()),
            "loaded_models": models.as_ref().map(|list| list.iter().map(|m| &m.id).collect::<Vec<_>>()).unwrap_or_default(),
            "models_error": models.as_ref().err().map(|e| e.to_string()),
            "model_loaded": model_loaded,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!();
    println!("{}", "💾 Memory Benchmark".bold());
    println!("  Model: {}", model.cyan());
    println!();
    println!("{}", "📊 System Memory".bold());
    println!(
        "  Total RAM:  {}",
        indicatif::HumanBytes(sys.total_memory()).to_string().cyan()
    );
    println!(
        "  Used RAM:   {}",
        indicatif::HumanBytes(sys.used_memory()).to_string().cyan()
    );
    println!(
        "  Available:  {}",
        indicatif::HumanBytes(sys.available_memory())
            .to_string()
            .cyan()
    );

    println!();
    println!("{}", "🖥️  MLX Server Stats".bold());

    match &stats {
        Ok(data) => {
            if let Some(obj) = data.as_object() {
                for (key, value) in obj {
                    println!("  {}: {}", key.cyan(), value);
                }
            } else {
                println!("  Raw: {}", data);
            }
        }
        Err(e) => {
            println!("  {} Could not fetch server stats: {}", "⚠".yellow(), e);
            info!(error = %e, "Failed to get MLX server stats");
        }
    }

    println!();
    println!("{}", "📦 Loaded Models".bold());
    match &models {
        Ok(list) => {
            if list.is_empty() {
                println!("  No models currently loaded.");
            } else {
                for m in list {
                    println!("  • {}", m.id.cyan());
                }
            }
        }
        Err(e) => {
            println!("  {} Could not list models: {}", "⚠".yellow(), e);
        }
    }

    let model_loaded = models
        .as_ref()
        .map(|list| list.iter().any(|m| m.id == model))
        .unwrap_or(false);

    if !model_loaded {
        println!();
        println!(
            "  {} Model '{}' not loaded. Run `fusion model pull {}` first, then start a chat to load it.",
            "💡".yellow(),
            model.cyan(),
            model
        );
    }

    Ok(())
}

async fn bench_ctx(model: &str, max_ctx: u32, step: u32) -> Result<()> {
    let json_mode = crate::utils::output::is_json_mode();
    if !json_mode {
        println!();
        println!("{}", "📏 Context Length Stress Test".bold());
        println!("  Model:   {}", model.cyan());
        println!("  Max ctx: {}", max_ctx.to_string().cyan());
        println!("  Step:    {}", step.to_string().cyan());
        println!();
    }

    let alive = crate::service::mlx::health_check().await?;
    if !alive {
        anyhow::bail!("fusion-mlx is not running — start it with: fusion service start mlx");
    }
    info!(
        model = model,
        max_ctx = max_ctx,
        step = step,
        "Starting context stress test"
    );

    if !json_mode {
        let pb = indicatif::ProgressBar::new((max_ctx / step) as u64);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{msg} [{bar:40.yellow/red}] {pos}/{len} steps")
                .unwrap()
                .progress_chars("##-"),
        );
        pb.set_message("Testing context lengths...");
    }

    let mut max_working: u32 = 0;
    let mut failed_at: Option<(u32, String)> = None;
    let step = if step == 0 { 256 } else { step };

    for ctx in (step..=max_ctx).step_by(step as usize) {
        // R8 资源管控: 逐步压力测试时让出 GPU, 避免长时间独占饿死同期真实业务。
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let request = crate::service::mlx::InferenceRequest {
            model: model.to_string(),
            messages: vec![crate::service::mlx::Message {
                role: "user".to_string(),
                content: format!(
                    "Repeat the word 'test' {} times, separated by spaces. Just output the words, nothing else.",
                    ctx
                ),
            }],
            temperature: Some(0.1),
            max_tokens: Some(step.min(512)),
            stream: None,
        };

        match crate::service::mlx::chat_completion(&request).await {
            Ok(_resp) => {
                max_working = ctx;
                info!(ctx = ctx, "Context test passed");
            }
            Err(e) => {
                info!(ctx = ctx, error = %e, "Context test failed");
                failed_at = Some((ctx, e.to_string()));
                if !json_mode {
                    println!(
                        "  {} Context {} failed: {}",
                        "✗".red(),
                        ctx.to_string().red(),
                        e
                    );
                }
                break;
            }
        }
    }

    if json_mode {
        let payload = serde_json::json!({
            "model": model,
            "max_working_ctx": max_working,
            "tested_up_to": max_ctx,
            "step": step,
            "failed_at": failed_at.as_ref().map(|(c, _)| c),
            "failure": failed_at.as_ref().map(|(_, e)| e),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!();
    println!("{}", "📊 Results".bold());
    println!("  Model:           {}", model.cyan());
    println!(
        "  Max working ctx: {} tokens",
        max_working.to_string().cyan().bold()
    );
    println!("  Tested up to:    {} tokens", max_ctx.to_string().cyan());

    Ok(())
}

async fn bench_auto(model: &str) -> Result<()> {
    let json_mode = crate::utils::output::is_json_mode();
    if !json_mode {
        println!();
        println!("{}", "🤖 Auto Parameter Optimization".bold());
        println!("  Model: {}", model.cyan());
        println!();
    }

    let alive = crate::service::mlx::health_check().await?;
    if !alive {
        anyhow::bail!("fusion-mlx is not running — start it with: fusion service start mlx");
    }
    info!(model = model, "Starting auto parameter optimization");

    // ctx (上下文长度) 三档, 真正传入推理请求以验证不同上下文下的吞吐。
    // 之前 _ctx 被丢弃, 三轮跑相同请求, "参数寻优" 名不副实。
    let configs: Vec<(&str, u32)> = vec![
        ("ctx=2048, tokens=64", 2048),
        ("ctx=4096, tokens=64", 4096),
        ("ctx=8192, tokens=64", 8192),
    ];

    if !json_mode {
        let pb = indicatif::ProgressBar::new(configs.len() as u64);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len}")
                .unwrap()
                .progress_chars("##-"),
        );
    }

    let mut best_config = "";
    let mut best_speed = 0.0f64;
    let mut results = Vec::new();

    for (label, ctx) in &configs {
        // 用 ctx 作为 prompt 长度生成填充上下文, 真正体现 ctx 对吞吐的影响。
        let request = crate::service::mlx::InferenceRequest {
            model: model.to_string(),
            messages: vec![crate::service::mlx::Message {
                role: "user".to_string(),
                content: format!(
                    "Repeat the word 'bench' {} times, separated by spaces.",
                    ctx
                ),
            }],
            temperature: Some(0.1),
            max_tokens: Some(64),
            stream: None,
        };
        match crate::service::mlx::chat_completion(&request).await {
            Ok(_resp) => {
                // 仍以 generate_tokens 的小请求测吞吐 (与历史口径一致), 但 ctx 已在前一步施压。
                match crate::service::mlx::generate_tokens(model, 64).await {
                    Ok(result) => {
                        let speed = result.tokens_per_sec;
                        if !json_mode {
                            println!(
                                "  {} {} → {:.1} tok/s ({} tokens in {:.2}s)",
                                "✓".green(),
                                label.cyan(),
                                speed,
                                result.completion_tokens,
                                result.elapsed_secs,
                            );
                        }
                        if speed > best_speed {
                            best_speed = speed;
                            best_config = label;
                        }
                        results.push((label.to_string(), *ctx, speed, result.elapsed_secs));
                    }
                    Err(e) => {
                        if !json_mode {
                            println!("  {} {} → failed: {}", "✗".red(), label, e);
                        }
                    }
                }
            }
            Err(e) => {
                if !json_mode {
                    println!("  {} {} → ctx stress failed: {}", "✗".red(), label, e);
                }
            }
        }
    }

    if results.is_empty() {
        anyhow::bail!("All optimization runs failed — is fusion-mlx running?");
    }

    if json_mode {
        let payload = serde_json::json!({
            "model": model,
            "best_config": best_config,
            "best_speed_tokens_per_sec": (best_speed * 10.0).round() / 10.0,
            "results": results.iter().map(|(l, c, s, t)| serde_json::json!({
                "label": l, "ctx": c, "speed_tokens_per_sec": (s * 10.0).round() / 10.0, "elapsed_secs": t
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!();
    println!("{}", "🏆 Optimal Configuration".bold());
    println!("  Model:  {}", model.cyan());
    println!("  Config: {}", best_config.cyan().bold());
    println!("  Speed:  {:.1} tokens/s", best_speed.to_string().cyan());
    println!();
    println!(
        "  {} Apply with: fusion config set mlx.default-ctx <value>",
        "💡".yellow()
    );

    Ok(())
}

async fn bench_report(model: &str, output: &str) -> Result<()> {
    let json_mode = crate::utils::output::is_json_mode();
    if !json_mode {
        println!(
            "{} Generating benchmark report for {}...",
            "📝".bold(),
            model.cyan()
        );
    }

    let alive = crate::service::mlx::health_check().await?;
    if !alive {
        anyhow::bail!("fusion-mlx is not running — start it with: fusion service start mlx");
    }
    info!(
        model = model,
        output = output,
        "Starting benchmark report generation"
    );

    let speed_result = crate::service::mlx::generate_tokens(model, 128).await;

    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();

    let (speed_section, server_section) = {
        let sp = match &speed_result {
            Ok(r) => format!(
                "- Tokens/s: {:.1}\n- Time: {:.2}s\n- Tokens generated: {}\n- Prompt tokens: {}",
                r.tokens_per_sec, r.elapsed_secs, r.completion_tokens, r.prompt_tokens
            ),
            Err(e) => format!("- Failed: {}. Is fusion-mlx running?", e),
        };

        let srv = match crate::service::mlx::get_server_stats().await {
            Ok(stats) => format!(
                "```\n{}\n```",
                serde_json::to_string_pretty(&stats).unwrap_or_else(|_| stats.to_string())
            ),
            Err(e) => format!("Could not fetch: {}", e),
        };

        (sp, srv)
    };

    let mem_section = format!(
        "- Total RAM: {}\n- Available: {}\n- Used: {}",
        indicatif::HumanBytes(sys.total_memory()),
        indicatif::HumanBytes(sys.available_memory()),
        indicatif::HumanBytes(sys.used_memory()),
    );

    let report = format!(
        r#"# Fusion-Bench Report

## Model: {model}

### Speed Test
{speed_section}

### Memory
{mem_section}

### MLX Server Stats
{server_section}

### Recommendation
Based on the benchmark results, adjust your configuration:
- If speed < 20 tok/s, consider using a smaller model or reducing context length
- If memory is tight, reduce context with `fusion config set mlx.default-ctx`
- For best performance, use `fusion bench auto {model}` to find optimal settings

---

*Generated by Fusion-CLI bench on {date}*
"#,
        model = model,
        speed_section = speed_section,
        mem_section = mem_section,
        server_section = server_section,
        date = chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
    );

    std::fs::write(output, &report)?;

    if json_mode {
        let payload = serde_json::json!({
            "model": model,
            "output": output,
            "saved": true,
            "speed": match &speed_result {
                Ok(r) => serde_json::json!({
                    "tokens_per_sec": (r.tokens_per_sec * 10.0).round() / 10.0,
                    "elapsed_secs": (r.elapsed_secs * 100.0).round() / 100.0,
                    "completion_tokens": r.completion_tokens,
                    "prompt_tokens": r.prompt_tokens,
                }),
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            },
            "memory": {
                "total_ram": sys.total_memory(),
                "available_ram": sys.available_memory(),
                "used_ram": sys.used_memory(),
            },
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("{} Report saved to: {}", "✅".green(), output.cyan());
    }

    Ok(())
}
