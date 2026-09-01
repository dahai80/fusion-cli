// G3: cron 持久化 — desk 定时任务落到 ~/.fusion/cron.json。
// 仅持久化 (task, rule, created_at); 不执行 (执行仍靠 crontab / fusion-desk 服务)。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronEntry {
    pub task: String,
    pub rule: String,
    pub created_at: String,
}

fn cron_file() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("cannot resolve $HOME")?;
    let dir = home.join(".fusion");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("cron.json"))
}

pub fn load() -> Result<Vec<CronEntry>> {
    let path = cron_file()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "cron.json 读取失败, 视为空");
            return Ok(Vec::new());
        }
    };
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    let entries: Vec<CronEntry> =
        serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("cron.json 解析失败: {}", e))?;
    Ok(entries)
}

fn save(entries: &[CronEntry]) -> Result<()> {
    let path = cron_file()?;
    let json = serde_json::to_string_pretty(entries)?;
    // 原子写: 先写临时文件再 rename, 避免半写损坏。
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &path)?;
    info!(path = %path.display(), count = entries.len(), "cron 持久化完成");
    Ok(())
}

// ISO8601 时间戳 (无外部 chrono 依赖, 用 std SystemTime → 秒 → 格式化)。
fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{}", secs)
}

pub fn upsert(task: &str, rule: &str) -> Result<CronEntry> {
    let mut entries = load()?;
    // 同 task 覆盖旧规则 (去重), 其余保留。
    entries.retain(|e| e.task != task);
    let entry = CronEntry {
        task: task.to_string(),
        rule: rule.to_string(),
        created_at: now_iso(),
    };
    entries.push(entry.clone());
    save(&entries)?;
    Ok(entry)
}

pub fn remove(task: &str) -> Result<bool> {
    let mut entries = load()?;
    let before = entries.len();
    entries.retain(|e| e.task != task);
    let removed = entries.len() < before;
    if removed {
        save(&entries)?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::HOME_LOCK;

    fn tmp_home() -> std::path::PathBuf {
        let tmp = std::env::temp_dir().join(format!(
            "fusion-cron-test-{}-{}",
            std::process::id(),
            "case"
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        tmp
    }

    #[test]
    fn test_upsert_then_load() {
        let _guard = HOME_LOCK.lock().unwrap();
        let home = tmp_home();
        // edition 2024: set_var/remove_var are unsafe.
        unsafe { std::env::set_var("HOME", &home) };
        std::fs::create_dir_all(home.join(".fusion")).unwrap();

        upsert("task_a", "0 9 * * *").unwrap();
        upsert("task_b", "0 21 * * *").unwrap();
        let loaded = load().unwrap();
        assert_eq!(loaded.len(), 2);

        unsafe { std::env::remove_var("HOME") };
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn test_upsert_overwrites_same_task() {
        let _guard = HOME_LOCK.lock().unwrap();
        let home = tmp_home();
        unsafe { std::env::set_var("HOME", &home) };
        std::fs::create_dir_all(home.join(".fusion")).unwrap();

        upsert("task_a", "0 9 * * *").unwrap();
        upsert("task_a", "0 10 * * *").unwrap();
        let loaded = load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].rule, "0 10 * * *");

        unsafe { std::env::remove_var("HOME") };
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn test_remove_existing() {
        let _guard = HOME_LOCK.lock().unwrap();
        let home = tmp_home();
        unsafe { std::env::set_var("HOME", &home) };
        std::fs::create_dir_all(home.join(".fusion")).unwrap();

        upsert("task_a", "0 9 * * *").unwrap();
        assert!(remove("task_a").unwrap());
        assert!(!remove("task_a").unwrap());
        assert!(load().unwrap().is_empty());

        unsafe { std::env::remove_var("HOME") };
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn test_load_missing_file_is_empty() {
        let _guard = HOME_LOCK.lock().unwrap();
        let home = tmp_home();
        unsafe { std::env::set_var("HOME", &home) };
        // 不创建 cron.json
        assert!(load().unwrap().is_empty());

        unsafe { std::env::remove_var("HOME") };
        let _ = std::fs::remove_dir_all(&home);
    }
}
