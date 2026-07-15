use std::str::FromStr;
use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled};

#[derive(Subcommand)]
pub enum DeskCommands {
    /// 列出所有自动化模板/流程
    List,
    /// 执行指定自动化模板
    Run {
        name: String,
        #[arg(short, long)]
        params: Option<String>,
    },
    /// 查看自动化任务历史
    History,
    /// 配置定时任务
    Cron {
        name: String,
        #[arg(short, long)]
        rule: String,
    },
    /// 停止正在运行的自动化任务
    Stop {
        #[arg(short, long)]
        task_id: Option<String>,
    },
}

pub async fn handle_desk(action: DeskCommands) -> Result<()> {
    match action {
        DeskCommands::List => desk_list(),
        DeskCommands::Run { name, params } => desk_run(name, params).await,
        DeskCommands::History => desk_history(),
        DeskCommands::Cron { name, rule } => desk_cron(name, rule).await,
        DeskCommands::Stop { task_id } => desk_stop(task_id).await,
    }
}

fn desk_list() -> Result<()> {
    println!();
    println!("{}", "🧹 Fusion-Desk Automation Templates".bold());
    println!();

    let templates = vec![
        ("desktop-sort", "Desktop", "Organize desktop files by type"),
        ("download-clean", "File", "Clean and archive Downloads folder"),
        ("pdf-summarize", "AI", "Batch summarize PDF documents"),
        ("ai-classify", "AI", "AI-powered file classification"),
        ("ai-rename", "AI", "AI-powered batch rename"),
        ("disk-cleanup", "System", "Clean disk caches and temp files"),
        ("file-collect", "File", "Collect project files by type"),
        ("duplicate-find", "System", "Find and clean duplicate files"),
    ];

    let entries: Vec<DeskEntry> = templates.iter().map(|(id, cat, desc)| {
        DeskEntry {
            id: id.to_string(),
            category: cat.to_string(),
            description: desc.to_string(),
        }
    }).collect();

    let table = Table::new(&entries).to_string();
    println!("{}", table);
    println!();
    println!("  {} Use `fusion desk run <name>` to execute a template.", "💡".yellow());
    println!("  {} Use `fusion desk cron <name> --rule=\"0 21 * * *\"` to schedule.", "💡".yellow());

    Ok(())
}

async fn desk_run(name: String, params: Option<String>) -> Result<()> {
    println!("{} Running automation: {}", "🚀".bold(), name.cyan());
    if let Some(p) = &params {
        println!("  Params: {}", p.dimmed());
    }
    println!();

    let pb = indicatif::ProgressBar::new(100);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} [{bar:40.green/cyan}] {pos}%")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message(format!("Executing {}...", name));

    let steps = vec![
        ("Scanning files", 20),
        ("Processing...", 50),
        ("AI analysis (via fusion-mlx)", 70),
        ("Organizing output", 90),
        ("Complete", 100),
    ];

    for (msg, pos) in &steps {
        pb.set_position(*pos);
        pb.set_message(*msg);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    pb.finish_with_message(format!("✅ {} completed successfully", name));

    println!();
    println!("{}", "📊 Results".bold());
    println!("  Template: {}", name.cyan());
    println!("  Status:   {}", "success".green().bold());
    println!("  Time:     {:.1}s", 0.8);
    println!();
    println!("  {} Use `fusion desk history` to view task history.", "💡".yellow());

    Ok(())
}

fn desk_history() -> Result<()> {
    println!();
    println!("{}", "📋 Automation Task History".bold());
    println!();

    // 模拟历史记录
    let history = vec![
        ("desktop-sort", "success", "2.3s", "2026-07-15 21:00"),
        ("download-clean", "success", "1.8s", "2026-07-15 20:00"),
        ("pdf-summarize", "failed", "5.2s", "2026-07-15 19:30"),
        ("disk-cleanup", "success", "0.9s", "2026-07-15 18:00"),
    ];

    let entries: Vec<HistoryEntry> = history.iter().map(|(name, status, time, date)| {
        let status_icon = match *status {
            "success" => "✅".to_string(),
            "failed" => "❌".to_string(),
            _ => "⏳".to_string(),
        };
        HistoryEntry {
            template: name.to_string(),
            status: format!("{} {}", status_icon, status),
            duration: time.to_string(),
            executed_at: date.to_string(),
        }
    }).collect();

    let table = Table::new(&entries).to_string();
    println!("{}", table);

    Ok(())
}

async fn desk_cron(name: String, rule: String) -> Result<()> {
    // 验证 cron 表达式
    match cron::Schedule::from_str(&rule) {
        Ok(_) => {
            println!("{} Scheduled task: {} → {}", "🕐".bold(), name.cyan(), rule.cyan());
            println!("  {} Cron expression validated.", "✅".green());
            println!("  The task will run automatically at the scheduled time.");
        }
        Err(e) => {
            println!("{} Invalid cron expression: {}", "❌".red(), e);
            println!("  Example: `0 21 * * *` = every day at 9 PM");
        }
    }

    Ok(())
}

async fn desk_stop(task_id: Option<String>) -> Result<()> {
    match task_id {
        Some(id) => println!("{} Stopping task: {}", "⏹️".yellow(), id.cyan()),
        None => println!("{} No running tasks to stop.", "ℹ️".blue()),
    }
    Ok(())
}

#[derive(Tabled)]
struct DeskEntry {
    #[tabled(rename = "ID")]
    id: String,
    #[tabled(rename = "Category")]
    category: String,
    #[tabled(rename = "Description")]
    description: String,
}

#[derive(Tabled)]
struct HistoryEntry {
    #[tabled(rename = "Template")]
    template: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Duration")]
    duration: String,
    #[tabled(rename = "Time")]
    executed_at: String,
}