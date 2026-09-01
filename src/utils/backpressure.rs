// utils/backpressure.rs — #52 限流熔断 (backpressure)
//
// 两道保护, 装在 MLX 推理调用点 (chat/embedding/bench) 之前:
//   1. RateLimiter (token bucket): 单进程 QPS 上限, 防 CLI 脚本短时狂打后端。
//   2. CircuitBreaker (Closed/Open/HalfOpen): 连续失败达阈值 → Open 拒绝 N 秒;
//      冷却后 HalfOpen 放 1 探测, 成功回 Closed / 失败回 Open。后端宕机时 fail-fast,
//      不让每个命令都等满 120s 超时。
// 全局 MLX breaker (once_cell), 配置驱动 ([backpressure] 段), 默认启用温和阈值。

use std::sync::Mutex;
use std::time::{Duration, Instant};

// ---------- 配置 ----------

// #52 backpressure 配置段。默认启用, 阈值温和 (不误伤正常使用)。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackpressureConfig {
    // 令牌桶容量 (突发上限), 默认 10。
    #[serde(default = "default_capacity")]
    pub capacity: u32,
    // 令牌补充速率 (tokens/sec), 默认 5。
    #[serde(default = "default_refill_rate")]
    pub refill_rate: u32,
    // 熔断: 连续失败多少次后 Open, 默认 5。
    #[serde(default = "default_failure_threshold")]
    pub failure_threshold: u32,
    // 熔断 Open 持续秒数, 默认 30。
    #[serde(default = "default_open_secs")]
    pub open_secs: u64,
    // 熔断启用开关 (false 时仅限流不限失败), 默认 true。
    #[serde(default = "default_breaker_enabled")]
    pub breaker_enabled: bool,
}

fn default_capacity() -> u32 {
    10
}
fn default_refill_rate() -> u32 {
    5
}
fn default_failure_threshold() -> u32 {
    5
}
fn default_open_secs() -> u64 {
    30
}
fn default_breaker_enabled() -> bool {
    true
}

impl Default for BackpressureConfig {
    fn default() -> Self {
        Self {
            capacity: default_capacity(),
            refill_rate: default_refill_rate(),
            failure_threshold: default_failure_threshold(),
            open_secs: default_open_secs(),
            breaker_enabled: default_breaker_enabled(),
        }
    }
}

// ---------- 令牌桶 ----------

pub struct TokenBucket {
    capacity: u32,
    refill_rate: u32,
    // (tokens, last_refill)
    state: Mutex<(f64, Instant)>,
}

impl TokenBucket {
    pub fn new(capacity: u32, refill_rate: u32) -> Self {
        Self {
            capacity,
            refill_rate,
            state: Mutex::new((capacity as f64, Instant::now())),
        }
    }

    // 尝试取 1 令牌。有则扣并返回 true; 无则返回 false (调用方自行决定丢弃/等待)。
    // 限流是保护后端, 不是硬 SLA — 取不到直接返回 false, 不阻塞。
    pub fn try_acquire(&self) -> bool {
        let mut s = self.state.lock().unwrap();
        let now = Instant::now();
        let (tokens, last) = *s;
        let elapsed = now.duration_since(last).as_secs_f64();
        let refilled = (tokens + elapsed * self.refill_rate as f64).min(self.capacity as f64);
        if refilled >= 1.0 {
            *s = (refilled - 1.0, now);
            true
        } else {
            *s = (refilled, now);
            false
        }
    }
}

// ---------- 熔断器 ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakerState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    failure_threshold: u32,
    open_for: Duration,
    state: Mutex<BreakerInner>,
}

struct BreakerInner {
    state: BreakerState,
    consecutive_failures: u32,
    opened_at: Instant,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, open_secs: u64) -> Self {
        Self {
            failure_threshold,
            open_for: Duration::from_secs(open_secs),
            state: Mutex::new(BreakerInner {
                state: BreakerState::Closed,
                consecutive_failures: 0,
                opened_at: Instant::now(),
            }),
        }
    }

    // 调用前检查: 允许放行返回 true; Open 期返回 false (fail-fast)。
    // Open 冷却到期 → 自动转 HalfOpen 放 1 探测。
    pub fn allow(&self) -> bool {
        let mut inner = self.state.lock().unwrap();
        match inner.state {
            BreakerState::Closed => true,
            BreakerState::HalfOpen => true,
            BreakerState::Open => {
                if inner.opened_at.elapsed() >= self.open_for {
                    inner.state = BreakerState::HalfOpen;
                    tracing::warn!("circuit breaker: Open → HalfOpen (cooldown elapsed), probing");
                    true
                } else {
                    false
                }
            }
        }
    }

    // 调用成功上报: 重置失败计数, HalfOpen → Closed (恢复)。
    pub fn on_success(&self) {
        let mut inner = self.state.lock().unwrap();
        inner.consecutive_failures = 0;
        if inner.state == BreakerState::HalfOpen {
            tracing::info!("circuit breaker: HalfOpen → Closed (probe succeeded)");
            inner.state = BreakerState::Closed;
        }
    }

    // 调用失败上报: 计数; 达阈值 → Open; HalfOpen 探测失败 → 回 Open。
    pub fn on_failure(&self) {
        let mut inner = self.state.lock().unwrap();
        inner.consecutive_failures += 1;
        match inner.state {
            BreakerState::HalfOpen => {
                tracing::warn!("circuit breaker: HalfOpen → Open (probe failed)");
                inner.state = BreakerState::Open;
                inner.opened_at = Instant::now();
            }
            BreakerState::Closed if inner.consecutive_failures >= self.failure_threshold => {
                tracing::warn!(
                    failures = inner.consecutive_failures,
                    "circuit breaker: Closed → Open (failure threshold reached)"
                );
                inner.state = BreakerState::Open;
                inner.opened_at = Instant::now();
            }
            _ => {}
        }
    }

    pub fn state(&self) -> BreakerState {
        self.state.lock().unwrap().state
    }
}

