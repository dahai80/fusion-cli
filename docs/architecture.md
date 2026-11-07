---
doc_id: fusion-cli/architecture
product: fusion-cli
class: core
entry: service
doc_type: architecture
title: { zh: "fusion-cli 架构", en: "fusion-cli architecture" }
audience: engineer
status: draft
version: 0.4.6
updated: 2026-11-07
owner: "@dahai"
source_of_truth:
  - src/main.rs
  - CLAUDE.md
  - Cargo.toml
  - src/config/mod.rs
---

# 架构 / Architecture

## 总览 / Overview

zh：fusion-cli 是一个纯 Rust 单二进制（`[[bin]] name = "fusion"`，edition 2024，约 27 个 clap 子命令）。它本身不监听端口、不承载业务逻辑：每个子命令都是一次对下游服务的 HTTP/UDS 调用加结果渲染（text 表格或 JSON）。请求默认走 fusion-gateway（:11432），不直连 fusion-mlx（:11434）。

en: fusion-cli is a pure-Rust single binary (`[[bin]] name = "fusion"`, edition 2024, roughly 27 clap subcommands). It listens on no port and owns no business logic: every subcommand is one HTTP/UDS call to a downstream service plus rendering (text table or JSON). Requests go through fusion-gateway (:11432) by default, never straight to fusion-mlx (:11434).

```text
fusion (single binary)
  ├─ clap CLI parse ── RBAC gate (rbac.rs) ── dispatch (src/cmd/*)
  │      │
  │      └─ audit record (utils/audit.rs, hash chain + flock)
  │      └─ metrics counters (utils/metrics.rs -> ~/.fusion/metrics/metrics.json)
  │
  ├─ HTTP   -> fusion-gateway :11432 (mlx), model-hub :11444, rag :11436,
  │            memory :11435, bench :11467, doc :11449, multi-node :11452, desk :9000
  ├─ UDS    -> fusion-supervisor (net), fusion-guard (guard)
  └─ local  -> ~/.fusion/{config.toml,audit,metrics,logs,models}
```

## 模块布局 / Module layout

| Path | 职责 / Responsibility |
|---|---|
| `src/main.rs` | clap 定义 + dispatch；tokio 多线程运行时；审计与 metrics 收口 / Clap defs, dispatch, audit and metrics funnel |
| `src/cmd/` | 每个子命令一个模块（model/chat/kb/bench/service/rag/desk/doc/memory/eval/guard/net/doctor/install/audit/metrics/log 等）/ One module per subcommand |
| `src/config/mod.rs` | `~/.fusion/config.toml` 读写、迁移、0600 收敛、坏文件备份 / Config load, migrate, 0600, corrupt-file backup |
| `src/rbac.rs` | reader/operator/admin 命令级门禁，`FUSION_API_KEY` 查表 / Command-level role gate |
| `src/agent/` | 只读工具 agent（list_models/model_info/health/bench_speed）+ loop_engine + permission / Read-only tool agent |
| `src/service/` | 生态服务启停管控 / Ecosystem service control |
| `src/tui/` | ratatui dashboard / TUI dashboard |
| `src/utils/` | audit（hash chain）、metrics、logger、crypto（AES-256-GCM）、backpressure / Shared utils |

## 关键设计 / Key decisions

### 1. 审计先行 / Audit on every command

zh：`async_main` 在 dispatch 前后各取一次时间戳，所有命令（含 denied）都经 `utils::audit::record` 落盘到 `~/.fusion/audit/audit.log`。并发写用 `flock(LOCK_EX)` 串行化，防 hash chain 竞态分叉；`fusion audit verify` 校验链完整性。敏感参数不入审计，仅记录顶层命令标签。

en: `async_main` timestamps around dispatch; every command (including denials) goes through `utils::audit::record` into `~/.fusion/audit/audit.log`. Concurrent writes are serialized with `flock(LOCK_EX)` to prevent hash-chain forks; `fusion audit verify` checks chain integrity. Only top-level command labels are audited, never sensitive arguments.

