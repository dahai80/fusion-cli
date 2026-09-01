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
                // 仅允许只读工具 (注册表中实际存在的)。bench_speed 有副作用 (发起推理/占 GPU) → 禁。
                let safe: HashSet<&str> = ["list_models", "model_info", "health"]
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
                // 仅对有副作用的已注册工具要求确认。bench_speed 真实发起推理, 占用 GPU。
                // 之前的危险名单 (delete_model/stop_task/shell/run_task) 全部未在注册表中,
                // 导致确认提示永不触发 — 现修正为实际注册的有副作用工具。
                let dangerous: HashSet<&str> = ["bench_speed"].into_iter().collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_allows_readonly_tools() {
        let tier = PermissionTier::Sandbox;
        assert!(tier.allow_tool("list_models"));
        assert!(tier.allow_tool("model_info"));
        assert!(tier.allow_tool("health"));
    }

    #[test]
    fn test_sandbox_blocks_side_effect_tools() {
        let tier = PermissionTier::Sandbox;
        // bench_speed 有副作用 (发起推理), sandbox 必须禁。
        assert!(!tier.allow_tool("bench_speed"));
        assert!(!tier.allow_tool("nonexistent_tool"));
    }

    #[test]
    fn test_sandbox_no_longer_refs_unregistered_list_templates() {
        let tier = PermissionTier::Sandbox;
        // list_templates 未在 ToolRegistry 注册, 不应在 sandbox 白名单中。
        assert!(!tier.allow_tool("list_templates"));
    }

    #[test]
    fn test_ask_requires_confirmation_for_bench_speed() {
        let tier = PermissionTier::Ask;
        // 关键回归: bench_speed 是实际注册的有副作用工具, Ask tier 必须要求确认。
        assert!(tier.requires_confirmation("bench_speed"));
    }

    #[test]
    fn test_ask_no_confirmation_for_readonly_tools() {
        let tier = PermissionTier::Ask;
        assert!(!tier.requires_confirmation("list_models"));
        assert!(!tier.requires_confirmation("model_info"));
        assert!(!tier.requires_confirmation("health"));
    }

    #[test]
    fn test_ask_no_confirmation_for_unregistered_phantom_tools() {
        let tier = PermissionTier::Ask;
        // 旧危险名单引用的未注册工具不应再触发确认 (避免幻觉闸门)。
        assert!(!tier.requires_confirmation("delete_model"));
        assert!(!tier.requires_confirmation("shell"));
        assert!(!tier.requires_confirmation("stop_task"));
    }

    #[test]
    fn test_auto_never_requires_confirmation() {
        let tier = PermissionTier::Auto;
        assert!(!tier.requires_confirmation("bench_speed"));
        assert!(tier.allow_tool("bench_speed"));
    }

    #[test]
    fn test_from_str_parses_tiers() {
        assert_eq!(PermissionTier::from_str("sandbox"), PermissionTier::Sandbox);
        assert_eq!(PermissionTier::from_str("auto"), PermissionTier::Auto);
        assert_eq!(PermissionTier::from_str("ask"), PermissionTier::Ask);
        assert_eq!(PermissionTier::from_str("unknown"), PermissionTier::Ask);
    }
}
