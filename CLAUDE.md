# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build --release          # Binary: target/release/fusion
cargo run -- <subcommand>      # Run in dev mode
cargo test                     # Only 2 inline unit tests in src/mlx_bind/mod.rs
cargo check                    # Type-check without building
```

No dedicated lint config. Compiler currently emits dead-code warnings from unused modules.

## Architecture

Rust 2024 edition CLI binary using clap derive macros. Single entry point dispatches to subcommand handlers.

```
main.rs          → Cli struct (global args) + Commands enum (subcommands) → dispatch
cmd/             → One file per subcommand group, each with its own *Commands enum + handle_*()
mlx_bind/        → Core binding: OpenAI-compatible HTTP client to fusion-mlx (localhost:11434/v1)
config/          → FusionConfig: TOML config at ~/.fusion/config.toml
ecosystem/       → Stub HTTP clients for Model-Hub/KB/Desk (currently UNUSED — cmd/ makes direct reqwest calls)
system/          → System info helpers (currently UNUSED)
utils/logger.rs  → tracing-subscriber init with EnvFilter (respects RUST_LOG, defaults to fusion_cli=info)
```

### Key Design Constraint

`mlx_bind/mod.rs` hard-codes `MLX_BASE_URL = "http://localhost:11434/v1"` with a compile-time assertion — this CLI is fusion-mlx only, no other backends.

### Known Issue: Dead Modules

`ecosystem/` and `system/` modules are defined but never called — `cmd/` modules make inline `reqwest` calls instead. This causes ~23 dead-code warnings. Several Cargo.toml dependencies (`sha2`, `hex`, `uuid`, `glob`, `csv`, `clap_complete`) are also unused.

## Subcommand Structure

Each `cmd/*.rs` follows the same pattern: a clap `Subcommand` enum + an async `handle_*()` function that matches on variants. The main subcommand groups are:

- `version`, `doctor`, `log` — standalone commands
- `config` — list/get/set/reset for ~/.fusion/config.toml
- `model` — list/pull/info/delete/clean/convert/quant
- `chat`, `run`, `code`, `embed` — inference (all go through mlx_bind)
- `kb` — knowledge base CRUD + ingest/query
- `bench` — speed/mem/ctx/auto/report benchmarking
- `service` — status/start/stop/restart/log for ecosystem services
- `rag` — start/stop/status/search/list for RAG service
- `desk` — list/run/history/cron/stop for desktop automation

Global args: `--offline`, `--verbose`, `--mlx-ctx`, `--mlx-cache`, `--no-gpu`

## Config & Data Paths

- Config: `~/.fusion/config.toml` (sections: model, kb, mlx, log)
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
