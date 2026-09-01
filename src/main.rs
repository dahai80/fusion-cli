// Fusion-CLI — One CLI, Control All Fusion-MLX Local AI Ecosystem.
// V0.1 MVP — Pure Rust, macOS Apple Silicon native, single binary.

mod agent;
mod cmd;
mod config;
mod service;
mod tools;
mod tui;
mod utils;

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fusion")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Fusion-CLI — One CLI, Control All Fusion-MLX Local AI Ecosystem.", long_about = None)]
struct Cli {
    /// 离线模式：禁止任何外部网络（huggingface 等），仅限本地 127.0.0.1 服务。
    /// 默认开启（local-first）。设 --offline=false 才允许外部网络拉取。
    #[arg(global = true, long, default_value_t = true)]
    offline: bool,

    /// 详细日志输出
    #[arg(global = true, short, long, default_value_t = false)]
    verbose: bool,

    /// 覆盖 MLX 上下文长度
    #[arg(global = true, long)]
    mlx_ctx: Option<u32>,

    /// 强制开启/关闭 KV Cache
    #[arg(global = true, long)]
    mlx_cache: Option<bool>,

    /// 仅使用 CPU（调试用）
    #[arg(global = true, long)]
    no_gpu: bool,

    /// 输出格式: text 或 json
    #[arg(global = true, long, default_value = "text", value_name = "FORMAT")]
    format: String,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    // ── 全局基础 ──
    /// 查看版本信息
    Version,
    /// 环境自检
    Doctor,
    /// 初始化 Fusion 环境
    Init,
    /// 生成 Shell 补全脚本
    Completions {
        /// Shell 类型: bash / zsh / fish / elvish / powershell
        shell: String,
    },
    /// 全局配置管理
    Config {
        #[command(subcommand)]
        action: config::ConfigCommands,
    },
    /// 审计轨迹查看 (合规只读)
    Audit {
        #[command(subcommand)]
        action: cmd::audit::AuditCommands,
    },
    /// 可观测性 metrics 快照
    Metrics {
        #[command(subcommand)]
        action: cmd::metrics::MetricsCommands,
    },
    /// 日志管理
    Log {
        #[command(subcommand)]
        action: cmd::log::LogCommands,
    },

    // ── 模型管理 ──
    /// 模型管理（对接 Fusion-Model-Hub）
    Model {
        #[command(subcommand)]
        action: cmd::model::ModelCommands,
    },

    // ── 推理交互 ──
    /// 终端交互式对话
    Chat {
        #[command(flatten)]
        args: cmd::chat::ChatArgs,
    },
    /// 单次 prompt 推理
    Run {
        #[command(flatten)]
        args: cmd::chat::RunArgs,
    },
    /// 代码专属推理
    Code {
        #[command(flatten)]
        args: cmd::chat::CodeArgs,
    },
    /// Embedding 生成
    Embed {
        #[command(flatten)]
        args: cmd::chat::EmbedArgs,
    },

    // ── 知识库 ──
    /// 知识库管理（对接 Fusion-KB）
    Kb {
        #[command(subcommand)]
        action: cmd::kb::KbCommands,
    },

    // ── 性能评测 ──
    /// 性能评测（对接 Fusion-Bench）
    Bench {
        #[command(subcommand)]
        action: cmd::bench::BenchCommands,
    },

    // ── 生态服务 ──
    /// 生态服务管控
    Service {
        #[command(subcommand)]
        action: cmd::service::ServiceCommands,
    },

    // ── RAG 服务 ──
    /// RAG 检索增强服务（对接 Fusion-RAG）
    Rag {
        #[command(subcommand)]
        action: cmd::rag::RagCommands,
    },

    // ── 桌面自动化 ──
    /// 桌面自动化（对接 Fusion-Desk）
    Desk {
        #[command(subcommand)]
        action: cmd::desk::DeskCommands,
    },

    // ── 文档服务 ──
    /// 文档服务（对接 Fusion-Doc）
    Doc {
        #[command(subcommand)]
        action: cmd::doc::DocCommands,
    },

    // ── Guard 安全守护 ──
    /// Guard 安全守护查询（经 UDS，只读 status/rules/audit）
    Guard {
        #[command(subcommand)]
        action: cmd::guard::GuardCommands,
    },

    // ── Supervisor 服务编排 ──
    /// 服务编排（转发至 fusion-supervisor UDS: up/down/status/restart/ping）
    Net {
        #[command(subcommand)]
        action: cmd::net::NetCommands,
    },

