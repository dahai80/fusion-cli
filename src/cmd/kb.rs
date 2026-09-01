use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled};

use crate::service::kb as kb_svc;
use crate::service::rag as rag_svc;

#[derive(Subcommand)]
pub enum KbCommands {
    List,
    Create {
        name: String,
    },
    Delete {
        name: String,
    },
    Ingest {
        name: String,
        #[arg(short, long)]
        path: String,
    },
    Query {
        name: String,
        #[arg(short, long)]
        question: String,
    },
    Clear {
        name: String,
    },
    Stat {
        name: String,
    },
}

pub async fn handle_kb(action: KbCommands) -> Result<()> {
    match action {
        KbCommands::List => list_kb(),
        KbCommands::Create { name } => {
            check_kb_name(&name)?;
            create_kb(name).await
        }
        KbCommands::Delete { name } => {
            check_kb_name(&name)?;
            delete_kb(name).await
        }
        KbCommands::Ingest { name, path } => {
            check_kb_name(&name)?;
            ingest_kb(name, path).await
        }
        KbCommands::Query { name, question } => {
            check_kb_name(&name)?;
            query_kb(name, question).await
        }
        KbCommands::Clear { name } => {
            check_kb_name(&name)?;
            clear_kb(name).await
        }
        KbCommands::Stat { name } => {
            check_kb_name(&name)?;
            stat_kb(name).await
        }
    }
}

// S5 修复: kb 名称经路径段校验, 防 delete "../models" 删除 ~/.fusion/models/ 等目录穿越。
fn check_kb_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("knowledge base name must not be empty");
    }
    crate::service::validate_path_segment("kb name", name)
}

fn list_kb() -> Result<()> {
    println!();
    println!("{}", "📚 Local Knowledge Bases".bold());

    let kb_dir = get_kb_dir();
    if !kb_dir.exists() {
        println!("  {} No knowledge bases found.", "ℹ️".blue());
        println!("  Use `fusion kb create <name>` to create one.");
        return Ok(());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&kb_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let doc_count = std::fs::read_dir(&path).map(|d| d.count()).unwrap_or(0);
            let size = format_bytes(dir_size(&path));
            entries.push(KbEntry {
                name,
                documents: doc_count.to_string(),
                size,
            });
        }
    }

    if entries.is_empty() {
        println!("  {} No knowledge bases found.", "ℹ️".blue());
    } else {
        let table = Table::new(&entries).to_string();
        println!("{}", table);
        println!();
        println!(
            "  Total: {} knowledge bases",
            entries.len().to_string().cyan()
        );
    }

    Ok(())
}

