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
            PermissionTier::Ask => true,
            PermissionTier::Auto => true,
        }
    }

    #[allow(dead_code)]
    pub fn requires_confirmation(&self, tool_name: &str) -> bool {
        match self {
            PermissionTier::Sandbox => false,
            PermissionTier::Ask => {
                let dangerous: HashSet<&str> = ["delete_model", "stop_task", "shell", "run_task"]
                    .into_iter()
                    .collect();
                dangerous.contains(tool_name)
            }
            PermissionTier::Auto => false,
        }
    }
}
