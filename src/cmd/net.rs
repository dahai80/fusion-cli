use anyhow::Result;
use clap::Subcommand;
use colored::*;

use crate::service::sv as sv_svc;
use crate::utils::output::is_json_mode;

// exit code 约定匹配 fusion-sv CLI: daemon-down → 3, rpc-error → 1, ok → 0。
const EXIT_DAEMON_DOWN: i32 = 3;
const EXIT_RPC_ERROR: i32 = 1;

#[derive(Subcommand)]
pub enum NetCommands {
    /// 启动所有受管服务
    Up,
    /// 停止所有受管服务
    Down,
    /// 查看所有服务状态
    Status,
    /// 重启指定服务
    Restart {
        /// 服务名 (如 fusion-mlx)
        service: String,
    },
    /// 探活 supervisor daemon
    Ping,
}

pub async fn handle_net(action: NetCommands) -> Result<()> {
    match action {
        NetCommands::Up => net_up().await,
        NetCommands::Down => net_down().await,
        NetCommands::Status => net_status().await,
        NetCommands::Restart { service } => net_restart(&service).await,
        NetCommands::Ping => net_ping().await,
    }
}

// 统一 daemon-down 处理: 打印提示 + exit 3。
fn handle_sv_error(e: sv_svc::SvError) -> Result<()> {
    let daemon_down = sv_svc::is_daemon_down(&e);
    let code = if daemon_down {
        EXIT_DAEMON_DOWN
    } else {
        EXIT_RPC_ERROR
    };
    if is_json_mode() {
        let payload = serde_json::json!({
            "error": e.to_string(),
            "daemon_down": daemon_down,
            "code": code,
            "socket": "/tmp/fusion-sv.sock",
            "hint": "override with FUSION_SV_SOCKET; start with: fusion-sv daemon",
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("  {} {}", "❌".red(), e);
        if daemon_down {
            println!("     Socket: /tmp/fusion-sv.sock (override with FUSION_SV_SOCKET)");
            println!("     启动: fusion-sv daemon");
        }
    }
    std::process::exit(code);
}

async fn net_ping() -> Result<()> {
    if !is_json_mode() {
        println!();
        println!("{}", "🏓 Fusion-Supervisor Ping".bold());
        println!();
    }
    match sv_svc::ping() {
        Ok(alive) => {
            if is_json_mode() {
                let json = serde_json::json!({ "alive": alive });
                println!("{}", serde_json::to_string_pretty(&json)?);
                return Ok(());
            }
            let mark = if alive { "✅ alive" } else { "⬜ no pong" };
            println!("  {}", mark.green());
            println!();
        }
        Err(e) => return handle_sv_error(e),
    }
    Ok(())
}

async fn net_status() -> Result<()> {
    if !is_json_mode() {
        println!();
        println!("{}", "📊 Fusion-Supervisor Status".bold());
        println!();
    }
    match sv_svc::status() {
        Ok(entries) => {
            if is_json_mode() {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }
            print_status_table(&entries);
        }
        Err(e) => return handle_sv_error(e),
    }
    Ok(())
}

fn print_status_table(entries: &serde_json::Value) {
    let arr = match entries.as_array() {
        Some(a) => a,
        None => {
            println!("  (无服务登记)");
            return;
        }
    };
    if arr.is_empty() {
        println!("  (无服务登记)");
        return;
    }
    println!(
        "  {:<28} {:<14} {}",
        "SERVICE".dimmed(),
        "STATE".dimmed(),
        "PORT".dimmed()
    );
    for e in arr {
        let name = e.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let state = e.get("state").and_then(|v| v.as_str()).unwrap_or("?");
        let port = e.get("port").and_then(|v| v.as_i64()).unwrap_or(0);
        let colored_state = match state {
            "Healthy" => state.green().to_string(),
            "Starting" | "Stopping" => state.yellow().to_string(),
            _ => state.red().to_string(),
        };
        println!("  {:<28} {:<14} {}", name, colored_state, port);
    }
    println!();
}

async fn net_up() -> Result<()> {
    println!();
    println!("{}", "⬆️  Fusion-Supervisor Up".bold());
    println!();
    match sv_svc::up() {
        Ok(res) => {
            if is_json_mode() {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                println!("  {} all services up", "✅".green());
            }
            println!();
        }
        Err(e) => return handle_sv_error(e),
    }
    Ok(())
}

async fn net_down() -> Result<()> {
    println!();
    println!("{}", "⬇️  Fusion-Supervisor Down".bold());
    println!();
    match sv_svc::down() {
        Ok(res) => {
            if is_json_mode() {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                println!("  {} all services down", "✅".green());
            }
            println!();
        }
        Err(e) => return handle_sv_error(e),
    }
    Ok(())
}

async fn net_restart(service: &str) -> Result<()> {
    println!();
    println!("{} {}", "🔄 Fusion-Supervisor Restart".bold(), service);
    println!();
    match sv_svc::restart(service) {
        Ok(res) => {
            if is_json_mode() {
                println!("{}", serde_json::to_string_pretty(&res)?);
            } else {
                println!("  {} {} restarted", "✅".green(), service);
            }
            println!();
        }
        Err(e) => return handle_sv_error(e),
    }
    Ok(())
}
