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
use std::sync::{Arc, Mutex};
use std::time::Duration;

// A4 修复: ServiceUrls::from_config() 之前每次调用都全量读盘 + 解析 TOML,
// 一次推理内 service_urls() + base_url() 触发 2 次读盘; TUI 刷新每 2s 10+ 次读盘。
// 改为 mtime 失效缓存: 仅当 config.toml 修改时间变化时重新解析。
static CONFIG_CACHE: Mutex<Option<(std::time::SystemTime, crate::config::FusionConfig)>> =
    Mutex::new(None);

pub fn cached_config() -> crate::config::FusionConfig {
    let path = crate::config::get_config_path();
    let mtime = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::UNIX_EPOCH);
    if let Ok(guard) = CONFIG_CACHE.lock()
        && let Some((cached_mtime, cfg)) = guard.as_ref()
        && *cached_mtime == mtime
    {
        return cfg.clone();
    }
    let cfg = crate::config::load_config();
    if let Ok(mut guard) = CONFIG_CACHE.lock() {
        *guard = Some((mtime, cfg.clone()));
    }
    cfg
}

pub fn invalidate_config_cache() {
    if let Ok(mut guard) = CONFIG_CACHE.lock() {
        *guard = None;
    }
}

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

// 共享路径段校验 (A14 修复): memory/multinode 原各有一份 byte-identical 副本。
// 现集中到 service 层, 供所有服务 URL 路径段 (kb/rag/desk/modelhub/benchsvc) 复用。
// 拒 '/' 和 '\', 防 id="x/../../delete" 注入额外路径段。
pub fn validate_path_segment(field: &str, value: &str) -> anyhow::Result<()> {
    if value.contains('/') || value.contains('\\') {
        anyhow::bail!(
            "invalid {}: must not contain path separators: '{}'",
            field,
            value
        );
    }
    Ok(())
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
        let config = cached_config();
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
        let base = self.mlx_base();
        format!("{}/v1", base)
    }

    // MLX 根路径 (不含 /v1), 供 /health /stats 等非 /v1/* 端点使用。
    // 集中 trim 逻辑, 消除各处重复 trim_end_matches('/v1') 的漂移风险 (A1 收敛)。
    pub fn mlx_base(&self) -> String {
        self.mlx
            .trim_end_matches('/')
            .trim_end_matches("/v1")
            .trim_end_matches('/')
            .to_string()
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
    // R4 修复: 之前所有错误统一 false → "not running", 无法区分服务未启动 / URL 配错 /
    // TLS 错 / 超时。保留 bool 入口 (调用方只关心存活), 但内部用 check_url_verbose
    // 记录错误类型, doctor 可据此给出根因。
    check_url_verbose(url, timeout_secs).await.0
}

// 返回 (alive, 可读根因)。alive=false 时 reason 区分: 连接拒绝 / 超时 / URL 非法 / 其他。
// R4: 让配置笔误 (URL 非法) 与服务未启动 (连接拒绝) 可区分。
pub async fn check_url_verbose(url: &str, timeout_secs: u64) -> (bool, String) {
    let client = get_client();
    match client
        .get(url)
        .timeout(Duration::from_secs(timeout_secs))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                (true, "ok".to_string())
            } else {
                (false, format!("HTTP {}", status))
            }
        }
        Err(e) => {
            let reason = if e.is_connect() {
                "connection refused (service not running or wrong port)".to_string()
            } else if e.is_timeout() {
                "timeout (service slow or firewall dropping)".to_string()
            } else if e.is_request() {
                format!("invalid request/URL: {}", e)
            } else {
                format!("{}", e)
            };
            (false, reason)
        }
    }
}

// R3 修复: 可重试 HTTP GET。对瞬时错误 (连接拒绝 / 超时 / 5xx) 退避重试,
// 致命错误 (URL 非法 / 4xx) 立即失败。用于 cluster/sync 跨机抖动场景。
pub async fn check_url_with_retry(url: &str, timeout_secs: u64, max_retries: u32) -> bool {
    let mut delay = Duration::from_millis(200);
    for attempt in 0..=max_retries {
        let (alive, reason) = check_url_verbose(url, timeout_secs).await;
        if alive {
            return true;
        }
        // 致命: HTTP 4xx 或 URL 非法 → 不重试。
        if reason.starts_with("HTTP 4") || reason.starts_with("invalid request") {
            tracing::warn!(url = url, reason = %reason, attempt, "check_url non-retryable failure");
            return false;
        }
        if attempt < max_retries {
            tracing::info!(url = url, reason = %reason, attempt, "check_url retrying after backoff");
            tokio::time::sleep(delay).await;
            delay = delay.saturating_mul(2).min(Duration::from_secs(2));
        }
    }
    false
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

    // A14/P1-2: 共享路径段校验必须拦死 '/' 和 '\' (路径穿越注入)。
    #[test]
    fn test_validate_path_segment_rejects_separators() {
        assert!(validate_path_segment("id", "x/../../delete").is_err());
        assert!(validate_path_segment("id", "a\\b").is_err());
        assert!(validate_path_segment("id", "/").is_err());
    }

    #[test]
    fn test_validate_path_segment_accepts_safe() {
        assert!(validate_path_segment("id", "node-1").is_ok());
        assert!(validate_path_segment("id", "mem-abc-123").is_ok());
    }
}
