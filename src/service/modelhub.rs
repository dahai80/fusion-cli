use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub size: String,
    pub quant: String,
}

fn base_url() -> String {
    ServiceUrls::from_config()
        .modelhub
        .trim_end_matches('/')
        .to_string()
}

#[allow(dead_code)]
pub async fn list_models() -> Result<Vec<ModelEntry>> {
    let client = get_client();
    let url = format!("{}/v1/models", base_url());
    info!(url = %url, "Listing ModelHub models");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "modelhub list_models").await?;
    let models: Vec<ModelEntry> = serde_json::from_value(data)?;
    Ok(models)
}

#[allow(dead_code)]
pub async fn search(name: &str) -> Result<Vec<ModelEntry>> {
    let client = get_client();
    let url = format!("{}/v1/models?q={}", base_url(), name);
    info!(url = %url, query = name, "Searching ModelHub");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "modelhub search").await?;
    let models: Vec<ModelEntry> = serde_json::from_value(data)?;
    Ok(models)
}

pub async fn download_model(name: &str) -> Result<String> {
    super::validate_path_segment("model name", name)?;
    let client = get_client();
    let url = format!("{}/v1/models/{}/download", base_url(), name);
    info!(url = %url, model = name, "Requesting model download");
    let resp = client
        .post(&url)
        .timeout(Duration::from_secs(300))
        .send()
        .await?;

    if resp.status().is_success() {
        let body: serde_json::Value = resp.json().await?;
        let path = body["path"].as_str().unwrap_or("unknown").to_string();
        Ok(path)
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Download failed: HTTP {} — {}", status, body)
    }
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/v1/models", base_url());
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
