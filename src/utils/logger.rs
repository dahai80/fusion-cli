use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

// P1-6 修复: 之前日志只输出到终端, 退出后无持久化记录, 排障无据。
// 现在并行写文件 (~/.fusion/fusion-cli.log), 终端保持无时间戳精简输出,
// 文件层带时间戳供事后审计。日志路径与 config paths 约定一致。
pub fn init_logger(verbose: bool) {
    let level = if verbose { "debug" } else { "info" };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("fusion_cli={}", level)));

    let file_appender = build_file_appender();
    let console_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .without_time();

    match file_appender {
        Some(writer) => {
            let file_layer = tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_target(true);
            tracing_subscriber::registry()
                .with(env_filter)
                .with(console_layer)
                .with(file_layer)
                .init();
        }
        None => {
            // 文件目录不可建时回退到纯终端, 不让日志初始化失败导致 CLI 退出。
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .without_time()
                .init();
        }
    }
}

// 构建文件 appender (rolling::never = 单文件追加, 不按日轮转)。
// 返回 None 时调用方回退纯终端日志。
fn build_file_appender() -> Option<tracing_appender::rolling::RollingFileAppender> {
    let home = dirs::home_dir()?;
    let log_dir = home.join(".fusion").join("logs");
    std::fs::create_dir_all(&log_dir).ok()?;
    Some(tracing_appender::rolling::never(&log_dir, "fusion-cli.log"))
}
