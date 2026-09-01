// fusion metrics — 可观测性快照查看 (进程级计数器 + 延迟分桶)。

use anyhow::Result;
use clap::Subcommand;
use colored::*;

#[derive(Subcommand)]
pub enum MetricsCommands {
    /// 查看当前 metrics 快照
    View,
    /// 显示 metrics 落盘路径
    Path,
    /// 以 JSON 输出快照 (便于外接 Prometheus exporter / 脚本采集)
    Json,
}

pub async fn handle_metrics(action: MetricsCommands) -> Result<()> {
    match action {
        MetricsCommands::View => view_metrics().await,
        MetricsCommands::Path => {
            println!(
                "{} Metrics: {}",
                "📊".bold(),
                crate::utils::metrics::metrics_path_display().cyan()
            );
            Ok(())
        }
        MetricsCommands::Json => {
            let snap = crate::utils::metrics::read_snapshot()?;
            println!("{}", serde_json::to_string_pretty(&snap)?);
            Ok(())
        }
    }
}

async fn view_metrics() -> Result<()> {
    let snap = crate::utils::metrics::read_snapshot()?;
    println!("{} Fusion-CLI Metrics Snapshot", "📊".bold());
    println!("{}", "─".repeat(50).dimmed());
    println!("  {} Requests:    {}", "→".cyan(), snap.request_count);
    println!(
        "  {} Errors:       {}",
        "→".cyan(),
        snap.request_error.to_string().red()
    );
    println!("  {} Model pulls:  {}", "→".cyan(), snap.model_pull_count);
    println!("  {} KB ingests:   {}", "→".cyan(), snap.kb_ingest_count);
    println!("  {} Bench runs:   {}", "→".cyan(), snap.bench_run_count);
    println!("  {} Service ops:  {}", "→".cyan(), snap.service_ops_count);
    println!();
    println!("  {} Latency buckets (ms):", "⏱".cyan());
    for (bucket, count) in &snap.latency_buckets_ms {
        println!("    {:<12} {}", bucket.yellow(), count);
    }
    println!("{}", "─".repeat(50).dimmed());
    let err_rate = if snap.request_count > 0 {
        snap.request_error as f64 / snap.request_count as f64 * 100.0
    } else {
        0.0
    };
    println!("  {} Error rate: {:.1}%", "ℹ️".blue(), err_rate);
    Ok(())
}