async fn create_kb(name: String) -> Result<()> {
    let kb_dir = get_kb_dir().join(&name);
    if kb_dir.exists() {
        anyhow::bail!("Knowledge base '{}' already exists", name);
    }
    std::fs::create_dir_all(&kb_dir)?;
    let meta = serde_json::json!({
        "name": name,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "document_count": 0,
        "status": "ready",
    });
    std::fs::write(
        kb_dir.join("_meta.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;
    println!("{} Created knowledge base: {}", "✅".green(), name.cyan());
    println!(
        "  Use `fusion kb ingest {} --path=<dir>` to import documents.",
        name.cyan()
    );
    Ok(())
}

async fn delete_kb(name: String) -> Result<()> {
    let kb_dir = get_kb_dir().join(&name);
    if !kb_dir.exists() {
        anyhow::bail!("Knowledge base '{}' not found", name);
    }
    let confirm = dialoguer::Confirm::new()
        .with_prompt(format!(
            "Delete knowledge base '{}'? This cannot be undone.",
            name.cyan()
        ))
        .default(false)
        .interact()?;
    if confirm {
        std::fs::remove_dir_all(&kb_dir)?;
        println!("{} Deleted: {}", "🗑️".green(), name.cyan());
    } else {
        println!("{} Cancelled.", "ℹ️".blue());
    }
    Ok(())
}

async fn ingest_kb(name: String, path: String) -> Result<()> {
    let kb_dir = get_kb_dir().join(&name);
    if !kb_dir.exists() {
        anyhow::bail!(
            "Knowledge base '{}' not found. Create it first with `fusion kb create {}`",
            name,
            name
        );
    }

    let source = std::path::Path::new(&path);
    if !source.exists() {
        anyhow::bail!("Path '{}' not found", path);
    }

    println!("{} Ingesting files into '{}'...", "📥".bold(), name.cyan());
    println!("  Source: {}", path.cyan());

    let mut files = Vec::new();
    if source.is_dir() {
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                files.push(entry.path());
            }
        }
    } else {
        files.push(source.to_path_buf());
    }

    println!("  Found {} files", files.len().to_string().cyan());

    let pb = indicatif::ProgressBar::new(files.len() as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} [{bar:40.green/cyan}] {pos}/{len}")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message("Ingesting...");

    // F6 修复: 记录成功复制数 (非 files.len() 尝试数), 元数据写真实入库文档数。
    let mut success_count = 0u64;
    for file in &files {
        let file_name = file.file_name().unwrap_or_default().to_string_lossy();
        let dest = kb_dir.join(&*file_name);
        match std::fs::copy(file, &dest) {
            Ok(_) => success_count += 1,
            Err(e) => println!("    {} Failed: {} ({})", "⚠️".yellow(), file_name, e),
        }
        pb.inc(1);
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }

    pb.finish_with_message(format!(
        "✅ Ingested {}/{} files into {}",
        success_count,
        files.len(),
        name
    ));
    if success_count < files.len() as u64 {
        println!(
            "  {} {} file(s) failed to ingest; see messages above.",
            "⚠️".yellow(),
            files.len() as u64 - success_count
        );
    }

    let meta_path = kb_dir.join("_meta.json");
    if let Ok(content) = std::fs::read_to_string(&meta_path)
        && let Ok(mut meta) = serde_json::from_str::<serde_json::Value>(&content)
    {
        meta["document_count"] = serde_json::json!(success_count);
        meta["updated_at"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
        let _ = std::fs::write(&meta_path, serde_json::to_string_pretty(&meta)?);
    }

    // 真正向量化: 对成功归集的文件调 fusion-rag ingest (embedding + 向量入库)。
    // fusion-rag 不可用时优雅降级 — 仅文件归集, 提示用户起服务后重跑, 不阻断 (Rule 12 显式可见)。
    if success_count == 0 {
        println!("  {} No files staged, skipping vectorization.", "ℹ️".blue());
        return Ok(());
    }
    println!();
    println!("{} Vectorizing via fusion-rag...", "🔬".bold());
    let rag_alive = rag_svc::health_check().await.unwrap_or(false);
    if !rag_alive {
        println!(
            "  {} fusion-rag not running — files staged but NOT vectorized.",
            "⚠️".yellow()
        );
        println!(
            "     Start it: fusion rag start, then re-run `fusion kb ingest {} --path {}`",
            name, path
        );
        return Ok(());
    }
    let mut vec_ok = 0u64;
    let mut vec_fail = 0u64;
    for file in &files {
        let file_name = file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let dest = kb_dir.join(&*file_name);
        if !dest.exists() {
            continue;
        }
        let content = match std::fs::read_to_string(&dest) {
            Ok(c) => c,
            Err(_) => {
                println!("  {} Skip non-UTF8 file: {}", "⚠️".yellow(), file_name);
                vec_fail += 1;
                continue;
            }
        };
        let ctype = content_type_for(&file_name);
        match rag_svc::ingest_content(&name, &content, &file_name, ctype).await {
            Ok(_) => {
                vec_ok += 1;
                println!("  {} Vectorized: {}", "✅".green(), file_name);
            }
            Err(e) => {
                vec_fail += 1;
                println!(
                    "  {} Vectorize failed: {} ({})",
                    "⚠️".yellow(),
                    file_name,
                    e
                );
            }
        }
    }
    println!(
        "  {} Vectorized {}/{} file(s) into RAG index (kb_id={}).",
        "✅".green(),
        vec_ok,
        vec_ok + vec_fail,
        name.cyan()
    );
    if vec_fail > 0 {
        println!(
            "  {} {} file(s) failed vectorization; see messages above.",
            "⚠️".yellow(),
            vec_fail
        );
    }
    println!("  Query with: fusion rag search {} \"<question>\"", name);

    Ok(())
}

// 按扩展名映射 fusion-rag content_type (CONTENT_TYPE_MAP 子集)。
fn content_type_for(file_name: &str) -> &'static str {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".md") {
        "markdown"
    } else if lower.ends_with(".json") {
        "json"
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        "html"
    } else if lower.ends_with(".csv") {
        "csv"
    } else {
        "text"
    }
}

