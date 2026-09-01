use anyhow::Result;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

// fusion-multi-node Master HTTP API (port 11452, see architecture/port-registry.yaml)。
// 路由源: fusion_multi_node/server/master_server.py (FastAPI @app.get/post)。
// 注意: Master 走 own HTTP server, 非 MLX 网关 — 旧 CLI 误把 /api/cluster/status 打到
// mlx.base_url (11432), 网关无此路由必 404。本模块直连 Master base_url。

fn base_url() -> String {
    ServiceUrls::from_config()
        .multinode
        .trim_end_matches('/')
        .to_string()
}

#[allow(dead_code)]
pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/api/health", base_url());
    match client
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

#[allow(dead_code)]
pub async fn get_health_detail() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/health/deep", base_url());
    info!(url = %url, "multi-node deep health");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn cluster_status() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/cluster/status", base_url());
    info!(url = %url, "multi-node cluster status");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn list_nodes() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes", base_url());
    info!(url = %url, "multi-node list nodes");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn get_node(node_id: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes/{}", base_url(), node_id);
    info!(url = %url, node_id = %node_id, "multi-node get node");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn remove_node(node_id: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes/{}", base_url(), node_id);
    info!(url = %url, node_id = %node_id, "multi-node remove node");
    let resp = client
        .delete(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn pending_nodes() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes/pending", base_url());
    info!(url = %url, "multi-node pending nodes");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn approve_node(node_id: &str, approved_by: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes/approve", base_url());
    info!(url = %url, node_id = %node_id, "multi-node approve node");
    let payload = serde_json::json!({
        "node_id": node_id,
        "approved_by": approved_by,
    });
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn reject_node(node_id: &str, reason: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes/reject", base_url());
    info!(url = %url, node_id = %node_id, "multi-node reject node");
    let payload = serde_json::json!({
        "node_id": node_id,
        "reason": reason,
    });
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn routing_summary() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/routing/summary", base_url());
    info!(url = %url, "multi-node routing summary");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn model_manifest(model_name: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/models/{}/manifest", base_url(), model_name);
    info!(url = %url, model = %model_name, "multi-node model manifest");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

// Master 的 /api/sync/incremental: 请求 Master 从 source_node 拉取模型增量。
// payload: {model_name, source} — source 为 "host:port" 节点地址。
pub async fn sync_incremental(source: &str, model_name: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/sync/incremental", base_url());
    info!(url = %url, source = %source, model = %model_name, "multi-node incremental sync");
    let payload = serde_json::json!({
        "model_name": model_name,
        "source": source,
    });
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(60))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}
