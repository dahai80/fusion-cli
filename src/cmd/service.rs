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
            // 逐个服务返回是否真正启动 (true) 还是需人工 (false), 汇总必须诚实。
            let results = [
                ("mlx", start_mlx().await),
                ("kb", start_kb().await),
                ("modelhub", start_modelhub().await),
                ("desk", start_desk().await),
                ("rag", start_rag().await),
                ("doc", start_doc().await),
            ];
            let started: Vec<&str> = results
                .iter()
                .filter(|(_, s)| *s)
                .map(|(n, _)| *n)
                .collect();
            let manual: Vec<&str> = results
                .iter()
                .filter(|(_, s)| !*s)
                .map(|(n, _)| *n)
                .collect();
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": "start",
                        "target": "all",
                        "started": started,
                        "manual_required": manual,
                    }))?
                );
            } else {
                println!();
                if !started.is_empty() {
                    println!("  {} Started: {}", "✅".green(), started.join(", "));
                }
                if !manual.is_empty() {
                    println!(
                        "  {} Manual start required: {}",
                        "⚠️".yellow(),
                        manual.join(", ")
                    );
                    println!(
                        "     These services have no CLI-managed launcher; use their start.sh."
                    );
                }
                println!("  Use `fusion service status` to verify.");
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
            let results = [
                ("mlx", stop_mlx().await),
                ("kb", stop_kb().await),
                ("modelhub", stop_modelhub().await),
                ("desk", stop_desk().await),
                ("rag", stop_rag().await),
                ("doc", stop_doc().await),
            ];
            let stopped: Vec<&str> = results
                .iter()
                .filter(|(_, s)| *s)
                .map(|(n, _)| *n)
                .collect();
            let manual: Vec<&str> = results
                .iter()
                .filter(|(_, s)| !*s)
                .map(|(n, _)| *n)
                .collect();
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": "stop",
                        "target": "all",
                        "stopped": stopped,
                        "manual_required": manual,
                    }))?
                );
            } else {
                println!();
                if !stopped.is_empty() {
                    println!("  {} Stopped: {}", "✅".green(), stopped.join(", "));
                }
                if !manual.is_empty() {
                    println!(
                        "  {} Manual stop required: {}",
                        "⚠️".yellow(),
                        manual.join(", ")
                    );
                    println!("     Try: pkill -f <service-name>");
                }
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

// 启停辅助: 返回 true 表示 CLI 真正管理了该进程 (启动/停止成功或已在运行),
// false 表示需要人工介入 (无脚本/二进制)。汇总据此诚实分类, 杜绝 "✅ All started" 造假。
async fn start_mlx() -> bool {
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
            Ok(_) => {
                println!("  {} fusion-mlx started ({})", "✅".green(), urls.mlx);
                true
            }
            Err(e) => {
                println!("  {} Failed to start fusion-mlx: {}", "❌".red(), e);
                false
            }
        }
    } else {
        println!(
            "  {} fusion-mlx start script not found at {}",
            "⚠️".yellow(),
            start_script.display()
        );
        false
    }
}

async fn start_kb() -> bool {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Fusion-KB...", "⏳".blue());
    if kb::health_check().await.unwrap_or(false) {
        println!(
            "  {} Fusion-KB already running ({})",
            "⚠️".yellow(),
            urls.kb
        );
        return true;
    }
    // 尝试项目 start.sh (kb 有独立 start.sh)。
    let script = dirs::home_dir()
        .unwrap_or_default()
        .join("fusion/fusion-kb/start.sh");
    if script.exists() {
        match tokio::process::Command::new(&script)
            .arg("start")
            .output()
            .await
        {
            Ok(o) if o.status.success() => {
                println!("  {} Fusion-KB started ({})", "✅".green(), urls.kb);
                return true;
            }
            _ => {}
        }
    }
    println!(
        "  {} Fusion-KB start: {} (manual start required, no start.sh found)",
        "ℹ️".blue(),
        urls.kb
    );
    false
}

async fn start_modelhub() -> bool {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Model-Hub...", "⏳".blue());
    if modelhub::health_check().await.unwrap_or(false) {
        println!(
            "  {} Model-Hub already running ({})",
            "⚠️".yellow(),
            urls.modelhub
        );
        return true;
    }
    let script = dirs::home_dir()
        .unwrap_or_default()
        .join("fusion/fusion-model-hub/start.sh");
    if script.exists() {
        match tokio::process::Command::new(&script)
            .arg("start")
            .output()
            .await
        {
            Ok(o) if o.status.success() => {
                println!("  {} Model-Hub started ({})", "✅".green(), urls.modelhub);
                return true;
            }
            _ => {}
        }
    }
    println!(
        "  {} Model-Hub start: {} (manual start required, no start.sh found)",
        "ℹ️".blue(),
        urls.modelhub
    );
    false
}

