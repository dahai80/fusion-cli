# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build --release          # Binary: target/release/fusion
cargo run -- <subcommand>      # Run in dev mode
cargo test                     # Unit tests in src/service/mlx.rs
cargo check                    # Type-check without building
```

No dedicated lint config. Some dead-code warnings from unused struct fields.

## Architecture

Rust 2024 edition CLI binary using clap derive macros. Single entry point dispatches to subcommand handlers.

```
main.rs          → Cli struct (global args) + Commands enum (subcommands) → dispatch
cmd/             → One file per subcommand group, each with its own *Commands enum + handle_*()
service/         → Unified HTTP service layer with global reqwest::Client and typed APIs
agent/           → AI Agent engine: loop, context, permissions, tool calling
tools/           → Tool registry with built-in tools (list_models, model_info, health, bench_speed)
config/          → FusionConfig: TOML config at ~/.fusion/config.toml
utils/           → logger (tracing-subscriber) + output (JSON format support)
```

### Key Design Constraint

All MLX inference goes through `service/mlx.rs` which uses `ServiceUrls::from_config()` for the base URL. Default: `http://localhost:11434/v1`. This CLI is fusion-mlx only, no other backends.

## Subcommand Structure

Each `cmd/*.rs` follows the same pattern: a clap `Subcommand` enum + an async `handle_*()` function that matches on variants. The main subcommand groups are:

- `version`, `doctor`, `init`, `completions` — standalone commands
- `config` — list/get/set/reset for ~/.fusion/config.toml
- `model` — list/pull/info/delete/clean/convert/quant (real operations: ModelHub API + huggingface-cli fallback + mlx_lm shell-out)
- `chat`, `run`, `code`, `embed` — inference (SSE streaming via eventsource-stream, all go through mlx_bind)
- `kb` — knowledge base CRUD + ingest/query
- `bench` — speed/mem/ctx/auto/report benchmarking (real MLX API calls via generate_tokens)
- `service` — status/start/stop/restart/log for ecosystem services
- `rag` — start/stop/status/search/list for RAG service
- `desk` — list/run/history/cron/stop for desktop automation (real API calls)
- `agent` — AI agent with tool calling (prompt, model, permission tier)
- `sync` — model sync
- `cluster` — cluster management

Global args: `--offline`, `--verbose`, `--mlx-ctx`, `--mlx-cache`, `--no-gpu`, `--format`

## Config & Data Paths

- Config: `~/.fusion/config.toml` (sections: model, kb, mlx, modelhub, rag, desk, log)
- Models: `~/.fusion/models/`
- KB data: `~/.fusion/kb/`
- Logs: `~/.fusion/logs/` and `~/.fusion/fusion-cli.log`
- RAG binary: `~/.fusion/bin/`, PID files in `~/.fusion/run/`

## Hard-coded Service URLs

| Service | URL |
|---------|-----|
| fusion-mlx | `http://localhost:11434/v1` |
| Model-Hub | `http://localhost:11444/v1/models` |
| Fusion-KB | `http://localhost:11434/kb/bases` |
| Fusion-Desk | `http://localhost:9000/health` |
| Fusion-RAG | `http://localhost:11436/health`, `http://localhost:11436/api/v1` |
