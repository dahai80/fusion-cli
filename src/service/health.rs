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

    // 并发探测所有服务, 最坏阻塞 = 单个超时 (2s), 而非串行 N×2s。
    let futs = checks.into_iter().map(|(name, base_url, health_path)| {
        let health_url = format!("{}{}", base_url.trim_end_matches('/'), health_path);
        let base_url = base_url.to_string();
        async move {
            info!(service = %name, url = %health_url, "Checking service health with latency");
            let start = std::time::Instant::now();
            // MultiNode 跨机, 网络抖动常态 → 1 次重试; 本地服务单发即止。
            let alive = if name == "MultiNode" {
                super::check_url_with_retry(&health_url, 2, 1).await
            } else {
                super::check_url(&health_url, 2).await
            };
            let elapsed = start.elapsed();
            let latency_ms = if alive {
                Some(elapsed.as_millis() as u64)
            } else {
                None
            };
            let port = extract_port(&base_url);
            ServiceStatus {
                name: name.to_string(),
                url: base_url,
                alive,
                port,
                latency_ms,
            }
        }
    });
    let results: Vec<ServiceStatus> = futures_util::future::join_all(futs).await;
    Ok(results)
}

pub async fn check_named(name: &str) -> Result<ServiceStatus> {
    let urls = ServiceUrls::from_config();
    // E8 修复: check_named 之前只覆盖 6 个旧服务, 漏掉 Memory/Bench/MultiNode,
    // 导致 doctor 与 service status 对同名服务存活判断不一致。现统一 9 服务并各自带正确 health 路径。
    let (svc_name, svc_url, health_path) = match name.to_lowercase().as_str() {
        "mlx" => ("MLX", urls.mlx.clone(), "/health"),
        "kb" => ("KB", urls.kb.clone(), "/kb/bases"),
        "modelhub" | "model-hub" => ("ModelHub", urls.modelhub.clone(), "/v1/models"),
        "rag" => ("RAG", urls.rag.clone(), "/health"),
        "desk" => ("Desk", urls.desk.clone(), "/health"),
        "doc" => ("Doc", urls.doc.clone(), "/api/health"),
        "memory" => ("Memory", urls.memory.clone(), "/healthz"),
        "bench" => ("Bench", urls.bench.clone(), "/api/v1/system/health"),
        "multinode" | "multi-node" => ("MultiNode", urls.multinode.clone(), "/api/health"),
        _ => anyhow::bail!("Unknown service: {}", name),
    };
    let health_url = format!("{}{}", svc_url.trim_end_matches('/'), health_path);
    let start = std::time::Instant::now();
    let alive = super::check_url(&health_url, 2).await;
    let elapsed = start.elapsed();
    let port = extract_port(&svc_url);
    Ok(ServiceStatus {
        name: svc_name.to_string(),
        url: svc_url,
        alive,
        port,
        latency_ms: if alive {
            Some(elapsed.as_millis() as u64)
        } else {
            None
        },
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
    // E7 修复: 旧实现 url.split(':').next_back() 对 IPv6 ([::1]:11434) 与带路径 URL 会错析。
    // 用 reqwest::Url (reqwest 重导出 url crate) 解析, authority 取 port; 解析失败回退 0。
    if let Ok(parsed) = reqwest::Url::parse(url)
        && let Some(port) = parsed.port()
    {
        return port;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::extract_port;

    #[test]
    fn test_extract_port_ipv4() {
        assert_eq!(extract_port("http://localhost:11434"), 11434);
        assert_eq!(extract_port("http://127.0.0.1:9000"), 9000);
    }

    #[test]
    fn test_extract_port_ipv6() {
        assert_eq!(extract_port("http://[::1]:11434"), 11434);
        assert_eq!(extract_port("http://[::1]:11434/v1/models"), 11434);
    }

    #[test]
    fn test_extract_port_with_path() {
        assert_eq!(extract_port("http://node-1.local:11452/api/health"), 11452);
    }

    #[test]
    fn test_extract_port_invalid() {
        assert_eq!(extract_port("not-a-url"), 0);
        assert_eq!(extract_port("http://localhost"), 0);
    }
}
