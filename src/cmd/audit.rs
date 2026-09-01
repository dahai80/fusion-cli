// fusion audit — 审计轨迹查看 (合规只读视图)。
// 操作日志由 main.rs::async_main 统一写入 ~/.fusion/audit/audit.log。

use anyhow::Result;
use clap::Subcommand;
use colored::*;

#[derive(Subcommand)]
pub enum AuditCommands {
    /// 查看最近 N 条审计记录
    View {
        /// 显示条数
        #[arg(short, long, default_value_t = 50)]
        count: usize,
    },
    /// 显示审计日志路径
    Path,
    /// #51 校验审计日志 hash chain 完整性 (防篡改)
    Verify,
}

pub async fn handle_audit(action: AuditCommands) -> Result<()> {
    match action {
        AuditCommands::View { count } => view_audit(count).await,
        AuditCommands::Path => {
            println!(
                "{} Audit log: {}",
                "📋".bold(),
                crate::utils::audit::audit_path_display().cyan()
            );
            Ok(())
        }
        AuditCommands::Verify => {
            println!(
                "{} Verifying audit hash chain at {}",
                "🔗".bold(),
                crate::utils::audit::audit_path_display().cyan()
            );
            match crate::utils::audit::verify_chain() {
                Ok(()) => {
                    println!(
                        "{} Audit chain intact — no tampering detected.",
                        "✅".green().bold()
                    );
                    Ok(())
                }
                Err(e) => {
                    println!("{} Audit chain BROKEN: {}", "❌".red().bold(), e);
                    println!("     Records above the break may be corrupted or modified in place.");
                    std::process::exit(1);
                }
            }
        }
    }
}

async fn view_audit(count: usize) -> Result<()> {
    let recs = crate::utils::audit::read_recent(count)?;
    if recs.is_empty() {
        println!(
            "{} No audit records yet at {}",
            "ℹ️".blue(),
            crate::utils::audit::audit_path_display().cyan()
        );
        return Ok(());
    }
    println!(
        "{} Last {} audit records:",
        "📋".bold(),
        recs.len().to_string().cyan()
    );
    println!("{}", "─".repeat(80).dimmed());
    for r in &recs {
        let icon = if r.outcome == "ok" {
            "✅".green()
        } else {
            "❌".red()
        };
        let ts = format_ts(r.ts);
        println!(
            "{} {} | {:<10} | {:<6} | {}ms | {}",
            icon,
            ts.dimmed(),
            r.command.cyan(),
            r.outcome.yellow(),
            r.duration_ms.to_string().dimmed(),
            r.detail.dimmed()
        );
    }
    println!("{}", "─".repeat(80).dimmed());
    println!(
        "{} Path: {}",
        "ℹ️".blue(),
        crate::utils::audit::audit_path_display().cyan()
    );
    Ok(())
}

fn format_ts(ts: f64) -> String {
    let secs = ts as u64;
    let sub = ((ts - secs as f64) * 1000.0) as u32;
    let dt = chrono::DateTime::from_timestamp(secs as i64, 0)
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| secs.to_string());
    format!("{}.{:03}", dt, sub)
}
