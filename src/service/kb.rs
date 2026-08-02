use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct KbInfo {
    pub id: String,
    pub name: String,
    pub document_count: u32,
    pub status: String,
}

fn base_url() -> String {
    ServiceUrls::from_config()
        .kb
        .trim_end_matches('/')
        .to_string()
}

#[allow(dead_code)]
pub async fn list_bases() -> Result<Vec<KbInfo>> {
    let client = get_client();
    let url = format!("{}/kb/bases", base_url());
    info!(url = %url, "Listing KB bases");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: Vec<KbInfo> = resp.json().await?;
    Ok(data)
}

pub async fn query(kb_id: &str, question: &str, top_k: usize) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/kb/bases/{}/query", base_url(), kb_id);
    info!(url = %url, kb_id = %kb_id, "Querying KB");
    let payload = serde_json::json!({
        "question": question,
        "top_k": top_k,
    });
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/kb/bases", base_url());
    match client
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}
