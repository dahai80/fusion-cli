use anyhow::Result;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

fn base_url() -> String {
    ServiceUrls::from_config()
        .rag
        .trim_end_matches('/')
        .to_string()
}

fn api_base() -> String {
    format!("{}/api/v1", base_url())
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
    let url = format!("{}/kb/{}/search", api_base(), kb_id);
    info!(url = %url, kb_id = %kb_id, "RAG search");
    let payload = serde_json::json!({
        "query": query,
        "top_k": top_k,
    });
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "rag search").await?;
    Ok(data)
}

pub async fn list_knowledge_bases() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/kb", api_base());
    info!(url = %url, "Listing RAG knowledge bases");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "rag").await?;
    Ok(data)
}

pub async fn list_embedding_models() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/models/embedding", api_base());
    info!(url = %url, "Listing RAG embedding models");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "rag").await?;
    Ok(data)
}
