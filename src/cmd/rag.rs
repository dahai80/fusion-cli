use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled, settings::Style};
use tracing::info;

use crate::service::get_client;
use crate::service::rag as rag_svc;
use crate::utils::output::is_json_mode;

const RAG_DEFAULT_PORT: u16 = 11436;

#[derive(Subcommand)]
pub enum RagCommands {
    Start {
        #[arg(short, long, default_value_t = RAG_DEFAULT_PORT)]
        port: u16,
    },
    Stop,
    Status,
    Search {
        kb_id: String,
        query: String,
        #[arg(short, long, default_value_t = 5)]
        top_k: usize,
    },
    List,
}

pub async fn handle_rag(action: RagCommands) -> Result<()> {
    match action {
        RagCommands::Start { port } => rag_start(port).await,
        RagCommands::Stop => rag_stop().await,
        RagCommands::Status => rag_status().await,
        RagCommands::Search {
            kb_id,
            query,
            top_k,
        } => rag_search(kb_id, query, top_k).await,
        RagCommands::List => rag_list().await,
    }
}

async fn rag_start(port: u16) -> Result<()> {
    let json_mode = is_json_mode();
    if !json_mode {
        println!("{} Starting fusion-rag service...", "🚀".bold());
    }

    let client = get_client();
    let health_url = format!("http://localhost:{}/health", port);
    match client
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": "start", "port": port, "status": "already-running"
                    }))?
                );
            } else {
                println!(
                    "  {} fusion-rag already running on port {}",
                    "⚠️".yellow(),
                    port
                );
            }
            return Ok(());
        }
        _ => {}
    }

    let home = dirs::home_dir().unwrap_or_default();
    let rag_bin = home.join(".fusion").join("bin").join("fusion-rag");
    if !rag_bin.exists() {
        if json_mode {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "action": "start", "port": port, "status": "error",
                    "error": "fusion-rag binary not found",
                    "binary": rag_bin.display().to_string(),
                    "hint": "install with: fusion service start rag",
                }))?
            );
            return Ok(());
        }
        println!(
            "  {} fusion-rag binary not found at {}",
            "❌".red(),
            rag_bin.display()
        );
        println!("     Install with: fusion service start rag");
        anyhow::bail!("fusion-rag not installed");
    }

    let pid_file = home.join(".fusion").join("run").join("fusion-rag.pid");
    std::fs::create_dir_all(pid_file.parent().unwrap())?;

    let log_file = home.join(".fusion").join("logs").join("fusion-rag.log");
    std::fs::create_dir_all(log_file.parent().unwrap())?;

    let log = std::fs::File::create(&log_file)?;
    let child = std::process::Command::new(&rag_bin)
        .arg("--port")
        .arg(port.to_string())
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()?;

    let pid = child.id();
    std::fs::write(&pid_file, pid.to_string())?;
    info!(port = port, pid = pid, "Spawned fusion-rag process");

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    match client
        .get(&health_url)
        .timeout(std::time::Duration::from_secs(3))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": "start", "port": port, "pid": pid, "status": "started"
                    }))?
                );
            } else {
                println!(
                    "  {} fusion-rag started on port {} (PID {})",
                    "✅".green(),
                    port,
                    pid
                );
            }
        }
        _ => {
            if json_mode {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "action": "start", "port": port, "pid": pid, "status": "pending",
                        "hint": "health check pending, run: fusion rag status"
                    }))?
                );
            } else {
                println!(
                    "  {} fusion-rag process started (PID {}) but health check pending",
                    "⏳".yellow(),
                    pid
                );
                println!("     Check status: fusion rag status");
            }
        }
    }

    Ok(())
}

