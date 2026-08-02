use anyhow::Result;
use colored::*;
use tracing::info;

pub async fn run_init() -> Result<()> {
    println!();
    println!("{}", "🚀 Fusion-CLI Init".bold());
    println!();

    let home = dirs::home_dir().unwrap_or_default();
    let fusion_dir = home.join(".fusion");
    let models_dir = fusion_dir.join("models");
    let kb_dir = fusion_dir.join("kb");
    let logs_dir = fusion_dir.join("logs");
    let bin_dir = fusion_dir.join("bin");
    let run_dir = fusion_dir.join("run");
    let config_path = fusion_dir.join("config.toml");

    println!("  Creating directories...");
    for dir in [&models_dir, &kb_dir, &logs_dir, &bin_dir, &run_dir] {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
            println!("    {} {}", "✓".green(), dir.display());
        } else {
            println!("    {} {} (exists)", "·".dimmed(), dir.display());
        }
    }

    println!();
    println!("  Creating config...");
    if !config_path.exists() {
        let config = crate::config::FusionConfig::default();
        let toml_str = toml::to_string_pretty(&config)?;
        std::fs::write(&config_path, &toml_str)?;
        println!("    {} {}", "✓".green(), config_path.display());
    } else {
        println!("    {} {} (exists)", "·".dimmed(), config_path.display());
    }

    println!();
    println!("  Checking fusion-mlx...");
    let mlx_alive = crate::service::mlx::health_check().await.unwrap_or(false);
    if mlx_alive {
        println!(
            "    {} fusion-mlx is running on localhost:11434",
            "✓".green()
        );
    } else {
        println!("    {} fusion-mlx not running", "⚠".yellow());
        println!("      Start with: ~/claude-home/fusion-mlx/start.sh start");
    }

    println!();
    println!(
        "  {} Init complete! Run `fusion doctor` for a full health check.",
        "✅".green()
    );
    info!("Fusion-CLI init completed");

    Ok(())
}
