use anyhow::Result;
use tracing::info;

use super::ServiceUrls;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServiceStatus {
    pub name: String,
    pub url: String,
    pub alive: bool,
    pub port: u16,
    pub latency_ms: Option<u64>,
}

#[allow(dead_code)]
pub async fn check_all() -> Result<Vec<ServiceStatus>> {
    let urls = ServiceUrls::from_config();
    let checks: Vec<(&str, &str)> = vec![
        ("MLX", &urls.mlx),
        ("KB", &urls.kb),
        ("ModelHub", &urls.modelhub),
        ("RAG", &urls.rag),
        ("Desk", &urls.desk),
        ("Doc", &urls.doc),
    ];

    let mut results = Vec::new();
    for (name, url) in checks {
        let health_url = format!("{}/health", url.trim_end_matches('/'));
        info!(service = %name, url = %health_url, "Checking service health");
        let alive = super::check_url(&health_url, 2).await;
        let port = extract_port(url);
        results.push(ServiceStatus {
            name: name.to_string(),
            url: url.to_string(),
            alive,
            port,
            latency_ms: None,
        });
    }
    Ok(results)
}

pub async fn check_all_with_latency() -> Result<Vec<ServiceStatus>> {
    let urls = ServiceUrls::from_config();
    let checks: Vec<(&str, &str, &str)> = vec![
        ("MLX", &urls.mlx, "/health"),
        ("KB", &urls.kb, "/kb/bases"),
        ("ModelHub", &urls.modelhub, "/v1/models"),
        ("RAG", &urls.rag, "/health"),
        ("Desk", &urls.desk, "/health"),
        ("Doc", &urls.doc, "/api/health"),
        ("Memory", &urls.memory, "/healthz"),
        ("Bench", &urls.bench, "/api/v1/system/health"),
        ("MultiNode", &urls.multinode, "/api/health"),
    ];

    let mut results = Vec::new();
    for (name, base_url, health_path) in checks {
        let health_url = format!("{}{}", base_url.trim_end_matches('/'), health_path);
        info!(service = %name, url = %health_url, "Checking service health with latency");

        let start = std::time::Instant::now();
        let alive = super::check_url(&health_url, 2).await;
        let elapsed = start.elapsed();

        let latency_ms = if alive {
            Some(elapsed.as_millis() as u64)
        } else {
            None
        };

        let port = extract_port(base_url);
        results.push(ServiceStatus {
            name: name.to_string(),
            url: base_url.to_string(),
            alive,
            port,
            latency_ms,
        });
    }
    Ok(results)
}

pub async fn check_named(name: &str) -> Result<ServiceStatus> {
    let urls = ServiceUrls::from_config();
    let (svc_name, svc_url) = match name.to_lowercase().as_str() {
        "mlx" => ("MLX", urls.mlx.clone()),
        "kb" => ("KB", urls.kb.clone()),
        "modelhub" => ("ModelHub", urls.modelhub.clone()),
        "rag" => ("RAG", urls.rag.clone()),
        "desk" => ("Desk", urls.desk.clone()),
        "doc" => ("Doc", urls.doc.clone()),
        _ => anyhow::bail!("Unknown service: {}", name),
    };
    let health_url = format!("{}/health", svc_url.trim_end_matches('/'));
    let alive = super::check_url(&health_url, 2).await;
    Ok(ServiceStatus {
        name: svc_name.to_string(),
        url: svc_url,
        alive,
        port: 0,
        latency_ms: None,
    })
}

#[allow(dead_code)]
pub fn format_status_table(statuses: &[ServiceStatus]) -> String {
    let mut out = String::new();
    for s in statuses {
        let mark = if s.alive { "✅" } else { "❌" };
        out.push_str(&format!("  {} {} — {}\n", mark, s.name, s.url));
    }
    out
}

fn extract_port(url: &str) -> u16 {
    url.split(':')
        .next_back()
        .and_then(|s| s.trim_end_matches('/').split('/').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
