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
        // G2 扩工具: 只读 kb_query/service_status + 有副作用 model_pull。
        tools.insert("kb_query".to_string(), Box::new(KbQueryTool));
        tools.insert("service_status".to_string(), Box::new(ServiceStatusTool));
        tools.insert("model_pull".to_string(), Box::new(ModelPullTool));
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
        let tokens: u32 = match args.get("tokens") {
            None => 128,
            Some(v) => v
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid tokens '{}', expected integer", v))?,
        };
        let result = crate::service::mlx::generate_tokens(model, tokens).await?;
        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "tokens_per_sec": result.tokens_per_sec,
            "elapsed_secs": result.elapsed_secs,
            "completion_tokens": result.completion_tokens,
        }))?)
    }
}

// G2: 只读知识库查询 — 调 service::kb::query。
struct KbQueryTool;

#[async_trait::async_trait]
impl Tool for KbQueryTool {
    async fn execute(&self, args: &HashMap<String, String>) -> Result<String> {
        let kb_id = args
            .get("kb_id")
            .ok_or_else(|| anyhow::anyhow!("Missing 'kb_id' argument"))?;
        let question = args
            .get("question")
            .ok_or_else(|| anyhow::anyhow!("Missing 'question' argument"))?;
        let top_k: usize = match args.get("top_k") {
            None => 3,
            Some(v) => v
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid top_k '{}', expected integer", v))?,
        };
        let data = crate::service::kb::query(kb_id, question, top_k).await?;
        Ok(serde_json::to_string_pretty(&data)?)
    }
}

// G2: 只读生态服务健康检查 — 调 service::health::check_named。
struct ServiceStatusTool;

#[async_trait::async_trait]
impl Tool for ServiceStatusTool {
    async fn execute(&self, args: &HashMap<String, String>) -> Result<String> {
        let name = args
            .get("name")
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' argument"))?;
        let status = crate::service::health::check_named(name).await?;
        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "name": status.name,
            "url": status.url,
            "alive": status.alive,
            "port": status.port,
            "latency_ms": status.latency_ms,
        }))?)
    }
}

// G2: 有副作用模型拉取 — 调 service::modelhub::download_model (占网络/磁盘)。
struct ModelPullTool;

#[async_trait::async_trait]
impl Tool for ModelPullTool {
    async fn execute(&self, args: &HashMap<String, String>) -> Result<String> {
        let name = args
            .get("name")
            .ok_or_else(|| anyhow::anyhow!("Missing 'name' argument"))?;
        let path = crate::service::modelhub::download_model(name).await?;
        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "model": name, "path": path, "pulled": true,
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
        // G2 扩工具
        assert!(exec.is_known("kb_query"));
        assert!(exec.is_known("service_status"));
        assert!(exec.is_known("model_pull"));
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
        assert_eq!(names.len(), 7);
        assert_eq!(names, {
            let mut v = names.to_vec();
            v.sort();
            v
        });
    }
}
