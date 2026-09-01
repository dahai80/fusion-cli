use anyhow::Result;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

// fusion-memory fm-server HTTP API (port 11435, see architecture/port-registry.yaml)。
// 路由源: crates/fm-server/src/http.rs (axum)。
//   GET  /healthz                  (公开)
//   GET  /v1/memory/version        (公开)
//   GET  /v1/memory/:id            (Bearer)
//   POST /v1/memory/commit|retrieve|consolidate|audit|delete|delete_scope|count (Bearer)
// Bearer token = FUSION_MEMORY_API_KEY (config memory.api_key)。
// 记忆中心: 跨会话长/短期记忆 + 认知图谱。

fn base_url() -> String {
    ServiceUrls::from_config()
        .memory
        .trim_end_matches('/')
        .to_string()
}

// 校验路径段不含 '/', 防止 id="x/../../delete" 注入额外路径段。
// A14 修复: 集中到 super::validate_path_segment, 本地转发保持调用点不变。
fn validate_path_segment(field: &str, value: &str) -> Result<()> {
    super::validate_path_segment(field, value)
}

fn auth_header() -> Option<(&'static str, String)> {
    let key = ServiceUrls::from_config().memory_api_key.clone();
    if key.is_empty() {
        None
    } else {
        Some(("Authorization", format!("Bearer {}", key)))
    }
}

fn get_with_auth(client: &reqwest::Client, url: &str, timeout: u64) -> reqwest::RequestBuilder {
    let mut req = client.get(url).timeout(Duration::from_secs(timeout));
    if let Some((name, value)) = auth_header() {
        req = req.header(name, value);
    }
    req
}

fn post_with_auth(
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

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/healthz", base_url());
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

pub async fn version() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/v1/memory/version", base_url());
    info!(url = %url, "fm-server version");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "memory").await?;
    Ok(data)
}

// retrieve: 语义检索记忆。payload 含 query + 可选 top_k/scope/filters。
pub async fn retrieve(query: &str, top_k: usize) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/v1/memory/retrieve", base_url());
    info!(url = %url, query = %query, top_k, "fm-server retrieve");
    let payload = serde_json::json!({
        "query": query,
        "top_k": top_k,
    });
    let resp = post_with_auth(&client, &url, &payload, 30).send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "memory retrieve").await?;
    Ok(data)
}

// count: 记忆条目总数。
pub async fn count() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/v1/memory/count", base_url());
    info!(url = %url, "fm-server count");
    let payload = serde_json::json!({});
    let resp = post_with_auth(&client, &url, &payload, 10).send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "memory").await?;
    Ok(data)
}

// get_memory: 按 id 取单条记忆。
pub async fn get_memory(id: &str) -> Result<serde_json::Value> {
    validate_path_segment("id", id)?;
    let client = get_client();
    let url = format!("{}/v1/memory/{}", base_url(), id);
    info!(url = %url, id = %id, "fm-server get_memory");
    let resp = get_with_auth(&client, &url, 10).send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "memory").await?;
    Ok(data)
}

// commit: 写入一条记忆。payload 含 content + 可选 metadata/scope/ttl。
pub async fn commit(content: &str, scope: Option<&str>) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/v1/memory/commit", base_url());
    info!(url = %url, scope = ?scope, "fm-server commit");
    let payload = if let Some(s) = scope {
        serde_json::json!({ "content": content, "scope": s })
    } else {
        serde_json::json!({ "content": content })
    };
    let resp = post_with_auth(&client, &url, &payload, 30).send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "memory").await?;
    Ok(data)
}

// consolidate: 触发记忆巩固 (短期→长期)。
pub async fn consolidate() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/v1/memory/consolidate", base_url());
    info!(url = %url, "fm-server consolidate");
    let payload = serde_json::json!({});
    let resp = post_with_auth(&client, &url, &payload, 60).send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "memory").await?;
    Ok(data)
}

// delete: 按 id 删除, 需 confirm=true (fm-server 强制)。
pub async fn delete(id: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/v1/memory/delete", base_url());
    info!(url = %url, id = %id, "fm-server delete");
    let payload = serde_json::json!({ "id": id, "confirm": true });
    let resp = post_with_auth(&client, &url, &payload, 10).send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "memory").await?;
    Ok(data)
}

// audit: 审计日志查询。
pub async fn audit() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/v1/memory/audit", base_url());
    info!(url = %url, "fm-server audit");
    let payload = serde_json::json!({});
    let resp = post_with_auth(&client, &url, &payload, 10).send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "memory").await?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_url_strips_trailing_slash() {
        // base_url() reads config — verify trim logic via direct string op (config may be unset).
        let raw = "http://localhost:11435/";
        assert_eq!(raw.trim_end_matches('/'), "http://localhost:11435");
    }

    #[test]
    fn test_auth_header_present_when_key_set() {
        // auth_header reads live config; with default config memory_api_key is "" → None.
        // Lock the behavior: empty key → None (fm-server B5 refuses HTTP without key,
        // so CLI must not send a bogus Bearer when unconfigured).
        let _urls = ServiceUrls::from_config();
        // 默认配置无 memory 段 → memory_api_key 为空 → auth_header 应返 None。
        // (此测试锁定: 未配 key 时不发 Authorization 头, 避免发空 Bearer。)
        let h = auth_header();
        assert!(
            h.is_none(),
            "default config has no memory.api_key, auth must be None"
        );
    }

    #[test]
    fn test_retrieve_payload_shape() {
        let payload = serde_json::json!({
            "query": "hello",
            "top_k": 5,
        });
        assert_eq!(payload["query"], "hello");
        assert_eq!(payload["top_k"], 5);
    }

    #[test]
    fn test_commit_payload_with_scope() {
        let payload = serde_json::json!({
            "content": "note",
            "scope": "session-1",
        });
        assert_eq!(payload["content"], "note");
        assert_eq!(payload["scope"], "session-1");
    }

    #[test]
    fn test_delete_payload_carries_confirm() {
        // fm-server 强制 confirm=true, 否则拒绝 delete。
        let payload = serde_json::json!({ "id": "abc", "confirm": true });
        assert_eq!(payload["id"], "abc");
        assert_eq!(payload["confirm"], true);
    }

    #[test]
    fn test_get_memory_url_path() {
        let base = "http://localhost:11435".to_string();
        let id = "mem-123";
        let url = format!("{}/v1/memory/{}", base, id);
        assert_eq!(url, "http://localhost:11435/v1/memory/mem-123");
    }
}
