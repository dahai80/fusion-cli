use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PermissionTier {
    Sandbox,
    Ask,
    Auto,
}

impl PermissionTier {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sandbox" => PermissionTier::Sandbox,
            "auto" => PermissionTier::Auto,
            _ => PermissionTier::Ask,
        }
    }

    pub fn allow_tool(&self, tool_name: &str) -> bool {
        match self {
            PermissionTier::Sandbox => {
                let safe: HashSet<&str> = ["list_models", "model_info", "health", "list_templates"]
                    .into_iter()
                    .collect();
                safe.contains(tool_name)
            }
            PermissionTier::Ask | PermissionTier::Auto => true,
        }
    }

    pub fn requires_confirmation(&self, tool_name: &str) -> bool {
        match self {
            PermissionTier::Sandbox | PermissionTier::Auto => false,
            PermissionTier::Ask => {
                let dangerous: HashSet<&str> = ["delete_model", "stop_task", "shell", "run_task"]
                    .into_iter()
                    .collect();
                dangerous.contains(tool_name)
            }
        }
    }

    pub async fn confirm(&self, tool_name: &str) -> bool {
        if !self.requires_confirmation(tool_name) {
            return true;
        }
        let prompt = format!("Allow agent to run tool '{}'?", tool_name);
        dialoguer::Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()
            .unwrap_or(false)
    }
}
