<div align="center">
  <h1>⚡ Fusion-CLI</h1>
  <p><strong>一个 CLI，掌控整个 Fusion-MLX 本地 AI 生态。</strong></p>
  <p><em>单一二进制文件管理模型、推理、知识库、基准测试、自动化和全部服务 — 全部基于 fusion-mlx。</em></p>

<p>
  <a href="README.md">English</a> | <strong>中文</strong>
</p>
</div>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-orange" alt="Rust">
  <img src="https://img.shields.io/badge/macOS-Apple%20Silicon-brightgreen" alt="macOS">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License">
  <img src="https://img.shields.io/badge/Backend-fusion--mlx--only-important" alt="fusion-mlx">
  <img src="https://img.shields.io/badge/version-0.2.5-blue" alt="Version">
</p>

---

## 📋 概览

**Fusion-CLI** 是 Fusion-MLX 本地 AI 生态的统一命令行入口，提供单一二进制文件管理本地 AI 的所有环节：

- **模型管理** — 列出、拉取、转换、量化、删除 MLX 模型
- **推理** — 对话（SSE 流式）、单轮提示、代码分析、嵌入向量（均通过 fusion-mlx）
- **知识库** — 创建、导入、查询、管理 Fusion-KB 知识库
- **RAG 服务** — 启停 Fusion-RAG、语义搜索、知识库列表
- **基准测试** — 真实速度测试、内存分析、上下文压力测试、自动优化
- **服务控制** — 启停重启所有 Fusion 生态服务，真实进程管理
- **桌面自动化** — 运行 Fusion-Desk 模板、查看历史、停止任务
- **AI 代理** — 自然语言 + 工具调用（sandbox/ask/auto 权限级别）
- **TUI 仪表盘** — 交互式终端仪表盘，服务监控、模型列表、系统状态
- **Shell 补全** — 生成 bash/zsh/fish/elvish/powershell 补全脚本
- **初始化** — 一键环境搭建

### 核心设计理念

| 原则 | 实现 |
|------|------|
| **唯一后端** | 🔒 100% fusion-mlx — 不依赖 omlx、ollama、OpenAI |
| **完全离线** | 无遥测、无回传、无云 API |
| **macOS 原生** | Apple Silicon 优化，单二进制，无 Python/Node 依赖 |
| **可脚本化** | 每条命令支持 `--format=json` 程序化输出 |
| **生态统一** | 一个 CLI 统管：Desk、Code、KB、Bench、Model-Hub |
| **可配置 URL** | 所有服务 URL 从 `~/.fusion/config.toml` 读取，无硬编码 |
| **连接池** | 全局 `reqwest::Client` 持久连接，跨命令复用 |

---

## 🚀 快速开始

### 安装

```bash
# 克隆
git clone https://github.com/dahai80/fusion-cli.git
cd fusion-cli

# 构建（单二进制）
cargo build --release

# 安装到 PATH
cp target/release/fusion /usr/local/bin/

# 初始化环境
fusion init

# 验证
fusion --version
fusion doctor
```

### 基本用法

```bash
# 环境检查
fusion doctor

# 列出可用模型
fusion model list

# 拉取模型
fusion model pull llama3-8b-instruct-4bit

# 开始对话（默认 SSE 流式）
fusion chat --model=llama3-8b-instruct-4bit

# 基准测试（真实推理）
fusion bench speed --model=llama3-8b-instruct-4bit --runs=3

# AI 代理
fusion agent "列出模型并基准测试最快的那个" --model=llama3-8b-instruct-4bit

# 启动 TUI 仪表盘
fusion dashboard

# 监控服务状态（每 5 秒自动刷新）
fusion service status --watch=5

# JSON 输出
fusion model list --format=json

# Shell 补全
fusion completions zsh
```

---

## 📖 命令参考

### 全局命令

