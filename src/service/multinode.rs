use anyhow::Result;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

// fusion-multi-node Master HTTP API (port 11452, see architecture/port-registry.yaml)。
// 路由源: fusion_multi_node/server/master_server.py (FastAPI @app.get/post)。
// 认证: BearerAuthMiddleware (fusion_multi_node/utils/auth.py) — 除 /api/health*、
// /docs、/ 外所有路由强制 Bearer cluster_token。token 源:
//   FUSION_CLUSTER_TOKEN env 或 ~/.fusion/multi-node/.cluster_token 文件。
// CLI 侧用 config multinode.api_key 携带; 未配则不发 (仅 /api/health 可用)。
// 注意: Master 走 own HTTP server, 非 MLX 网关 — 旧 CLI 误把 /api/cluster/status 打到
// mlx.base_url (11432), 网关无此路由必 404。本模块直连 Master base_url。

fn base_url() -> String {
    ServiceUrls::from_config()
        .multinode
        .trim_end_matches('/')
        .to_string()
}

fn auth_header() -> Option<(&'static str, String)> {
    let key = ServiceUrls::from_config().multinode_api_key.clone();
    if key.is_empty() {
        None
    } else {
        Some(("Authorization", format!("Bearer {}", key)))
    }
}

fn authed_get(client: &reqwest::Client, url: &str, timeout: u64) -> reqwest::RequestBuilder {
    let mut req = client.get(url).timeout(Duration::from_secs(timeout));
    if let Some((name, value)) = auth_header() {
        req = req.header(name, value);
    }
    req
}

fn authed_post(
    client: &reqwest::Client,
    url: &str,
    payload: &serde_json::Value,
    timeout: u64,
) -> reqwest::RequestBuilder {
    let mut req = client
        .post(url)
        .json(payload)
        .timeout(Duration::from_secs(timeout));
    if let Some((name, value)) = auth_header() {
        req = req.header(name, value);
    }
    req
}

fn authed_delete(client: &reqwest::Client, url: &str, timeout: u64) -> reqwest::RequestBuilder {
    let mut req = client.delete(url).timeout(Duration::from_secs(timeout));
    if let Some((name, value)) = auth_header() {
        req = req.header(name, value);
    }
    req
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
    let resp = authed_get(&client, &url, 10).send().await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn list_nodes() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes", base_url());
    info!(url = %url, "multi-node list nodes");
    let resp = authed_get(&client, &url, 10).send().await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn get_node(node_id: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes/{}", base_url(), node_id);
    info!(url = %url, node_id = %node_id, "multi-node get node");
    let resp = authed_get(&client, &url, 5).send().await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn remove_node(node_id: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes/{}", base_url(), node_id);
    info!(url = %url, node_id = %node_id, "multi-node remove node");
    let resp = authed_delete(&client, &url, 10).send().await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn pending_nodes() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/nodes/pending", base_url());
    info!(url = %url, "multi-node pending nodes");
    let resp = authed_get(&client, &url, 5).send().await?;
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
    let resp = authed_post(&client, &url, &payload, 10).send().await?;
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
    let resp = authed_post(&client, &url, &payload, 10).send().await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn routing_summary() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/routing/summary", base_url());
    info!(url = %url, "multi-node routing summary");
    let resp = authed_get(&client, &url, 5).send().await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn model_manifest(model_name: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/api/models/{}/manifest", base_url(), model_name);
    info!(url = %url, model = %model_name, "multi-node model manifest");
    let resp = authed_get(&client, &url, 10).send().await?;
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
    let resp = authed_post(&client, &url, &payload, 60).send().await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cluster_status_url_shape() {
        let base = "http://localhost:11452".to_string();
        let url = format!("{}/api/cluster/status", base);
        assert_eq!(url, "http://localhost:11452/api/cluster/status");
    }

    #[test]
    fn test_nodes_url_shape() {
        let base = "http://localhost:11452".to_string();
        let url = format!("{}/api/nodes", base);
        assert_eq!(url, "http://localhost:11452/api/nodes");
    }

    #[test]
    fn test_node_detail_url_shape() {
        let base = "http://localhost:11452".to_string();
        let url = format!("{}/api/nodes/{}", base, "node-1");
        assert_eq!(url, "http://localhost:11452/api/nodes/node-1");
    }

    #[test]
    fn test_manifest_url_shape() {
        let base = "http://localhost:11452".to_string();
        let url = format!("{}/api/models/{}/manifest", base, "llama3");
        assert_eq!(url, "http://localhost:11452/api/models/llama3/manifest");
    }

    #[test]
    fn test_sync_payload_shape() {
        let payload = serde_json::json!({
            "model_name": "llama3",
            "source": "192.168.1.10:11452",
        });
        assert_eq!(payload["model_name"], "llama3");
        assert_eq!(payload["source"], "192.168.1.10:11452");
    }
}
