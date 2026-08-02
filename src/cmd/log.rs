use anyhow::Result;
use clap::Subcommand;
use colored::*;

#[derive(Subcommand)]
pub enum LogCommands {
    /// 实时查看全生态日志
    View {
        /// 显示行数
        #[arg(short, long, default_value_t = 50)]
        lines: usize,
        /// 日志文件路径
        #[arg(short, long)]
        path: Option<String>,
    },
    /// 清空所有日志
    Clear,
}

pub async fn handle_log(action: LogCommands) -> Result<()> {
    match action {
        LogCommands::View { lines, path } => view_log(lines, path).await,
        LogCommands::Clear => clear_log().await,
    }
}

async fn view_log(lines: usize, path: Option<String>) -> Result<()> {
    let log_path = path.unwrap_or_else(|| {
        let home = dirs::home_dir().unwrap_or_default();
        home.join(".fusion")
            .join("fusion-cli.log")
            .to_string_lossy()
            .to_string()
    });

    let path = std::path::Path::new(&log_path);
    if !path.exists() {
        println!("{} No log file found at: {}", "ℹ️".blue(), log_path.cyan());
        return Ok(());
    }

    let content = std::fs::read_to_string(path)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let total = all_lines.len();
    let start = total.saturating_sub(lines);

    println!(
        "{} Last {} lines of {}:",
        "📋".bold(),
        lines.to_string().cyan(),
        log_path.cyan()
    );
    println!("{}", "─".repeat(60).dimmed());

    for line in &all_lines[start..] {
        println!("{}", line);
    }

    if start > 0 {
        println!("{}", "─".repeat(60).dimmed());
        println!(
            "{} ... and {} more lines (use --lines to show more)",
            "ℹ️".blue(),
            (total - lines).to_string().cyan()
        );
    }

    Ok(())
}

async fn clear_log() -> Result<()> {
    let home = dirs::home_dir().unwrap_or_default();
    let log_path = home.join(".fusion").join("fusion-cli.log");

    if log_path.exists() {
        std::fs::write(&log_path, "")?;
        println!(
            "{} Log file cleared: {}",
            "✅".green(),
            log_path.display().to_string().cyan()
        );
    } else {
        println!("{} No log file found.", "ℹ️".blue());
    }

    Ok(())
}
