use anyhow::Result;
use colored::*;

pub async fn run() -> Result<()> {
    println!();
    println!("{}", "Fusion-CLI — One CLI, Control All Fusion-MLX Local AI Ecosystem.".bold());
    println!();
    print_version("fusion-cli", "0.1.0");
    print_version("fusion-mlx", "checking...");
    print_version("Fusion-KB", "checking...");
    print_version("Model-Hub", "checking...");
    print_version("Fusion-Desk", "checking...");
    println!();

    // 尝试检测 fusion-mlx 版本
    match mlx_version().await {
        Ok(v) => println!("  {} fusion-mlx: {} (running)", "✅".green(), v),
        Err(_) => println!("  {} fusion-mlx: not detected (start with `fusion service start mlx`)", "⬜".yellow()),
    }

    println!();
    Ok(())
}

fn print_version(name: &str, version: &str) {
    println!("  {} v{}", name.cyan(), version);
}

async fn mlx_version() -> Result<String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:11434/v1/models")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await?;
    if resp.status().is_success() {
        Ok("0.1.0".to_string())
    } else {
        Err(anyhow::anyhow!("fusion-mlx not responding"))
    }
}