async fn rag_stop() -> Result<()> {
    let json_mode = is_json_mode();
    if !json_mode {
        println!("{} Stopping fusion-rag service...", "⏹️".bold());
    }

    let home = dirs::home_dir().unwrap_or_default();
    let pid_file = home.join(".fusion").join("run").join("fusion-rag.pid");

    if let Ok(pid_str) = std::fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<u32>() {
            let output = std::process::Command::new("kill")
                .arg(pid.to_string())
                .output();

            match output {
                Ok(o) if o.status.success() => {
                    if json_mode {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "action": "stop", "pid": pid, "stopped": true
                            }))?
                        );
                    } else {
                        println!("  {} fusion-rag stopped (PID {})", "✅".green(), pid);
                    }
                    let _ = std::fs::remove_file(&pid_file);
                }
                _ => {
                    if json_mode {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "action": "stop", "pid": pid, "stopped": false,
                                "warning": "process not found (already exited?)"
                            }))?
                        );
                    } else {
                        println!(
                            "  {} Process {} not found (may have already exited)",
                            "⚠️".yellow(),
                            pid
                        );
                    }
                    let _ = std::fs::remove_file(&pid_file);
                }
            }
        } else if json_mode {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "action": "stop", "stopped": false, "error": "invalid PID in pid file"
                }))?
            );
        } else {
            println!("  {} Invalid PID in {}", "❌".red(), pid_file.display());
        }
    } else {
        match rag_svc::health_check().await {
            Ok(true) => {
                if json_mode {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "action": "stop", "stopped": false,
                            "running": true,
                            "warning": "service running but no PID file",
                            "hint": "try: pkill -f fusion-rag"
                        }))?
                    );
                } else {
                    println!(
                        "  {} Service is running but no PID file found",
                        "⚠️".yellow()
                    );
                    println!("     Try: pkill -f fusion-rag");
                }
            }
            _ => {
                if json_mode {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "action": "stop", "stopped": false, "running": false
                        }))?
                    );
                } else {
                    println!("  {} fusion-rag is not running", "ℹ️".blue());
                }
            }
        }
    }

    Ok(())
}

