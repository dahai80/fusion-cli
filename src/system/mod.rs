// system/mod.rs — 进程、系统信息、定时任务

use colored::*;
use std::ffi::OsStr;
use sysinfo::{ProcessesToUpdate, System};

/// 获取系统资源信息
pub fn get_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    SystemInfo {
        total_memory: sys.total_memory(),
        used_memory: sys.used_memory(),
        available_memory: sys.available_memory(),
        cpu_count: sys.cpus().len(),
        hostname: System::host_name().unwrap_or_default(),
        kernel: System::kernel_version().unwrap_or_default(),
    }
}

/// 系统信息结构
pub struct SystemInfo {
    pub total_memory: u64,
    pub used_memory: u64,
    pub available_memory: u64,
    pub cpu_count: usize,
    pub hostname: String,
    pub kernel: String,
}

/// 检测进程是否在运行
pub fn is_process_running(name: &str) -> bool {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, false);
    sys.processes_by_exact_name(OsStr::new(name)).next().is_some()
}

/// 获取进程 PID
pub fn get_process_pid(name: &str) -> Option<u32> {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, false);
    sys.processes_by_exact_name(OsStr::new(name)).next().map(|p| p.pid().as_u32())
}

/// 渲染系统信息
pub fn print_system_info() {
    let info = get_system_info();
    println!("  CPU:      {} cores", info.cpu_count.to_string().cyan());
    println!("  Memory:   {} total / {} available",
        format_bytes(info.total_memory).cyan(),
        format_bytes(info.available_memory).cyan());
    println!("  Hostname: {}", info.hostname.cyan());
    println!("  Kernel:   {}", info.kernel.cyan());
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}