async fn start_desk() -> bool {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Fusion-Desk...", "⏳".blue());
    if desk::health_check().await.unwrap_or(false) {
        println!(
            "  {} Fusion-Desk already running ({})",
            "⚠️".yellow(),
            urls.desk
        );
        return true;
    }
    let script = dirs::home_dir()
        .unwrap_or_default()
        .join("fusion/fusion-cowork/start.sh");
    if script.exists() {
        match tokio::process::Command::new(&script)
            .arg("start")
            .output()
            .await
        {
            Ok(o) if o.status.success() => {
                println!("  {} Fusion-Desk started ({})", "✅".green(), urls.desk);
                return true;
            }
            _ => {}
        }
    }
    println!(
        "  {} Fusion-Desk start: {} (manual start required, no start.sh found)",
        "ℹ️".blue(),
        urls.desk
    );
    false
}

async fn start_rag() -> bool {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Fusion-RAG...", "⏳".blue());
    if rag::health_check().await.unwrap_or(false) {
        println!(
            "  {} Fusion-RAG already running ({})",
            "⚠️".yellow(),
            urls.rag
        );
        return true;
    }
    // RAG 有真正的 PID 管理 (fusion-rag 二进制), 委托给 rag_start 处理。
    match crate::cmd::rag::handle_rag(crate::cmd::rag::RagCommands::Start { port: 11436 }).await {
        Ok(_) => true,
        Err(e) => {
            println!("  {} Fusion-RAG start failed: {}", "❌".red(), e);
            false
        }
    }
}

async fn stop_mlx() -> bool {
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
            Ok(_) => {
                println!("  {} fusion-mlx stopped", "✅".green());
                true
            }
            Err(e) => {
                println!("  {} Failed to stop fusion-mlx: {}", "❌".red(), e);
                false
            }
        }
    } else {
        println!("  {} fusion-mlx stop script not found", "⚠️".yellow());
        false
    }
}

async fn stop_kb() -> bool {
    // 无 PID 管理, 尝试 pkill。
    match tokio::process::Command::new("pkill")
        .args(["-f", "fusion-kb"])
        .output()
        .await
    {
        Ok(o) if o.status.success() => {
            println!("  {} Fusion-KB stopped (pkill)", "✅".green());
            true
        }
        _ => {
            println!(
                "  {} Fusion-KB stop: not running or pkill failed (manual: pkill -f fusion-kb)",
                "ℹ️".blue()
            );
            false
        }
    }
}

async fn stop_modelhub() -> bool {
    match tokio::process::Command::new("pkill")
        .args(["-f", "fusion-model-hub"])
        .output()
        .await
    {
        Ok(o) if o.status.success() => {
            println!("  {} Model-Hub stopped (pkill)", "✅".green());
            true
        }
        _ => {
            println!(
                "  {} Model-Hub stop: not running or pkill failed (manual: pkill -f fusion-model-hub)",
                "ℹ️".blue()
            );
            false
        }
    }
}

async fn stop_desk() -> bool {
    match tokio::process::Command::new("pkill")
        .args(["-f", "fusion-desk"])
        .output()
        .await
    {
        Ok(o) if o.status.success() => {
            println!("  {} Fusion-Desk stopped (pkill)", "✅".green());
            true
        }
        _ => {
            println!(
                "  {} Fusion-Desk stop: not running or pkill failed (manual: pkill -f fusion-desk)",
                "ℹ️".blue()
            );
            false
        }
    }
}

async fn stop_rag() -> bool {
    // RAG 有 PID 管理, 委托给 rag_stop。
    match crate::cmd::rag::handle_rag(crate::cmd::rag::RagCommands::Stop).await {
        Ok(_) => true,
        Err(e) => {
            println!("  {} Fusion-RAG stop failed: {}", "❌".red(), e);
            false
        }
    }
}

async fn start_doc() -> bool {
    let urls = ServiceUrls::from_config();
    println!("  {} Starting Fusion-Doc...", "⏳".blue());
    if doc::health_check().await.unwrap_or(false) {
        println!(
            "  {} Fusion-Doc already running ({})",
            "⚠️".yellow(),
            urls.doc
        );
        return true;
    }
    // Doc 有真正的 start.sh 管理, 委托给 doc_start。
    match crate::cmd::doc::handle_doc(crate::cmd::doc::DocCommands::Start { port: 11449 }).await {
        Ok(_) => true,
        Err(e) => {
            println!("  {} Fusion-Doc start failed: {}", "❌".red(), e);
            false
        }
    }
}

async fn stop_doc() -> bool {
    // Doc 有真正的 start.sh stop, 委托给 doc_stop。
    match crate::cmd::doc::handle_doc(crate::cmd::doc::DocCommands::Stop).await {
        Ok(_) => true,
        Err(e) => {
            println!("  {} Fusion-Doc stop failed: {}", "❌".red(), e);
            false
        }
    }
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
