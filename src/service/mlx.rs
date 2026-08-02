use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::info;

use super::{ServiceUrls, get_client};

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct InferenceResponse {
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Choice {
    pub message: ResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMessage {
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChatChunk {
    pub choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChunkChoice {
    pub delta: ChunkDelta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct ChunkDelta {
    pub content: Option<String>,
}

fn base_url() -> String {
    ServiceUrls::from_config().mlx_api()
}

#[allow(dead_code)]
fn stats_url() -> String {
    ServiceUrls::from_config()
        .mlx
        .trim_end_matches("/v1")
        .trim_end_matches('/')
        .to_string()
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let url = format!("{}/models", base_url());
    match client
        .get(&url)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(e) => {
            info!("MLX health check failed: {}", e);
            Ok(false)
        }
    }
}

#[allow(dead_code)]
pub async fn list_models() -> Result<Vec<ModelInfo>> {
    let client = get_client();
    let url = format!("{}/models", base_url());
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    let models: Vec<ModelInfo> = serde_json::from_value(data["data"].clone())?;
    Ok(models)
}

pub async fn chat_completion(request: &InferenceRequest) -> Result<InferenceResponse> {
    let client = get_client();
    let url = format!("{}/chat/completions", base_url());
    info!(url = %url, model = %request.model, "Sending chat completion request");
    let resp = client
        .post(&url)
        .json(request)
        .timeout(Duration::from_secs(120))
        .send()
        .await?;
    let response: InferenceResponse = resp.json().await?;
    Ok(response)
}

#[allow(dead_code)]
pub async fn chat_completion_stream(request: &InferenceRequest) -> Result<reqwest::Response> {
    let mut stream_request = request.clone();
    stream_request.stream = Some(true);
    let client = get_client();
    let url = format!("{}/chat/completions", base_url());
    info!(url = %url, model = %stream_request.model, "Sending streaming chat request");
    let resp = client
        .post(&url)
        .json(&stream_request)
        .timeout(Duration::from_secs(120))
        .send()
        .await?;
    Ok(resp)
}

pub async fn create_embedding(model: &str, input: &str) -> Result<Vec<f64>> {
    let client = get_client();
    let url = format!("{}/embeddings", base_url());
    let payload = serde_json::json!({
        "model": model,
        "input": input,
    });
    let resp = client
        .post(&url)
        .json(&payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    let embedding: Vec<f64> = serde_json::from_value(data["data"][0]["embedding"].clone())?;
    Ok(embedding)
}

#[allow(dead_code)]
pub async fn get_server_stats() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/stats", stats_url());
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

#[allow(dead_code)]
pub fn assert_fusion_mlx_only() -> Result<()> {
    let urls = ServiceUrls::from_config();
    let base = urls.mlx;
    assert!(
        base.contains("localhost:11434") || base.contains("127.0.0.1:11434"),
        "Fusion-CLI only supports fusion-mlx (localhost:11434)"
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
}
