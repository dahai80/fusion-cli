use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tracing::{info, error};

#[derive(Subcommand)]
pub enum ClusterCommands {
    /// 查看集群状态
    Status,
}

pub async fn handle_cluster(action: ClusterCommands) -> Result<()> {
    match action {
        ClusterCommands::Status => cluster_status().await,
    }
}

async fn cluster_status() -> Result<()> {
    println!();
    println!("{}", "🌐 Cluster Status".bold());
    println!();

    let base_url = get_base_url();
    let url = format!("{}/api/cluster/status", base_url);
    info!(url = %url, "Requesting cluster status");

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await;

    match resp {
        Ok(resp) if resp.status().is_success() => {
            let body = resp.text().await?;
            info!(body = %body, "Cluster status response received");
            println!("  {} Cluster is reachable.", "✅".green());
            match serde_json::from_str::<serde_json::Value>(&body) {
                Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap_or(body)),
                Err(_) => println!("{}", body),
            }
        }
        Ok(resp) => {
            let status = resp.status();
            error!(status = %status, "Cluster status request failed");
            println!("  {} Cluster returned HTTP {}", "⚠️".yellow(), status);
            anyhow::bail!("Failed to get cluster status: HTTP {}", status);
        }
        Err(e) => {
            error!(error = %e, "Cluster status request error");
            println!("  {} Cannot reach cluster at {}", "❌".red(), base_url.cyan());
            anyhow::bail!("Failed to connect to cluster: {}", e);
        }
    }

    Ok(())
}

fn get_base_url() -> String {
    std::env::var("FUSION_MLX_URL").unwrap_or_else(|_| "http://localhost:11434".to_string())
}
