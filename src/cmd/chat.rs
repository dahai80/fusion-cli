use anyhow::Result;
use clap::Args;
use colored::*;

/// 通用推理参数（全局复用）
#[derive(Args, Clone)]
pub struct InferenceArgs {
    /// 模型名称
    #[arg(short, long)]
    pub model: String,

    /// 上下文长度
    #[arg(long, default_value_t = 4096)]
    pub ctx: u32,

    /// 温度系数
    #[arg(long, default_value_t = 0.7)]
    pub temperature: f32,

    /// Top-P 采样
    #[arg(long, default_value_t = 0.9)]
    pub top_p: f32,

    /// 关闭 KV Cache
    #[arg(long)]
    pub no_cache: bool,

    /// 仅使用 CPU
    #[arg(long)]
    pub no_metal: bool,

    /// 静默模式（仅输出结果）
    #[arg(short, long)]
    pub quiet: bool,

    /// 超时秒数
    #[arg(long, default_value_t = 120)]
    pub timeout: u64,
}

#[derive(Args, Clone)]
pub struct ChatArgs {
    #[command(flatten)]
    pub inference: InferenceArgs,
}

#[derive(Args, Clone)]
pub struct RunArgs {
    #[command(flatten)]
    pub inference: InferenceArgs,

    /// Prompt 内容
    #[arg(short, long)]
    pub prompt: String,
}

#[derive(Args, Clone)]
pub struct CodeArgs {
    #[command(flatten)]
    pub inference: InferenceArgs,

    /// Prompt 内容
    #[arg(short, long)]
    pub prompt: Option<String>,

    /// 分析本地代码文件
    #[arg(short, long)]
    pub file: Option<String>,

    /// 任务类型
    #[arg(long, default_value = "chat")]
    pub task: String,
}

#[derive(Args, Clone)]
pub struct EmbedArgs {
    /// 文本内容
    #[arg(short, long)]
    pub text: Option<String>,

    /// 批量文件目录
    #[arg(short, long)]
    pub dir: Option<String>,

    /// 输出文件
    #[arg(short, long)]
    pub output: Option<String>,

    /// 模型名称
    #[arg(short, long, default_value = "BGE-M3")]
    pub model: String,
}

pub async fn handle_chat(args: ChatArgs) -> Result<()> {
    let model = &args.inference.model;
    println!("{} Starting chat with {} (ctx={}, temp={})",
        "💬".bold(), model.cyan(), args.inference.ctx.to_string().cyan(), args.inference.temperature.to_string().cyan());
    println!("{} Type your messages. Press Ctrl+C to exit.", "ℹ️".blue());
    println!("{} All inference goes through fusion-mlx (http://localhost:11434/v1)", "🔌".dimmed());
    println!();

    // 验证 fusion-mlx
    let client = reqwest::Client::new();
    match client.get("http://localhost:11434/v1/models")
        .timeout(std::time::Duration::from_secs(2))
        .send().await
    {
        Ok(_) => println!("  {} fusion-mlx connected", "✅".green()),
        Err(_) => {
            println!("  {} fusion-mlx not running. Start with: fusion service start mlx", "⬜".yellow());
            return Ok(());
        }
    }
    println!();

    // 交互式对话循环
    let mut conversation = Vec::new();
    loop {
        let input: String = dialoguer::Input::new()
            .with_prompt("You".cyan().to_string())
            .interact_text()?;

        if input.trim().is_empty() {
            continue;
        }
        if input.trim().eq_ignore_ascii_case("exit") || input.trim().eq_ignore_ascii_case("quit") {
            break;
        }

        conversation.push(serde_json::json!({"role": "user", "content": input}));

        // 调用 fusion-mlx
        let payload = serde_json::json!({
            "model": model,
            "messages": conversation,
            "temperature": args.inference.temperature,
            "max_tokens": args.inference.ctx,
        });

        match client.post("http://localhost:11434/v1/chat/completions")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(args.inference.timeout))
            .send().await
        {
            Ok(resp) => {
                if let Ok(data) = resp.json::<serde_json::Value>().await {
                    if let Some(content) = data["choices"][0]["message"]["content"].as_str() {
                        println!("\n{} {}", "Assistant:".green().bold(), content);
                        conversation.push(serde_json::json!({"role": "assistant", "content": content}));
                    }
                }
            }
            Err(e) => {
                println!("\n{} Error: {}", "❌".red(), e);
            }
        }
        println!();
    }

    println!("{} Chat ended.", "👋".yellow());
    Ok(())
}

