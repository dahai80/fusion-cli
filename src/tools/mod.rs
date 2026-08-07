use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

pub struct ToolExecutor {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        let mut tools: HashMap<String, Box<dyn Tool>> = HashMap::new();
        tools.insert("list_models".to_string(), Box::new(ListModelsTool));
        tools.insert("model_info".to_string(), Box::new(ModelInfoTool));
        tools.insert("health".to_string(), Box::new(HealthTool));
        tools.insert("bench_speed".to_string(), Box::new(BenchSpeedTool));
        Self { tools }
    }

    pub async fn execute(&self, name: &str, args: &HashMap<String, String>) -> Result<String> {
        match self.tools.get(name) {
            Some(tool) => {
                info!(tool = name, "Executing tool");
                tool.execute(args).await
            }
            None => anyhow::bail!("Unknown tool: {}", name),
        }
    }

    #[allow(dead_code)]
    pub fn list_tools(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.tools.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    pub fn is_known(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

#[async_trait::async_trait]
trait Tool: Send + Sync {
    async fn execute(&self, args: &HashMap<String, String>) -> Result<String>;
}

struct ListModelsTool;

#[async_trait::async_trait]
impl Tool for ListModelsTool {
    async fn execute(&self, _args: &HashMap<String, String>) -> Result<String> {
        let models = crate::service::mlx::list_models().await?;
        let list: Vec<String> = models.iter().map(|m| m.id.clone()).collect();
        Ok(serde_json::to_string_pretty(&list)?)
    }
}

struct ModelInfoTool;

#[async_trait::async_trait]
impl Tool for ModelInfoTool {
    async fn execute(&self, args: &HashMap<String, String>) -> Result<String> {
        let model = args
            .get("model")
            .ok_or_else(|| anyhow::anyhow!("Missing 'model' argument"))?;
        let models = crate::service::mlx::list_models().await?;
        let found = models.iter().find(|m| m.id == *model);
        match found {
            Some(m) => Ok(serde_json::to_string_pretty(&m)?),
            None => Ok(format!("Model '{}' not found", model)),
        }
    }
}

struct HealthTool;

#[async_trait::async_trait]
impl Tool for HealthTool {
    async fn execute(&self, _args: &HashMap<String, String>) -> Result<String> {
        let alive = crate::service::mlx::health_check().await?;
        Ok(if alive {
            "MLX is running"
        } else {
            "MLX is not running"
        }
        .to_string())
    }
}

struct BenchSpeedTool;

#[async_trait::async_trait]
impl Tool for BenchSpeedTool {
    async fn execute(&self, args: &HashMap<String, String>) -> Result<String> {
        let model = args
            .get("model")
            .ok_or_else(|| anyhow::anyhow!("Missing 'model' argument"))?;
        let tokens: u32 = args
            .get("tokens")
            .and_then(|v| v.parse().ok())
            .unwrap_or(128);
        let result = crate::service::mlx::generate_tokens(model, tokens).await?;
        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "tokens_per_sec": result.tokens_per_sec,
            "elapsed_secs": result.elapsed_secs,
            "completion_tokens": result.completion_tokens,
        }))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_known_recognizes_registered_tools() {
        let exec = ToolExecutor::new();
        assert!(exec.is_known("list_models"));
        assert!(exec.is_known("model_info"));
        assert!(exec.is_known("health"));
        assert!(exec.is_known("bench_speed"));
    }

    #[test]
    fn test_is_known_rejects_unknown_tool() {
        let exec = ToolExecutor::new();
        assert!(!exec.is_known("nonexistent_tool"));
        assert!(!exec.is_known(""));
    }

    #[test]
    fn test_list_tools_is_sorted_and_complete() {
        let exec = ToolExecutor::new();
        let names = exec.list_tools();
        assert_eq!(names.len(), 4);
        assert_eq!(names, {
            let mut v = names.to_vec();
            v.sort();
            v
        });
    }
}
