use anyhow::Result;
use serde::Deserialize;
use std::time::Duration;
use tracing::info;

use super::ServiceUrls;
use super::get_client;

#[allow(dead_code)]
const DEFAULT_GATEWAY_URL: &str = "http://localhost:11432";

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ServiceEntry {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub health_path: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DiscoveryResponse {
    services: Vec<ServiceEntry>,
}

#[allow(dead_code)]
pub struct GatewayClient {
    base_url: String,
    enabled: bool,
}

#[allow(dead_code)]
impl GatewayClient {
    pub fn from_config() -> Self {
        let config = crate::config::load_config();
        let gateway_url = config
            .gateway
            .as_ref()
            .map(|g| g.base_url.clone())
            .unwrap_or_else(|| DEFAULT_GATEWAY_URL.to_string());
        let enabled = config.gateway.as_ref().map(|g| g.enabled).unwrap_or(false);
        info!(url = %gateway_url, enabled = enabled, "Gateway client initialized");
        Self {
            base_url: gateway_url,
            enabled,
        }
    }

    pub fn with_url(url: &str) -> Self {
        Self {
            base_url: url.to_string(),
            enabled: true,
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        let client = get_client();
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        match client
            .get(&url)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => {
                info!("Gateway health check failed: {}", e);
                Ok(false)
            }
        }
    }

    pub async fn discover_services(&self) -> Result<Vec<ServiceEntry>> {
        if !self.enabled {
            info!("Gateway disabled, using fallback URLs");
            return Ok(fallback_entries());
        }

        let client = get_client();
        let url = format!("{}/api/v1/services", self.base_url.trim_end_matches('/'));
        info!(url = %url, "Discovering services via gateway");

        match client
            .get(&url)
            .timeout(Duration::from_secs(3))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let data: DiscoveryResponse = resp.json().await?;
                info!(
                    count = data.services.len(),
                    "Discovered services via gateway"
                );
                Ok(data.services)
            }
            Ok(resp) => {
                info!(status = %resp.status(), "Gateway returned non-success, using fallback");
                Ok(fallback_entries())
            }
            Err(e) => {
                info!(error = %e, "Gateway unreachable, using fallback URLs");
                Ok(fallback_entries())
            }
        }
    }

    pub async fn get_service_url(&self, service_name: &str) -> String {
        if self.enabled
            && let Ok(services) = self.discover_services().await
            && let Some(svc) = services.iter().find(|s| s.name == service_name)
        {
            return format!("http://{}:{}", svc.host, svc.port);
        }
        let urls = ServiceUrls::from_config();
        match service_name {
            "mlx" => urls.mlx,
            "kb" => urls.kb,
            "modelhub" => urls.modelhub,
            "rag" => urls.rag,
            "desk" => urls.desk,
            _ => String::new(),
        }
    }
}

#[allow(dead_code)]
fn fallback_entries() -> Vec<ServiceEntry> {
    let urls = ServiceUrls::from_config();
    let extract_port = |url: &str| -> u16 {
        url.split(':')
            .next_back()
            .and_then(|s| s.trim_end_matches('/').parse().ok())
            .unwrap_or(0)
    };

    vec![
        ServiceEntry {
            name: "mlx".into(),
            host: "localhost".into(),
            port: extract_port(&urls.mlx),
            health_path: "/v1/models".into(),
            status: "unknown".into(),
        },
        ServiceEntry {
            name: "kb".into(),
            host: "localhost".into(),
            port: extract_port(&urls.kb),
            health_path: "/kb/bases".into(),
            status: "unknown".into(),
        },
        ServiceEntry {
            name: "modelhub".into(),
            host: "localhost".into(),
            port: extract_port(&urls.modelhub),
            health_path: "/v1/models".into(),
            status: "unknown".into(),
        },
        ServiceEntry {
            name: "rag".into(),
            host: "localhost".into(),
            port: extract_port(&urls.rag),
            health_path: "/health".into(),
            status: "unknown".into(),
        },
        ServiceEntry {
            name: "desk".into(),
            host: "localhost".into(),
            port: extract_port(&urls.desk),
            health_path: "/health".into(),
            status: "unknown".into(),
        },
    ]
}