async fn query_kb(name: String, question: String) -> Result<()> {
    println!(
        "{} Querying knowledge base '{}'...",
        "🔍".bold(),
        name.cyan()
    );
    println!("  Question: {}", question.dimmed());
    println!();

    match kb_svc::query(&name, &question, 5).await {
        Ok(data) => {
            if let Some(answer) = data.get("answer").and_then(|v| v.as_str()) {
                println!("{}", "Answer:".green().bold());
                println!("{}", answer);
            }
            if let Some(sources) = data.get("sources").and_then(|v| v.as_array())
                && !sources.is_empty()
            {
                println!();
                println!("{} Sources:", "📚".blue());
                for source in sources {
                    if let Some(content) = source.get("content").and_then(|v| v.as_str()) {
                        let preview: String = content.chars().take(150).collect();
                        println!("  • {}...", preview.dimmed());
                    }
                }
            }
        }
        Err(e) => {
            println!("  {} Fusion-KB not available: {}", "⬜".yellow(), e);
            println!("     Start with: fusion service start kb");
        }
    }

    Ok(())
}

async fn clear_kb(name: String) -> Result<()> {
    let kb_dir = get_kb_dir().join(&name);
    if !kb_dir.exists() {
        anyhow::bail!("Knowledge base '{}' not found", name);
    }

    let confirm = dialoguer::Confirm::new()
        .with_prompt(format!("Clear all documents in '{}'?", name.cyan()))
        .default(false)
        .interact()?;

    if confirm {
        for entry in std::fs::read_dir(&kb_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.file_name().unwrap_or_default() != "_meta.json" {
                std::fs::remove_file(&path)?;
            }
        }
        println!("{} Cleared knowledge base: {}", "✅".green(), name.cyan());
    } else {
        println!("{} Cancelled.", "ℹ️".blue());
    }

    Ok(())
}

async fn stat_kb(name: String) -> Result<()> {
    let kb_dir = get_kb_dir().join(&name);
    if !kb_dir.exists() {
        anyhow::bail!("Knowledge base '{}' not found", name);
    }

    let mut doc_count = 0u64;
    let mut total_size = 0u64;

    for entry in std::fs::read_dir(&kb_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.file_name().unwrap_or_default() != "_meta.json" {
            doc_count += 1;
            total_size += path.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }

    println!();
    println!("{} Knowledge Base: {}", "📊".bold(), name.cyan());
    println!("  Documents: {}", doc_count.to_string().cyan());
    println!("  Size:      {}", format_bytes(total_size).cyan());
    println!("  Location:  {}", kb_dir.display().to_string().cyan());

    let meta_path = kb_dir.join("_meta.json");
    if let Ok(content) = std::fs::read_to_string(&meta_path)
        && let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(created) = meta.get("created_at").and_then(|v| v.as_str())
    {
        println!("  Created:   {}", created.cyan());
    }

    Ok(())
}

fn get_kb_dir() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_default();
    home.join(".fusion").join("kb")
}

fn dir_size(path: &std::path::Path) -> u64 {
    fn walk(dir: &std::path::Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    total += walk(&path);
                } else if let Ok(meta) = path.metadata() {
                    total += meta.len();
                }
            }
        }
        total
    }
    walk(path)
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

#[derive(Tabled)]
struct KbEntry {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Documents")]
    documents: String,
    #[tabled(rename = "Size")]
    size: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_for_extensions() {
        assert_eq!(content_type_for("notes.md"), "markdown");
        assert_eq!(content_type_for("data.json"), "json");
        assert_eq!(content_type_for("page.html"), "html");
        assert_eq!(content_type_for("page.htm"), "html");
        assert_eq!(content_type_for("rows.csv"), "csv");
        assert_eq!(content_type_for("readme.txt"), "text");
        assert_eq!(content_type_for("noext"), "text");
        assert_eq!(content_type_for("UPPER.MD"), "markdown");
    }

    #[test]
    fn test_check_kb_name_rejects_separator() {
        assert!(check_kb_name("../escape").is_err());
        assert!(check_kb_name("a/b").is_err());
        assert!(check_kb_name("ok-name").is_ok());
        assert!(check_kb_name("").is_err());
    }
}
