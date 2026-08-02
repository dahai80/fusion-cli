use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled, settings::Style};

use crate::service::doc as doc_svc;
use crate::service::get_client;

const DOC_DEFAULT_PORT: u16 = 11449;

#[derive(Subcommand)]
pub enum DocCommands {
    Start {
        #[arg(short, long, default_value_t = DOC_DEFAULT_PORT)]
        port: u16,
    },
    Stop,
    Status,
    Log {
        #[arg(short, long, default_value_t = 50)]
        lines: usize,
    },
}

pub async fn handle_doc(action: DocCommands) -> Result<()> {
    match action {
        DocCommands::Start { port } => doc_start(port).await,
        DocCommands::Stop => doc_stop().await,
        DocCommands::Status => doc_status().await,
        DocCommands::Log { lines } => doc_log(lines).await,
    }
}

async fn doc_start(port: u16) -> Result<()> {
    println!("{} Starting fusion-doc service...", "🚀".bold());

    let client = get_client();
    let health_url = format!("http://localhost:{}/api/health", port);
    match client
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            println!(
                "  {} fusion-doc already running on port {}",
                "⚠️".yellow(),
                port
            );
            return Ok(());
        }
        _ => {}
    }

    let home = dirs::home_dir().unwrap_or_default();
    let start_script = home
        .join("fusion")
        .join("fusion-doc")
        .join("scripts")
        .join("start.sh");

    if !start_script.exists() {
        println!(
            "  {} fusion-doc start script not found at {}",
            "❌".red(),
            start_script.display()
        );
        anyhow::bail!("fusion-doc start script not found");
    }

    let env_port = format!("{}", port);
    match tokio::process::Command::new(&start_script)
        .env("FUSION_DOC_PORT", &env_port)
        .arg("start")
        .output()
        .await
    {
        Ok(output) => {
            if output.status.success() {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                match client
                    .get(&health_url)
                    .timeout(std::time::Duration::from_secs(3))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        println!("  {} fusion-doc started on port {}", "✅".green(), port);
                    }
                    _ => {
                        println!(
                            "  {} fusion-doc process started but health check pending",
                            "⏳".yellow()
                        );
                        println!("     Check status: fusion doc status");
                    }
                }
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!(
                    "  {} Failed to start fusion-doc: {}",
                    "❌".red(),
                    stderr.trim()
                );
            }
        }
        Err(e) => {
            println!("  {} Failed to start fusion-doc: {}", "❌".red(), e);
        }
    }

    Ok(())
}

async fn doc_stop() -> Result<()> {
    println!("{} Stopping fusion-doc service...", "⏹️".bold());

    let home = dirs::home_dir().unwrap_or_default();
    let start_script = home
        .join("fusion")
        .join("fusion-doc")
        .join("scripts")
        .join("start.sh");

    if start_script.exists() {
        match tokio::process::Command::new(&start_script)
            .arg("stop")
            .output()
            .await
        {
            Ok(_) => println!("  {} fusion-doc stopped", "✅".green()),
            Err(e) => println!("  {} Failed to stop fusion-doc: {}", "❌".red(), e),
        }
    } else {
        match doc_svc::health_check().await {
            Ok(true) => {
                println!(
                    "  {} Service is running but no stop script found",
                    "⚠️".yellow()
                );
                println!("     Try: pkill -f fusion-doc");
            }
            _ => {
                println!("  {} fusion-doc is not running", "ℹ️".blue());
            }
        }
    }

    Ok(())
}

async fn doc_status() -> Result<()> {
    println!();
    println!("{}", "🔍 Fusion-Doc Service Status".bold());
    println!();

    let (status, version, uptime) = match doc_svc::get_health_detail().await {
        Ok(data) => {
            let v = data
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let u = data
                .get("uptime")
                .and_then(|v| v.as_str())
                .unwrap_or("-")
                .to_string();
            ("✅ running".green().to_string(), v, u)
        }
        Err(_) => (
            "⬜ stopped".yellow().to_string(),
            "-".to_string(),
            "-".to_string(),
        ),
    };

    let entries = vec![
        StatusEntry {
            key: "Service".to_string(),
            value: status,
        },
        StatusEntry {
            key: "Version".to_string(),
            value: version.cyan().to_string(),
        },
        StatusEntry {
            key: "Uptime".to_string(),
            value: uptime.cyan().to_string(),
        },
        StatusEntry {
            key: "Port".to_string(),
            value: DOC_DEFAULT_PORT.to_string().cyan().to_string(),
        },
        StatusEntry {
            key: "Health".to_string(),
            value: format!("http://localhost:{}/api/health", DOC_DEFAULT_PORT)
                .cyan()
                .to_string(),
        },
    ];

    let mut table = Table::new(&entries);
    table.with(Style::modern());
    println!("{}", table);
    println!();

    Ok(())
}

async fn doc_log(lines: usize) -> Result<()> {
    let log_file = dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion")
        .join("logs")
        .join("fusion-doc.log");

    if !log_file.exists() {
        println!(
            "{} No fusion-doc log at {}",
            "ℹ️".blue(),
            log_file.display()
        );
        return Ok(());
    }

    println!("{} {}:", "📋".bold(), "fusion-doc.log".cyan());
    println!("{}", "─".repeat(60).dimmed());
    if let Ok(content) = std::fs::read_to_string(&log_file) {
        let log_lines: Vec<&str> = content.lines().collect();
        let total = log_lines.len();
        let start = total.saturating_sub(lines);
        for line in &log_lines[start..] {
            println!("{}", line);
        }
    }
    println!("{}", "─".repeat(60).dimmed());

    Ok(())
}

#[derive(Tabled)]
struct StatusEntry {
    #[tabled(rename = "Key")]
    key: String,
    #[tabled(rename = "Value")]
    value: String,
}