| 命令 | 说明 |
|------|------|
| `fusion version` | 显示所有生态组件版本信息 |
| `fusion doctor` | 完整环境诊断（系统、MLX、KB、ModelHub、RAG、Desk） |
| `fusion init` | 初始化 Fusion 环境（目录、配置、健康检查） |
| `fusion completions <shell>` | 生成 Shell 补全脚本（bash/zsh/fish/elvish/powershell） |
| `fusion dashboard` | 启动交互式 TUI 仪表盘 |
| `fusion config list` | 查看所有配置 |
| `fusion config set <key> <value>` | 设置配置值 |
| `fusion log` | 查看实时日志 |
| `fusion log clear` | 清除所有日志 |

**全局参数**：`--offline`、`--verbose`、`--mlx-ctx`、`--mlx-cache`、`--no-gpu`、`--format=json`

### TUI 仪表盘（`fusion dashboard`）

交互式终端仪表盘，实时监控：

- **服务标签** — 服务状态、延迟、端口，仪表盘内启停
- **模型标签** — 已加载 MLX 模型列表
- **系统标签** — CPU、内存、温度、架构
- **日志标签** — 最近日志条目

| 按键 | 动作 |
|------|------|
| `1-4` | 切换标签 |
| `↑/↓` 或 `j/k` | 列表导航 |
| `r` | 强制刷新 |
| `s` | 启动选中服务 |
| `x` | 停止选中服务 |
| `q` / `Esc` | 退出仪表盘 |

### 模型管理（`fusion model`）

| 命令 | 说明 |
|------|------|
| `list` | 列出所有本地 MLX 模型 |
| `pull <name> [--mirror=URL]` | 下载模型（ModelHub API 或 huggingface-cli 回退） |
| `info <name>` | 显示模型详情（大小、量化、参数量） |
| `delete <name>` | 删除模型 |
| `clean` | 清理模型缓存 |
| `convert <source> --quant=4/8/fp16` | 通过 mlx_lm 转换第三方模型为 MLX 格式 |
| `quant <name> --target=4bit` | 通过 mlx_lm 重新量化已有 MLX 模型 |

### 推理（`fusion chat / run / code / embed`）

| 命令 | 说明 |
|------|------|
| `chat --model=<name>` | 交互式终端对话（SSE 流式） |
| `run --model=<name> --prompt="..."` | 单轮推理 |
| `code --model=<name> --file=path --task=explain` | 代码分析 |
| `embed --text="..." / --dir=path` | 为 Fusion-KB 生成嵌入向量 |

**通用参数**：`--ctx`、`--temperature`、`--top-p`、`--no-cache`、`--quiet`、`--timeout`、`--no-stream`

### AI 代理（`fusion agent`）

| 命令 | 说明 |
|------|------|
| `agent <prompt>` | 自然语言 + 自动工具调用 |
| `agent <prompt> --model=<name>` | 指定模型 |
| `agent <prompt> --permission=sandbox` | 仅只读工具 |
| `agent <prompt> --permission=ask` | 危险操作前询问（默认） |
| `agent <prompt> --permission=auto` | 完全自动 |

**可用工具**：`list_models`、`model_info`、`health`、`bench_speed`

### 知识库（`fusion kb`）

| 命令 | 说明 |
|------|------|
| `list` | 列出所有知识库 |
| `create <name>` | 创建知识库 |
| `ingest <name> --path=<dir>` | 导入文档到知识库 |
| `query <name> --question="..."` | 语义搜索和问答 |
| `stat <name>` | 查看知识库统计 |
| `clear <name>` | 清空所有文档 |
| `delete <name>` | 删除知识库 |

### RAG 服务（`fusion rag`）

| 命令 | 说明 |
|------|------|
| `start [--port=11436]` | 启动 Fusion-RAG 服务 |
| `stop` | 停止 Fusion-RAG 服务 |
| `status` | 查看服务状态 + 嵌入可用性 |
| `search <kb_id> --query="..."` | RAG 语义搜索 |
| `list` | 列出所有 RAG 知识库 |

### 基准测试（`fusion bench`）

| 命令 | 说明 |
|------|------|
| `speed --model=<name> [--tokens=128] [--runs=1]` | 真实 token 生成速度测试 |
| `mem --model=<name>` | 内存 + MLX 服务器统计 |
| `ctx --model=<name> --max-ctx=8192 [--step=256]` | 上下文长度压力测试（真实推理） |
| `auto --model=<name>` | 自动参数优化（真实基准测试） |
| `report --model=<name> --output=report.md` | 导出真实数据基准报告 |

