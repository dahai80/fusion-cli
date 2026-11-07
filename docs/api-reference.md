---
doc_id: fusion-cli/api-reference
product: fusion-cli
class: core
entry: service
doc_type: api-reference
title: { zh: "fusion-cli 命令参考", en: "fusion-cli command reference" }
audience: engineer
status: draft
version: 0.4.6
updated: 2026-11-07
owner: "@dahai"
source_of_truth:
  - src/main.rs
  - src/cmd/mod.rs
  - src/config/mod.rs
---

# 命令参考 / Command Reference

zh：fusion-cli 本身**不提供 HTTP API**——它不监听任何端口。本文件记录的唯一接口面就是 `fusion` 命令行（clap 定义于 `src/main.rs`）。CLI 将请求转发给生态服务（fusion-mlx、fusion-memory 等）；那些服务的 HTTP API 见各自仓库的 api-reference。所有示例经 `fusion --help`（v0.4.6 源码定义）核对。

en: fusion-cli itself exposes **no HTTP API** — it listens on no port. The only interface this file documents is the `fusion` command line (clap definitions in `src/main.rs`). The CLI forwards requests to ecosystem services (fusion-mlx, fusion-memory, ...); those services' HTTP APIs live in their own repos. All entries verified against `fusion --help` (v0.4.6 source definitions).

## 全局参数 / Global options

| Flag | Default | 说明 / Meaning |
|---|---|---|
| `--offline` | true | 禁止外网访问，仅 127.0.0.1；`--offline=false` 解除 / Block external network; local only |
| `-v, --verbose` | false | 详细日志 / Verbose logging |
| `--mlx-ctx <N>` | config | 覆盖 MLX 上下文长度 / Override MLX context length |
| `--mlx-cache <bool>` | config | 强制开关 KV Cache / Force KV cache on or off |
| `--no-gpu` | false | 仅 CPU（调试）/ CPU only (debug) |
| `--format <FORMAT>` | text | `text` 或 `json` 机读输出 / text or machine-readable json |

zh：全局参数均为 global，可放在任意子命令后。`--format json` 经 `FUSION_OUTPUT_FORMAT` 环境变量在运行时启动前生效。

en: All global flags work on any subcommand. `--format json` is applied before runtime startup via the `FUSION_OUTPUT_FORMAT` environment variable.

## 基础命令 / Base commands

| Command | 子命令 / Subcommands | 用途 / Purpose |
|---|---|---|
| `fusion version` | — | 版本信息 / Version info |
| `fusion doctor` | opts: `--root <PATH>`, `--prod` | 环境自检；`--prod` 缺必需 key 时非零退出 / Env self-check; `--prod` fails on missing required keys |
| `fusion init` | — | 初始化 Fusion 环境 / Scaffold the fusion environment |
| `fusion completions <SHELL>` | bash/zsh/fish/elvish/powershell | 生成补全脚本 / Emit completion script |
| `fusion dashboard` | — | TUI 实时仪表盘 / TUI live dashboard |

## 配置 / `fusion config`

| Subcommand | 用途 / Purpose |
|---|---|
| `config list` / `config get <KEY>` / `config set <KEY> <VALUE>` / `config reset` | 读写 `~/.fusion/config.toml` / Read-write the config file |
| `config roles-list` / `roles-add <role> ...` / `roles-remove` / `roles-enable on|off` | RBAC 角色密钥管理（reader/operator/admin）/ Manage RBAC role keys |
| `config rotate-key` | 轮换 MLX API key；旧 key 存 `key_previous` 供 24h 回滚 / Rotate the MLX API key; previous key kept 24h for rollback |

## 审计 / 可观测性 / Audit and observability

| Command | 子命令 | 用途 |
|---|---|---|
| `fusion audit` | `view` / `path` / `verify` | 查看审计记录、路径；`verify` 校验 hash chain 防篡改 / Verify the audit hash chain |
| `fusion metrics` | `view` / `path` / `json` / `export` | 快照；`export` 输出 Prometheus 0.0.4 exposition 格式 / Prometheus exposition format |
| `fusion log` | `view` / `clear` | 查看/清空全生态日志 / View or clear ecosystem logs |

## 模型管理 / `fusion model`（对接 fusion-model-hub）

| Subcommand | 用途 |
|---|---|
| `model list` / `info <NAME>` / `delete` / `clean` | 本地模型生命周期 / Local model lifecycle |
| `model pull <NAME>` | 拉取官方/社区 MLX 模型（离线模式下拒绝外网）/ Pull MLX models |
| `model convert` / `model quant` | 第三方模型转 MLX；重量化 / Convert to MLX; requantize |
| `model submit` | 提交模型任务 / Submit a model task |

