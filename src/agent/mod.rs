pub mod context;
pub mod loop_engine;
pub mod permission;

use anyhow::Result;
use colored::*;
use std::collections::HashMap;
use tracing::info;

use crate::service::mlx;
use crate::tools::ToolExecutor;

pub struct AgentConfig {
    pub model: String,
    pub max_turns: u32,
    pub permission_tier: permission::PermissionTier,
    pub system_prompt: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model: "default".to_string(),
            max_turns: 10,
            permission_tier: permission::PermissionTier::Ask,
            system_prompt: None,
        }
    }
}

pub struct Agent {
    config: AgentConfig,
    context: context::ContextManager,
    tools: ToolExecutor,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        let tools = ToolExecutor::new();
        let context = context::ContextManager::new(config.max_turns);
        Self {
            config,
            context,
            tools,
        }
    }

    pub async fn run(&mut self, user_input: &str) -> Result<String> {
        info!(model = %self.config.model, input_len = user_input.len(), "Starting agent loop");

        self.context.add_user_message(user_input);

        let mut turns = 0u32;
        let mut final_response = String::new();

        loop {
            if turns >= self.config.max_turns {
                info!(turns = turns, "Agent reached max turns");
                final_response.push_str("\n[Agent reached maximum turns]");
                break;
            }

            let messages = self.context.build_messages(&self.config.system_prompt);
            let request = mlx::InferenceRequest {
                model: self.config.model.clone(),
                messages,
                temperature: Some(0.7),
                max_tokens: Some(2048),
                stream: None,
            };

            info!(turn = turns, "Sending inference request");
            let response = match mlx::chat_completion(&request).await {
                Ok(r) => r,
                Err(e) => {
                    info!(error = %e, "Inference request failed");
                    final_response = format!("Error: {}", e);
                    break;
                }
            };

            let content = response
                .choices
                .first()
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();

            let tool_calls = extract_tool_calls(&content);

            if tool_calls.is_empty() {
                self.context.add_assistant_message(&content);
                final_response = content;
                break;
            }

            self.context.add_assistant_message(&content);

            for tc in &tool_calls {
                if !self.config.permission_tier.allow_tool(&tc.tool_name) {
                    let msg = format!("[Permission denied for tool: {}]", tc.tool_name);
                    info!(tool = %tc.tool_name, "Tool call denied by permission tier");
                    self.context.add_system_message(&msg);
                    continue;
                }

                info!(tool = %tc.tool_name, args = ?tc.args, "Executing tool");
                match self.tools.execute(&tc.tool_name, &tc.args).await {
                    Ok(result) => {
                        self.context.add_system_message(&format!(
                            "[Tool {} result]: {}",
                            tc.tool_name, result
                        ));
                    }
                    Err(e) => {
                        info!(tool = %tc.tool_name, error = %e, "Tool execution failed");
                        self.context
                            .add_system_message(&format!("[Tool {} error]: {}", tc.tool_name, e));
                    }
                }
            }

            turns += 1;
        }

        Ok(final_response)
    }
}

struct ToolCall {
    tool_name: String,
    args: HashMap<String, String>,
}

fn extract_tool_calls(content: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let mut remaining = content;

    while let Some(start) = remaining.find("```tool") {
        remaining = &remaining[start + 7..];
        let end = remaining.find("```").unwrap_or(remaining.len());
        let body = remaining[..end].trim();
        remaining = &remaining[end.min(remaining.len())..];

        let mut lines = body.lines();
        if let Some(tool_name) = lines.next() {
            let tool_name = tool_name.trim().to_string();
            let mut args = HashMap::new();
            for line in lines {
                let line = line.trim();
                if let Some((k, v)) = line.split_once('=') {
                    args.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
            calls.push(ToolCall { tool_name, args });
        }
    }

    calls
}

pub async fn run_agent(model: &str, prompt: &str, permission: &str) -> Result<()> {
    println!();
    println!("{}", "🤖 Fusion Agent".bold());
    println!("  Model:      {}", model.cyan());
    println!("  Permission: {}", permission.cyan());
    println!();

    let tier = permission::PermissionTier::from_str(permission);
    let config = AgentConfig {
        model: model.to_string(),
        permission_tier: tier,
        max_turns: 10,
        system_prompt: Some(
            "You are a helpful AI assistant with access to tools. When you need to use a tool, \
             output it in a code block starting with ```tool followed by the tool name on the \
             first line and key=value arguments on subsequent lines. Close with ```."
                .to_string(),
        ),
    };

    let mut agent = Agent::new(config);
    let response = agent.run(prompt).await?;

    println!("{} {}", "Assistant:".green().bold(), response);
    Ok(())
}
