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
        let mut config = crate::config::FusionConfig::default();
        // S1/P3-1 修复: init 生成随机 mlx api_key, 不用硬编码默认 fg-admin-key。
        // 落盘经 save_config → chmod 0600, 防止明文密钥 0644 世界可读。
        config.mlx.api_key = generate_api_key();
        crate::config::save_config(&config)?;
        println!(
            "    {} {} (0600, random api-key)",
            "✓".green(),
            config_path.display()
        );
    } else {
        println!("    {} {} (exists)", "·".dimmed(), config_path.display());
    }

    println!();
    println!("  Checking fusion-mlx...");
    let mlx_alive = crate::service::mlx::health_check().await.unwrap_or(false);
    if mlx_alive {
        println!(
            "    {} fusion-mlx reachable via gateway localhost:11432",
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

// 生成 32 字节随机 hex api_key。读 /dev/urandom (Unix 原生, 无需 rand 依赖)。
// 失败则回退到时间戳+进程 id 拼接 (不安全但保证可用, init 不会因密钥生成失败而中断)。
fn generate_api_key() -> String {
    #[cfg(unix)]
    {
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            let mut buf = [0u8; 32];
            if f.read_exact(&mut buf).is_ok() {
                return buf.iter().map(|b| format!("{:02x}", b)).collect();
            }
        }
    }
    let fallback = format!(
        "fk-{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        std::process::id()
    );
    tracing::warn!("urandom unavailable, using weak fallback api-key");
    fallback
}
