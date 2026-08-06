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
pub struct InferenceResponse {
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: ResponseMessage,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub total_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatChunk {
    pub choices: Vec<ChunkChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkChoice {
    pub delta: ChunkDelta,
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkDelta {
    pub content: Option<String>,
}

fn base_url() -> String {
    ServiceUrls::from_config().mlx_api()
}

fn stats_url() -> String {
    ServiceUrls::from_config()
        .mlx
        .trim_end_matches("/v1")
        .trim_end_matches('/')
        .to_string()
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let base: &str = &base_url();
    let base = base.trim_end_matches("/v1").trim_end_matches('/');
    let url = format!("{}/health", base);
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

pub async fn get_server_stats() -> Result<serde_json::Value> {
    let client = get_client();
    let url = format!("{}/stats", stats_url());
    info!(url = %url, "Fetching MLX server stats");
    let resp = client
        .get(&url)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = resp.json().await?;
    Ok(data)
}

pub struct BenchResult {
    #[allow(dead_code)]
    pub tokens_generated: u32,
    pub elapsed_secs: f64,
    pub tokens_per_sec: f64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

pub async fn generate_tokens(model: &str, max_tokens: u32) -> Result<BenchResult> {
    let request = InferenceRequest {
        model: model.to_string(),
        messages: vec![Message {
            role: "user".to_string(),
            content: "Write a detailed essay about artificial intelligence and its impact on society. Include examples, analysis, and future predictions.".to_string(),
        }],
        temperature: Some(0.7),
        max_tokens: Some(max_tokens),
        stream: None,
    };
    let start = std::time::Instant::now();
    let response = chat_completion(&request).await?;
    let elapsed = start.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    let prompt_tokens = response
        .usage
        .as_ref()
        .map(|u| u.prompt_tokens)
        .unwrap_or(0);
    let completion_tokens = response
        .usage
        .as_ref()
        .map(|u| u.completion_tokens)
        .unwrap_or(0);
    let tokens_per_sec = if elapsed_secs > 0.0 && completion_tokens > 0 {
        completion_tokens as f64 / elapsed_secs
    } else {
        0.0
    };
    Ok(BenchResult {
        tokens_generated: completion_tokens,
        elapsed_secs,
        tokens_per_sec,
        prompt_tokens,
        completion_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_url_uses_gateway_port() {
        let urls = ServiceUrls::from_config();
        assert!(
            urls.mlx.contains("localhost:11432") || urls.mlx.contains("127.0.0.1:11432"),
            "base_url should route through gateway (localhost:11432), got: {}",
            urls.mlx
        );
    }
}
