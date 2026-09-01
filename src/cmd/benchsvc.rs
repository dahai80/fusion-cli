use anyhow::Result;
use clap::Subcommand;
use colored::*;
use tabled::{Table, Tabled, settings::Style};
use tracing::error;

use crate::service::benchsvc as bs;
use crate::utils::output;

// 评测服务管理 — 对接 fusion-bench HTTP API (11467)。
// 与 `fusion bench speed/mem/ctx/auto` (本地直测 MLX) 区分:
//   `fusion bench` = 本地自测速度/显存/上下文
//   `fusion eval`  = 查询 fusion-bench 服务端: 任务/套件/结果/基线/质量门
const BENCH_DEFAULT_PORT: u16 = 11467;

#[derive(Subcommand)]
pub enum EvalCommands {
    /// 服务状态
    Status,
    /// 系统资源 (CPU/GPU/内存)
    Resources,
    /// 任务列表
    Tasks,
    /// 任务详情
    Task { task_id: String },
    /// 套件列表
    Suites,
    /// 评测结果
    Result { task_id: String },
    /// 结果趋势
    Trend,
    /// 基线列表
    Baselines,
    /// 质量门列表
    Gates,
}

pub async fn handle_eval(action: EvalCommands) -> Result<()> {
    match action {
        EvalCommands::Status => eval_status().await,
        EvalCommands::Resources => eval_resources().await,
        EvalCommands::Tasks => eval_tasks().await,
        EvalCommands::Task { task_id } => eval_task(task_id).await,
        EvalCommands::Suites => eval_suites().await,
        EvalCommands::Result { task_id } => eval_result(task_id).await,
        EvalCommands::Trend => eval_trend().await,
        EvalCommands::Baselines => eval_baselines().await,
        EvalCommands::Gates => eval_gates().await,
    }
}

async fn eval_status() -> Result<()> {
    println!();
    println!("{}", "📊 Fusion-Bench Service Status".bold());
    println!();

    let alive = bs::health_check().await.unwrap_or(false);

    if output::is_json_mode() {
        let payload = serde_json::json!({
            "service": "fusion-bench",
            "alive": alive,
            "port": BENCH_DEFAULT_PORT,
        });
        output::print_json(&payload)?;
        return Ok(());
    }

    let status = if alive {
        "✅ running".green().to_string()
    } else {
        "⬜ stopped".yellow().to_string()
    };

    let mut entries = vec![
        StatusEntry {
            key: "Service".to_string(),
            value: status,
        },
        StatusEntry {
            key: "Port".to_string(),
            value: BENCH_DEFAULT_PORT.to_string().cyan().to_string(),
        },
    ];

    if alive && let Ok(d) = bs::get_health_detail().await {
        let st = d
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("ok")
            .to_string();
        entries.push(StatusEntry {
            key: "Health".to_string(),
            value: st.cyan().to_string(),
        });
    }

    let mut table = Table::new(&entries);
    table.with(Style::modern());
    println!("{}", table);
    println!();

    if !alive {
        println!(
            "  {} Start: fusion-bench/start.sh (port 11467)",
            "💡".yellow()
        );
    }

    Ok(())
}

async fn eval_resources() -> Result<()> {
    println!();
    println!("{}", "🖥️  Bench System Resources".bold());
    println!();
    match bs::system_resources().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "bench resources error");
            println!("  {} fusion-bench not reachable: {}", "❌".red(), e);
            anyhow::bail!("Failed to get bench resources: {}", e);
        }
    }
    Ok(())
}

async fn eval_tasks() -> Result<()> {
    println!();
    println!("{}", "📋 Bench Tasks".bold());
    println!();
    match bs::list_tasks().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let tasks = data.as_array().cloned().unwrap_or_default();
            if tasks.is_empty() {
                println!("  {} No tasks.", "ℹ️".blue());
                return Ok(());
            }
            let mut entries = Vec::new();
            for t in &tasks {
                let id = t
                    .get("task_id")
                    .or_else(|| t.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let model = t
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let status = t
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let suite = t
                    .get("suite_id")
                    .or_else(|| t.get("suite"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                entries.push(TaskEntry {
                    id,
                    model,
                    suite,
                    status,
                });
            }
            let mut table = Table::new(&entries);
            table.with(Style::modern());
            println!("{}", table);
            println!();
            println!("  Total: {} tasks", entries.len().to_string().cyan());
        }
        Err(e) => {
            error!(error = %e, "bench list tasks error");
            println!("  {} fusion-bench not reachable: {}", "❌".red(), e);
            anyhow::bail!("Failed to list bench tasks: {}", e);
        }
    }
    Ok(())
}

async fn eval_task(task_id: String) -> Result<()> {
    println!();
    println!("{} Task {}", "🔍".bold(), task_id.cyan());
    println!();
    match bs::get_task(&task_id).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "bench get task error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Failed to get bench task {}: {}", task_id, e);
        }
    }
    Ok(())
}

