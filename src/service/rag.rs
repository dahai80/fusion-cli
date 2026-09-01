use anyhow::Result;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

// fusion-rag 实际路由前缀为 /kb (routes.py APIRouter(prefix="/kb")), 无 /api/v1。
// 文档/搜索/CRUD 端点: /kb/bases/{kb_id}/...。旧 CLI 误用 /api/v1/kb/{id}/search
// 与上游不符, 此处统一修正为真实路径。
fn base_url() -> String {
    ServiceUrls::from_config()
        .rag
        .trim_end_matches('/')
        .to_string()
}

fn api_base() -> String {
    format!("{}/kb", base_url())
}

// fusion-rag auth: X-API-Key header (auth.py API_KEY_HEADER)。本地默认 NoAuth (空 key → 开放);
// 配 FUSION_RAG_API_KEY 后服务端启用 ApiKeyBackend, CLI 需带同 header。
fn auth_header(urls: &ServiceUrls) -> Option<(&'static str, String)> {
    if urls.rag_api_key.is_empty() {
        None
    } else {
        Some(("X-API-Key", urls.rag_api_key.clone()))
    }
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/health", base_url());
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

pub async fn get_health_detail() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/health", base_url());
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "rag").await?;
    Ok(data)
}

pub async fn search(kb_id: &str, query: &str, top_k: usize) -> Result<serde_json::Value> {
    super::validate_path_segment("kb_id", kb_id)?;
    let client = get_client();
    let urls = ServiceUrls::from_config();
    let url = format!("{}/bases/{}/search", api_base(), kb_id);
    info!(url = %url, kb_id = %kb_id, "RAG search");
    let payload = serde_json::json!({
        "query": query,
        "top_k": top_k,
    });
    let mut req = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(30));
    if let Some((name, value)) = auth_header(&urls) {
        req = req.header(name, value);
    }
    let resp = req.send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "rag search").await?;
    Ok(data)
}

pub async fn list_knowledge_bases() -> Result<serde_json::Value> {
    let client = get_client();
    let urls = ServiceUrls::from_config();
    let url = format!("{}/bases", api_base());
    info!(url = %url, "Listing RAG knowledge bases");
    let mut req = client.get(&url).timeout(Duration::from_secs(5));
    if let Some((name, value)) = auth_header(&urls) {
        req = req.header(name, value);
    }
    let resp = req.send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "rag").await?;
    Ok(data)
}

pub async fn list_embedding_models() -> Result<serde_json::Value> {
    let client = get_client();
    let urls = ServiceUrls::from_config();
    let url = format!("{}/models/embedding", base_url());
    info!(url = %url, "Listing RAG embedding models");
    let mut req = client.get(&url).timeout(Duration::from_secs(5));
    if let Some((name, value)) = auth_header(&urls) {
        req = req.header(name, value);
    }
    let resp = req.send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "rag").await?;
    Ok(data)
}

// 真正向量化: 调 fusion-rag POST /kb/bases/{kb_id}/documents/ingest。
// 服务端做 chunk + embed (embedding 模型) + 向量入库, 非本地文件归集。
// content_type: text/markdown/json 等 (CONTENT_TYPE_MAP); contextualize 默认 true。
pub async fn ingest_content(
    kb_id: &str,
    content: &str,
    doc_name: &str,
    content_type: &str,
) -> Result<serde_json::Value> {
    super::validate_path_segment("kb_id", kb_id)?;
    if content.is_empty() {
        anyhow::bail!("content is required for ingest");
    }
    let client = get_client();
    let urls = ServiceUrls::from_config();
    let url = format!("{}/bases/{}/documents/ingest", api_base(), kb_id);
    info!(url = %url, kb_id = %kb_id, doc_name = %doc_name, content_type = %content_type, content_len = content.len(), "RAG ingest");
    let payload = serde_json::json!({
        "content": content,
        "content_type": content_type,
        "doc_name": doc_name,
        "metadata": {},
        "contextualize": true,
    });
    let mut req = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(120));
    if let Some((name, value)) = auth_header(&urls) {
        req = req.header(name, value);
    }
    let resp = req.send().await?;
    let data: serde_json::Value = super::json_or_error(resp, "rag ingest").await?;
    Ok(data)
}
