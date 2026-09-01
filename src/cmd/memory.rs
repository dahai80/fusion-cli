use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled, settings::Style};
use tracing::error;

use crate::service::memory as mem;
use crate::utils::output;

// 记忆管理 — 对接 fusion-memory fm-server (11435, HTTP + Bearer)。
// 记忆中心: 跨会话长/短期记忆 + 认知图谱。
const MEMORY_DEFAULT_PORT: u16 = 11435;

#[derive(Subcommand)]
pub enum MemoryCommands {
    /// 服务状态
    Status,
    /// API 版本
    Version,
    /// 语义检索记忆
    Search {
        /// 查询文本
        query: String,
        /// 返回条数
        #[arg(short, long, default_value_t = 5)]
        top_k: usize,
    },
    /// 记忆条目总数
    Count,
    /// 按 ID 取单条记忆
    Get {
        /// 记忆 ID
        id: String,
    },
    /// 写入一条记忆
    Commit {
        /// 记忆内容
        content: String,
        /// 作用域 (可选, 如 session-xxx)
        #[arg(short, long)]
        scope: Option<String>,
    },
    /// 触发记忆巩固 (短期→长期)
    Consolidate,
    /// 按 ID 删除记忆
    Delete {
        /// 记忆 ID
        id: String,
    },
    /// 审计日志
    Audit,
}

pub async fn handle_memory(action: MemoryCommands) -> Result<()> {
    match action {
        MemoryCommands::Status => memory_status().await,
        MemoryCommands::Version => memory_version().await,
        MemoryCommands::Search { query, top_k } => memory_search(query, top_k).await,
        MemoryCommands::Count => memory_count().await,
        MemoryCommands::Get { id } => memory_get(id).await,
        MemoryCommands::Commit { content, scope } => memory_commit(content, scope).await,
        MemoryCommands::Consolidate => memory_consolidate().await,
        MemoryCommands::Delete { id } => memory_delete(id).await,
        MemoryCommands::Audit => memory_audit().await,
    }
}

async fn memory_status() -> Result<()> {
    println!();
    println!("{}", "🧠 Fusion-Memory Service Status".bold());
    println!();

    let alive = mem::health_check().await.unwrap_or(false);

    if output::is_json_mode() {
        let payload = serde_json::json!({
            "service": "fusion-memory",
            "alive": alive,
            "port": MEMORY_DEFAULT_PORT,
        });
        output::print_json(&payload)?;
        return Ok(());
    }

    let status = if alive {
        "✅ running".green().to_string()
    } else {
        "⬜ stopped".yellow().to_string()
    };

    let mut entries = vec![
        StatusEntry {
            key: "Service".to_string(),
            value: status,
        },
        StatusEntry {
            key: "Port".to_string(),
            value: MEMORY_DEFAULT_PORT.to_string().cyan().to_string(),
        },
    ];

    if alive && let Ok(v) = mem::version().await {
        let ver = v
            .get("api_version")
            .or_else(|| v.get("version"))
            .map(|x| x.to_string())
            .unwrap_or_else(|| "-".into());
        entries.push(StatusEntry {
            key: "API Version".to_string(),
            value: ver.cyan().to_string(),
        });
    }

    let mut table = Table::new(&entries);
    table.with(Style::modern());
    println!("{}", table);
    println!();

    if !alive {
        println!(
            "  {} Start: ~/claude-home/fusion-memory/start.sh (or fm-server)",
            "💡".yellow()
        );
    }

    Ok(())
}

async fn memory_version() -> Result<()> {
    match mem::version().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "memory version error");
            println!("  {} fm-server not reachable: {}", "❌".red(), e);
            anyhow::bail!("Failed to get fm-server version: {}", e);
        }
    }
    Ok(())
}

