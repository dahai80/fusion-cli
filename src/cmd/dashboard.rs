use anyhow::Result;
use tracing::info;

pub async fn run_dashboard() -> Result<()> {
    info!("Launching TUI dashboard");
    crate::tui::run_dashboard().await
}
