// system/mod.rs — 进程、系统信息、定时任务

use std::ffi::OsStr;
use sysinfo::{ProcessesToUpdate, System};

/// 获取系统资源信息
pub fn get_system_info() -> (u64, u64, u64, usize) {
    let sys = System::new_all();
    (sys.total_memory(), sys.used_memory(), sys.available_memory(), sys.cpus().len())
}

/// 检测进程是否在运行
pub fn is_process_running(name: &str) -> bool {
    let mut sys = System::new_all();
    sys.refresh_processes(ProcessesToUpdate::All, false);
    sys.processes_by_exact_name(OsStr::new(name)).next().is_some()
}