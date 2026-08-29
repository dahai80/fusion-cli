use anyhow::Result;
use clap::Subcommand;
use colored::*;

use crate::service::guard as guard_svc;
use crate::utils::output::is_json_mode;

#[derive(Subcommand)]
pub enum GuardCommands {
    /// Show guard daemon status (ping)
    Status,
    /// List current guard rules + epoch
    Rules,
    /// Show recent guard audit events
    Audit {
        #[arg(short, long, default_value_t = 20)]
        limit: u32,
    },
}

pub async fn handle_guard(action: GuardCommands) -> Result<()> {
    match action {
        GuardCommands::Status => guard_status().await,
        GuardCommands::Rules => guard_rules().await,
        GuardCommands::Audit { limit } => guard_audit(limit).await,
    }
}

async fn guard_status() -> Result<()> {
    println!();
    println!("{}", "🛡️  Fusion-Guard Status".bold());
    println!();

    match guard_svc::ping() {
        Ok(p) => {
            if is_json_mode() {
                let json = serde_json::json!({
                    "alive": p.pong,
                    "version": p.version,
                    "rules_epoch": p.rules_epoch,
                });
                println!("{}", serde_json::to_string_pretty(&json)?);
                return Ok(());
            }
            let mark = if p.pong {
                "✅ alive"
            } else {
                "❌ not responding"
            };
            println!("  {}", mark.green());
            println!("  {} {}", "Version:".dimmed(), p.version.cyan());
            println!(
                "  {} {}",
                "Rules epoch:".dimmed(),
                p.rules_epoch.to_string().cyan()
            );
            println!();
        }
        Err(e) => {
            println!("  {} guard daemon not reachable: {}", "⬜".yellow(), e);
            println!("     Socket: /tmp/fusion-guard.sock (override with FUSION_GUARD_SOCK)");
            println!("     Is fusion-guard running?");
        }
    }
    Ok(())
}

async fn guard_rules() -> Result<()> {
    println!();
    println!("{}", "📜 Fusion-Guard Rules".bold());
    println!();

    match guard_svc::list_rules() {
        Ok(rules) => {
            println!("{}", serde_json::to_string_pretty(&rules)?);
            println!();
        }
        Err(e) => {
            println!("  {} guard daemon not reachable: {}", "⬜".yellow(), e);
            println!("     Start fusion-guard, then retry: fusion guard rules");
        }
    }
    Ok(())
}

async fn guard_audit(limit: u32) -> Result<()> {
    println!();
    println!("{} (limit {})", "🔍 Fusion-Guard Audit Log".bold(), limit);
    println!();

    match guard_svc::list_audit(limit) {
        Ok(audit) => {
            println!("{}", serde_json::to_string_pretty(&audit)?);
            println!();
        }
        Err(e) => {
            println!("  {} guard daemon not reachable: {}", "⬜".yellow(), e);
            println!("     Start fusion-guard, then retry: fusion guard audit");
        }
    }
    Ok(())
}