async fn eval_suites() -> Result<()> {
    println!();
    println!("{}", "📦 Bench Suites".bold());
    println!();
    match bs::list_suites().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let suites = data.as_array().cloned().unwrap_or_default();
            if suites.is_empty() {
                println!("  {} No suites.", "ℹ️".blue());
                return Ok(());
            }
            let mut entries = Vec::new();
            for s in &suites {
                let id = s
                    .get("suite_id")
                    .or_else(|| s.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let name = s
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let cases = s
                    .get("case_count")
                    .or_else(|| s.get("cases"))
                    .and_then(|v| v.as_u64())
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "-".into());
                entries.push(SuiteEntry { id, name, cases });
            }
            let mut table = Table::new(&entries);
            table.with(Style::modern());
            println!("{}", table);
        }
        Err(e) => {
            error!(error = %e, "bench list suites error");
            println!("  {} fusion-bench not reachable: {}", "❌".red(), e);
            anyhow::bail!("Failed to list bench suites: {}", e);
        }
    }
    Ok(())
}

async fn eval_result(task_id: String) -> Result<()> {
    println!();
    println!("{} Result for {}", "📈".bold(), task_id.cyan());
    println!();
    match bs::get_result(&task_id).await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&data).unwrap_or_default()
            );
        }
        Err(e) => {
            error!(error = %e, "bench get result error");
            println!("  {} Failed: {}", "❌".red(), e);
            anyhow::bail!("Failed to get bench result {}: {}", task_id, e);
        }
    }
    Ok(())
}

async fn eval_trend() -> Result<()> {
    println!();
    println!("{}", "📊 Results Trend".bold());
    println!();
    match bs::results_trend().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let pts = data.as_array().cloned().unwrap_or_default();
            if pts.is_empty() {
                println!("  {} No trend data.", "ℹ️".blue());
                return Ok(());
            }
            let mut entries = Vec::new();
            for p in &pts {
                let ts = p
                    .get("timestamp")
                    .or_else(|| p.get("ts"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let metric = p
                    .get("metric")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let val = p
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .map(|x| format!("{:.3}", x))
                    .unwrap_or_else(|| "-".into());
                entries.push(TrendEntry {
                    timestamp: ts,
                    metric,
                    value: val,
                });
            }
            let mut table = Table::new(&entries);
            table.with(Style::modern());
            println!("{}", table);
        }
        Err(e) => {
            error!(error = %e, "bench trend error");
            println!("  {} fusion-bench not reachable: {}", "❌".red(), e);
            anyhow::bail!("Failed to get bench trend: {}", e);
        }
    }
    Ok(())
}

async fn eval_baselines() -> Result<()> {
    println!();
    println!("{}", "📏 Bench Baselines".bold());
    println!();
    match bs::list_baselines().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let bl = data.as_array().cloned().unwrap_or_default();
            if bl.is_empty() {
                println!("  {} No baselines.", "ℹ️".blue());
                return Ok(());
            }
            let mut entries = Vec::new();
            for b in &bl {
                let name = b
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let model = b
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let created = b
                    .get("created_at")
                    .or_else(|| b.get("timestamp"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                entries.push(BaselineEntry {
                    name,
                    model,
                    created,
                });
            }
            let mut table = Table::new(&entries);
            table.with(Style::modern());
            println!("{}", table);
        }
        Err(e) => {
            error!(error = %e, "bench baselines error");
            println!("  {} fusion-bench not reachable: {}", "❌".red(), e);
            anyhow::bail!("Failed to list baselines: {}", e);
        }
    }
    Ok(())
}

async fn eval_gates() -> Result<()> {
    println!();
    println!("{}", "🚦 Quality Gates".bold());
    println!();
    match bs::list_gates().await {
        Ok(data) => {
            if output::is_json_mode() {
                output::print_json(&data)?;
                return Ok(());
            }
            let gates = data.as_array().cloned().unwrap_or_default();
            if gates.is_empty() {
                println!("  {} No gates.", "ℹ️".blue());
                return Ok(());
            }
            let mut entries = Vec::new();
            for g in &gates {
                let id = g
                    .get("gate_id")
                    .or_else(|| g.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let name = g
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let tier = g
                    .get("tier")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                let status = g
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .to_string();
                entries.push(GateEntry {
                    id,
                    name,
                    tier,
                    status,
                });
            }
            let mut table = Table::new(&entries);
            table.with(Style::modern());
            println!("{}", table);
        }
        Err(e) => {
            error!(error = %e, "bench gates error");
            println!("  {} fusion-bench not reachable: {}", "❌".red(), e);
            anyhow::bail!("Failed to list gates: {}", e);
        }
    }
    Ok(())
}

#[derive(Tabled)]
struct StatusEntry {
    #[tabled(rename = "Key")]
    key: String,
    #[tabled(rename = "Value")]
    value: String,
}

#[derive(Tabled)]
struct TaskEntry {
    #[tabled(rename = "Task ID")]
    id: String,
    #[tabled(rename = "Model")]
    model: String,
    #[tabled(rename = "Suite")]
    suite: String,
    #[tabled(rename = "Status")]
    status: String,
}

#[derive(Tabled)]
struct SuiteEntry {
    #[tabled(rename = "Suite ID")]
    id: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Cases")]
    cases: String,
}

#[derive(Tabled)]
struct TrendEntry {
    #[tabled(rename = "Timestamp")]
    timestamp: String,
    #[tabled(rename = "Metric")]
    metric: String,
    #[tabled(rename = "Value")]
    value: String,
}

#[derive(Tabled)]
struct BaselineEntry {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Model")]
    model: String,
    #[tabled(rename = "Created")]
    created: String,
}

#[derive(Tabled)]
struct GateEntry {
    #[tabled(rename = "Gate ID")]
    id: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Tier")]
    tier: String,
    #[tabled(rename = "Status")]
    status: String,
}
