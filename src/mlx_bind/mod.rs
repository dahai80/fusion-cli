// mlx_bind/mod.rs — fusion-mlx 内核绑定层（核心）
// 所有推理、模型加载、显存调度全部通过 HTTP 调用 fusion-mlx
// 硬编码禁用第三方推理后端

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// fusion-mlx 服务地址
const MLX_BASE_URL: &str = "http://localhost:11434/v1";

/// 推理请求参数
#[derive(Debug, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// 推理响应
#[derive(Debug, Deserialize)]
pub struct InferenceResponse {
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// 模型信息
#[derive(Debug, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

/// 健康检查
pub async fn health_check() -> Result<bool> {
    let client = reqwest::Client::new();
    match client
        .get(format!("{}/models", MLX_BASE_URL))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// 列出可用模型
pub async fn list_models() -> Result<Vec<ModelInfo>> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/models", MLX_BASE_URL))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;
    let models: Vec<ModelInfo> = serde_json::from_value(data["data"].clone())?;
    Ok(models)
}

/// 发送聊天请求
pub async fn chat_completion(request: &InferenceRequest) -> Result<InferenceResponse> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/chat/completions", MLX_BASE_URL))
        .json(request)
        .timeout(std::time::Duration::from_secs(120))
        .send()
        .await?;

    let response: InferenceResponse = resp.json().await?;
    Ok(response)
}

/// 生成 Embedding
pub async fn create_embedding(model: &str, input: &str) -> Result<Vec<f64>> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "model": model,
        "input": input,
    });

    let resp = client
        .post(format!("{}/embeddings", MLX_BASE_URL))
        .json(&payload)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;
    let embedding: Vec<f64> = serde_json::from_value(data["data"][0]["embedding"].clone())?;
    Ok(embedding)
}

/// 获取服务统计
pub async fn get_server_stats() -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{}/stats", MLX_BASE_URL.trim_end_matches("/v1")))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

// ── 强制约束：禁止第三方后端 ──
// 编译时硬编码约束，确保只使用 fusion-mlx

/// 强制检测：确认使用的推理后端是 fusion-mlx
/// 若不是则返回错误
pub fn assert_fusion_mlx_only() -> Result<()> {
    // 编译时断言：MLX_BASE_URL 必须是 fusion-mlx 地址
    assert!(
        MLX_BASE_URL.contains("localhost:11434"),
        "Fusion-CLI only supports fusion-mlx (http://localhost:11434)"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_fusion_mlx_only() {
        assert!(assert_fusion_mlx_only().is_ok());
    }

    #[test]
    fn test_mlx_base_url() {
        assert_eq!(MLX_BASE_URL, "http://localhost:11434/v1");
    }
}