pub async fn handle_run(args: RunArgs) -> Result<()> {
    if args.inference.quiet {
        // 静默模式：仅输出结果
        let result = call_fusion_mlx(&args.inference.model, &args.prompt, &args.inference).await?;
        println!("{}", result);
    } else {
        println!("{} Running inference with {}...", "⚡".bold(), args.inference.model.cyan());
        println!("  Prompt: {}", args.prompt.dimmed());
        let result = call_fusion_mlx(&args.inference.model, &args.prompt, &args.inference).await?;
        println!();
        println!("{}", result);
    }
    Ok(())
}

pub async fn handle_code(args: CodeArgs) -> Result<()> {
    let prompt = if let Some(p) = &args.prompt {
        p.clone()
    } else if let Some(file) = &args.file {
        let content = std::fs::read_to_string(file)?;
        format!("Analyze the following code:\n\n```\n{}\n```\n\nTask: {}", content, args.task)
    } else {
        anyhow::bail!("Either --prompt or --file is required for code command");
    };

    if args.inference.quiet {
        let result = call_fusion_mlx(&args.inference.model, &prompt, &args.inference).await?;
        println!("{}", result);
    } else {
        println!("{} Code analysis with {}...", "💻".bold(), args.inference.model.cyan());
        let result = call_fusion_mlx(&args.inference.model, &prompt, &args.inference).await?;
        println!();
        println!("{}", result);
    }
    Ok(())
}

pub async fn handle_embed(args: EmbedArgs) -> Result<()> {
    println!("{} Generating embeddings with {}...", "📐".bold(), args.model.cyan());

    let text = if let Some(t) = &args.text {
        t.clone()
    } else if let Some(dir) = &args.dir {
        // 读取目录下所有文件
        let mut all_text = String::new();
        let path = std::path::Path::new(dir);
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        all_text.push_str(&format!("\n--- {} ---\n{}", path.display(), content));
                    }
                }
            }
        }
        all_text
    } else {
        anyhow::bail!("Either --text or --dir is required");
    };

    println!("  Input length: {} characters", text.len().to_string().cyan());

    // 调用 fusion-mlx embedding API
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "model": args.model,
        "input": text,
    });

    match client.post("http://localhost:11434/v1/embeddings")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send().await
    {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                let dims = data["data"][0]["embedding"].as_array().map(|a| a.len()).unwrap_or(0);
                println!("  ✅ Embedding generated: {} dimensions", dims.to_string().cyan());

                if let Some(output) = &args.output {
                    let output_data = serde_json::json!({
                        "model": args.model,
                        "dimensions": dims,
                        "vector": data["data"][0]["embedding"],
                        "usage": data["usage"],
                    });
                    std::fs::write(output, serde_json::to_string_pretty(&output_data)?)?;
                    println!("  💾 Saved to: {}", output.cyan());
                }
            }
        }
        Err(e) => {
            println!("  {} Error: {}", "❌".red(), e);
        }
    }

    Ok(())
}

async fn call_fusion_mlx(model: &str, prompt: &str, args: &InferenceArgs) -> Result<String> {
    let client = reqwest::Client::new();

    let messages = vec![
        serde_json::json!({"role": "system", "content": "You are Fusion-CLI, a helpful AI assistant powered by fusion-mlx."}),
        serde_json::json!({"role": "user", "content": prompt}),
    ];

    let payload = serde_json::json!({
        "model": model,
        "messages": messages,
        "temperature": args.temperature,
        "max_tokens": args.ctx,
    });

    let resp = client.post("http://localhost:11434/v1/chat/completions")
        .json(&payload)
        .timeout(std::time::Duration::from_secs(args.timeout))
        .send().await?;

    let data: serde_json::Value = resp.json().await?;
    Ok(data["choices"][0]["message"]["content"].as_str().unwrap_or("(no response)").to_string())
}