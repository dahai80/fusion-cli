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
tui/             → TUI dashboard (ratatui + crossterm): event loop, app state, rendering, data fetcher
agent/           → AI Agent engine: loop, context, permissions, tool calling
tools/           → Tool registry with built-in tools (list_models, model_info, health, bench_speed)
config/          → FusionConfig: TOML config at ~/.fusion/config.toml
utils/           → logger (tracing-subscriber) + output (JSON format support)
```

### Key Design Constraint

All MLX inference goes through `service/mlx.rs` which uses `ServiceUrls::from_config()` for the base URL. Default: `http://localhost:11434/v1`. This CLI is fusion-mlx only, no other backends.

## Subcommand Structure

Each `cmd/*.rs` follows the same pattern: a clap `Subcommand` enum + an async `handle_*()` function that matches on variants. The main subcommand groups are:

- `version`, `doctor`, `init`, `completions`, `dashboard` — standalone commands
- `config` — list/get/set/reset for ~/.fusion/config.toml
- `model` — list/pull/info/delete/clean/convert/quant (real operations: ModelHub API + huggingface-cli fallback + mlx_lm shell-out)
- `chat`, `run`, `code`, `embed` — inference (SSE streaming via eventsource-stream, all go through mlx_bind)
- `kb` — knowledge base CRUD + ingest/query
- `bench` — speed/mem/ctx/auto/report benchmarking (real MLX API calls via generate_tokens)
- `service` — status(start/stop/restart/log for ecosystem services, --watch for continuous refresh)
- `rag` — start/stop/status/search/list for RAG service
- `doc` — start/stop/status/log for document service
- `desk` — list/run/history/cron/stop for desktop automation (real API calls)
- `agent` — AI agent with tool calling (prompt, model, permission tier)
- `sync` — model sync
- `cluster` — cluster management

Global args: `--offline`, `--verbose`, `--mlx-ctx`, `--mlx-cache`, `--no-gpu`, `--format`

## TUI Dashboard

`fusion dashboard` launches an interactive TUI built with ratatui + crossterm:

- `tui/mod.rs` — Event loop, terminal setup/teardown, key handling
- `tui/app.rs` — App state machine (Tab enum, selection, data)
- `tui/ui.rs` — ratatui rendering (services table, models list, system bars, log viewer)
- `tui/service_fetcher.rs` — Background data fetch (health+latency, models, sysinfo, logs)

4 tabs: Services (status+latency+port), Models (loaded list), System (CPU/mem/temp), Logs (recent entries).
Keys: 1-4 tabs, ↑↓ nav, r refresh, s start, x stop, q quit.
Auto-refresh every 2s (10 ticks × 200ms).

## Service Discovery

`service/gateway.rs` provides GatewayClient for dynamic service discovery via fusion-gateway:11432.
When gateway is disabled (default), falls back to config.toml URLs via `fallback_entries()`.
Config section: `[gateway]` with `enabled` and `base_url`.

## Config & Data Paths

- Config: `~/.fusion/config.toml` (sections: model, kb, mlx, modelhub, rag, desk, doc, log, gateway)
- Models: `~/.fusion/models/`
- KB data: `~/.fusion/kb/`
- Logs: `~/.fusion/logs/` and `~/.fusion/fusion-cli.log`
- RAG binary: `~/.fusion/bin/`, PID files in `~/.fusion/run/`

## Service URLs

| Service | URL | Config Key |
|---------|-----|------------|
| fusion-mlx | `http://localhost:11434/v1` | `mlx.base_url` |
| Model-Hub | `http://localhost:11444/v1/models` | `modelhub.base_url` |
| Fusion-KB | `http://localhost:11434/kb/bases` | `kb.base_url` |
| Fusion-Desk | `http://localhost:9000/health` | `desk.base_url` |
| Fusion-RAG | `http://localhost:11436/health` | `rag.base_url` |
| Fusion-Doc | `http://localhost:11449/api/health` | `doc.base_url` |
| Gateway | `http://localhost:11432` | `gateway.base_url` |
