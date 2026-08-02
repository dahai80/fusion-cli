use anyhow::Result;
use clap::Args;
use colored::*;

use crate::service::{ServiceUrls, mlx};

#[derive(Args, Clone)]
pub struct InferenceArgs {
    #[arg(short, long)]
    pub model: String,

    #[arg(long, default_value_t = 4096)]
    pub ctx: u32,

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
    println!(
        "{} Starting chat with {} (ctx={}, temp={})",
        "💬".bold(),
        model.cyan(),
        args.inference.ctx.to_string().cyan(),
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
            max_tokens: Some(args.inference.ctx),
            stream: None,
        };

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
        let result = call_fusion_mlx(&args.inference.model, &args.prompt, &args.inference).await?;
        println!("{}", result);
    } else {
        println!(
            "{} Running inference with {}...",
            "⚡".bold(),
            args.inference.model.cyan()
        );
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

pub async fn handle_embed(args: EmbedArgs) -> Result<()> {
    println!(
        "{} Generating embeddings with {}...",
        "📐".bold(),
        args.model.cyan()
    );

    let text = if let Some(t) = &args.text {
        t.clone()
    } else if let Some(dir) = &args.dir {
        let mut all_text = String::new();
        let path = std::path::Path::new(dir);
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file()
                    && let Ok(content) = std::fs::read_to_string(&path)
                {
                    all_text.push_str(&format!("\n--- {} ---\n{}", path.display(), content));
                }
            }
        }
        all_text
    } else {
        anyhow::bail!("Either --text or --dir is required");
    };

    println!(
        "  Input length: {} characters",
        text.len().to_string().cyan()
    );

    match mlx::create_embedding(&args.model, &text).await {
        Ok(embedding) => {
            let dims = embedding.len();
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
        max_tokens: Some(args.ctx),
        stream: None,
    };

    let response = mlx::chat_completion(&request).await?;
    Ok(response
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_else(|| "(no response)".to_string()))
}
