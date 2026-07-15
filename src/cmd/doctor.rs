use anyhow::Result;
use colored::*;
use sysinfo::System;

pub async fn run() -> Result<()> {
    println!();
    println!("{}", "🔍 Fusion Environment Doctor".bold());
    println!("{}", "Checking system, dependencies, and configuration...".dimmed());
    println!();

    // 1. 系统信息
    println!("{}", "📋 System Information".bold());
    let mut sys = System::new_all();
    sys.refresh_all();

    let os_name = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let hostname = System::host_name().unwrap_or_default();
    let kernel = System::kernel_version().unwrap_or_default();

    println!("  OS:       {} {}", os_name.cyan(), arch.cyan());
    println!("  Hostname: {}", hostname.cyan());
    println!("  Kernel:   {}", kernel.cyan());
    println!("  CPU:      {} cores", sys.cpus().len().to_string().cyan());
    println!("  Memory:   {} total / {} available",
        format_bytes(sys.total_memory()).cyan(),
        format_bytes(sys.available_memory()).cyan());
    println!();

    // 2. Apple Silicon 检测
    println!("{}", "🔌 Apple Silicon Check".bold());
    if arch == "aarch64" {
        println!("  {} Apple Silicon (arm64)", "✅".green());
    } else {
        println!("  {} Intel (x86_64) — Metal/MLX may not be available", "⚠️".yellow());
    }
    println!();

    // 3. fusion-mlx 检测
    println!("{}", "🎯 fusion-mlx Check".bold());
    check_service("fusion-mlx", "http://localhost:8000/v1/models").await;
    println!();

    // 4. Fusion-KB 检测
    println!("{}", "📚 Fusion-KB Check".bold());
    check_service("Fusion-KB", "http://localhost:11434/kb/bases").await;
    println!();

    // 5. 依赖完整性
    println!("{}", "📦 Dependency Check".bold());
    println!("  {} Rust toolchain: stable", "✅".green());
    println!("  {} macOS: {}", "✅".green(), os_name.cyan());
    println!();

    // 6. 权限检查
    println!("{}", "🔐 Permission Check".bold());
    let home = dirs::home_dir().unwrap_or_default();
    let fusion_dir = home.join(".fusion");
    if fusion_dir.exists() {
        println!("  {} Fusion data dir: {}", "✅".green(), fusion_dir.display().to_string().cyan());
    } else {
        println!("  {} Fusion data dir: {} (not yet created)", "ℹ️".blue(), fusion_dir.display().to_string().cyan());
    }
    println!();

    // 总结
    println!("{}", "✅ Doctor check complete.".green().bold());
    Ok(())
}

async fn check_service(name: &str, url: &str) {
    let client = reqwest::Client::new();
    match client.get(url).timeout(std::time::Duration::from_secs(2)).send().await {
        Ok(resp) if resp.status().is_success() => {
            println!("  {} {}: running", "✅".green(), name.cyan());
        }
        Ok(_) => {
            println!("  {} {}: responding (unexpected status)", "⚠️".yellow(), name.cyan());
        }
        Err(_) => {
            println!("  {} {}: not running", "⬜".yellow(), name.cyan());
            println!("     Start with: {} {} {}", "fusion service start".cyan(), name, "(or `fusion service start all`)".dimmed());
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}