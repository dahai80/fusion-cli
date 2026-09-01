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
use crate::utils::output::is_json_mode;

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
    let json_mode = is_json_mode();
    match service.as_deref() {
        None | Some("all") => {
            if !json_mode {
                println!("{} Starting all Fusion services...", "🚀".bold());
            }
            start_mlx().await;
            start_kb().await;
            start_modelhub().await;
            start_desk().await;
            start_rag().await;
            start_doc().await;
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "action": "start", "target": "all", "status": "dispatched" })
                    )?
                );
            } else {
                println!();
                println!(
                    "{} All services started. Use `fusion service status` to verify.",
                    "✅".green()
                );
            }
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
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "action": "start", "target": s, "error": "unknown service" })
                    )?
                );
            } else {
                println!("{} Unknown service: {}", "❌".red(), s.cyan());
            }
        }
    }
    Ok(())
}

async fn service_stop(service: Option<String>) -> Result<()> {
    let json_mode = is_json_mode();
    match service.as_deref() {
        None | Some("all") => {
            if !json_mode {
                println!("{} Stopping all Fusion services...", "⏹️".bold());
            }
            stop_mlx().await;
            stop_kb().await;
            stop_modelhub().await;
            stop_desk().await;
            stop_rag().await;
            stop_doc().await;
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "action": "stop", "target": "all", "status": "dispatched" })
                    )?
                );
            } else {
                println!();
                println!("{} All services stopped.", "✅".green());
            }
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
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({ "action": "stop", "target": s, "error": "unknown service" })
                    )?
                );
            } else {
                println!("{} Unknown service: {}", "❌".red(), s.cyan());
            }
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
    let json_mode = is_json_mode();
    let log_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion")
        .join("logs");
    if !log_dir.exists() {
        if json_mode {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({ "logs": [], "error": format!("no logs directory at {}", log_dir.display()) })
                )?
            );
        } else {
            println!(
                "{} No logs directory found at {}",
                "ℹ️".blue(),
                log_dir.display().to_string().cyan()
            );
        }
        return Ok(());
    }

    let mut files: Vec<(String, Vec<String>)> = Vec::new();
    for entry in std::fs::read_dir(&log_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let matches = match service.as_deref() {
                None => name.ends_with(".log"),
                Some(s) => name == format!("{}.log", s) || name.starts_with(&format!("{}.", s)),
            };
            if matches {
                let tail: Vec<String> = std::fs::read_to_string(&path)
                    .map(|content| {
                        let all: Vec<&str> = content.lines().collect();
                        let total = all.len();
                        let start = total.saturating_sub(lines);
                        all[start..].iter().map(|s| s.to_string()).collect()
                    })
                    .unwrap_or_default();
                files.push((name.to_string(), tail));
            }
        }
    }

    if json_mode {
        let map: serde_json::Map<String, serde_json::Value> = files
            .iter()
            .map(|(name, tail)| (name.clone(), serde_json::Value::from(tail.clone())))
            .collect();
        let payload = serde_json::json!({ "logs": map, "files": files.len() });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    if files.is_empty() {
        println!(
            "{} No logs found for: {}",
            "ℹ️".blue(),
            service.unwrap_or_default().cyan()
        );
        return Ok(());
    }

    for (name, tail) in &files {
        println!("{} {}:", "📋".bold(), name.cyan());
        println!("{}", "─".repeat(60).dimmed());
        for line in tail {
            println!("{}", line);
        }
        println!("{}", "─".repeat(60).dimmed());
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
    println!(
        "  {} Fusion-KB stop: manual stop required (no PID management).",
        "ℹ️".blue()
    );
    println!("     Try: pkill -f fusion-kb");
}

async fn stop_modelhub() {
    println!(
        "  {} Model-Hub stop: manual stop required (no PID management).",
        "ℹ️".blue()
    );
    println!("     Try: pkill -f fusion-model-hub");
}

async fn stop_desk() {
    println!(
        "  {} Fusion-Desk stop: manual stop required (no PID management).",
        "ℹ️".blue()
    );
    println!("     Try: pkill -f fusion-desk");
}

async fn stop_rag() {
    println!(
        "  {} Fusion-RAG: use `fusion rag stop` (PID-managed).",
        "ℹ️".blue()
    );
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
    println!(
        "  {} Fusion-Doc stop: manual stop required (no PID management).",
        "ℹ️".blue()
    );
    println!("     Try: pkill -f fusion-doc");
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