### 2. 环境变量在运行时前写入 / Env vars written before runtime startup

zh：`FUSION_OUTPUT_FORMAT`、`FUSION_OFFLINE` 在 tokio 运行时建立前的同步 `main` 中用 `unsafe set_var` 写入——Rust 2024 将 `set_var` 标记 unsafe，正是因为多线程启动后写 env 是未定义行为（见 `src/main.rs` 注释 R6）。

en: `FUSION_OUTPUT_FORMAT` and `FUSION_OFFLINE` are written via `unsafe set_var` in synchronous `main`, before the tokio runtime exists — Rust 2024 marks `set_var` unsafe because writing env after worker threads start is undefined behavior (see the R6 note in `src/main.rs`).

### 3. RBAC 纵深防御 / RBAC as defense-in-depth

zh：`rbac::check` 在 dispatch 之前预过滤：reader 级命令始终放行，更高权限命令按 `auth.keys` 表（`FUSION_API_KEY` 查询）裁决；拒绝立即返回并记审计 denied。`mlx.api_key` 隐式 admin，保证 owner 不会被锁死。`auth.enabled=false`（默认）时门禁关闭，保持旧行为。

en: `rbac::check` pre-filters before dispatch: reader-level commands always pass; higher-privilege commands are decided against the `auth.keys` table via `FUSION_API_KEY`; denials return immediately and are audited as denied. `mlx.api_key` is implicitly admin so the owner is never locked out. With `auth.enabled=false` (default) the gate is off, preserving old behavior.

### 4. 密钥封套加密 / Envelope encryption at rest

zh：`encrypt_secrets=true` 时，保存配置将 4 个 api_key 字段用 AES-256-GCM 加密（`enc:` 前缀），密钥是 `~/.fusion/master.key`（32B 随机、0600、OS 熵源），加载时解密回内存明文。`key_previous` 一并加密。

en: With `encrypt_secrets=true`, saving config encrypts the 4 api_key fields with AES-256-GCM (`enc:` prefix) under `~/.fusion/master.key` (32B random, 0600, OS entropy); loading decrypts back to memory plaintext. `key_previous` is encrypted too.

### 5. local-first 离线 / Local-first offline

zh：`--offline` 默认 true，经 `FUSION_OFFLINE=1` 传导到所有网络路径：仅允许 127.0.0.1，huggingface 等外网拉取被拒绝。需要外网时显式 `--offline=false`。

en: `--offline` defaults to true and propagates as `FUSION_OFFLINE=1` into every network path: only 127.0.0.1 is allowed and external pulls (huggingface) are refused. Pass `--offline=false` explicitly to allow them.

## 依赖 / Dependencies

zh：核心依赖：clap 4（CLI）、reqwest 0.12 + tokio（HTTP）、serde/serde_json/toml（序列化）、ratatui/crossterm（TUI）、aes-gcm/sha2（安全）、sysinfo（系统信息）、cron（定时）。完整清单见 `Cargo.toml`；`Cargo.lock` 已提交以保证可复现构建。

en: Core dependencies: clap 4, reqwest 0.12 + tokio, serde/serde_json/toml, ratatui/crossterm, aes-gcm/sha2, sysinfo, cron. Full list in `Cargo.toml`; `Cargo.lock` is committed for reproducible builds.

## 约束 / Constraints

- zh：仅 macOS Apple Silicon（`native` feature 假定 Metal 可用）；`--no-gpu` 只降级到 CPU，不解决平台适配。
- en: macOS Apple Silicon only (the `native` feature assumes Metal); `--no-gpu` falls back to CPU but does not port the platform.
- zh：CLI 不做结果缓存与重试编排（除 `failover_urls` 按序尝试）；下游服务宕机即失败。
- en: The CLI keeps no result cache and no retry orchestration (beyond ordered `failover_urls`); a down service fails the command.
- zh：metrics 为单进程累计快照，跨进程聚合需外部采集 `fusion metrics export` 输出。
- en: Metrics are one process's cumulative snapshot; cross-process aggregation requires collecting `fusion metrics export` externally.
