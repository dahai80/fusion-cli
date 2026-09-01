use anyhow::Result;
use clap::Args;
use colored::*;
use eventsource_stream::Eventsource;
use futures_util::StreamExt;
use tracing::info;

use crate::service::{ServiceUrls, mlx};

#[derive(Args, Clone)]
pub struct InferenceArgs {
    #[arg(short, long)]
    pub model: String,

    /// Maximum tokens to generate (generation length cap). --ctx is a legacy alias.
    #[arg(long, default_value_t = 2048)]
    pub max_tokens: u32,

    /// Legacy alias for --max-tokens (generation length, NOT context window size).
    #[arg(long)]
    pub ctx: Option<u32>,

    #[arg(long, default_value_t = 0.7)]
    pub temperature: f32,

    #[arg(long, default_value_t = 0.9)]
    pub top_p: f32,

    #[arg(long)]
    pub no_cache: bool,

    #[arg(long)]
    pub no_metal: bool,

    #[arg(short, long)]
    pub quiet: bool,

    #[arg(long, default_value_t = 120)]
    pub timeout: u64,

    #[arg(long, default_value_t = true)]
    pub stream: bool,

    #[arg(long, default_value_t = false)]
    pub no_stream: bool,
}

impl InferenceArgs {
    pub fn effective_max_tokens(&self) -> u32 {
        // --ctx is a legacy alias for --max-tokens; if both given, --ctx wins (back-compat).
        self.ctx.unwrap_or(self.max_tokens)
    }
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

    #[arg(short, long)]
    pub prompt: String,
}

#[derive(Args, Clone)]
pub struct CodeArgs {
    #[command(flatten)]
    pub inference: InferenceArgs,

    #[arg(short, long)]
    pub prompt: Option<String>,

    #[arg(short, long)]
    pub file: Option<String>,

    #[arg(long, default_value = "chat")]
    pub task: String,
}

#[derive(Args, Clone)]
pub struct EmbedArgs {
    #[arg(short, long)]
    pub text: Option<String>,

    #[arg(short, long)]
    pub dir: Option<String>,

    #[arg(short, long)]
    pub output: Option<String>,

    #[arg(short, long, default_value = "BGE-M3")]
    pub model: String,
}

pub async fn handle_chat(args: ChatArgs) -> Result<()> {
    let model = &args.inference.model;
    let urls = ServiceUrls::from_config();

    // 交互式 REPL 无法产出结构化 JSON 流，json 模式直接拒绝，避免 banner 污染。
    if crate::utils::output::is_json_mode() {
        anyhow::bail!(
            "chat is an interactive REPL and does not support --format=json. Use `fusion run --format=json` for one-shot JSON output."
        );
    }

    println!(
        "{} Starting chat with {} (max-tokens={}, temp={})",
        "💬".bold(),
        model.cyan(),
        args.inference.effective_max_tokens().to_string().cyan(),
        args.inference.temperature.to_string().cyan()
    );
    println!("{} Type your messages. Press Ctrl+C to exit.", "ℹ️".blue());
    println!(
        "{} All inference goes through fusion-mlx ({})",
        "🔌".dimmed(),
        urls.mlx_api()
    );
    println!();

    match mlx::health_check().await {
        Ok(true) => println!("  {} fusion-mlx connected", "✅".green()),
        _ => {
            println!(
                "  {} fusion-mlx not running. Start with: fusion service start mlx",
                "⬜".yellow()
            );
            return Ok(());
        }
    }
    println!();

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

        conversation.push(mlx::Message {
            role: "user".to_string(),
            content: input,
        });

        let request = mlx::InferenceRequest {
            model: model.clone(),
            messages: conversation.clone(),
            temperature: Some(args.inference.temperature),
            max_tokens: Some(args.inference.effective_max_tokens()),
            stream: Some(args.inference.stream),
        };

        let want_stream = args.inference.stream && !args.inference.no_stream;
        if want_stream {
            match stream_chat(&request).await {
                Ok(content) => {
                    conversation.push(mlx::Message {
                        role: "assistant".to_string(),
                        content: content.clone(),
                    });
                }
                Err(e) => {
                    info!(error = %e, "Stream failed, falling back to non-streaming");
                    match mlx::chat_completion(&request).await {
                        Ok(response) => {
                            if let Some(choice) = response.choices.first() {
                                let content =
                                    choice.message.content.as_deref().unwrap_or("(no response)");
                                println!("\n{} {}", "Assistant:".green().bold(), content);
                                conversation.push(mlx::Message {
                                    role: "assistant".to_string(),
                                    content: content.to_string(),
                                });
                            }
                        }
                        Err(e2) => println!("\n{} Error: {}", "❌".red(), e2),
                    }
                }
            }
        } else {
            match mlx::chat_completion(&request).await {
                Ok(response) => {
                    if let Some(choice) = response.choices.first() {
                        let content = choice.message.content.as_deref().unwrap_or("(no response)");
                        println!("\n{} {}", "Assistant:".green().bold(), content);
                        conversation.push(mlx::Message {
                            role: "assistant".to_string(),
                            content: content.to_string(),
                        });
                    }
                }
                Err(e) => println!("\n{} Error: {}", "❌".red(), e),
            }
        }
        println!();
    }

    println!("{} Chat ended.", "👋".yellow());
    Ok(())
}

