use anyhow::Result;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

fn base_url() -> String {
    ServiceUrls::from_config()
        .doc
        .trim_end_matches('/')
        .to_string()
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/api/health", base_url());
    info!(url = %url, "Doc health check");
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
    let url = format!("{}/api/health", base_url());
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(3))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}
