pub mod benchsvc;
pub mod desk;
pub mod doc;
pub mod guard;
pub mod health;
pub mod kb;
pub mod memory;
pub mod mlx;
pub mod modelhub;
pub mod multinode;
pub mod rag;
pub mod sv;

use once_cell::sync::Lazy;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

static GLOBAL_CLIENT: Lazy<Arc<Client>> = Lazy::new(|| {
    Arc::new(
        Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(4)
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default(),
    )
});

pub fn get_client() -> Arc<Client> {
    GLOBAL_CLIENT.clone()
}

pub struct ServiceUrls {
    pub mlx: String,
    pub mlx_api_key: String,
    pub kb: String,
    pub modelhub: String,
    pub rag: String,
    pub desk: String,
    pub doc: String,
    pub memory: String,
    pub memory_api_key: String,
    pub bench: String,
    pub multinode: String,
    pub multinode_api_key: String,
}

impl ServiceUrls {
    pub fn from_config() -> Self {
        let config = crate::config::load_config();
        Self {
            mlx: config.mlx.base_url.clone(),
            mlx_api_key: config.mlx.api_key.clone(),
            kb: config.kb.base_url.clone(),
            modelhub: config.modelhub.base_url.clone(),
            rag: config.rag.base_url.clone(),
            desk: config.desk.base_url.clone(),
            doc: config.doc.base_url.clone(),
            memory: config.memory.base_url.clone(),
            memory_api_key: config.memory.api_key.clone(),
            bench: config.bench.base_url.clone(),
            multinode: config.multinode.base_url.clone(),
            multinode_api_key: config.multinode.api_key.clone(),
        }
    }

    pub fn mlx_api(&self) -> String {
        let base = self
            .mlx
            .trim_end_matches('/')
            .trim_end_matches("/v1")
            .trim_end_matches('/');
        format!("{}/v1", base)
    }

    pub fn mlx_auth_header(&self) -> Option<(&'static str, String)> {
        if self.mlx_api_key.is_empty() {
            None
        } else {
            Some(("Authorization", format!("Bearer {}", self.mlx_api_key)))
        }
    }
}

pub async fn check_url(url: &str, timeout_secs: u64) -> bool {
    let client = get_client();
    match client
        .get(url)
        .timeout(Duration::from_secs(timeout_secs))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

// 统一: 检查 HTTP 状态码, 非 2xx bail 出可读错误 (服务名 + 状态 + 截断 body),
// 否则解析 JSON。避免裸 resp.json() 把 nginx 502 HTML 解析错吐成不可懂的 serde 错。
pub async fn json_or_error(
    resp: reqwest::Response,
    service: &str,
) -> anyhow::Result<serde_json::Value> {
    let status = resp.status();
    if status.is_success() {
        let data: serde_json::Value = resp.json().await?;
        return Ok(data);
    }
    let text = resp.text().await.unwrap_or_default();
    let snippet: String = text.chars().take(200).collect();
    anyhow::bail!("{} HTTP {}: {}", service, status, snippet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mlx_api_appends_v1_when_missing() {
        let mut urls = ServiceUrls::from_config();
        urls.mlx = "http://localhost:11432".to_string();
        assert_eq!(urls.mlx_api(), "http://localhost:11432/v1");
    }

    #[test]
    fn test_mlx_api_idempotent_when_v1_present() {
        let mut urls = ServiceUrls::from_config();
        urls.mlx = "http://localhost:11432/v1".to_string();
        assert_eq!(urls.mlx_api(), "http://localhost:11432/v1");
    }

    #[test]
    fn test_mlx_api_strips_trailing_slash() {
        let mut urls = ServiceUrls::from_config();
        urls.mlx = "http://localhost:11432/v1/".to_string();
        assert_eq!(urls.mlx_api(), "http://localhost:11432/v1");
    }

    #[test]
    fn test_mlx_api_strips_trailing_slash_without_v1() {
        let mut urls = ServiceUrls::from_config();
        urls.mlx = "http://localhost:11432/".to_string();
        assert_eq!(urls.mlx_api(), "http://localhost:11432/v1");
    }

    #[test]
    fn test_mlx_auth_header_returns_bearer_when_key_present() {
        let mut urls = ServiceUrls::from_config();
        urls.mlx_api_key = "secret-key".to_string();
        let (name, value) = urls.mlx_auth_header().expect("header must be present");
        assert_eq!(name, "Authorization");
        assert_eq!(value, "Bearer secret-key");
    }
}