    // ── 记忆中心 ──
    /// 记忆管理（对接 fusion-memory fm-server: status/search/count/commit/delete/audit）
    Memory {
        #[command(subcommand)]
        action: cmd::memory::MemoryCommands,
    },

    // ── 评测服务 ──
    /// 评测服务管理（对接 fusion-bench HTTP: status/tasks/suites/results/baselines/gates）
    Eval {
        #[command(subcommand)]
        action: cmd::benchsvc::EvalCommands,
    },

    // ── 模型同步 ──
    /// 模型同步（对接 fusion-multi-node Master）
    Sync {
        #[command(subcommand)]
        action: cmd::sync::SyncCommands,
    },

    // ── 集群管理 ──
    /// 集群管理
    Cluster {
        #[command(subcommand)]
        action: cmd::cluster::ClusterCommands,
    },

    // ── AI Agent ──
    /// AI 只读助手 (带只读工具调用: list_models/model_info/health/bench_speed)
    Agent {
        /// 输入提示
        prompt: String,
        /// 模型名称
        #[arg(short, long)]
        model: Option<String>,
        /// 权限级别: sandbox / ask / auto
        #[arg(short, long, default_value = "ask")]
        permission: String,
    },

    // ── TUI Dashboard ──
    /// TUI 交互式仪表盘（实时服务状态 + 系统监控）
    Dashboard,
}

// R6 修复: set_var 在多线程 tokio 运行时启动后执行是未定义行为 (glibc setenv 可能 free
// 他人正在读的缓冲)。Rust 2024 将 set_var 标 unsafe 正因如此。改为在运行时建立前,
// 即同步 main 入口处解析并设置 env, 之后所有 worker 线程读到的是稳定值。
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 必须在 tokio 运行时启动前完成 env 写入, 杜绝与 worker 线程的并发读。
    if cli.format == "json" {
        unsafe {
            std::env::set_var("FUSION_OUTPUT_FORMAT", "json");
        }
    }
    unsafe {
        std::env::set_var("FUSION_OFFLINE", if cli.offline { "1" } else { "0" });
    }

    // 初始化日志 (同步, 安全)。
    utils::logger::init_logger(cli.verbose);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async_main(cli))
}

async fn async_main(cli: Cli) -> anyhow::Result<()> {
    // 审计轨迹: 记录命令标签 + 耗时 + 结果。match 消费 cli.command, 故先提取标签。
    let audit_cmd = audit_label(&cli.command);
    let audit_start = std::time::Instant::now();

    // 执行命令
    let result = dispatch(cli).await;

    let (outcome, detail) = match &result {
        Ok(_) => ("ok".to_string(), String::new()),
        Err(e) => ("error".to_string(), e.to_string()),
    };
    let elapsed_ms = audit_start.elapsed().as_millis() as u64;
    crate::utils::audit::record(&audit_cmd, &outcome, elapsed_ms, &detail);

    // 可观测性: 按命令类型增量计数 + 记录延迟。
    crate::utils::metrics::inc_request();
    if outcome == "error" {
        crate::utils::metrics::inc_request_error();
    }
    match audit_cmd.as_str() {
        "model" => crate::utils::metrics::inc_model_pull(),
        "kb" => crate::utils::metrics::inc_kb_ingest(),
        "bench" => crate::utils::metrics::inc_bench_run(),
        "service" | "rag" | "doc" => crate::utils::metrics::inc_service_op(),
        _ => {}
    }
    crate::utils::metrics::observe_latency_ms(elapsed_ms);
    // 落盘快照 (旁路, 失败仅记 tracing 不阻断)。
    if let Err(e) = crate::utils::metrics::flush() {
        tracing::error!(error = %e, "metrics flush failed");
    }

    result
}

