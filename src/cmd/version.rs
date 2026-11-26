use crate::service::mlx;
use crate::utils::output::{is_json_mode, print_json};
use anyhow::Result;
use colored::*;
use serde::Serialize;

#[derive(Serialize)]
struct VersionInfo {
    fusion_cli: String,
    install_kind: String,
    install_path: String,
    services: Vec<ServiceVersion>,
}

#[derive(Serialize)]
struct ServiceVersion {
    name: String,
    status: String,
}

pub async fn run() -> Result<()> {
    let services = check_all_services().await;

    if is_json_mode() {
        let (install_kind, install_path) = crate::utils::install::classify_current_exe();
        let info = VersionInfo {
            fusion_cli: env!("CARGO_PKG_VERSION").to_string(),
            install_kind: match install_kind {
                crate::utils::install::InstallKind::Durable => "durable".to_string(),
                crate::utils::install::InstallKind::Ephemeral => "ephemeral".to_string(),
            },
            install_path,
            services,
        };
        print_json(&info)?;
        return Ok(());
    }

    println!();
    println!(
        "{}",
        "Fusion-CLI — One CLI, Control All Fusion-MLX Local AI Ecosystem.".bold()
    );
    println!();
    println!("  {} v{}", "fusion-cli".cyan(), env!("CARGO_PKG_VERSION"));
    // Issue #40: flag ephemeral installs (app-bundle shim on a mounted DMG,
    // temp dirs) so broken shim installs are diagnosable, not silent.
    let (kind, path) = crate::utils::install::classify_current_exe();
    println!("  install: {} ({})", path.dimmed(), path_kind_label(kind));
    if kind == crate::utils::install::InstallKind::Ephemeral {
        eprintln!(
            "  {} running from an ephemeral path — this binary dies on eject/reboot.",
            "⚠️".yellow()
        );
        eprintln!(
            "    Install standalone: curl -fsSL https://raw.githubusercontent.com/dahai80/fusion-cli/main/install.sh | bash"
        );
    }
    for svc in &services {
        let mark = if svc.status == "running" {
            "✅".green()
        } else {
            "⬜".yellow()
        };
        println!("  {} {}: {}", mark, svc.name.cyan(), svc.status);
    }
    println!();
    Ok(())
}

fn path_kind_label(kind: crate::utils::install::InstallKind) -> &'static str {
    match kind {
        crate::utils::install::InstallKind::Durable => "durable",
        crate::utils::install::InstallKind::Ephemeral => "ephemeral",
    }
}

async fn check_all_services() -> Vec<ServiceVersion> {
    let names = [
        "fusion-mlx",
        "Fusion-KB",
        "Model-Hub",
        "Fusion-RAG",
        "Fusion-Desk",
        "Fusion-Doc",
    ];
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let alive = match name {
            "fusion-mlx" => mlx::health_check().await.unwrap_or(false),
            other => crate::service::health::check_named(other)
                .await
                .map(|s| s.alive)
                .unwrap_or(false),
        };
        out.push(ServiceVersion {
            name: name.to_string(),
            status: if alive {
                "running".to_string()
            } else {
                "stopped".to_string()
            },
        });
    }
    out
}
