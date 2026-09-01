use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tracing::error;

use crate::service::multinode as mn;
use crate::utils::output;

// 模型同步 — 对接 fusion-multi-node Master (11452), 非 MLX 网关。
// 旧实现误打到 mlx.base_url/api/... (网关无此路由), 已修正直连 Master。
#[derive(Subcommand)]
pub enum SyncCommands {
    /// 获取模型同步清单
    Manifest {
        /// 模型名称
        model_name: String,
    },
    /// 增量同步模型数据
    Incremental {
        /// 源节点地址 (host:port)
        #[arg(long)]
        source: String,
        /// 模型名称
        model_name: String,
    },
}

pub async fn handle_sync(action: SyncCommands) -> Result<()> {
    match action {
        SyncCommands::Manifest { model_name } => sync_manifest(model_name).await,
        SyncCommands::Incremental { source, model_name } => {
            sync_incremental(source, model_name).await
        }
    }
}

async fn sync_manifest(model_name: String) -> Result<()> {
    if output::is_json_mode() {
        match mn::model_manifest(&model_name).await {
            Ok(data) => output::print_json(&data)?,
            Err(e) => {
                error!(error = %e, "manifest request error");
                anyhow::bail!("Failed to get manifest from multi-node Master: {}", e)
            }
        }
        return Ok(());
    }

    println!();
    println!("{}", "📋 Model Sync Manifest (multi-node Master)".bold());
    println!("  Model: {}", model_name.cyan());
    println!();

    match mn::model_manifest(&model_name).await {
        Ok(data) => {
            println!("  {} Manifest for '{}':", "✅".green(), model_name.cyan());
            println!();
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "manifest request error");
            println!("  {} multi-node Master not reachable: {}", "❌".red(), e);
            println!("     Start Master: fusion-multi-node/start.sh (port 11452)");
            anyhow::bail!("Failed to get manifest: {}", e);
        }
    }

    Ok(())
}

async fn sync_incremental(source: String, model_name: String) -> Result<()> {
    if output::is_json_mode() {
        match mn::sync_incremental(&source, &model_name).await {
            Ok(data) => output::print_json(&data)?,
            Err(e) => {
                error!(error = %e, "incremental sync error");
                anyhow::bail!("Incremental sync failed: {}", e)
            }
        }
        return Ok(());
    }

    println!();
    println!("{}", "🔄 Incremental Sync (multi-node Master)".bold());
    println!("  Model:  {}", model_name.cyan());
    println!("  Source: {}", source.cyan());
    println!();

    match mn::sync_incremental(&source, &model_name).await {
        Ok(data) => {
            println!(
                "  {} Incremental sync for '{}' completed.",
                "✅".green(),
                model_name.cyan()
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "incremental sync error");
            println!("  {} multi-node Master not reachable: {}", "❌".red(), e);
            println!("     Start Master: fusion-multi-node/start.sh (port 11452)");
            anyhow::bail!("Incremental sync failed: {}", e);
        }
    }

    Ok(())
}