// 提取命令的人类可读标签 (不消费 Commands, 不读子命令参数 — 避免给 15 个子命令 enum 加 Debug,
// 且敏感参数绝不进审计)。仅记录顶层命令名。
fn audit_label(cmd: &Option<Commands>) -> String {
    match cmd {
        None => "help".to_string(),
        Some(Commands::Version) => "version".to_string(),
        Some(Commands::Doctor) => "doctor".to_string(),
        Some(Commands::Init) => "init".to_string(),
        Some(Commands::Completions { .. }) => "completions".to_string(),
        Some(Commands::Config { .. }) => "config".to_string(),
        Some(Commands::Audit { .. }) => "audit".to_string(),
        Some(Commands::Metrics { .. }) => "metrics".to_string(),
        Some(Commands::Log { .. }) => "log".to_string(),
        Some(Commands::Model { .. }) => "model".to_string(),
        Some(Commands::Chat { .. }) => "chat".to_string(),
        Some(Commands::Run { .. }) => "run".to_string(),
        Some(Commands::Code { .. }) => "code".to_string(),
        Some(Commands::Embed { .. }) => "embed".to_string(),
        Some(Commands::Kb { .. }) => "kb".to_string(),
        Some(Commands::Bench { .. }) => "bench".to_string(),
        Some(Commands::Service { .. }) => "service".to_string(),
        Some(Commands::Rag { .. }) => "rag".to_string(),
        Some(Commands::Desk { .. }) => "desk".to_string(),
        Some(Commands::Doc { .. }) => "doc".to_string(),
        Some(Commands::Guard { .. }) => "guard".to_string(),
        Some(Commands::Net { .. }) => "net".to_string(),
        Some(Commands::Memory { .. }) => "memory".to_string(),
        Some(Commands::Eval { .. }) => "eval".to_string(),
        Some(Commands::Sync { .. }) => "sync".to_string(),
        Some(Commands::Cluster { .. }) => "cluster".to_string(),
        Some(Commands::Agent { .. }) => "agent".to_string(),
        Some(Commands::Dashboard) => "dashboard".to_string(),
    }
}

async fn dispatch(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        None => {
            // 无子命令时显示帮助
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
        }
        Some(Commands::Version) => {
            cmd::version::run().await?;
        }
        Some(Commands::Doctor) => {
            cmd::doctor::run().await?;
        }
        Some(Commands::Init) => {
            cmd::init::run_init().await?;
        }
        Some(Commands::Completions { shell }) => {
            cmd::completions::run_completions(&shell)?;
        }
        Some(Commands::Config { action }) => {
            config::handle_config(action).await?;
        }
        Some(Commands::Audit { action }) => {
            cmd::audit::handle_audit(action).await?;
        }
        Some(Commands::Metrics { action }) => {
            cmd::metrics::handle_metrics(action).await?;
        }
        Some(Commands::Log { action }) => {
            cmd::log::handle_log(action).await?;
        }
        Some(Commands::Model { action }) => {
            cmd::model::handle_model(action).await?;
        }
        Some(Commands::Chat { args }) => {
            cmd::chat::handle_chat(args).await?;
        }
        Some(Commands::Run { args }) => {
            cmd::chat::handle_run(args).await?;
        }
        Some(Commands::Code { args }) => {
            cmd::chat::handle_code(args).await?;
        }
        Some(Commands::Embed { args }) => {
            cmd::chat::handle_embed(args).await?;
        }
        Some(Commands::Kb { action }) => {
            cmd::kb::handle_kb(action).await?;
        }
        Some(Commands::Bench { action }) => {
            cmd::bench::handle_bench(action).await?;
        }
        Some(Commands::Service { action }) => {
            cmd::service::handle_service(action).await?;
        }
        Some(Commands::Rag { action }) => {
            cmd::rag::handle_rag(action).await?;
        }
        Some(Commands::Desk { action }) => {
            cmd::desk::handle_desk(action).await?;
        }
        Some(Commands::Doc { action }) => {
            cmd::doc::handle_doc(action).await?;
        }
        Some(Commands::Guard { action }) => {
            cmd::guard::handle_guard(action).await?;
        }
        Some(Commands::Net { action }) => {
            cmd::net::handle_net(action).await?;
        }
        Some(Commands::Memory { action }) => {
            cmd::memory::handle_memory(action).await?;
        }
        Some(Commands::Eval { action }) => {
            cmd::benchsvc::handle_eval(action).await?;
        }
        Some(Commands::Sync { action }) => {
            cmd::sync::handle_sync(action).await?;
        }
        Some(Commands::Cluster { action }) => {
            cmd::cluster::handle_cluster(action).await?;
        }
        Some(Commands::Agent {
            prompt,
            model,
            permission,
        }) => {
            let model_name = model.unwrap_or_else(|| {
                let config = config::load_config();
                config.model.default_path.clone()
            });
            agent::run_agent(&model_name, &prompt, &permission).await?;
        }
        Some(Commands::Dashboard) => {
            cmd::dashboard::run_dashboard().await?;
        }
    }

    Ok(())
}
