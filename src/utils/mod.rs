pub mod audit;
pub mod backpressure;
pub mod cron_store;
pub mod crypto;
pub mod logger;
pub mod metrics;
pub mod output;

// #51: HOME 是进程全局 env; audit/crypto 测试各自改 HOME 会跨模块竞态。
// 共享此锁串行化所有 HOME-mutating 测试 (audit + crypto)。
#[cfg(test)]
pub static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
