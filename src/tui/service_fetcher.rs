use crate::service::health::{self, ServiceStatus};
use crate::service::mlx;
use sysinfo::{Components, System};

pub struct SystemInfo {
    pub cpu_usage: f32,
    pub mem_total: u64,
    pub mem_used: u64,
    pub cpu_temp: Option<f32>,
}

pub struct DashboardData {
    pub services: Vec<ServiceStatus>,
    pub models: Vec<String>,
    pub system: SystemInfo,
    pub logs: Vec<String>,
}

impl DashboardData {
    pub fn empty() -> Self {
        Self {
            services: Vec::new(),
            models: Vec::new(),
            system: SystemInfo {
                cpu_usage: 0.0,
                mem_total: 0,
                mem_used: 0,
                cpu_temp: None,
            },
            logs: Vec::new(),
        }
    }
}

pub async fn fetch_all() -> DashboardData {
    let services = health::check_all_with_latency().await.unwrap_or_default();

    let models = fetch_models().await;

    let system = fetch_system_info();

    let logs = fetch_recent_logs(20);

    DashboardData {
        services,
        models,
        system,
        logs,
    }
}

async fn fetch_models() -> Vec<String> {
    match mlx::list_models().await {
        Ok(models) => models.into_iter().map(|m| m.id).collect(),
        Err(_) => Vec::new(),
    }
}

fn fetch_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_usage();
    let mem_total = sys.total_memory();
    let mem_used = sys.used_memory();

    let cpu_temp = {
        let components = Components::new_with_refreshed_list();
        components
            .iter()
            .find(|c| c.label().to_lowercase().contains("cpu"))
            .and_then(|c| c.temperature())
    };

    SystemInfo {
        cpu_usage,
        mem_total,
        mem_used,
        cpu_temp,
    }
}

fn fetch_recent_lines(content: &str, max: usize) -> Vec<String> {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();
    let start = total.saturating_sub(max);
    lines[start..].iter().map(|s| s.to_string()).collect()
}

fn fetch_recent_logs(max: usize) -> Vec<String> {
    let log_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion")
        .join("logs");

    if !log_dir.exists() {
        return vec!["No logs directory found".to_string()];
    }

    let mut all_logs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&log_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().map(|e| e == "log").unwrap_or(false)
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                all_logs.push(format!("── {} ──", name));
                all_logs.extend(fetch_recent_lines(&content, max / 2));
            }
        }
    }

    if all_logs.is_empty() {
        vec!["No log files found".to_string()]
    } else {
        let total = all_logs.len();
        let start = total.saturating_sub(max);
        all_logs[start..].to_vec()
    }
}
