use anyhow::Result;
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

// fusion-bench HTTP API (port 11467, see architecture/port-registry.yaml)。
// 路由源: fusion_bench/api/app.py (FastAPI @app.get/post)。
// 注意: 与 CLI 自带的 `fusion bench speed/mem/ctx/auto` (本地直测 MLX generate_tokens) 区分。
// 本模块对接 fusion-bench 服务端: 任务管理/套件/结果/质量门/基线。
// 所有路由前缀 /api/v1。

fn base_url() -> String {
    ServiceUrls::from_config()
        .bench
        .trim_end_matches('/')
        .to_string()
}

fn api_base() -> String {
    format!("{}/api/v1", base_url())
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/system/health", api_base());
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
    let url = format!("{}/system/health", api_base());
    info!(url = %url, "bench health");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn list_tasks() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/tasks", api_base());
    info!(url = %url, "bench list tasks");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn get_task(task_id: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/tasks/{}", api_base(), task_id);
    info!(url = %url, task_id = %task_id, "bench get task");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn list_suites() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/suites", api_base());
    info!(url = %url, "bench list suites");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn get_result(task_id: &str) -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/results/{}", api_base(), task_id);
    info!(url = %url, task_id = %task_id, "bench get result");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn results_trend() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/results/trend", api_base());
    info!(url = %url, "bench results trend");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn list_baselines() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/baselines", api_base());
    info!(url = %url, "bench list baselines");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn list_gates() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/gates", api_base());
    info!(url = %url, "bench list gates");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub async fn system_resources() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/system/resources", api_base());
    info!(url = %url, "bench system resources");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_api_base_prepends_v1() {
        let base = "http://localhost:11467".to_string();
        let api = format!("{}/api/v1", base.trim_end_matches('/'));
        assert_eq!(api, "http://localhost:11467/api/v1");
    }

    #[test]
    fn test_tasks_url_shape() {
        let api = "http://localhost:11467/api/v1".to_string();
        let url = format!("{}/tasks", api);
        assert_eq!(url, "http://localhost:11467/api/v1/tasks");
    }

    #[test]
    fn test_task_detail_url_shape() {
        let api = "http://localhost:11467/api/v1".to_string();
        let url = format!("{}/tasks/{}", api, "t-abc");
        assert_eq!(url, "http://localhost:11467/api/v1/tasks/t-abc");
    }

    #[test]
    fn test_results_trend_url_shape() {
        let api = "http://localhost:11467/api/v1".to_string();
        let url = format!("{}/results/trend", api);
        assert_eq!(url, "http://localhost:11467/api/v1/results/trend");
    }
}