// ---------- 全局 MLX backpressure (once_cell) ----------

use once_cell::sync::Lazy;

struct MlxBackpressure {
    bucket: TokenBucket,
    breaker: CircuitBreaker,
}

static MLX_BP: Lazy<MlxBackpressure> = Lazy::new(|| {
    let cfg = load_bp_config();
    MlxBackpressure {
        bucket: TokenBucket::new(cfg.capacity, cfg.refill_rate),
        breaker: CircuitBreaker::new(cfg.failure_threshold, cfg.open_secs),
    }
});

fn load_bp_config() -> BackpressureConfig {
    crate::config::load_config()
        .backpressure
        .clone()
        .unwrap_or_default()
}

// 调用 MLX 前调此门: 限流 + 熔断。返回 Ok(()) 放行; Err 拒绝 (带原因)。
// 限流被拒 → RateLimited; 熔断 Open → BreakerOpen。两者都不阻塞, fail-fast。
pub fn mlx_admit() -> Result<(), BackpressureError> {
    let cfg = load_bp_config();
    if !MLX_BP.bucket.try_acquire() {
        return Err(BackpressureError::RateLimited);
    }
    if cfg.breaker_enabled && !MLX_BP.breaker.allow() {
        return Err(BackpressureError::BreakerOpen);
    }
    Ok(())
}

// MLX 调用后上报结果。成功 reset, 失败计数。
pub fn mlx_report(success: bool) {
    if success {
        MLX_BP.breaker.on_success();
    } else {
        MLX_BP.breaker.on_failure();
    }
}

// doctor / 状态查看: 当前熔断态。
pub fn mlx_breaker_state() -> BreakerState {
    MLX_BP.breaker.state()
}

#[derive(Debug)]
pub enum BackpressureError {
    RateLimited,
    BreakerOpen,
}

impl std::fmt::Display for BackpressureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackpressureError::RateLimited => write!(
                f,
                "rate limited (token bucket empty) — slow down or raise backpressure.refill_rate"
            ),
            BackpressureError::BreakerOpen => write!(
                f,
                "circuit breaker open (MLX backend failing) — retry after backpressure.open_secs or check fusion doctor"
            ),
        }
    }
}

impl std::error::Error for BackpressureError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_burst_then_refill() {
        let b = TokenBucket::new(3, 100); // cap 3, fast refill
        assert!(b.try_acquire());
        assert!(b.try_acquire());
        assert!(b.try_acquire());
        // 突发用尽 → 拒。
        assert!(!b.try_acquire());
        // 等一点时间补令牌。
        std::thread::sleep(Duration::from_millis(30));
        assert!(b.try_acquire());
    }

    #[test]
    fn test_token_bucket_refill_capped_at_capacity() {
        let b = TokenBucket::new(2, 100);
        // 全用尽。
        assert!(b.try_acquire());
        assert!(b.try_acquire());
        assert!(!b.try_acquire());
        // 长时间不补超过 capacity。
        std::thread::sleep(Duration::from_millis(50));
        // 只能补到 2。
        assert!(b.try_acquire());
        assert!(b.try_acquire());
        assert!(!b.try_acquire());
    }

    #[test]
    fn test_breaker_closed_to_open_after_threshold() {
        let cb = CircuitBreaker::new(3, 60);
        assert_eq!(cb.state(), BreakerState::Closed);
        assert!(cb.allow());
        cb.on_failure();
        cb.on_failure();
        assert_eq!(cb.state(), BreakerState::Closed); // 2 < 3
        cb.on_failure();
        assert_eq!(cb.state(), BreakerState::Open); // 3 >= 3
        assert!(!cb.allow(), "Open must reject");
    }

    #[test]
    fn test_breaker_success_resets() {
        let cb = CircuitBreaker::new(3, 60);
        cb.on_failure();
        cb.on_failure();
        cb.on_success();
        assert_eq!(cb.state(), BreakerState::Closed);
        // 失败计数归零, 再 2 次失败不应 Open。
        cb.on_failure();
        cb.on_failure();
        assert_eq!(cb.state(), BreakerState::Closed);
    }

    #[test]
    fn test_breaker_open_to_halfopen_after_cooldown() {
        let cb = CircuitBreaker::new(1, 0); // 1 失败即 Open, 0s 冷却
        cb.on_failure();
        assert_eq!(cb.state(), BreakerState::Open);
        std::thread::sleep(Duration::from_millis(5));
        // 冷却到期 → allow() 转 HalfOpen 放行。
        assert!(cb.allow());
        assert_eq!(cb.state(), BreakerState::HalfOpen);
    }

    #[test]
    fn test_breaker_halfopen_probe_success_closes() {
        let cb = CircuitBreaker::new(1, 0);
        cb.on_failure();
        std::thread::sleep(Duration::from_millis(5));
        assert!(cb.allow()); // → HalfOpen
        cb.on_success(); // 探测成功 → Closed
        assert_eq!(cb.state(), BreakerState::Closed);
    }

    #[test]
    fn test_breaker_halfopen_probe_failure_reopens() {
        let cb = CircuitBreaker::new(1, 0);
        cb.on_failure();
        std::thread::sleep(Duration::from_millis(5));
        assert!(cb.allow()); // → HalfOpen
        cb.on_failure(); // 探测失败 → Open
        assert_eq!(cb.state(), BreakerState::Open);
    }
}
