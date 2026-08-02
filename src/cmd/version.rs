use crate::service::mlx;
use anyhow::Result;
use colored::*;

pub async fn run() -> Result<()> {
    println!();
    println!(
        "{}",
        "Fusion-CLI — One CLI, Control All Fusion-MLX Local AI Ecosystem.".bold()
    );
    println!();
    print_version("fusion-cli", env!("CARGO_PKG_VERSION"));
    print_version("fusion-mlx", "checking...");
    print_version("Fusion-KB", "checking...");
    print_version("Model-Hub", "checking...");
    print_version("Fusion-Desk", "checking...");
    println!();

    match mlx::health_check().await {
        Ok(true) => println!("  {} fusion-mlx: running", "✅".green()),
        _ => println!(
            "  {} fusion-mlx: not detected (start with `fusion service start mlx`)",
            "⬜".yellow()
        ),
    }

    println!();
    Ok(())
}

fn print_version(name: &str, version: &str) {
    println!("  {} v{}", name.cyan(), version);
}
