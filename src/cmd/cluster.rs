use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled, settings::Style};
use tracing::error;

use crate::service::multinode as mn;
use crate::utils::output;

// 集群管理 — 对接 fusion-multi-node Master (11452), 非 MLX 网关。
// 旧实现误打到 mlx.base_url/api/cluster/status (网关无此路由), 已修正直连 Master。
#[derive(Subcommand)]
pub enum ClusterCommands {
    /// 查看集群状态
    Status,
    /// 列出所有节点
    Nodes,
    /// 查看单个节点详情
    Node { node_id: String },
    /// 移除节点
    Remove { node_id: String },
    /// 待审批节点列表
    Pending,
    /// 审批通过节点
    Approve {
        node_id: String,
        #[arg(long, default_value = "fusion-cli")]
        approved_by: String,
    },
    /// 拒绝节点
    Reject {
        node_id: String,
        #[arg(long, default_value = "rejected")]
        reason: String,
    },
    /// 路由策略摘要
    Routing,
}

pub async fn handle_cluster(action: ClusterCommands) -> Result<()> {
    match action {
        ClusterCommands::Status => cluster_status().await,
        ClusterCommands::Nodes => cluster_nodes().await,
        ClusterCommands::Node { node_id } => cluster_node(node_id).await,
        ClusterCommands::Remove { node_id } => cluster_remove(node_id).await,
        ClusterCommands::Pending => cluster_pending().await,
        ClusterCommands::Approve {
            node_id,
            approved_by,
        } => cluster_approve(node_id, approved_by).await,
        ClusterCommands::Reject { node_id, reason } => cluster_reject(node_id, reason).await,
        ClusterCommands::Routing => cluster_routing().await,
    }
}

async fn cluster_status() -> Result<()> {
    println!();
    println!("{}", "🌐 Cluster Status (multi-node Master)".bold());
    println!();

    match mn::cluster_status().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!("  {} Cluster Master is reachable.", "✅".green());
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "cluster status error");
            if output::is_json_mode() {
                anyhow::bail!("Failed to get cluster status: {}", e);
            }
            println!("  {} Cannot reach multi-node Master: {}", "❌".red(), e);
            println!("     Start Master: fusion-multi-node/start.sh (port 11452)");
            anyhow::bail!("Failed to get cluster status: {}", e);
        }
    }

    Ok(())
}

async fn cluster_nodes() -> Result<()> {
    println!();
    println!("{}", "🖥️  Cluster Nodes (multi-node Master)".bold());
    println!();

    match mn::list_nodes().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let nodes = data.as_array().cloned().unwrap_or_default();
            if nodes.is_empty() {
                println!("  {} No nodes registered.", "ℹ️".blue());
                return Ok(());
            }
            let mut entries = Vec::new();
            for n in &nodes {
                let id = n
                    .get("node_id")
                    .or_else(|| n.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let ip = n
                    .get("ip_address")
                    .or_else(|| n.get("ip"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let state = n
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let role = n
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                entries.push(NodeEntry {
                    id,
                    ip,
                    state,
                    role,
                });
            }
            let mut table = Table::new(&entries);
            table.with(Style::modern());
            println!("{}", table);
            println!();
            println!("  Total: {} nodes", entries.len().to_string().cyan());
        }
        Err(e) => {
            error!(error = %e, "list nodes error");
            println!("  {} Cannot reach multi-node Master: {}", "❌".red(), e);
            anyhow::bail!("Failed to list nodes: {}", e);
        }
    }

    Ok(())
}

async fn cluster_node(node_id: String) -> Result<()> {
    println!();
    println!("{} {}", "🔍 Node Detail".bold(), node_id.cyan());
    println!();

    match mn::get_node(&node_id).await {
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
            error!(error = %e, "get node error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Failed to get node {}: {}", node_id, e);
        }
    }

    Ok(())
}

async fn cluster_remove(node_id: String) -> Result<()> {
    println!("{} Removing node {}...", "⏹️".bold(), node_id.cyan());

    match mn::remove_node(&node_id).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!("  {} Node {} removed.", "✅".green(), node_id.cyan());
        }
        Err(e) => {
            error!(error = %e, "remove node error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Failed to remove node {}: {}", node_id, e);
        }
    }

    Ok(())
}

async fn cluster_pending() -> Result<()> {
    println!();
    println!("{}", "⏳ Pending Nodes (awaiting approval)".bold());
    println!();

    match mn::pending_nodes().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let nodes = data.as_array().cloned().unwrap_or_default();
            if nodes.is_empty() {
                println!("  {} No pending nodes.", "ℹ️".blue());
                return Ok(());
            }
            let mut entries = Vec::new();
            for n in &nodes {
                let id = n
                    .get("node_id")
                    .or_else(|| n.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let ip = n
                    .get("ip_address")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                entries.push(PendingEntry { id, ip });
            }
            let mut table = Table::new(&entries);
            table.with(Style::modern());
            println!("{}", table);
            println!();
            println!("  Approve: fusion cluster approve <id>");
        }
        Err(e) => {
            error!(error = %e, "pending nodes error");
            println!("  {} Cannot reach multi-node Master: {}", "❌".red(), e);
            anyhow::bail!("Failed to get pending nodes: {}", e);
        }
    }

    Ok(())
}

async fn cluster_approve(node_id: String, approved_by: String) -> Result<()> {
    println!(
        "{} Approving node {} (by {})...",
        "✅".bold(),
        node_id.cyan(),
        approved_by.cyan()
    );

    match mn::approve_node(&node_id, &approved_by).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!("  {} Node {} approved.", "✅".green(), node_id.cyan());
        }
        Err(e) => {
            error!(error = %e, "approve node error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Failed to approve node {}: {}", node_id, e);
        }
    }

    Ok(())
}

async fn cluster_reject(node_id: String, reason: String) -> Result<()> {
    println!(
        "{} Rejecting node {} (reason: {})...",
        "⚠️".bold(),
        node_id.cyan(),
        reason.cyan()
    );

    match mn::reject_node(&node_id, &reason).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!("  {} Node {} rejected.", "⚠️".yellow(), node_id.cyan());
        }
        Err(e) => {
            error!(error = %e, "reject node error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Failed to reject node {}: {}", node_id, e);
        }
    }

    Ok(())
}

async fn cluster_routing() -> Result<()> {
    println!();
    println!("{}", "🧭 Routing Summary (multi-node Master)".bold());
    println!();

    match mn::routing_summary().await {
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
            error!(error = %e, "routing summary error");
            println!("  {} Cannot reach multi-node Master: {}", "❌".red(), e);
            anyhow::bail!("Failed to get routing summary: {}", e);
        }
    }

    Ok(())
}

#[derive(Tabled)]
struct NodeEntry {
    #[tabled(rename = "Node ID")]
    id: String,
    #[tabled(rename = "IP")]
    ip: String,
    #[tabled(rename = "State")]
    state: String,
    #[tabled(rename = "Role")]
    role: String,
}

#[derive(Tabled)]
struct PendingEntry {
    #[tabled(rename = "Node ID")]
    id: String,
    #[tabled(rename = "IP")]
    ip: String,
}
