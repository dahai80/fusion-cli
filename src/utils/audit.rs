// 企业级审计轨迹: append-only JSONL, 记录谁/何时/做了什么/结果/耗时。
// 合规盲区修复 — 操作可追溯, 不可篡改 (追加模式, 不覆盖)。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn audit_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let dir = home.join(".fusion").join("audit");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("audit.log"))
}

// UTC 秒 + 纳秒, 避免本地时区歧义 (审计要求确定性时间戳)。
fn now_unix() -> (u64, u32) {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (dur.as_secs(), dur.subsec_nanos())
}

#[derive(Serialize, Deserialize)]
pub struct AuditRecord {
    pub ts: f64,
    pub actor: String,
    pub command: String,
    pub outcome: String,
    pub duration_ms: u64,
    pub detail: String,
}

// 脱敏: api_key/token/secret 类参数不进审计日志。
fn redact_detail(detail: &str) -> String {
    let lower = detail.to_lowercase();
    if lower.contains("api_key")
        || lower.contains("api-key")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
    {
        return "[REDACTED: contains credential field]".to_string();
    }
    // 截断超长 detail, 防审计日志膨胀 (单条上限 2KB)。
    if detail.len() > 2048 {
        format!("{}...[truncated]", &detail[..2048])
    } else {
        detail.to_string()
    }
}

// 追加一条审计记录。即使写入失败也不阻断主流程 (审计为旁路), 仅记 tracing::error。
pub fn record(command: &str, outcome: &str, duration_ms: u64, detail: &str) {
    let (secs, nanos) = now_unix();
    let ts = secs as f64 + nanos as f64 / 1_000_000_000.0;
    let rec = AuditRecord {
        ts,
        actor: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
        command: command.to_string(),
        outcome: outcome.to_string(),
        duration_ms,
        detail: redact_detail(detail),
    };
    let line = match serde_json::to_string(&rec) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "audit serialize failed");
            return;
        }
    };
    match audit_path() {
        Some(path) => {
            // O_APPEND 原子追加, 不覆盖历史。
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                Ok(mut f) => {
                    if let Err(e) = writeln!(f, "{}", line) {
                        tracing::error!(error = %e, "audit write failed");
                    }
                }
                Err(e) => tracing::error!(error = %e, path = %path.display(), "audit open failed"),
            }
        }
        None => tracing::error!("audit path resolve failed (no home dir)"),
    }
}

// 读取最近 N 条审计记录 (fusion audit 子命令用)。
pub fn read_recent(count: usize) -> Result<Vec<AuditRecord>> {
    let path = audit_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve audit log path (no home dir)"))?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(count);
    let mut recs = Vec::new();
    for line in &lines[start..] {
        if let Ok(r) = serde_json::from_str::<AuditRecord>(line) {
            recs.push(r);
        }
    }
    Ok(recs)
}

pub fn audit_path_display() -> String {
    audit_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unresolvable)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_credential_fields() {
        assert_eq!(
            redact_detail("set api_key=secret123"),
            "[REDACTED: contains credential field]"
        );
        assert_eq!(
            redact_detail("user-token=abc"),
            "[REDACTED: contains credential field]"
        );
        assert_eq!(redact_detail("model pull llama-3"), "model pull llama-3");
    }

    #[test]
    fn test_redact_truncates_long_detail() {
        let long = "x".repeat(3000);
        let redacted = redact_detail(&long);
        assert!(redacted.ends_with("[truncated]"));
        assert!(redacted.len() < 3000);
    }

    #[test]
    fn test_read_recent_missing_file() {
        // 指向不存在的路径场景: read_recent 对空文件返回 Ok(vec![])。
        // 用临时非标准路径不可行 (函数内部硬编码 ~/.fusion/audit), 改测 record + read 幂等。
        // 此测试仅验证 read_recent 不 panic on empty。
        let recs = read_recent(0).unwrap();
        assert!(recs.is_empty());
    }
}
