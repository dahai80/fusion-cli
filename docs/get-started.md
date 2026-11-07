---
doc_id: fusion-cli/get-started
product: fusion-cli
class: core
entry: service
doc_type: get-started
title: { zh: "fusion-cli 快速开始", en: "fusion-cli get started" }
audience: engineer
status: draft
version: 0.4.6
updated: 2026-11-07
owner: "@dahai"
source_of_truth:
  - README.md
  - src/main.rs
  - src/cmd/init.rs
---

# 快速开始 / Get Started

## 前置条件 / Prerequisites

zh：macOS 14+ 与 Apple Silicon（M 系列芯片）、Xcode Command Line Tools、Rust 工具链。CLI 默认离线模式，仅访问 127.0.0.1 服务；首次使用前需先启动 fusion-mlx（`~/fusion/fusion-mlx/start.sh start`）。

en: macOS 14+ on Apple Silicon, Xcode Command Line Tools, and a Rust toolchain. The CLI defaults to offline mode and only reaches 127.0.0.1 services; start fusion-mlx first (`~/fusion/fusion-mlx/start.sh start`).

## 安装 / Install

```bash
git clone https://github.com/dahai80/fusion-cli.git
cd fusion-cli
cargo build --release
cp target/release/fusion /usr/local/bin/   # or any directory on PATH
```

## 初始化 / Initialize

```bash
fusion init        # scaffold ~/fusion workspace layout and default config
fusion doctor      # environment self-check; add --prod to fail on missing required keys
```

zh：`init` 生成工作目录与默认配置；`doctor` 自检环境并在异常时给出修复提示。配置写入 `~/.fusion/config.toml`（0600）。

en: `init` scaffolds the workspace and default config; `doctor` verifies the environment and reports fixes. Config lives at `~/.fusion/config.toml` (0600).

## 第一个对话 / First chat

```bash
fusion model list                       # list local MLX models
fusion model pull <model>               # download via hf-mirror (set HF_MIRROR=https://hf-mirror.com)
fusion chat -m <model> --stream         # interactive chat with SSE streaming
fusion run -m <model> -p "hi"           # single prompt, exit after answer
fusion run -m <model> -p "hi" --format json
```

zh：`--model` 必填，可用 `--max-tokens`（默认 2048）、`--temperature`（默认 0.7）、`--top-p`（默认 0.9）控制生成。`--ctx` 是 `--max-tokens` 的旧别名，指生成上限而非上下文窗口。

en: `--model` is required; tune generation with `--max-tokens` (default 2048), `--temperature` (default 0.7), and `--top-p` (default 0.9). `--ctx` is a legacy alias for `--max-tokens` — a generation cap, not the context window.

## 日常运维 / Day-2 operations

```bash
fusion service status               # ecosystem service health
fusion bench speed -m <model>       # generation speed benchmark
fusion agent "list local models" --permission ask   # read-only AI assistant
fusion dashboard                    # TUI: live services + system monitor
```

## 下一步 / Next steps

zh：完整子命令见 `docs/api-reference.md`；配置项与默认值见 `docs/config-reference.md`；故障排查见 `docs/troubleshooting.md`。生成 shell 补全：`fusion completions zsh`（支持 bash/zsh/fish/elvish/powershell）。

en: Full subcommand list in `docs/api-reference.md`; config keys and defaults in `docs/config-reference.md`; failures in `docs/troubleshooting.md`. Shell completions: `fusion completions zsh` (bash/zsh/fish/elvish/powershell).

## 约束 / Constraints

- zh：仅 macOS Apple Silicon；`--offline` 默认开启，外网拉取需显式 `--offline=false`。
- en: macOS Apple Silicon only; `--offline` is on by default, pass `--offline=false` for external downloads.
- zh：RBAC 启用后（`fusion config roles-enable on`），reader 角色无法执行 operator/admin 级命令。
- en: Once RBAC is enabled (`fusion config roles-enable on`), the reader role cannot run operator/admin commands.
