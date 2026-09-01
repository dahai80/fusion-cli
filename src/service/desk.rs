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

#[derive(Debug, Deserialize, serde::Serialize)]
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
    let data: serde_json::Value = super::json_or_error(resp, "desk list_templates").await?;
    let templates: Vec<DeskTemplate> = serde_json::from_value(data)?;
    Ok(templates)
}

pub async fn run_task(template: &str, params: Option<&str>) -> Result<String> {
    let client = get_client();
    let url = format!("{}/api/tasks/run", base_url());
    info!(url = %url, template = %template, params = ?params, "Running desk task");
    let mut payload = serde_json::json!({
        "template": template,
    });
    if let Some(p) = params
        && !p.is_empty()
    {
        // params may be a JSON object string or plain string; send as-is for the service to parse.
        payload["params"] = serde_json::Value::String(p.to_string());
    }
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(60))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "desk run_task").await?;
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
    let data: serde_json::Value = super::json_or_error(resp, "desk get_history").await?;
    let history: Vec<DeskHistoryEntry> = serde_json::from_value(data)?;
    Ok(history)
}

pub async fn stop_task(task_id: &str) -> Result<bool> {
    super::validate_path_segment("task_id", task_id)?;
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