### 服务控制（`fusion service`）

| 命令 | 说明 |
|------|------|
| `status [--watch=N]` | 查看所有服务状态（每 N 秒自动刷新） |
| `start [mlx/kb/modelhub/desk/rag/doc/all]` | 启动一个或所有服务 |
| `stop [mlx/kb/modelhub/desk/rag/doc/all]` | 停止一个或所有服务 |
| `restart [mlx/kb/modelhub/desk/rag/doc/all]` | 重启一个或所有服务 |
| `log [name] --lines=50` | 查看服务日志 |

### 桌面自动化（`fusion desk`）

| 命令 | 说明 |
|------|------|
| `list` | 列出自动化模板（API 或回退） |
| `run <name>` | 执行自动化模板 |
| `history [--limit=20]` | 查看任务执行历史 |
| `cron <name> --rule="0 21 * * *"` | 定时任务 |
| `stop --task-id=<id>` | 停止运行中任务 |

### 文档服务（`fusion doc`）

| 命令 | 说明 |
|------|------|
| `start [--port=11449]` | 启动 Fusion-Doc 服务 |
| `stop` | 停止 Fusion-Doc 服务 |
| `status` | 查看 Fusion-Doc 服务状态 |
| `log [--lines=50]` | 查看 Fusion-Doc 日志 |

---

## 🔧 架构

```
src/
├── main.rs              # 入口，CLI 框架（clap）
├── cmd/                 # 命令实现
│   ├── mod.rs
│   ├── version.rs       # fusion version
│   ├── doctor.rs        # fusion doctor
│   ├── init.rs          # fusion init (V0.2)
│   ├── completions.rs   # fusion completions (V0.2)
│   ├── dashboard.rs     # fusion dashboard (V0.2.1)
│   ├── log.rs           # fusion log
│   ├── model.rs         # fusion model（真实 pull/convert/quant）
│   ├── chat.rs          # fusion chat/run/code/embed（SSE 流式）
│   ├── kb.rs            # fusion kb
│   ├── bench.rs         # fusion bench（真实基准测试）
│   ├── service.rs       # fusion service（watch 模式 V0.2.1）
│   ├── rag.rs           # fusion rag
│   ├── doc.rs           # fusion doc（start/stop/status/log）
│   ├── desk.rs          # fusion desk（真实 API 调用）
│   ├── sync.rs          # fusion sync（模型同步）
│   └── cluster.rs       # fusion cluster
├── service/             # 统一服务层
│   ├── mod.rs           # 全局 reqwest::Client + ServiceUrls + check_url()
│   ├── mlx.rs           # MLX 推理客户端（chat, stream, embed, bench, health）
│   ├── kb.rs            # Fusion-KB 客户端（list, query, health）
│   ├── modelhub.rs      # Model-Hub 客户端（list, search, download, health）
│   ├── rag.rs           # Fusion-RAG 客户端（search, health, list KBs）
│   ├── desk.rs          # Fusion-Desk 客户端（templates, tasks, history, stop, health）
│   ├── doc.rs           # Fusion-Doc 客户端（health check, status detail）
│   ├── gateway.rs       # Gateway 客户端（服务发现）（V0.2.1）
│   └── health.rs        # 统一健康检查（check_all, check_all_with_latency）
├── tui/                 # TUI 仪表盘（V0.2.1）
│   ├── mod.rs           # 事件循环 + 终端设置
│   ├── app.rs           # App 状态机（标签页、选择、数据）
│   ├── ui.rs            # ratatui 渲染（服务、模型、系统、日志）
│   └── service_fetcher.rs # 后台数据获取（health, models, system info）
├── agent/               # AI 代理引擎（V0.2）
│   ├── mod.rs           # 代理循环 + 工具调用
│   ├── context.rs       # 上下文管理器（消息历史、裁剪）
│   ├── loop_engine.rs   # 循环统计追踪
│   └── permission.rs    # 权限级别（sandbox/ask/auto）
├── tools/               # 工具注册（V0.2）
│   └── mod.rs           # ToolExecutor + 内置工具
├── config/              # 全局配置管理
│   └── mod.rs           # FusionConfig: TOML 配置于 ~/.fusion/config.toml
└── utils/               # 日志、工具
    ├── mod.rs
    ├── logger.rs
    └── output.rs        # JSON 输出支持（V0.2）
```

