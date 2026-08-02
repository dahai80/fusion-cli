use anyhow::Result;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

fn base_url() -> String {
    ServiceUrls::from_config()
        .desk
        .trim_end_matches('/')
        .to_string()
}

#[allow(dead_code)]
pub async fn run_task(template: &str) -> Result<String> {
    let client = get_client();
    let url = format!("{}/api/tasks/run", base_url());
    info!(url = %url, template = %template, "Running desk task");
    let payload = serde_json::json!({
        "template": template,
    });
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(60))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data["task_id"].as_str().unwrap_or("").to_string())
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/health", base_url());
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
