use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

fn base_url() -> String {
    ServiceUrls::from_config()
        .desk
        .trim_end_matches('/')
        .to_string()
}

#[derive(Debug, Deserialize)]
pub struct DeskTemplate {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct DeskHistoryEntry {
    pub id: String,
    pub task: String,
    pub status: String,
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub finished_at: String,
}

pub async fn list_templates() -> Result<Vec<DeskTemplate>> {
    let client = get_client();
    let url = format!("{}/api/templates", base_url());
    info!(url = %url, "Listing desk templates");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: Vec<DeskTemplate> = resp.json().await.unwrap_or_default();
    Ok(data)
}

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

pub async fn get_history(limit: u32) -> Result<Vec<DeskHistoryEntry>> {
    let client = get_client();
    let url = format!("{}/api/tasks/history?limit={}", base_url(), limit);
    info!(url = %url, limit = limit, "Fetching desk task history");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: Vec<DeskHistoryEntry> = resp.json().await.unwrap_or_default();
    Ok(data)
}

pub async fn stop_task(task_id: &str) -> Result<bool> {
    let client = get_client();
    let url = format!("{}/api/tasks/{}/stop", base_url(), task_id);
    info!(url = %url, task_id = task_id, "Stopping desk task");
    let resp = client
        .post(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    Ok(resp.status().is_success())
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
