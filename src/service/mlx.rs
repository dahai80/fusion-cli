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
    #[serde(default)]
    pub created: u64,
    #[serde(default)]
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
        .trim_end_matches('/')
        .trim_end_matches("/v1")
        .trim_end_matches('/')
        .to_string()
}

fn service_urls() -> ServiceUrls {
    ServiceUrls::from_config()
}

fn apply_auth(builder: reqwest::RequestBuilder, urls: &ServiceUrls) -> reqwest::RequestBuilder {
    if let Some((name, value)) = urls.mlx_auth_header() {
        builder.header(name, value)
    } else {
        builder
    }
}

pub async fn health_check() -> Result<bool> {
    let client = get_client();
    let urls = service_urls();
    let base: &str = &base_url();
    let base = base
        .trim_end_matches('/')
        .trim_end_matches("/v1")
        .trim_end_matches('/');
    let url = format!("{}/health", base);
    let req = apply_auth(client.get(&url), &urls);
    match req.timeout(Duration::from_secs(2)).send().await {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(e) => {
            info!("MLX health check failed: {}", e);
            Ok(false)
        }
    }
}

pub async fn list_models() -> Result<Vec<ModelInfo>> {
    let client = get_client();
    let urls = service_urls();
    let url = format!("{}/models", base_url());
    let resp = apply_auth(client.get(&url), &urls)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "MLX").await?;
    let models: Vec<ModelInfo> = serde_json::from_value(data["data"].clone())?;
    Ok(models)
}

pub async fn chat_completion(request: &InferenceRequest) -> Result<InferenceResponse> {
    let client = get_client();
    let urls = service_urls();
    let url = format!("{}/chat/completions", base_url());
    info!(url = %url, model = %request.model, "Sending chat completion request");
    let resp = apply_auth(client.post(&url), &urls)
        .json(request)
        .timeout(Duration::from_secs(120))
        .send()
        .await?;
    let response = parse_completion_response(resp).await?;
    Ok(response)
}

async fn parse_completion_response(resp: reqwest::Response) -> Result<InferenceResponse> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        info!(status = %status, body = %text.chars().take(300).collect::<String>(), "chat completion non-success");
        anyhow::bail!(
            "MLX request failed ({}): {}",
            status,
            text.chars().take(200).collect::<String>()
        );
    }
    let trimmed = text.trim_start();
    if trimmed.starts_with("data:") || trimmed.starts_with("event:") {
        info!("gateway returned SSE stream for non-streaming request; aggregating chunks");
        return aggregate_sse_to_response(&text);
    }
    let response: InferenceResponse = serde_json::from_str(&text)?;
    Ok(response)
}

fn aggregate_sse_to_response(raw: &str) -> Result<InferenceResponse> {
    let mut content = String::new();
    let mut model = String::new();
    let mut prompt_tokens = 0u32;
    let mut completion_tokens = 0u32;
    let mut finish_reason: Option<String> = None;
    for line in raw.lines() {
        let line = line.trim();
        if !line.starts_with("data:") {
            continue;
        }
        let data = line.trim_start_matches("data:").trim();
        if data == "[DONE]" || data.is_empty() {
            continue;
        }
        let chunk: serde_json::Value = match serde_json::from_str(data) {
            Ok(v) => v,
            Err(e) => {
                info!(data = %data, error = %e, "skipping malformed SSE chunk");
                continue;
            }
        };
        if model.is_empty()
            && let Some(m) = chunk.get("model").and_then(|v| v.as_str())
        {
            model = m.to_string();
        }
        if let Some(usage) = chunk.get("usage") {
            if let Some(pt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
                prompt_tokens = pt as u32;
            }
            if let Some(ct) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
                completion_tokens = ct as u32;
            }
        }
        if let Some(choices) = chunk.get("choices").and_then(|v| v.as_array()) {
            for choice in choices {
                if let Some(delta) = choice.get("delta")
                    && let Some(c) = delta.get("content").and_then(|v| v.as_str())
                {
                    content.push_str(c);
                }
                if let Some(fr) = choice.get("finish_reason").and_then(|v| v.as_str())
                    && !fr.is_empty()
                {
                    finish_reason = Some(fr.to_string());
                }
            }
        }
    }
    info!(
        content_len = content.len(),
        tokens = completion_tokens,
        "aggregated SSE into single response"
    );
    Ok(InferenceResponse {
        choices: vec![Choice {
            message: ResponseMessage {
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
            },
            finish_reason: Some(finish_reason.unwrap_or_else(|| "stop".to_string())),
        }],
        usage: if prompt_tokens > 0 || completion_tokens > 0 {
            Some(Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            })
        } else {
            None
        },
    })
}

pub async fn chat_completion_stream(request: &InferenceRequest) -> Result<reqwest::Response> {
    let mut stream_request = request.clone();
    stream_request.stream = Some(true);
    let client = get_client();
    let urls = service_urls();
    let url = format!("{}/chat/completions", base_url());
    info!(url = %url, model = %stream_request.model, "Sending streaming chat request");
    let resp = apply_auth(client.post(&url), &urls)
        .json(&stream_request)
        .timeout(Duration::from_secs(120))
        .send()
        .await?;
    Ok(resp)
}