async fn rag_status() -> Result<()> {
    let json_mode = is_json_mode();
    if !json_mode {
        println!();
        println!("{}", "🔍 Fusion-RAG Service Status".bold());
        println!();
    }

    let (status, version, uptime, health_data) = match rag_svc::get_health_detail().await {
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
            ("✅ running".green().to_string(), v, u, Some(data))
        }
        Err(_) => (
            "⬜ stopped".yellow().to_string(),
            "-".to_string(),
            "-".to_string(),
            None,
        ),
    };

    let embedding_status = match rag_svc::list_embedding_models().await {
        Ok(data) => {
            if json_mode {
                data
            } else if let Some(models) = data.get("models").and_then(|v| v.as_array()) {
                serde_json::json!(format!("{} model(s) available", models.len()))
            } else {
                serde_json::json!("available")
            }
        }
        Err(_) => serde_json::json!("unavailable"),
    };

    if json_mode {
        let payload = serde_json::json!({
            "running": health_data.is_some(),
            "port": RAG_DEFAULT_PORT,
            "version": version,
            "uptime": uptime,
            "health": health_data,
            "embedding_models": embedding_status,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    let mut entries = vec![
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
            value: RAG_DEFAULT_PORT.to_string().cyan().to_string(),
        },
    ];

    let emb_text = match embedding_status.as_str() {
        Some(s) => s.to_string(),
        None => "available".to_string(),
    };
    entries.push(StatusEntry {
        key: "Embedding".to_string(),
        value: emb_text.cyan().to_string(),
    });

    let mut table = Table::new(&entries);
    table.with(Style::modern());
    println!("{}", table);
    println!();

    Ok(())
}

async fn rag_search(kb_id: String, query: String, top_k: usize) -> Result<()> {
    let json_mode = is_json_mode();
    if !json_mode {
        println!("{} Searching in '{}'...", "🔍".bold(), kb_id.cyan());
        println!("  Query: {}", query.dimmed());
        println!();
    }

    match rag_svc::search(&kb_id, &query, top_k).await {
        Ok(data) => {
            if json_mode {
                let payload = serde_json::json!({
                    "kb_id": kb_id, "query": query, "top_k": top_k, "result": data,
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }
            if let Some(results) = data.get("results").and_then(|v| v.as_array()) {
                if results.is_empty() {
                    println!("  {} No results found.", "ℹ️".blue());
                } else {
                    let mut entries = Vec::new();
                    for (i, item) in results.iter().enumerate() {
                        let content = item
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("(no content)");
                        let preview: String = content.chars().take(120).collect();
                        let score = item
                            .get("score")
                            .and_then(|v| v.as_f64())
                            .map(|s| format!("{:.3}", s))
                            .unwrap_or_else(|| "-".to_string());
                        let source = item.get("source").and_then(|v| v.as_str()).unwrap_or("-");
                        entries.push(SearchEntry {
                            rank: (i + 1).to_string(),
                            score,
                            source: source.to_string(),
                            preview,
                        });
                    }
                    let mut table = Table::new(&entries);
                    table.with(Style::modern());
                    println!("{}", table);
                    println!();
                    println!("  Found {} results", entries.len().to_string().cyan());
                }
            } else if let Some(answer) = data.get("answer").and_then(|v| v.as_str()) {
                println!("{}", "Answer:".green().bold());
                println!("{}", answer);
            } else {
                println!("  {} Unexpected response format", "⚠️".yellow());
            }
        }
        Err(e) => {
            if json_mode {
                let payload = serde_json::json!({
                    "kb_id": kb_id, "query": query, "error": e.to_string(),
                    "hint": "is fusion-rag running? start with: fusion rag start",
                });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }
            println!(
                "  {} fusion-rag not available: {} (is it running?)",
                "⬜".yellow(),
                e
            );
            println!("     Start with: fusion rag start");
        }
    }

    Ok(())
}

async fn rag_list() -> Result<()> {
    let json_mode = is_json_mode();
    if !json_mode {
        println!();
        println!("{}", "📚 Fusion-RAG Knowledge Bases".bold());
        println!();
    }

    match rag_svc::list_knowledge_bases().await {
        Ok(data) => {
            let bases = if let Some(arr) = data.get("bases").and_then(|v| v.as_array()) {
                arr.clone()
            } else if let Some(arr) = data.as_array() {
                arr.clone()
            } else {
                vec![]
            };

            if json_mode {
                let payload = serde_json::json!({ "bases": bases, "total": bases.len() });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }

            if bases.is_empty() {
                println!("  {} No knowledge bases found.", "ℹ️".blue());
            } else {
                let mut entries = Vec::new();
                for item in &bases {
                    let kb_id = item
                        .get("id")
                        .or(item.get("name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("-")
                        .to_string();
                    let doc_count = item
                        .get("document_count")
                        .or(item.get("docs"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let status = item
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ready");
                    entries.push(KbListEntry {
                        id: kb_id,
                        documents: doc_count.to_string(),
                        status: status.to_string(),
                    });
                }
                let mut table = Table::new(&entries);
                table.with(Style::modern());
                println!("{}", table);
                println!();
                println!(
                    "  Total: {} knowledge bases",
                    entries.len().to_string().cyan()
                );
            }
        }
        Err(e) => {
            if json_mode {
                let payload = serde_json::json!({ "bases": [], "error": e.to_string(), "hint": "start with: fusion rag start" });
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(());
            }
            println!(
                "  {} fusion-rag not available: {} (is it running?)",
                "⬜".yellow(),
                e
            );
            println!("     Start with: fusion rag start");
        }
    }

    Ok(())
}

#[derive(Tabled)]
struct StatusEntry {
    #[tabled(rename = "Key")]
    key: String,
    #[tabled(rename = "Value")]
    value: String,
}

#[derive(Tabled)]
struct SearchEntry {
    #[tabled(rename = "#")]
    rank: String,
    #[tabled(rename = "Score")]
    score: String,
    #[tabled(rename = "Source")]
    source: String,
    #[tabled(rename = "Preview")]
    preview: String,
}

#[derive(Tabled)]
struct KbListEntry {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Documents")]
    documents: String,
    #[tabled(rename = "Status")]
    status: String,
}
