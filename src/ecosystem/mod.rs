// ecosystem/mod.rs — 对接 Model-Hub / KB / Desk 接口

use anyhow::Result;
use serde::Deserialize;

/// Model-Hub 客户端
pub mod model_hub {
    use super::*;

    const HUB_URL: &str = "http://localhost:11444";

    /// 模型信息
    #[derive(Debug, Deserialize)]
    pub struct ModelEntry {
        pub id: String,
        pub name: String,
        pub size: String,
        pub quant: String,
    }

    /// 列出所有可用模型
    pub async fn list_models() -> Result<Vec<ModelEntry>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/v1/models", HUB_URL))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;
        let data: Vec<ModelEntry> = resp.json().await?;
        Ok(data)
    }
}

/// Fusion-KB 客户端
pub mod knowledge_base {
    use super::*;

    const KB_URL: &str = "http://localhost:11434";

    /// 知识库信息
    #[derive(Debug, Deserialize)]
    pub struct KbInfo {
        pub id: String,
        pub name: String,
        pub document_count: u32,
        pub status: String,
    }

    /// 列出所有知识库
    pub async fn list_bases() -> Result<Vec<KbInfo>> {
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/kb/bases", KB_URL))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await?;
        let data: Vec<KbInfo> = resp.json().await?;
        Ok(data)
    }

    /// 语义检索
    pub async fn query(kb_id: &str, question: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "question": question,
            "top_k": 5,
        });
        let resp = client
            .post(format!("{}/kb/bases/{}/query", KB_URL, kb_id))
            .json(&payload)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;
        let data: serde_json::Value = resp.json().await?;
        Ok(data["answer"].as_str().unwrap_or("").to_string())
    }
}

/// Fusion-Desk 客户端
pub mod desk {
    use super::*;

    const DESK_URL: &str = "http://localhost:9000";

    /// 触发自动化任务
    pub async fn run_task(template: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "template": template,
        });
        let resp = client
            .post(format!("{}/api/tasks/run", DESK_URL))
            .json(&payload)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await?;
        let data: serde_json::Value = resp.json().await?;
        Ok(data["task_id"].as_str().unwrap_or("").to_string())
    }
}