pub async fn create_embedding(model: &str, input: &str) -> Result<Vec<f64>> {
    let client = get_client();
    let urls = service_urls();
    let url = format!("{}/embeddings", base_url());
    let payload = serde_json::json!({
        "model": model,
        "input": input,
    });
    let resp = apply_auth(client.post(&url), &urls)
        .json(&payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "MLX").await?;
    let embedding: Vec<f64> = serde_json::from_value(data["data"][0]["embedding"].clone())?;
    Ok(embedding)
}

pub async fn get_server_stats() -> Result<serde_json::Value> {
    let client = get_client();
    let urls = service_urls();
    let url = format!("{}/stats", stats_url());
    info!(url = %url, "Fetching MLX server stats");
    let resp = apply_auth(client.get(&url), &urls)
        .timeout(Duration::from_secs(5))
        .send()
        .await?;
    let data: serde_json::Value = super::json_or_error(resp, "MLX stats").await?;
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

    #[test]
    fn test_mlx_auth_header_present() {
        let urls = ServiceUrls::from_config();
        let header = urls.mlx_auth_header();
        assert!(
            header.is_some(),
            "mlx api_key should default to a non-empty gateway key"
        );
        let (name, value) = header.unwrap();
        assert_eq!(name, "Authorization");
        assert!(
            value.starts_with("Bearer "),
            "auth value must be Bearer scheme"
        );
    }

    #[test]
    fn test_mlx_auth_header_empty_when_no_key() {
        let mut urls = ServiceUrls::from_config();
        urls.mlx_api_key = String::new();
        assert!(
            urls.mlx_auth_header().is_none(),
            "empty key must yield no header"
        );
    }

    #[test]
    fn test_non_mlx_services_use_direct_ports() {
        let urls = ServiceUrls::from_config();
        assert!(
            !urls.kb.contains("11432")
                && !urls.modelhub.contains("11432")
                && !urls.rag.contains("11432")
                && !urls.desk.contains("11432")
                && !urls.doc.contains("11432"),
            "kb/modelhub/rag/desk/doc must use direct service ports, not gateway 11432"
        );
    }

    #[test]
    fn test_aggregate_sse_concats_delta_content() {
        let chunk1 = r#"{"model":"qwen","choices":[{"delta":{"content":"Hello"}}]}"#;
        let chunk2 = r#"{"model":"qwen","choices":[{"delta":{"content":" world"}}]}"#;
        let raw = format!("data: {}\n\ndata: {}\n\ndata: [DONE]\n", chunk1, chunk2);
        let resp = aggregate_sse_to_response(&raw).unwrap();
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(
            resp.choices[0].message.content.as_deref(),
            Some("Hello world")
        );
    }

    #[test]
    fn test_aggregate_sse_captures_usage() {
        let payload = r#"{"model":"qwen","choices":[{"delta":{"content":"hi"}}],"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7}}"#;
        let raw = format!("data: {}\n\ndata: [DONE]\n", payload);
        let resp = aggregate_sse_to_response(&raw).unwrap();
        assert!(resp.usage.is_some());
        let u = resp.usage.unwrap();
        assert_eq!(u.prompt_tokens, 5);
        assert_eq!(u.completion_tokens, 2);
    }

    #[test]
    fn test_aggregate_sse_empty_returns_none_content() {
        let resp = aggregate_sse_to_response("data: [DONE]\n").unwrap();
        assert_eq!(resp.choices[0].message.content, None);
        assert!(resp.usage.is_none());
    }

    #[test]
    fn test_stats_url_strips_v1_suffix() {
        let mut urls = ServiceUrls::from_config();
        urls.mlx = "http://localhost:11432/v1".to_string();
        let s = urls
            .mlx
            .trim_end_matches("/v1")
            .trim_end_matches('/')
            .to_string();
        assert_eq!(s, "http://localhost:11432");
    }

    #[test]
    fn test_aggregate_sse_propagates_finish_reason_length() {
        let chunk = r#"{"model":"qwen","choices":[{"delta":{},"finish_reason":"length"}]}"#;
        let raw = format!(
            "data: {}

data: [DONE]
",
            chunk
        );
        let resp = aggregate_sse_to_response(&raw).unwrap();
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("length"));
    }

    #[test]
    fn test_aggregate_sse_finish_reason_defaults_stop() {
        let chunk = r#"{"model":"qwen","choices":[{"delta":{"content":"hi"}}]}"#;
        let raw = format!(
            "data: {}

data: [DONE]
",
            chunk
        );
        let resp = aggregate_sse_to_response(&raw).unwrap();
        assert_eq!(resp.choices[0].finish_reason.as_deref(), Some("stop"));
    }
}
