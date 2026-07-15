use tracing_subscriber::EnvFilter;

pub fn init_logger(verbose: bool) {
    let level = if verbose { "debug" } else { "info" };
    
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(format!("fusion_cli={}", level)))
        )
        .with_target(false)
        .without_time()
        .init();
}