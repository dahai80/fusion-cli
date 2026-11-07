---
doc_id: fusion-cli/product-page
product: fusion-cli
class: core
entry: service
doc_type: product-page
title: { zh: "fusion-cli 产品页", en: "fusion-cli product page" }
audience: engineer
status: draft
version: 0.4.6
updated: 2026-11-07
owner: @dahai
source_of_truth:
  - README.md
  - CLAUDE.md
  - Cargo.toml
  - src/main.rs
---

# fusion-cli

```yaml
slug: fusion-cli
name: fusion-cli
tagline:
  zh: 一个 Rust 单二进制 CLI，统一管理 fusion-mlx 本地 AI 生态：模型、推理、知识库、评测与全部服务
  en: One Rust binary that operates the whole fusion-mlx local AI ecosystem: models, inference, knowledge base, benchmarks, and every service
cta:
  zh: 不再为每个 fusion 服务记一套命令，一条 fusion 命令全部覆盖
  en: Stop memorizing one command set per fusion service; a single fusion command covers them all
repo: https://github.com/dahai80/fusion-cli
```

## 定位

```yaml
product:
  type: core-cli
  serves: 在 macOS Apple Silicon 上使用 fusion 生态的工程师与运维脚本
  builds_on:
    - fusion-mlx（推理引擎，CLI 经 gateway :11432 转发，直连 :11434 属调试行为）
    - fusion-model-hub（:11444 模型目录与拉取）
    - fusion-rag（:11436 检索增强）、fusion-bench（:11467 评测服务）
    - fusion-memory（:11435 记忆中枢）、fusion-multi-node（:11452 多节点）
    - fusion-supervisor（服务编排，经 UDS 转发）
  hosts: 27 个子命令、RBAC 门禁、审计 hash chain、Prometheus 格式 metrics 导出、配置加密
  integration_surface: 终端命令行 + shell 补全 + `--format json` 机读输出；CLI 本身不监听端口
  deployment: 单二进制（cargo build --release），默认 local-first 离线模式
```

zh：fusion-cli 是 fusion 生态的统一入口。它不做推理——推理在 fusion-mlx；它把模型管理、对话、知识库、评测、服务管控、审计与可观测性收敛到一个二进制里，输出 text 或 JSON。默认 `--offline` 开启，只允许访问 127.0.0.1 的本地服务。

en: fusion-cli is the unified entry point of the fusion ecosystem. It does not run inference itself — that is fusion-mlx. It folds model management, chat, knowledge base, benchmarks, service control, audit, and observability into one binary with text or JSON output. `--offline` is on by default and restricts access to local 127.0.0.1 services.

## capabilities

```yaml
capabilities:
  - icon: 🎛
    name: { zh: 统一命令面, en: Unified command surface }
    tag:
      zh: 27 个子命令覆盖 model/chat/kb/bench/service/memory 等，消费者不再为每个服务拼 curl
      en: 27 subcommands cover model/chat/kb/bench/service/memory; consumers stop hand-rolling curl calls per service
    points:
      - zh: 全局参数一次生效：--offline / --format json / --mlx-ctx / --no-gpu
        en: Global flags apply everywhere: --offline / --format json / --mlx-ctx / --no-gpu
      - zh: 约束：仅 macOS Apple Silicon（Metal/MLX native feature）；Linux 未适配
        en: Constraint: macOS Apple Silicon only (Metal/MLX native feature); Linux is unsupported
  - icon: 🔐
    name: { zh: RBAC 门禁 + 审计, en: RBAC gate and audit trail }
    tag:
      zh: reader/operator/admin 三级角色按 FUSION_API_KEY 预过滤命令，每条命令落审计
      en: reader/operator/admin roles pre-filter commands via FUSION_API_KEY; every command lands in the audit log
    points:
      - zh: 审计日志 SHA-256 hash chain 防篡改，fusion audit verify 校验完整性；flock 串行化并发写
        en: Audit log is a SHA-256 hash chain verified by fusion audit verify; flock serializes concurrent writes
      - zh: 约束：审计仅记录顶层命令标签，敏感参数不入审计
        en: Constraint: only the top-level command label is recorded; sensitive arguments are never audited
  - icon: 🗝
    name: { zh: 密钥轮换 + 落盘加密, en: Key rotation and at-rest encryption }
    tag:
      zh: config rotate-key 一条命令轮换 MLX API key，旧 key 保留 24h 宽限回滚
      en: config rotate-key swaps the MLX API key in one command; the old key stays 24h for rollback
    points:
      - zh: auth.encrypt_secrets=true 时 api_key 以 AES-256-GCM 加密落盘，master.key（0600）本地封套
        en: With auth.encrypt_secrets=true, api_key values are AES-256-GCM encrypted on disk under a local master.key (0600)
      - zh: 约束：加密仅覆盖配置文件中的 4 个 api_key 段，不覆盖其他明文段
        en: Constraint: encryption covers only the 4 api_key fields in config.toml, not other plaintext sections
  - icon: 📊
    name: { zh: 可观测性出口, en: Observability outlet }
    tag:
      zh: metrics json/export 两种格式，Prometheus node_exporter textfile collector 直接可用
      en: metrics json and export modes feed a Prometheus node_exporter textfile collector directly
    points:
      - zh: 按命令类型计数（model pull / kb ingest / bench run）并记录延迟快照
        en: Per-command-type counters (model pull / kb ingest / bench run) plus latency snapshots
      - zh: 约束：metrics 是进程级累计快照，落盘 ~/.fusion/metrics/metrics.json，非时序库
        en: Constraint: metrics are process-level cumulative snapshots written to ~/.fusion/metrics/metrics.json, not a time-series store
  - icon: 🖥
    name: { zh: TUI 仪表盘, en: TUI dashboard }
    tag:
      zh: dashboard 一条命令看实时服务状态 + 系统监控，不用轮流敲 status
      en: dashboard shows live service status and system monitoring in one screen instead of repeated status calls
    points:
      - zh: ratatui 实现的交互式面板，覆盖生态服务健康与资源占用
        en: An interactive ratatui panel covering ecosystem service health and resource usage
      - zh: 约束：需要终端 TTY，管道/CI 场景请改用 service status --format json
        en: Constraint: requires a TTY; in pipes or CI use service status --format json instead
```

zh：消费方接入后不再需要编写的东西：各服务的 curl 封装脚本、服务健康轮询、审计日志采集、Prometheus 指标转换。约束：CLI 的能力边界就是其下游服务的边界——fusion-mlx 未启动时推理类命令全部失败。

en: What consumers no longer write: per-service curl wrappers, service-health polling loops, audit-log collection, Prometheus metric translation. Constraint: the CLI can never exceed its downstream services — every inference command fails while fusion-mlx is down.