async fn stream_chat(request: &mlx::InferenceRequest) -> Result<String> {
    let response = mlx::chat_completion_stream(request).await?;
    print!("{} ", "Assistant:".green().bold());
    let mut full_content = String::new();
    let mut stream = response.bytes_stream().eventsource();
    while let Some(event) = stream.next().await {
        match event {
            Ok(event) => {
                if event.data == "[DONE]" {
                    break;
                }
                if let Ok(chunk) = serde_json::from_str::<mlx::ChatChunk>(&event.data)
                    && let Some(choice) = chunk.choices.first()
                    && let Some(content) = &choice.delta.content
                {
                    print!("{}", content);
                    full_content.push_str(content);
                }
            }
            Err(e) => {
                info!(error = %e, "SSE stream error");
                break;
            }
        }
    }
    println!();
    Ok(full_content)
}

pub async fn handle_run(args: RunArgs) -> Result<()> {
    let result = call_fusion_mlx(&args.inference.model, &args.prompt, &args.inference).await?;

    if crate::utils::output::is_json_mode() {
        let payload = serde_json::json!({
            "model": args.inference.model,
            "prompt": args.prompt,
            "response": result,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if args.inference.quiet {
        println!("{}", result);
    } else {
        println!(
            "{} Running inference with {}...",
            "⚡".bold(),
            args.inference.model.cyan()
        );
        println!("  Prompt: {}", args.prompt.dimmed());
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
        format!(
            "Analyze the following code:\n\n```\n{}\n```\n\nTask: {}",
            content, args.task
        )
    } else {
        anyhow::bail!("Either --prompt or --file is required for code command");
    };

    if args.inference.quiet {
        let result = call_fusion_mlx(&args.inference.model, &prompt, &args.inference).await?;
        println!("{}", result);
    } else {
        println!(
            "{} Code analysis with {}...",
            "💻".bold(),
            args.inference.model.cyan()
        );
        let result = call_fusion_mlx(&args.inference.model, &prompt, &args.inference).await?;
        println!();
        println!("{}", result);
    }
    Ok(())
}

// 递归收集目录下文本文件内容, 累计不超过 max_bytes, 超出即 bail。
fn collect_dir_text(
    dir: &std::path::Path,
    out: &mut String,
    files_read: &mut u32,
    max_bytes: usize,
) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_dir_text(&path, out, files_read, max_bytes)?;
        } else if path.is_file() {
            if out.len() >= max_bytes {
                tracing::warn!(
                    dir = %dir.display(),
                    bytes = out.len(),
                    cap = max_bytes,
                    "embed --dir size cap reached, further files skipped"
                );
                anyhow::bail!(
                    "embedded dir text exceeds {} byte cap ({} files read); reduce directory size or batch manually",
                    max_bytes,
                    files_read
                );
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                out.push_str(&format!("\n--- {} ---\n{}", path.display(), content));
                *files_read += 1;
            }
        }
    }
    Ok(())
}

pub async fn handle_embed(args: EmbedArgs) -> Result<()> {
    let json_mode = crate::utils::output::is_json_mode();

    if !json_mode {
        println!(
            "{} Generating embeddings with {}...",
            "📐".bold(),
            args.model.cyan()
        );
    }

    let text = if let Some(t) = &args.text {
        t.clone()
    } else if let Some(dir) = &args.dir {
        const MAX_DIR_BYTES: usize = 512 * 1024;
        let root = std::path::Path::new(dir);
        if !root.is_dir() {
            anyhow::bail!("--dir '{}' is not a directory", dir);
        }
        let mut all_text = String::new();
        let mut files_read = 0u32;
        collect_dir_text(root, &mut all_text, &mut files_read, MAX_DIR_BYTES)?;
        if all_text.is_empty() {
            anyhow::bail!("--dir '{}' contained no readable text files", dir);
        }
        if !json_mode {
            println!(
                "  Read {} files ({} chars) from {}",
                files_read,
                all_text.len(),
                dir.cyan()
            );
        }
        all_text
    } else {
        anyhow::bail!("Either --text or --dir is required");
    };

    if !json_mode {
        println!(
            "  Input length: {} characters",
            text.len().to_string().cyan()
        );
    }

    match mlx::create_embedding(&args.model, &text).await {
        Ok(embedding) => {
            let dims = embedding.len();

            if json_mode {
                let payload = serde_json::json!({
                    "model": args.model,
                    "dimensions": dims,
                    "vector": embedding,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }

            println!(
                "  ✅ Embedding generated: {} dimensions",
                dims.to_string().cyan()
            );

            if let Some(output) = &args.output {
                let output_data = serde_json::json!({
                    "model": args.model,
                    "dimensions": dims,
                    "vector": embedding,
                });
                std::fs::write(output, serde_json::to_string_pretty(&output_data)?)?;
                println!("  💾 Saved to: {}", output.cyan());
            }
        }
        Err(e) => {
            if json_mode {
                let payload = serde_json::json!({
                    "error": e.to_string(),
                    "model": args.model,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }
            println!("  {} Error: {}", "❌".red(), e);
        }
    }

    Ok(())
}

async fn call_fusion_mlx(model: &str, prompt: &str, args: &InferenceArgs) -> Result<String> {
    let messages = vec![
        mlx::Message {
            role: "system".to_string(),
            content: "You are Fusion-CLI, a helpful AI assistant powered by fusion-mlx."
                .to_string(),
        },
        mlx::Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        },
    ];

    let request = mlx::InferenceRequest {
        model: model.to_string(),
        messages,
        temperature: Some(args.temperature),
        max_tokens: Some(args.effective_max_tokens()),
        stream: Some(args.stream),
    };

    if args.stream && !args.no_stream {
        match stream_chat(&request).await {
            Ok(content) => return Ok(content),
            Err(e) => {
                info!(error = %e, "Stream failed, falling back to non-streaming");
            }
        }
    }

    let response = mlx::chat_completion(&request).await?;
    Ok(response
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_else(|| "(no response)".to_string()))
}
