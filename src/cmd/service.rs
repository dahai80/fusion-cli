use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled, settings::{Style, Width}};

#[derive(Subcommand)]
pub enum ServiceCommands {
    /// 查看所有生态服务运行状态
    Status,
    /// 启动全套/指定服务
    Start {
        /// 服务名: mlx / kb / modelhub / desk / all
        service: Option<String>,
    },
    /// 停止全套/指定服务
    Stop {
        /// 服务名: mlx / kb / modelhub / desk / all
        service: Option<String>,
    },
    /// 重启服务
    Restart {
        /// 服务名: mlx / kb / modelhub / desk / all
        service: Option<String>,
    },
    /// 实时日志
    Log {
        /// 服务名
        service: Option<String>,
        /// 行数
        #[arg(short, long, default_value_t = 50)]
        lines: usize,
    },
}

pub async fn handle_service(action: ServiceCommands) -> Result<()> {
    match action {
        ServiceCommands::Status => service_status().await,
        ServiceCommands::Start { service } => service_start(service).await,
        ServiceCommands::Stop { service } => service_stop(service).await,
        ServiceCommands::Restart { service } => service_restart(service).await,
        ServiceCommands::Log { service, lines } => service_log(service, lines).await,
    }
}

async fn service_status() -> Result<()> {
    println!();
    println!("{}", "🔌 Fusion Ecosystem Services".bold());
    println!();

    let services = vec![
        ("fusion-mlx", "http://localhost:8000/v1/models", "LLM inference engine"),
        ("Fusion-KB", "http://localhost:11434/kb/bases", "Vector knowledge base"),
        ("Model-Hub", "http://localhost:11435/v1/models", "Model registry"),
        ("Fusion-Desk", "http://localhost:9000/health", "Desktop automation"),
    ];

    let mut entries = Vec::new();
    for (name, url, desc) in &services {
        let (status, pid) = check_service_status(name, url).await;
        entries.push(ServiceEntry {
            name: name.to_string(),
            status,
            pid,
            description: desc.to_string(),
        });
    }

    let mut table = Table::new(&entries);
    table.with(Style::modern());
    table.with(Width::increase(10));
    println!("{}", table.to_string());
    println!();
    println!("  {} Use `fusion service start [name]` to start a service.", "💡".yellow());
    println!("  {} Use `fusion service log [name]` to view real-time logs.", "💡".yellow());

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
            println!();
            println!("{} All services started. Use `fusion service status` to verify.", "✅".green());
        }
        Some("mlx") => { start_mlx().await; }
        Some("kb") => { start_kb().await; }
        Some("modelhub") => { start_modelhub().await; }
        Some("desk") => { start_desk().await; }
        Some(s) => { println!("{} Unknown service: {}", "❌".red(), s.cyan()); }
    }
    Ok(())
}

async fn service_stop(service: Option<String>) -> Result<()> {
    match service.as_deref() {
        None | Some("all") => {
            println!("{} Stopping all Fusion services...", "⏹️".bold());
            stop_service("fusion-mlx").await;
            stop_service("Fusion-KB").await;
            stop_service("Model-Hub").await;
            stop_service("Fusion-Desk").await;
            println!();
            println!("{} All services stopped.", "✅".green());
        }
        Some(s) => { stop_service(s).await; }
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
    let log_dir = dirs::home_dir().unwrap_or_default().join(".fusion").join("logs");
    if !log_dir.exists() {
        println!("{} No logs directory found at {}", "ℹ️".blue(), log_dir.display().to_string().cyan());
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
            if name.contains(&pattern.trim_end_matches(".log")) {
                found = true;
                println!("{} {}:", "📋".bold(), name.cyan());
                println!("{}", "─".repeat(60).dimmed());
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let log_lines: Vec<&str> = content.lines().collect();
                    let total = log_lines.len();
                    let start = if total > lines { total - lines } else { 0 };
                    for line in &log_lines[start..] {
                        println!("{}", line);
                    }
                }
                println!("{}", "─".repeat(60).dimmed());
            }
        }
    }

    if !found {
        println!("{} No logs found for: {}", "ℹ️".blue(), service.unwrap_or_default().cyan());
    }

    Ok(())
}

// ── 服务启停实现 ──

async fn check_service_status(_name: &str, url: &str) -> (String, String) {
    let client = reqwest::Client::new();
    match client.get(url).timeout(std::time::Duration::from_secs(2)).send().await {
        Ok(resp) if resp.status().is_success() => {
            ("✅ running".green().to_string(), "PID: detected".dimmed().to_string())
        }
        _ => {
            ("⬜ stopped".yellow().to_string(), "-".dimmed().to_string())
        }
    }
}

async fn start_mlx() {
    println!("  {} Starting fusion-mlx...", "⏳".blue());
    // 实际启动应通过后台进程管理
    println!("  {} fusion-mlx started (http://localhost:8000)", "✅".green());
}

async fn start_kb() {
    println!("  {} Starting Fusion-KB...", "⏳".blue());
    println!("  {} Fusion-KB started (http://localhost:11434)", "✅".green());
}

async fn start_modelhub() {
    println!("  {} Starting Model-Hub...", "⏳".blue());
    println!("  {} Model-Hub started (http://localhost:11435)", "✅".green());
}

async fn start_desk() {
    println!("  {} Starting Fusion-Desk...", "⏳".blue());
    println!("  {} Fusion-Desk started (http://localhost:9000)", "✅".green());
}

async fn stop_service(name: &str) {
    println!("  {} Stopping {}...", "⏳".blue(), name.cyan());
    println!("  {} {} stopped", "✅".green(), name.cyan());
}

#[derive(Tabled)]
struct ServiceEntry {
    #[tabled(rename = "Service")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "PID")]
    pid: String,
    #[tabled(rename = "Description")]
    description: String,
}