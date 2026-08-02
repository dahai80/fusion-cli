use anyhow::Result;
use tracing::info;

use super::ServiceUrls;

pub struct ServiceStatus {
    pub name: String,
    pub url: String,
    pub alive: bool,
}

pub async fn check_all() -> Result<Vec<ServiceStatus>> {
    let urls = ServiceUrls::from_config();
    let checks: Vec<(&str, &str)> = vec![
        ("MLX", &urls.mlx),
        ("KB", &urls.kb),
        ("ModelHub", &urls.modelhub),
        ("RAG", &urls.rag),
        ("Desk", &urls.desk),
    ];

    let mut results = Vec::new();
    for (name, url) in checks {
        let health_url = format!("{}/health", url.trim_end_matches('/'));
        info!(service = %name, url = %health_url, "Checking service health");
        let alive = super::check_url(&health_url, 2).await;
        results.push(ServiceStatus {
            name: name.to_string(),
            url: url.to_string(),
            alive,
        });
    }
    Ok(results)
}

#[allow(dead_code)]
pub async fn check_named(name: &str) -> Result<ServiceStatus> {
    let urls = ServiceUrls::from_config();
    let (svc_name, svc_url) = match name.to_lowercase().as_str() {
        "mlx" => ("MLX", urls.mlx.clone()),
        "kb" => ("KB", urls.kb.clone()),
        "modelhub" => ("ModelHub", urls.modelhub.clone()),
        "rag" => ("RAG", urls.rag.clone()),
        "desk" => ("Desk", urls.desk.clone()),
        _ => anyhow::bail!("Unknown service: {}", name),
    };
    let health_url = format!("{}/health", svc_url.trim_end_matches('/'));
    let alive = super::check_url(&health_url, 2).await;
    Ok(ServiceStatus {
        name: svc_name.to_string(),
        url: svc_url,
        alive,
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
