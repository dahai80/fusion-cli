// 企业级审计轨迹: append-only JSONL, 记录谁/何时/做了什么/结果/耗时。
// #51 hash chain 防篡改: 每条含 prev_hash + hash = SHA256(prev_hash || canon)。
// 篡改任意记录 → 后续 hash 不匹配, `fusion audit verify` 检出第一处断裂。
// 合规盲区修复 — 操作可追溯, 不可篡改 (追加模式, 不覆盖 + 链式哈希)。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
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

// 创世 prev_hash (全 0), 首条记录的 prev_hash。
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Serialize, Deserialize, Clone)]
pub struct AuditRecord {
    pub ts: f64,
    pub actor: String,
    pub command: String,
    pub outcome: String,
    pub duration_ms: u64,
    pub detail: String,
    // #51: 上一条记录的 hash; 首条 = GENESIS_HASH。无此字段的老记录按 GENESIS 处理。
    #[serde(default = "genesis_hash_default")]
    pub prev_hash: String,
    // #51: 本条 hash = SHA256(prev_hash || ts || actor || command || outcome || duration_ms || detail)。
    #[serde(default)]
    pub hash: String,
}

fn genesis_hash_default() -> String {
    GENESIS_HASH.to_string()
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

// 计算单条记录的 hash: SHA256(prev_hash || ts || actor || command || outcome || duration_ms || detail)。
// 字段用 \x1f (unit separator) 连接防字段值含分隔符导致碰撞。
pub fn compute_hash(rec: &AuditRecord) -> String {
    let mut hasher = Sha256::new();
    hasher.update(rec.prev_hash.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(rec.ts.to_string().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(rec.actor.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(rec.command.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(rec.outcome.as_bytes());
    hasher.update(b"\x1f");
    hasher.update(rec.duration_ms.to_string().as_bytes());
    hasher.update(b"\x1f");
    hasher.update(rec.detail.as_bytes());
    format!("{:x}", hasher.finalize())
}

// 跨进程互斥: flock(LOCK_EX) 排它锁整个 audit.log 文件描述符。
// 并发 fusion run 各为独立进程, 共享 Mutex 无效; 文件锁是唯一可靠串行化手段。
// 调用方持锁期间 read last_hash → append, 保证原子性, 防分叉链。
fn flock_exclusive(fd: std::os::unix::io::RawFd) -> std::io::Result<()> {
    let r = unsafe { libc::flock(fd, libc::LOCK_EX) };
    if r == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn funlock(fd: std::os::unix::io::RawFd) {
    unsafe {
        libc::flock(fd, libc::LOCK_UN);
    }
}

// 追加一条审计记录。即使写入失败也不阻断主流程 (审计为旁路), 仅记 tracing::error。
// #51 并发安全: 持 flock(LOCK_EX) 期间 read last_hash + append, 原子, 防竞态分叉链。
pub fn record(command: &str, outcome: &str, duration_ms: u64, detail: &str) {
    let (secs, nanos) = now_unix();
    let ts = secs as f64 + nanos as f64 / 1_000_000_000.0;
    let path = match audit_path() {
        Some(p) => p,
        None => {
            tracing::error!("audit path resolve failed (no home dir)");
            return;
        }
    };
    let mut f = match std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&path)
    {
        Ok(f) => f,
        Err(e) => {
            tracing::error!(error = %e, path = %path.display(), "audit open failed");
            return;
        }
    };
    let fd = f.as_raw_fd();
    if let Err(e) = flock_exclusive(fd) {
        tracing::error!(error = %e, path = %path.display(), "audit flock(LOCK_EX) failed");
        return;
    }
    let prev_hash = {
        let _u = f.seek(SeekFrom::Start(0)).ok();
        let content = std::io::read_to_string(&mut f).unwrap_or_default();
        content
            .lines()
            .next_back()
            .and_then(|l| serde_json::from_str::<AuditRecord>(l).ok())
            .filter(|r| !r.hash.is_empty())
            .map(|r| r.hash)
            .unwrap_or_else(|| GENESIS_HASH.to_string())
    };
    let rec = AuditRecord {
        ts,
        actor: std::env::var("USER").unwrap_or_else(|_| "unknown".to_string()),
        command: command.to_string(),
        outcome: outcome.to_string(),
        duration_ms,
        detail: redact_detail(detail),
        prev_hash,
        hash: String::new(),
    };
    let mut rec = rec;
    rec.hash = compute_hash(&rec);
    let line = match serde_json::to_string(&rec) {
        Ok(s) => s,
        Err(e) => {
            funlock(fd);
            tracing::error!(error = %e, "audit serialize failed");
            return;
        }
    };
    if let Err(e) = f
        .seek(SeekFrom::End(0))
        .and_then(|_| writeln!(f, "{}", line))
    {
        tracing::error!(error = %e, "audit write failed");
    }
    funlock(fd);
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

// #51 校验 hash chain: 逐条重算 hash, 与记录的 hash + 下一条 prev_hash 比对。
// 返回 Ok(()) 全链有效; Err(断裂信息) 首处 tamper。
// 链断类型: (1) hash 不匹配 (本条被篡改) (2) prev_hash 不指向上一条 (链被插/删/重排)。
pub fn verify_chain() -> Result<()> {
    let path = audit_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve audit log path (no home dir)"))?;
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&path)?;
    let mut prev = GENESIS_HASH.to_string();
    for (idx, line) in content.lines().enumerate() {
        let rec: AuditRecord = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => anyhow::bail!("line {}: parse failed: {}", idx + 1, e),
        };
        if rec.prev_hash != prev {
            anyhow::bail!(
                "line {}: prev_hash mismatch (expected {}, got {}) — chain tampered or reordered",
                idx + 1,
                &prev[..16],
                &rec.prev_hash[..16]
            );
        }
        let expected = compute_hash(&rec);
        if rec.hash != expected {
            anyhow::bail!(
                "line {}: hash mismatch (record tampered) — stored {}, recomputed {}",
                idx + 1,
                &rec.hash[..16],
                &expected[..16]
            );
        }
        prev = rec.hash;
    }
    Ok(())
}

pub fn audit_path_display() -> String {
    audit_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unresolvable)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 跨模块共享 HOME 锁 (audit + crypto 串行改 HOME)。
    use crate::utils::HOME_LOCK;

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
        let recs = read_recent(0).unwrap();
        assert!(recs.is_empty());
    }

    // #51: compute_hash 确定性 — 同字段同输入同输出。
    #[test]
    fn test_compute_hash_deterministic() {
        let rec = AuditRecord {
            ts: 1000.0,
            actor: "u".to_string(),
            command: "cmd".to_string(),
            outcome: "ok".to_string(),
            duration_ms: 5,
            detail: "d".to_string(),
            prev_hash: GENESIS_HASH.to_string(),
            hash: String::new(),
        };
        let h1 = compute_hash(&rec);
        let h2 = compute_hash(&rec);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        // 改任一字段 → hash 变。
        let mut rec2 = rec.clone();
        rec2.detail = "d2".to_string();
        assert_ne!(compute_hash(&rec2), h1);
    }

    // #51: prev_hash 不同 → hash 不同 (链式依赖)。
    #[test]
    fn test_compute_hash_depends_on_prev() {
        let base = AuditRecord {
            ts: 1.0,
            actor: "u".to_string(),
            command: "cmd".to_string(),
            outcome: "ok".to_string(),
            duration_ms: 1,
            detail: "d".to_string(),
            prev_hash: GENESIS_HASH.to_string(),
            hash: String::new(),
        };
        let mut changed = base.clone();
        changed.prev_hash = "abc".to_string();
        assert_ne!(compute_hash(&base), compute_hash(&changed));
    }

    // #51: 老记录 (无 prev_hash/hash 字段) 解析 → 默认 GENESIS, 不 panic。
    #[test]
    fn test_old_record_parses_with_defaults() {
        let old =
            r#"{"ts":1.0,"actor":"u","command":"cmd","outcome":"ok","duration_ms":1,"detail":"d"}"#;
        let rec: AuditRecord = serde_json::from_str(old).expect("old record must parse");
        assert_eq!(rec.prev_hash, GENESIS_HASH);
        assert!(rec.hash.is_empty());
    }

    // #51: verify_chain 全链有效 (用临时 HOME, 写真实 record 流程)。
    #[test]
    fn test_verify_chain_valid() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "fusion-audit-verify-{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let prev_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &tmp);
        }
        // 写 3 条。
        record("cmd-a", "ok", 1, "d1");
        record("cmd-b", "ok", 2, "d2");
        record("cmd-c", "fail", 3, "d3");
        let res = verify_chain();
        if let Some(h) = prev_home {
            unsafe {
                std::env::set_var("HOME", h);
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(res.is_ok(), "untampered chain must verify: {:?}", res);
    }

    // #51: 篡改中间一行 → verify_chain 检出断裂。
    #[test]
    fn test_verify_chain_detects_tamper() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "fusion-audit-tamper-{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let prev_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &tmp);
        }
        record("cmd-a", "ok", 1, "d1");
        record("cmd-b", "ok", 2, "d2");
        record("cmd-c", "ok", 3, "d3");
        // 篡改第 2 行的 detail (不重算 hash) → 第 2 行 hash 不匹配。
        let log_path = tmp.join(".fusion").join("audit").join("audit.log");
        let content = std::fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert!(
            lines.len() >= 3,
            "expected 3 records, got {} — HOME race?",
            lines.len()
        );
        let tampered = lines[1].replace("\"d2\"", "\"TAMPERED\"");
        let new_content = format!("{}\n{}\n{}\n", lines[0], tampered, lines[2]);
        std::fs::write(&log_path, new_content).unwrap();
        let res = verify_chain();
        if let Some(h) = prev_home {
            unsafe {
                std::env::set_var("HOME", h);
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);
        assert!(res.is_err(), "tampered chain must fail verify: {:?}", res);
        let msg = res.unwrap_err().to_string();
        assert!(
            msg.contains("line 2"),
            "error must point at tampered line: {}",
            msg
        );
    }

    // #51 并发安全: 多线程同时 record() → 链不分叉, verify_chain 通过。
    // 回归: 旧实现 last_hash+append 非原子, 并发各读同 prev_hash → 分叉链。
    // flock(LOCK_EX) 串行化后, 每条 prev_hash 正确指向上条 hash。
    #[test]
    fn test_record_concurrent_no_chain_fork() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "fusion-audit-conc-{}",
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let prev_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", &tmp);
        }
        let n_threads = 12;
        let per_thread = 8;
        let handles: Vec<_> = (0..n_threads)
            .map(|t| {
                std::thread::spawn(move || {
                    for i in 0..per_thread {
                        record(
                            &format!("cmd-{}-{}", t, i),
                            "ok",
                            i as u64,
                            &format!("detail-{}-{}", t, i),
                        );
                    }
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let res = verify_chain();
        if let Some(h) = prev_home {
            unsafe {
                std::env::set_var("HOME", h);
            }
        }
        let log_path = tmp.join(".fusion").join("audit").join("audit.log");
        let count = std::fs::read_to_string(&log_path)
            .map(|c| c.lines().filter(|l| !l.is_empty()).count())
            .unwrap_or(0);
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(
            count,
            n_threads * per_thread,
            "every concurrent record must land exactly once"
        );
        assert!(
            res.is_ok(),
            "concurrent record must not fork chain: {:?}",
            res
        );
    }
}
