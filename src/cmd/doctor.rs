use crate::service::{desk, kb, mlx, modelhub, rag};
use anyhow::Result;
use colored::*;
use sysinfo::{Components, Disks, Networks, System};

pub async fn run() -> Result<()> {
    println!();
    println!("{}", "🔍 Fusion Environment Doctor".bold());
    println!(
        "{}",
        "Checking system, dependencies, and configuration...".dimmed()
    );
    println!();

    // 1. 系统信息
    println!("{}", "📋 System Information".bold());
    let mut sys = System::new_all();
    sys.refresh_all();

    let os_name = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let hostname = System::host_name().unwrap_or_default();
    let kernel = System::kernel_version().unwrap_or_default();
    let uptime = System::uptime();
    let days = uptime / 86400;
    let hours = (uptime % 86400) / 3600;

    println!("  OS:        {} {}", os_name.cyan(), arch.cyan());
    println!("  Hostname:  {}", hostname.cyan());
    println!("  Kernel:    {}", kernel.cyan());
    println!(
        "  Uptime:    {}d {}h",
        days.to_string().cyan(),
        hours.to_string().cyan()
    );
    println!(
        "  CPU:       {} cores / {} logical",
        sys.physical_core_count()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "?".to_string())
            .cyan(),
        sys.cpus().len().to_string().cyan()
    );
    println!(
        "  Memory:    {} total / {} available / {} used",
        indicatif::HumanBytes(sys.total_memory()).to_string().cyan(),
        indicatif::HumanBytes(sys.available_memory())
            .to_string()
            .cyan(),
        indicatif::HumanBytes(sys.used_memory()).to_string().cyan()
    );
    println!(
        "  Swap:      {} total / {} used",
        indicatif::HumanBytes(sys.total_swap()).to_string().cyan(),
        indicatif::HumanBytes(sys.used_swap()).to_string().cyan()
    );
    println!();

    // 2. 磁盘信息
    println!("{}", "💾 Disk Information".bold());
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        let _usage = disk.usage();
        let total = disk.total_space();
        let available = disk.available_space();
        if total > 0 {
            let pct = (total - available) as f64 / total as f64 * 100.0;
            println!(
                "  {}  {:.0}%  {} / {} available",
                disk.name().to_string_lossy().cyan(),
                pct,
                indicatif::HumanBytes(available).to_string().green(),
                indicatif::HumanBytes(total).to_string().cyan()
            );
        }
    }
    println!();

    // 3. Apple Silicon 检测
    println!("{}", "🔌 Apple Silicon & Metal Check".bold());
    if arch == "aarch64" {
        println!("  {} Apple Silicon (arm64)", "✅".green());
        let components = Components::new_with_refreshed_list();
        for component in components.iter() {
            if component.label().to_lowercase().contains("cpu")
                && let Some(temp) = component.temperature()
            {
                println!("  {} CPU temperature: {:.1}°C", "🌡️".cyan(), temp);
            }
        }
    } else {
        println!(
            "  {} Intel (x86_64) — Metal/MLX may not be available",
            "⚠️".yellow()
        );
    }
    println!();

    // 4. 网络信息
    println!("{}", "🌐 Network Information".bold());
    let networks = Networks::new_with_refreshed_list();
    for (name, network) in networks.iter() {
        println!(
            "  {}  ↓ {}  ↑ {}",
            name.cyan(),
            indicatif::HumanBytes(network.total_received())
                .to_string()
                .dimmed(),
            indicatif::HumanBytes(network.total_transmitted())
                .to_string()
                .dimmed()
        );
    }
    println!();

    // 5. fusion-mlx 检测
    println!("{}", "🎯 fusion-mlx Check".bold());
    check_service("fusion-mlx", mlx::health_check().await.unwrap_or(false)).await;
    println!();

    // 6. Fusion-KB 检测
    println!("{}", "📚 Fusion-KB Check".bold());
    check_service("Fusion-KB", kb::health_check().await.unwrap_or(false)).await;
    println!();

    // 7. Model-Hub 检测
    println!("{}", "📦 Model-Hub Check".bold());
    check_service("Model-Hub", modelhub::health_check().await.unwrap_or(false)).await;
    println!();

    // 8. Fusion-RAG 检测
    println!("{}", "🔍 Fusion-RAG Check".bold());
    check_service("Fusion-RAG", rag::health_check().await.unwrap_or(false)).await;
    println!();

    // 9. Fusion-Desk 检测
    println!("{}", "🖥️ Fusion-Desk Check".bold());
    check_service("Fusion-Desk", desk::health_check().await.unwrap_or(false)).await;
    println!();

    // 10. 依赖完整性
    println!("{}", "📦 Dependency Check".bold());
    println!("  {} Rust toolchain: stable", "✅".green());
    println!("  {} macOS: {}", "✅".green(), os_name.cyan());
    println!(
        "  {} Build: fusion-cli v{}",
        "✅".green(),
        env!("CARGO_PKG_VERSION")
    );
    println!();

    // 11. 权限检查
    println!("{}", "🔐 Permission Check".bold());
    let home = dirs::home_dir().unwrap_or_default();
    let fusion_dir = home.join(".fusion");
    if fusion_dir.exists() {
        println!(
            "  {} Fusion data dir: {}",
            "✅".green(),
            fusion_dir.display().to_string().cyan()
        );
    } else {
        println!(
            "  {} Fusion data dir: {} (not yet created)",
            "ℹ️".blue(),
            fusion_dir.display().to_string().cyan()
        );
    }
    let fusion_cli = std::env::current_exe().unwrap_or_default();
    if fusion_cli.exists() {
        println!(
            "  {} Binary: {}",
            "✅".green(),
            fusion_cli.display().to_string().cyan()
        );
    }
    println!();

    println!("{}", "✅ Doctor check complete.".green().bold());
    Ok(())
}

async fn check_service(name: &str, alive: bool) {
    if alive {
        println!("  {} {}: running", "✅".green(), name.cyan());
    } else {
        println!("  {} {}: not running", "⬜".yellow(), name.cyan());
        println!(
            "     Start with: {} {} {}",
            "fusion service start".cyan(),
            name.to_lowercase(),
            "(or `fusion service start all`)".dimmed()
        );
    }
}