async fn memory_search(query: String, top_k: usize) -> Result<()> {
    println!("{} Searching memory...", "🔍".bold());
    println!("  Query: {}", query.dimmed());
    println!();

    match mem::retrieve(&query, top_k).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let results = data
                .get("results")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if results.is_empty() {
                println!("  {} No memories found.", "ℹ️".blue());
                return Ok(());
            }
            let mut entries = Vec::new();
            for (i, item) in results.iter().enumerate() {
                let id = item
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let score = item
                    .get("score")
                    .and_then(|v| v.as_f64())
                    .map(|s| format!("{:.3}", s))
                    .unwrap_or_else(|| "-".into());
                let content = item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no content)");
                let preview: String = content.chars().take(100).collect();
                entries.push(SearchEntry {
                    rank: (i + 1).to_string(),
                    id,
                    score,
                    preview,
                });
            }
            let mut table = Table::new(&entries);
            table.with(Style::modern());
            println!("{}", table);
            println!();
            println!("  Found {} memories", entries.len().to_string().cyan());
        }
        Err(e) => {
            error!(error = %e, "memory retrieve error");
            println!("  {} fm-server not reachable: {}", "⬜".yellow(), e);
            println!("     Start: ~/claude-home/fusion-memory/start.sh");
            anyhow::bail!("Memory search failed: {}", e);
        }
    }

    Ok(())
}

async fn memory_count() -> Result<()> {
    match mem::count().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let n = data
                .get("count")
                .and_then(|v| v.as_u64())
                .map(|c| c.to_string())
                .unwrap_or_else(|| serde_json::to_string(&data).unwrap_or_else(|_| "-".into()));
            println!("  {} Total memories: {}", "🧠".bold(), n.cyan());
        }
        Err(e) => {
            error!(error = %e, "memory count error");
            println!("  {} fm-server not reachable: {}", "❌".red(), e);
            anyhow::bail!("Memory count failed: {}", e);
        }
    }
    Ok(())
}

async fn memory_get(id: String) -> Result<()> {
    println!("{} Memory {}", "🔍".bold(), id.cyan());
    println!();
    match mem::get_memory(&id).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "memory get error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Failed to get memory {}: {}", id, e);
        }
    }
    Ok(())
}

async fn memory_commit(content: String, scope: Option<String>) -> Result<()> {
    println!("{} Committing memory...", "📝".bold());
    match mem::commit(&content, scope.as_deref()).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let id = data
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("(unknown)");
            println!("  {} Memory committed: {}", "✅".green(), id.cyan());
        }
        Err(e) => {
            error!(error = %e, "memory commit error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Memory commit failed: {}", e);
        }
    }
    Ok(())
}

async fn memory_consolidate() -> Result<()> {
    println!("{} Consolidating memories (short→long)...", "🔄".bold());
    match mem::consolidate().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!("  {} Consolidation completed.", "✅".green());
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "memory consolidate error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Memory consolidation failed: {}", e);
        }
    }
    Ok(())
}

async fn memory_delete(id: String) -> Result<()> {
    println!("{} Deleting memory {}...", "🗑️".bold(), id.cyan());
    match mem::delete(&id).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!("  {} Memory {} deleted.", "✅".green(), id.cyan());
        }
        Err(e) => {
            error!(error = %e, "memory delete error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Failed to delete memory {}: {}", id, e);
        }
    }
    Ok(())
}

async fn memory_audit() -> Result<()> {
    println!();
    println!("{}", "📋 Memory Audit Log".bold());
    println!();
    match mem::audit().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let entries = data
                .get("entries")
                .or_else(|| data.get("audit"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if entries.is_empty() {
                println!("  {} No audit entries.", "ℹ️".blue());
                return Ok(());
            }
            let mut rows = Vec::new();
            for e in &entries {
                let ts = e
                    .get("timestamp")
                    .or_else(|| e.get("ts"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let action = e
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let actor = e
                    .get("actor")
                    .or_else(|| e.get("by"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                rows.push(AuditEntry {
                    timestamp: ts,
                    action,
                    actor,
                });
            }
            let mut table = Table::new(&rows);
            table.with(Style::modern());
            println!("{}", table);
        }
        Err(e) => {
            error!(error = %e, "memory audit error");
            println!("  {} fm-server not reachable: {}", "❌".red(), e);
            anyhow::bail!("Memory audit failed: {}", e);
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
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Score")]
    score: String,
    #[tabled(rename = "Preview")]
    preview: String,
}

#[derive(Tabled)]
struct AuditEntry {
    #[tabled(rename = "Timestamp")]
    timestamp: String,
    #[tabled(rename = "Action")]
    action: String,
    #[tabled(rename = "Actor")]
    actor: String,
}
