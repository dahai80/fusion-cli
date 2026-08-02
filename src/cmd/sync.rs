use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tracing::{error, info};

use crate::service::{ServiceUrls, get_client};

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
    println!();
    println!("{}", "📋 Model Sync Manifest".bold());
    println!("  Model: {}", model_name.cyan());
    println!();

    let base_url = get_base_url();
    let url = format!("{}/api/models/{}/manifest", base_url, model_name);
    info!(url = %url, "Requesting model manifest");

    let client = get_client();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match resp {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await?;
            info!(body = %body, "Manifest response received");
            println!("  {} Manifest for '{}':", "✅".green(), model_name.cyan());
            println!();
            match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap_or(body)),
                Err(_) => println!("{}", body),
            }
        }
        Ok(resp) => {
            let status = resp.status();
            error!(status = %status, "Manifest request failed");
            anyhow::bail!(
                "Failed to get manifest for '{}': HTTP {}",
                model_name,
                status
            );
        }
        Err(e) => {
            error!(error = %e, "Manifest request error");
            anyhow::bail!("Failed to connect to fusion-mlx for manifest: {}", e);
        }
    }

    Ok(())
}

async fn sync_incremental(source: String, model_name: String) -> Result<()> {
    println!();
    println!("{}", "🔄 Incremental Sync".bold());
    println!("  Model:  {}", model_name.cyan());
    println!("  Source: {}", source.cyan());
    println!();

    let base_url = get_base_url();
    let url = format!("{}/api/sync/incremental", base_url);
    info!(url = %url, source = %source, model = %model_name, "Starting incremental sync");

    let payload = serde_json::json!({
        "model_name": model_name,
        "source": source,
    });

    let client = get_client();
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await;

    match resp {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await?;
            info!(body = %body, "Incremental sync response received");
            println!(
                "  {} Incremental sync for '{}' completed.",
                "✅".green(),
                model_name.cyan()
            );
            match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap_or(body)),
                Err(_) => println!("{}", body),
            }
        }
        Ok(resp) => {
            let status = resp.status();
            error!(status = %status, "Incremental sync request failed");
            anyhow::bail!(
                "Incremental sync failed for '{}': HTTP {}",
                model_name,
                status
            );
        }
        Err(e) => {
            error!(error = %e, "Incremental sync request error");
            anyhow::bail!(
                "Failed to connect to fusion-mlx for incremental sync: {}",
                e
            );
        }
    }

    Ok(())
}

fn get_base_url() -> String {
    ServiceUrls::from_config()
        .mlx
        .trim_end_matches('/')
        .to_string()
}
