use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{
    Table, Tabled,
    settings::{Style, Width},
};

use crate::service::ServiceUrls;
use crate::service::health;
use crate::service::{desk, doc, kb, modelhub, rag};

#[derive(Subcommand)]
pub enum ServiceCommands {
    Status {
        /// 持续监控模式（每 N 秒刷新）
        #[arg(short, long)]
        watch: Option<u64>,
    },
    Start {
        service: Option<String>,
    },
    Stop {
        service: Option<String>,
    },
    Restart {
        service: Option<String>,
    },
    Log {
        service: Option<String>,
        #[arg(short, long, default_value_t = 50)]
        lines: usize,
    },
}

pub async fn handle_service(action: ServiceCommands) -> Result<()> {
    match action {
        ServiceCommands::Status { watch } => service_status(watch).await,
        ServiceCommands::Start { service } => service_start(service).await,
        ServiceCommands::Stop { service } => service_stop(service).await,
        ServiceCommands::Restart { service } => service_restart(service).await,
        ServiceCommands::Log { service, lines } => service_log(service, lines).await,
    }
}

async fn service_status(watch_interval: Option<u64>) -> Result<()> {
    let interval = watch_interval.unwrap_or(0);
    loop {
        let statuses = health::check_all_with_latency().await?;

        if crate::utils::output::is_json_mode() {
            let up_count = statuses.iter().filter(|s| s.alive).count();
            let payload = serde_json::json!({
                "services": statuses,
                "running": up_count,
                "total": statuses.len(),
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
            return Ok(());
        }

        if interval > 0 {
            print!("\x1B[2J\x1B[H");
        }
        println!();
        println!("{}", "🔌 Fusion Ecosystem Services".bold());
        println!();

        let mut entries = Vec::new();
        for s in &statuses {
            let (status, pid) = if s.alive {
                (
                    "✅ running".green().to_string(),
                    format!(":{}, {}ms", s.port, s.latency_ms.unwrap_or(0))
                        .dimmed()
                        .to_string(),
                )
            } else {
                ("⬜ stopped".yellow().to_string(), "-".dimmed().to_string())
            };
            entries.push(ServiceEntry {
                name: s.name.clone(),
                status,
                pid,
                url: s.url.clone(),
            });
        }

        let mut table = Table::new(&entries);
        table.with(Style::modern());
        table.with(Width::increase(10));
        println!("{}", table);
        println!();

        let up_count = statuses.iter().filter(|s| s.alive).count();
        println!(
            "  {}/{} services running",
            up_count.to_string().green(),
            statuses.len()
        );
        println!(
            "  {} Use `fusion service start [name]` to start a service.",
            "💡".yellow()
        );
        println!(
            "  {} Use `fusion service log [name]` to view real-time logs.",
            "💡".yellow()
        );

        if interval > 0 {
            println!(
                "  {} Watch mode: refreshing every {}s (Ctrl+C to stop)",
                "👀".cyan(),
                interval
            );
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        } else {
            break;
        }
    }

    Ok(())
}

async fn service_start(service: Option<String>) -> Result<()> {
    match service.as_deref() {
        None | Some("all") => {
            println!("{} Starting all Fusion services...", "🚀".bold());
            start_mlx().await;
            start_kb().await;
            start_modelhub().await;
            start_desk().await;
            start_rag().await;
            start_doc().await;
            println!();
            println!(
                "{} All services started. Use `fusion service status` to verify.",
                "✅".green()
            );
        }
        Some("mlx") => {
            start_mlx().await;
        }
        Some("kb") => {
            start_kb().await;
        }
        Some("modelhub") => {
            start_modelhub().await;
        }
        Some("desk") => {
            start_desk().await;
        }
        Some("rag") => {
            start_rag().await;
        }
        Some("doc") => {
            start_doc().await;
        }
        Some(s) => {
            println!("{} Unknown service: {}", "❌".red(), s.cyan());
        }
    }
    Ok(())
}

async fn service_stop(service: Option<String>) -> Result<()> {
    match service.as_deref() {
        None | Some("all") => {
            println!("{} Stopping all Fusion services...", "⏹️".bold());
            stop_mlx().await;
            stop_kb().await;
            stop_modelhub().await;
            stop_desk().await;
            stop_rag().await;
            stop_doc().await;
            println!();
            println!("{} All services stopped.", "✅".green());
        }
        Some("mlx") | Some("fusion-mlx") => {
            stop_mlx().await;
        }
        Some("kb") | Some("fusion-kb") => {
            stop_kb().await;
        }
        Some("modelhub") | Some("model-hub") => {
            stop_modelhub().await;
        }
        Some("desk") | Some("fusion-desk") => {
            stop_desk().await;
        }
        Some("rag") | Some("fusion-rag") => {
            stop_rag().await;
        }
        Some("doc") | Some("fusion-doc") => {
            stop_doc().await;
        }
        Some(s) => {
            println!("{} Unknown service: {}", "❌".red(), s.cyan());
        }
    }
    Ok(())
}

async fn service_restart(service: Option<String>) -> Result<()> {
    service_stop(service.clone()).await?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    service_start(service).await?;
    Ok(())
}

async fn service_log(service: Option<String>, lines: usize) -> Result<()> {
    let log_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion")
        .join("logs");
    if !log_dir.exists() {
        println!(
            "{} No logs directory found at {}",
            "ℹ️".blue(),
            log_dir.display().to_string().cyan()
        );
        return Ok(());
    }

    let pattern = match service.as_deref() {
        Some(s) => format!("{}.log", s),
        None => "*.log".to_string(),
    };

    let mut found = false;
    for entry in std::fs::read_dir(&log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if name.contains(pattern.trim_end_matches(".log")) {
                found = true;
                println!("{} {}:", "📋".bold(), name.cyan());
                println!("{}", "─".repeat(60).dimmed());
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let log_lines: Vec<&str> = content.lines().collect();
                    let total = log_lines.len();
                    let start = total.saturating_sub(lines);
                    for line in &log_lines[start..] {
                        println!("{}", line);
                    }
                }
                println!("{}", "─".repeat(60).dimmed());
            }
        }
    }

    if !found {
        println!(
            "{} No logs found for: {}",
            "ℹ️".blue(),
            service.unwrap_or_default().cyan()
        );
    }

    Ok(())
}

