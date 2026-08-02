pub mod desk;
pub mod doc;
pub mod gateway;
pub mod health;
pub mod kb;
pub mod mlx;
pub mod modelhub;
pub mod rag;

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
    pub kb: String,
    pub modelhub: String,
    pub rag: String,
    pub desk: String,
    pub doc: String,
}

impl ServiceUrls {
    pub fn from_config() -> Self {
        let config = crate::config::load_config();
        Self {
            mlx: config.mlx.base_url.clone(),
            kb: config.kb.base_url.clone(),
            modelhub: config.modelhub.base_url.clone(),
            rag: config.rag.base_url.clone(),
            desk: config.desk.base_url.clone(),
            doc: config.doc.base_url.clone(),
        }
    }

    pub fn mlx_api(&self) -> String {
        let base = self.mlx.trim_end_matches("/v1").trim_end_matches('/');
        format!("{}/v1", base)
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
