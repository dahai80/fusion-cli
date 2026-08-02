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
#[command(version = "0.2.2")]
#[command(about = "Fusion-CLI — One CLI, Control All Fusion-MLX Local AI Ecosystem.", long_about = None)]
struct Cli {
    /// 强制离线模式（默认开启）
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

    // ── 模型同步 ──
    /// 模型同步（对接 Fusion-Multi-Node）
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
    /// AI Agent 模式（带工具调用）
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // 初始化日志
    utils::logger::init_logger(cli.verbose);

    // 设置输出格式
    if cli.format == "json" {
        unsafe {
            std::env::set_var("FUSION_OUTPUT_FORMAT", "json");
        }
    }

    // 执行命令
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
