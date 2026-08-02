use anyhow::Result;
use clap::Subcommand;
use colored::*;
use std::str::FromStr;
use tabled::{Table, Tabled};
use tracing::info;

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
    History {
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
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
        DeskCommands::List => desk_list().await,
        DeskCommands::Run { name, params } => desk_run(&name, params).await,
        DeskCommands::History { limit } => desk_history(limit).await,
        DeskCommands::Cron { name, rule } => desk_cron(&name, &rule).await,
        DeskCommands::Stop { task_id } => desk_stop(task_id).await,
    }
}

async fn desk_list() -> Result<()> {
    println!();
    println!("{}", "🧹 Fusion-Desk Automation Templates".bold());
    println!();

    let alive = crate::service::desk::health_check().await.unwrap_or(false);
    if !alive {
        println!("  {} Fusion-Desk service is not running.", "⚠".yellow());
        println!("  Start it with: fusion service start desk");
        println!();
        println!("  Available built-in templates:");
        let fallback = [
            ("desktop-sort", "Desktop", "Organize desktop files by type"),
            (
                "download-clean",
                "File",
                "Clean and archive Downloads folder",
            ),
            ("pdf-summarize", "AI", "Batch summarize PDF documents"),
            ("ai-classify", "AI", "AI-powered file classification"),
            ("ai-rename", "AI", "AI-powered batch rename"),
            ("disk-cleanup", "System", "Clean disk caches and temp files"),
            ("file-collect", "File", "Collect project files by type"),
            ("duplicate-find", "System", "Find and clean duplicate files"),
        ];
        let entries: Vec<DeskEntry> = fallback
            .iter()
            .map(|(id, cat, desc)| DeskEntry {
                id: id.to_string(),
                category: cat.to_string(),
                description: desc.to_string(),
            })
            .collect();
        let mut table = Table::new(&entries);
        table.with(tabled::settings::Style::modern());
        table.with(tabled::settings::Width::increase(10));
        println!("{}", table);
        return Ok(());
    }

    match crate::service::desk::list_templates().await {
        Ok(templates) => {
            if templates.is_empty() {
                println!("  {} No templates available.", "ℹ️".blue());
            } else {
                let entries: Vec<DeskEntry> = templates
                    .iter()
                    .map(|t| DeskEntry {
                        id: t.id.clone(),
                        category: t.name.clone(),
                        description: t.description.clone(),
                    })
                    .collect();
                let mut table = Table::new(&entries);
                table.with(tabled::settings::Style::modern());
                table.with(tabled::settings::Width::increase(10));
                println!("{}", table);
            }
        }
        Err(e) => {
            info!(error = %e, "Failed to list desk templates");
            println!("  {} Failed to fetch templates: {}", "⚠".yellow(), e);
        }
    }

    println!();
    println!(
        "  {} Use `fusion desk run <name>` to execute a template.",
        "💡".yellow()
    );
    println!(
        "  {} Use `fusion desk cron <name> --rule=\"0 21 * * *\"` to schedule.",
        "💡".yellow()
    );

    Ok(())
}

async fn desk_run(name: &str, params: Option<String>) -> Result<()> {
    println!("{} Running automation: {}", "🚀".bold(), name.cyan());
    if let Some(p) = &params {
        println!("  Params: {}", p.dimmed());
    }
    println!();

    let alive = crate::service::desk::health_check().await.unwrap_or(false);
    if !alive {
        anyhow::bail!(
            "Fusion-Desk service is not running. Start it with: fusion service start desk"
        );
    }

    info!(template = name, params = ?params, "Running desk automation task");

    match crate::service::desk::run_task(name).await {
        Ok(task_id) => {
            if task_id.is_empty() {
                println!("  {} Task submitted (no task ID returned).", "✅".green());
            } else {
                println!("  {} Task submitted: {}", "✅".green(), task_id.cyan());
            }
            println!();
            println!(
                "  {} Use `fusion desk history` to view task history.",
                "💡".yellow()
            );
        }
        Err(e) => {
            info!(error = %e, "Desk task failed");
            anyhow::bail!("Failed to run desk task: {}", e);
        }
    }

    Ok(())
}

async fn desk_history(limit: u32) -> Result<()> {
    println!();
    println!("{}", "📋 Automation Task History".bold());
    println!();

    let alive = crate::service::desk::health_check().await.unwrap_or(false);
    if !alive {
        println!(
            "  {} Fusion-Desk service is not running. No history available.",
            "⚠".yellow()
        );
        return Ok(());
    }

    match crate::service::desk::get_history(limit).await {
        Ok(history) => {
            if history.is_empty() {
                println!("  {} No task history found.", "ℹ️".blue());
            } else {
                let entries: Vec<HistoryEntry> = history
                    .iter()
                    .map(|h| {
                        let status_icon = match h.status.as_str() {
                            "success" | "completed" => "✅".to_string(),
                            "failed" | "error" => "❌".to_string(),
                            "running" => "▶️".to_string(),
                            _ => "⏳".to_string(),
                        };
                        HistoryEntry {
                            template: h.task.clone(),
                            status: format!("{} {}", status_icon, h.status),
                            duration: format!("{} → {}", h.started_at, h.finished_at),
                            task_id: h.id.clone(),
                        }
                    })
                    .collect();
                let mut table = Table::new(&entries);
                table.with(tabled::settings::Style::modern());
                table.with(tabled::settings::Width::increase(10));
                println!("{}", table);
            }
        }
        Err(e) => {
            info!(error = %e, "Failed to fetch desk history");
            println!("  {} Failed to fetch history: {}", "⚠".yellow(), e);
        }
    }

    Ok(())
}

async fn desk_cron(name: &str, rule: &str) -> Result<()> {
    match cron::Schedule::from_str(rule) {
        Ok(schedule) => {
            println!(
                "{} Scheduled task: {} → {}",
                "🕐".bold(),
                name.cyan(),
                rule.cyan()
            );
            println!("  {} Cron expression validated.", "✅".green());
            if let Some(next) = schedule.upcoming(chrono::Local).next() {
                println!("  Next run: {}", next.to_string().cyan());
            }
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
        Some(id) => {
            println!("{} Stopping task: {}", "⏹️".yellow(), id.cyan());
            let alive = crate::service::desk::health_check().await.unwrap_or(false);
            if !alive {
                anyhow::bail!("Fusion-Desk service is not running. Cannot stop task.");
            }
            match crate::service::desk::stop_task(&id).await {
                Ok(true) => println!("  {} Task {} stopped.", "✅".green(), id.cyan()),
                Ok(false) => println!(
                    "  {} Task {} could not be stopped (not running?).",
                    "⚠".yellow(),
                    id.cyan()
                ),
                Err(e) => anyhow::bail!("Failed to stop task: {}", e),
            }
        }
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
    #[tabled(rename = "Time")]
    duration: String,
    #[tabled(rename = "Task ID")]
    task_id: String,
}