### 服务 URL 配置

fusion-mlx 推理通过网关（`http://localhost:11432`，OpenAI 兼容 `/v1/*` 端点，需 `mlx.api_key`）。其他生态服务默认连接各自直连本地端口；网关当前仅代理 mlx 推理 API。

| 服务 | 默认 URL | 配置键 |
|------|----------|--------|
| fusion-mlx | `http://localhost:11432` | `mlx.base_url` |
| fusion-mlx API key | `fg-admin-key` | `mlx.api_key` |
| Fusion-KB | `http://localhost:11434` | `kb.base_url` |
| Model-Hub | `http://localhost:11444` | `modelhub.base_url` |
| Fusion-RAG | `http://localhost:11436` | `rag.base_url` |
| Fusion-Desk | `http://localhost:9000` | `desk.base_url` |
| Fusion-Doc | `http://localhost:11449` | `doc.base_url` |
| Gateway | `http://localhost:11432` | `gateway.base_url` |

---

## 🛣️ 路线图

### V0.1 (MVP) ✅
- [x] 全局命令：version、doctor、config、log
- [x] 模型管理：list、pull、info、delete、clean、convert、quant
- [x] 推理：chat、run、code、embed（均通过 fusion-mlx）
- [x] 知识库：list、create、ingest、query、stat、clear、delete
- [x] 基准测试：speed、mem、ctx、auto、report
- [x] 服务控制：status、start、stop、restart、log
- [x] 桌面自动化：list、run、history、cron、stop
- [x] Rust 单二进制，无运行时依赖

### V0.2 ✅
- [x] 统一服务层（`src/service/`）替代废弃的 `ecosystem/`、`mlx_bind/`、`system/`
- [x] 全局 `reqwest::Client` 连接池
- [x] 可配置服务 URL（从 `config.toml` 读取）
- [x] 类型化服务 API（mlx、kb、modelhub、rag、desk、health）
- [x] SSE 流式支持（`chat_completion_stream`）
- [x] 真实 MLX 启停（通过 `~/claude-home/fusion-mlx/start.sh`）
- [x] RAG 服务集成（start/stop/status/search/list）
- [x] 真实基准测试（通过 MLX API `generate_tokens`）
- [x] 真实模型拉取（ModelHub API + huggingface-cli 回退）
- [x] 真实模型转换/量化（通过 `mlx_lm.convert`）
- [x] 真实 Desk 操作（API 调用、历史、停止）
- [x] 代理模式（自然语言 → 工具调用，权限级别）
- [x] 工具注册（list_models、model_info、health、bench_speed）
- [x] `--format=json` 全局输出模式
- [x] `fusion init` 一键搭建
- [x] `fusion completions` Shell 补全生成
- [x] 上下文管理器 + 消息裁剪
- [x] 循环统计追踪

### V0.2.1 ✅
- [x] TUI 仪表盘（`fusion dashboard`），基于 ratatui — 4 标签页：服务/模型/系统/日志
- [x] Gateway 集成（`src/service/gateway.rs`）— 服务发现 + 回退
- [x] 带延迟的服务健康检测（`check_all_with_latency`）
- [x] Watch 模式（`fusion service status --watch=N`）— 每 N 秒自动刷新
- [x] 仪表盘内服务启停（s/x 键）

### V0.4（未来）
- [ ] 分布式节点管理
- [ ] 一键生态部署
- [ ] 完整 CI/CD 本地 AI 流水线
- [ ] 代理自修复循环
- [ ] MCP 协议支持

---

## 📄 许可证

Apache 2.0 许可证。详见 [LICENSE](LICENSE)。

---

<p align="center">
  <strong>Fusion-CLI — 一个 CLI，掌控整个 Fusion-MLX 本地 AI 生态。</strong>
</p>
<p align="center">
  <sub>用 ❤️ 和 Rust 🦀 构建</sub>
</p>
