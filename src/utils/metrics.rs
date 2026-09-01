// 可观测性: 进程级 metrics 计数器/直方图, 落盘快照供 fusion metrics 子命令读取。
// 原子计数, 无锁; 每次操作增量更新 ~/.fusion/metrics/metrics.json 快照。
// 设计原则 (Rule 5): 计数用代码, 不走模型; 快照为 JSON 便于外接 Prometheus exporter。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNT: AtomicU64 = AtomicU64::new(0);
static REQUEST_ERROR: AtomicU64 = AtomicU64::new(0);
static MODEL_PULL_COUNT: AtomicU64 = AtomicU64::new(0);
static KB_INGEST_COUNT: AtomicU64 = AtomicU64::new(0);
static BENCH_RUN_COUNT: AtomicU64 = AtomicU64::new(0);
static SERVICE_OPS_COUNT: AtomicU64 = AtomicU64::new(0);

// 延迟分桶 (P50/P95 近似): ms 桶 [0-50, 50-200, 200-500, 500-2000, 2000+]。
static LAT_0_50: AtomicU64 = AtomicU64::new(0);
static LAT_50_200: AtomicU64 = AtomicU64::new(0);
static LAT_200_500: AtomicU64 = AtomicU64::new(0);
static LAT_500_2000: AtomicU64 = AtomicU64::new(0);
static LAT_2000_INF: AtomicU64 = AtomicU64::new(0);

pub fn inc_request() {
    REQUEST_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_request_error() {
    REQUEST_ERROR.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_model_pull() {
    MODEL_PULL_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_kb_ingest() {
    KB_INGEST_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_bench_run() {
    BENCH_RUN_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn inc_service_op() {
    SERVICE_OPS_COUNT.fetch_add(1, Ordering::Relaxed);
}

// 记录一次操作延迟 (ms) 到分桶。
pub fn observe_latency_ms(ms: u64) {
    let bucket = match ms {
        0..=49 => &LAT_0_50,
        50..=199 => &LAT_50_200,
        200..=499 => &LAT_200_500,
        500..=1999 => &LAT_500_2000,
        _ => &LAT_2000_INF,
    };
    bucket.fetch_add(1, Ordering::Relaxed);
}

fn metrics_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let dir = home.join(".fusion").join("metrics");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("metrics.json"))
}

#[derive(Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub request_count: u64,
    pub request_error: u64,
    pub model_pull_count: u64,
    pub kb_ingest_count: u64,
    pub bench_run_count: u64,
    pub service_ops_count: u64,
    pub latency_buckets_ms: BTreeMap<String, u64>,
}

pub fn snapshot() -> MetricsSnapshot {
    let mut buckets = BTreeMap::new();
    buckets.insert("0-50".to_string(), LAT_0_50.load(Ordering::Relaxed));
    buckets.insert("50-200".to_string(), LAT_50_200.load(Ordering::Relaxed));
    buckets.insert("200-500".to_string(), LAT_200_500.load(Ordering::Relaxed));
    buckets.insert("500-2000".to_string(), LAT_500_2000.load(Ordering::Relaxed));
    buckets.insert("2000+".to_string(), LAT_2000_INF.load(Ordering::Relaxed));
    MetricsSnapshot {
        request_count: REQUEST_COUNT.load(Ordering::Relaxed),
        request_error: REQUEST_ERROR.load(Ordering::Relaxed),
        model_pull_count: MODEL_PULL_COUNT.load(Ordering::Relaxed),
        kb_ingest_count: KB_INGEST_COUNT.load(Ordering::Relaxed),
        bench_run_count: BENCH_RUN_COUNT.load(Ordering::Relaxed),
        service_ops_count: SERVICE_OPS_COUNT.load(Ordering::Relaxed),
        latency_buckets_ms: buckets,
    }
}

// 落盘快照 (覆盖写, metrics 是当前累计值非追加日志)。
pub fn flush() -> Result<()> {
    let path = metrics_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve metrics path (no home dir)"))?;
    let snap = snapshot();
    let json = serde_json::to_string_pretty(&snap)?;
    std::fs::write(&path, json)?;
    Ok(())
}

// 读取已落盘快照 (fusion metrics 子命令用)。
pub fn read_snapshot() -> Result<MetricsSnapshot> {
    let path = metrics_path()
        .ok_or_else(|| anyhow::anyhow!("cannot resolve metrics path (no home dir)"))?;
    if !path.exists() {
        return Ok(empty_snapshot());
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content).unwrap_or_else(|_| empty_snapshot()))
}

fn empty_snapshot() -> MetricsSnapshot {
    MetricsSnapshot {
        request_count: 0,
        request_error: 0,
        model_pull_count: 0,
        kb_ingest_count: 0,
        bench_run_count: 0,
        service_ops_count: 0,
        latency_buckets_ms: BTreeMap::new(),
    }
}

pub fn metrics_path_display() -> String {
    metrics_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "(unresolvable)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observe_latency_buckets() {
        // 注意: 原子计数器是进程级全局, 此测试验证桶路由逻辑非绝对值。
        let before_0_50 = LAT_0_50.load(Ordering::Relaxed);
        let before_2000 = LAT_2000_INF.load(Ordering::Relaxed);
        observe_latency_ms(10);
        observe_latency_ms(3000);
        assert_eq!(LAT_0_50.load(Ordering::Relaxed), before_0_50 + 1);
        assert_eq!(LAT_2000_INF.load(Ordering::Relaxed), before_2000 + 1);
    }

    #[test]
    fn test_snapshot_has_all_fields() {
        let snap = snapshot();
        assert!(snap.latency_buckets_ms.len() == 5);
        assert!(snap.latency_buckets_ms.contains_key("0-50"));
        assert!(snap.latency_buckets_ms.contains_key("2000+"));
    }

    #[test]
    fn test_empty_snapshot_no_panic() {
        let e = empty_snapshot();
        assert_eq!(e.request_count, 0);
        assert!(e.latency_buckets_ms.is_empty());
    }
}
