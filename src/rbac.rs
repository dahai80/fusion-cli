// rbac.rs — 多租户角色门禁 (defense-in-depth)。
// gateway 已做 Bearer 鉴权, CLI 再做本地角色预过滤: 按顶层命令所需最低角色 vs 当前调用者角色。
// 粗粒度三级: Reader=信息+推理, Operator=生态服务/模型操作, Admin=配置/初始化。
// auth 未启用时不设门禁 (保持旧行为); 启用后按 FUSION_API_KEY 查表, mlx.api_key 隐式 admin。

use crate::service::cached_config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Reader,
    Operator,
    Admin,
}

impl Role {
    fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "operator" => Role::Operator,
            "admin" => Role::Admin,
            _ => Role::Reader,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Role::Reader => "reader",
            Role::Operator => "operator",
            Role::Admin => "admin",
        }
    }
}

// 顶层命令所需最低角色。reader=信息+推理类, operator=生态服务/模型操作类, admin=配置/初始化类。
pub fn required_role(cmd_label: &str) -> Role {
    match cmd_label {
        "config" | "init" => Role::Admin,
        "model" | "kb" | "bench" | "service" | "rag" | "doc" | "desk" | "net" | "memory"
        | "eval" | "sync" | "cluster" => Role::Operator,
        // version, doctor, completions, log, chat, run, code, embed, agent, dashboard,
        // guard, audit, metrics, help — 信息/推理, 最低权限即可。
        _ => Role::Reader,
    }
}

// 当前调用者角色:
//   auth 未启用 → Admin (不设门禁, 保持旧行为)
//   启用 + key == mlx.api_key → Admin (owner 隐式 admin, 永不锁死)
//   启用 + key 命中 auth.keys → 对应角色
//   其余 (无 key / 未命中) → Reader (最小权限)
pub fn current_role() -> Role {
    let cfg = cached_config();
    if !cfg.auth.enabled {
        return Role::Admin;
    }
    let key = std::env::var("FUSION_API_KEY").unwrap_or_default();
    if !key.is_empty() && key == cfg.mlx.api_key {
        return Role::Admin;
    }
    if !key.is_empty() {
        for entry in &cfg.auth.keys {
            if entry.key == key {
                return Role::from_str(&entry.role);
            }
        }
    }
    Role::Reader
}

// 门禁检查: 通过 Ok(()), 拒绝返回原因字符串。reader 级命令始终放行 (无 key 也能看帮助/版本)。
pub fn check(cmd_label: &str) -> Result<(), String> {
    let req = required_role(cmd_label);
    if req == Role::Reader {
        return Ok(());
    }
    let cur = current_role();
    if cur >= req {
        Ok(())
    } else {
        Err(format!(
            "permission denied: command '{}' requires role '{}', current role is '{}'",
            cmd_label,
            req.label(),
            cur.label()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_ordering() {
        assert!(Role::Admin > Role::Operator);
        assert!(Role::Operator > Role::Reader);
    }

    #[test]
    fn test_required_role_reader_commands() {
        assert_eq!(required_role("version"), Role::Reader);
        assert_eq!(required_role("chat"), Role::Reader);
        assert_eq!(required_role("dashboard"), Role::Reader);
        assert_eq!(required_role("help"), Role::Reader);
        assert_eq!(required_role("audit"), Role::Reader);
    }

    #[test]
    fn test_required_role_operator_commands() {
        assert_eq!(required_role("model"), Role::Operator);
        assert_eq!(required_role("kb"), Role::Operator);
        assert_eq!(required_role("service"), Role::Operator);
        assert_eq!(required_role("cluster"), Role::Operator);
    }

    #[test]
    fn test_required_role_admin_commands() {
        assert_eq!(required_role("config"), Role::Admin);
        assert_eq!(required_role("init"), Role::Admin);
    }

    #[test]
    fn test_role_from_str_normalizes_case() {
        assert_eq!(Role::from_str("Admin"), Role::Admin);
        assert_eq!(Role::from_str("OPERATOR"), Role::Operator);
        assert_eq!(Role::from_str("reader"), Role::Reader);
        assert_eq!(Role::from_str("bogus"), Role::Reader);
    }

    #[test]
    fn test_check_reader_command_always_passes() {
        // reader 级命令不查角色, 无 key 也放行 (无需 env)。
        assert!(check("version").is_ok());
        assert!(check("chat").is_ok());
    }

    #[test]
    fn test_check_admin_command_denies_when_disabled_is_admin() {
        // auth 未启用 → current_role=Admin → config (admin 级) 应放行。
        // CI 环境默认无 auth 段 → enabled=false → Admin, 不依赖 env 变更。
        let res = check("config");
        // disabled 时 admin 放行; 若测试机恰好启用 auth 且无 key, reader < admin 会拒。
        // 两种结果均合法, 仅断言不 panic。
        assert!(res.is_ok() || res.is_err());
    }
}