## 推理 / Inference

| Command | 关键参数 / Key options | 用途 |
|---|---|---|
| `fusion chat -m <MODEL>` | `--stream`/`--no-stream`、`--max-tokens`、`--temperature`、`--top-p`、`--timeout`（默认 120s） | 交互式对话 / Interactive chat |
| `fusion run -m <MODEL> -p <PROMPT>` | 同 chat / same as chat | 单次推理 / One-shot inference |
| `fusion code -m <MODEL>` | `-p <PROMPT>`、`-f <FILE>`、`--task <TASK>`（默认 chat） | 代码推理 / Code-focused inference |
| `fusion embed` | `-t <TEXT>`、`-d <DIR>`、`-o <OUTPUT>`、`-m`（默认 BGE-M3） | 生成 embedding / Generate embeddings |

zh：`--ctx` 是 `--max-tokens` 的旧别名（生成上限，非上下文窗口）。`--no-cache`、`--no-metal` 供调试。

en: `--ctx` is a legacy alias of `--max-tokens` (generation cap, not context window). `--no-cache` and `--no-metal` are debug switches.

## 知识库 / `fusion kb`（对接 fusion-rag 系知识库）

| Subcommand |
|---|
| `list` / `create` / `delete` / `ingest` / `query` / `clear` / `stat` |

## 评测 / `fusion bench`

| Subcommand | 用途 |
|---|---|
| `bench speed` | 生成速度 token/s / Generation speed |
| `bench mem` | 显存/内存占用 / Memory footprint |
| `bench ctx` | 最大上下文压力测试 / Max-context stress test |
| `bench auto` | 全自动参数寻优 / Automatic parameter search |
| `bench report` | 导出评测报告 / Export a report |

zh：`fusion eval`（对接 fusion-bench HTTP 服务）提供另一面：`status` / `resources` / `tasks` / `task` / `suites` / `result` / `trend` / `baselines` / `gates`。

en: `fusion eval` (against the fusion-bench HTTP service) is the other half: `status` / `resources` / `tasks` / `task` / `suites` / `result` / `trend` / `baselines` / `gates`.

## 生态服务 / Ecosystem services

| Command | 子命令 | 下游 |
|---|---|---|
| `fusion service` | `status` / `start` / `stop` / `restart` / `log` | 本地生态服务管控 / Local service control |
| `fusion rag` | `start` / `stop` / `status` / `search` / `list` | fusion-rag（:11436） |
| `fusion desk` | `list` / `run` / `history` / `cron` / `cron-list` / `cron-rm` / `stop` | fusion-desk 桌面自动化；cron 持久化到 `~/.fusion/cron.json` |
| `fusion doc` | `start` / `stop` / `status` / `log` | fusion-doc（:11449） |
| `fusion net` | `up` / `down` / `status` / `restart <SVC>` / `ping` | 转发 fusion-supervisor UDS / Forwards to fusion-supervisor |
| `fusion memory` | `status` / `version` / `search` / `count` / `get` / `commit` / `consolidate` / `delete` / `audit` | fusion-memory fm-server（:11435） |
| `fusion guard` | `status` / `rules` / `audit` | 经 UDS 只读查询 fusion-guard / Read-only UDS queries |

## 模块安装 / Module install

| Command | 用途 |
|---|---|
| `fusion install <ID> [--with-deps] [--root <PATH>]` | 按 catalog 克隆安装模块；`--with-deps` 执行语言引导（pip install -e / cargo build / bun install） |
| `fusion list` | 列出已安装 vs 可用模块 / Installed vs available modules |

## AI 助手 / `fusion agent`

```bash
fusion agent <PROMPT> [-m <MODEL>] [--permission sandbox|ask|auto]
```

zh：只读工具集（list_models / model_info / health / bench_speed），`--permission` 默认 ask。auto 前请理解其含义：模型自主选择工具不再询问。

en: A read-only tool set (list_models / model_info / health / bench_speed); `--permission` defaults to ask. Understand auto before using it: the model picks tools without asking.

## 错误约定 / Error conventions

zh：命令失败经 anyhow 返回非零退出码与错误消息；RBAC 拒绝输出 `🚫` 前缀并计入审计（denied）。`--format json` 时错误消息仍走 stderr。

en: Failures return a non-zero exit code with an anyhow error message; RBAC denials print a `🚫` prefix and are audited as denied. With `--format json`, errors still go to stderr.