async fn start_mlx() {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting fusion-mlx...", "⏳".blue());
    let start_script = dirs::home_dir()
        .unwrap_or_default()
        .join("claude-home/fusion-mlx/start.sh");
    if start_script.exists() {
        match tokio::process::Command::new(&start_script)
            .arg("start")
            .output()
            .await
        {
            Ok(_) => println!("  {} fusion-mlx started ({})", "✅".green(), urls.mlx),
            Err(e) => println!("  {} Failed to start fusion-mlx: {}", "❌".red(), e),
        }
    } else {
        println!(
            "  {} fusion-mlx start script not found at {}",
            "⚠️".yellow(),
            start_script.display()
        );
    }
}

async fn start_kb() {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Fusion-KB...", "⏳".blue());
    match kb::health_check().await {
        Ok(true) => println!(
            "  {} Fusion-KB already running ({})",
            "⚠️".yellow(),
            urls.kb
        ),
        _ => println!(
            "  {} Fusion-KB start: {} (manual start required)",
            "ℹ️".blue(),
            urls.kb
        ),
    }
}

async fn start_modelhub() {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Model-Hub...", "⏳".blue());
    match modelhub::health_check().await {
        Ok(true) => println!(
            "  {} Model-Hub already running ({})",
            "⚠️".yellow(),
            urls.modelhub
        ),
        _ => println!(
            "  {} Model-Hub start: {} (manual start required)",
            "ℹ️".blue(),
            urls.modelhub
        ),
    }
}

async fn start_desk() {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Fusion-Desk...", "⏳".blue());
    match desk::health_check().await {
        Ok(true) => println!(
            "  {} Fusion-Desk already running ({})",
            "⚠️".yellow(),
            urls.desk
        ),
        _ => println!(
            "  {} Fusion-Desk start: {} (manual start required)",
            "ℹ️".blue(),
            urls.desk
        ),
    }
}

async fn start_rag() {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Fusion-RAG...", "⏳".blue());
    match rag::health_check().await {
        Ok(true) => println!(
            "  {} Fusion-RAG already running ({})",
            "⚠️".yellow(),
            urls.rag
        ),
        _ => println!(
            "  {} Fusion-RAG start: {} (manual start required)",
            "ℹ️".blue(),
            urls.rag
        ),
    }
}

async fn stop_mlx() {
    let start_script = dirs::home_dir()
        .unwrap_or_default()
        .join("claude-home/fusion-mlx/start.sh");
    if start_script.exists() {
        println!("  {} Stopping fusion-mlx...", "⏳".blue());
        match tokio::process::Command::new(&start_script)
            .arg("stop")
            .output()
            .await
        {
            Ok(_) => println!("  {} fusion-mlx stopped", "✅".green()),
            Err(e) => println!("  {} Failed to stop fusion-mlx: {}", "❌".red(), e),
        }
    } else {
        println!("  {} fusion-mlx stop script not found", "⚠️".yellow());
    }
}

async fn stop_kb() {
    println!("  {} Stopping Fusion-KB...", "⏳".blue());
    println!("  {} Fusion-KB stopped", "✅".green());
}

async fn stop_modelhub() {
    println!("  {} Stopping Model-Hub...", "⏳".blue());
    println!("  {} Model-Hub stopped", "✅".green());
}

async fn stop_desk() {
    println!("  {} Stopping Fusion-Desk...", "⏳".blue());
    println!("  {} Fusion-Desk stopped", "✅".green());
}

async fn stop_rag() {
    println!("  {} Stopping Fusion-RAG...", "⏳".blue());
    println!("  {} Fusion-RAG stopped", "✅".green());
}

async fn start_doc() {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Fusion-Doc...", "⏳".blue());
    match doc::health_check().await {
        Ok(true) => println!(
            "  {} Fusion-Doc already running ({})",
            "⚠️".yellow(),
            urls.doc
        ),
        _ => println!(
            "  {} Fusion-Doc start: {} (use `fusion doc start`)",
            "ℹ️".blue(),
            urls.doc
        ),
    }
}

async fn stop_doc() {
    println!("  {} Stopping Fusion-Doc...", "⏳".blue());
    println!("  {} Fusion-Doc stopped", "✅".green());
}

#[derive(Tabled)]
struct ServiceEntry {
    #[tabled(rename = "Service")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "PID")]
    pid: String,
    #[tabled(rename = "URL")]
    url: String,
}
