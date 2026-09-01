# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build --release          # Binary: target/release/fusion
cargo run -- <subcommand>      # Run in dev mode
cargo test                     # 63 unit tests across 9 files
cargo check --all-targets      # Type-check incl. tests/benches
cargo fmt --all -- --check     # Format check (CI enforces)
cargo clippy --all-targets -- -D warnings   # Lint, warnings = errors (CI enforces)
```

Tests live in `#[cfg(test)]` blocks at the bottom of: `service/mlx.rs`, `service/mod.rs`, `tools/mod.rs`, `config/mod.rs`, `agent/mod.rs`, `utils/output.rs`. Most are pure-logic (URL normalization, SSE aggregation, config defaults, JSON mode, tool registry) — no network needed.

CI (`.github/workflows/ci.yml`, runs on `macos-latest`): check → test → fmt → clippy → build release, all gating on push to `main`/`port-standardization` and PRs to `main`. clippy and fmt are hard gates (`-D warnings`).

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

All MLX inference goes through `service/mlx.rs` → `ServiceUrls::from_config()`. MLX inference routes through the **gateway** at `http://localhost:11432` (OpenAI-compatible `/v1/*` endpoints, requires `Authorization: Bearer <mlx.api_key>`, default key `fg-admin-key`). `mlx_api()` normalizes the base URL — strips trailing `/` and any `/v1` suffix, then appends `/v1` — so it's idempotent against trailing-slash or double-`/v1` bugs. Other ecosystem services (kb/modelhub/rag/desk/doc) hit their direct local ports, NOT the gateway. This CLI is fusion-mlx only, no other backends.

Non-streaming chat (`chat_completion`) transparently aggregates SSE if the gateway returns a stream for a non-stream request (`aggregate_sse_to_response`): concatenates `delta.content` across `data:` chunks and extracts `usage`.

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
| fusion-mlx (via gateway) | `http://localhost:11432` (+ `/v1/*` appended) | `mlx.base_url` |
| MLX API key | `fg-admin-key` (default) | `mlx.api_key` |
| Model-Hub | `http://localhost:11444` | `modelhub.base_url` |
| Fusion-KB | `http://localhost:11434` | `kb.base_url` |
| Fusion-Desk | `http://localhost:9000` | `desk.base_url` |
| Fusion-RAG | `http://localhost:11436` | `rag.base_url` |
| Fusion-Doc | `http://localhost:11449` | `doc.base_url` |
| Gateway | `http://localhost:11432` | `gateway.base_url` |

MLX is the only service routed through the gateway; all others use direct ports. The gateway table entries (base URLs) are config values; endpoints like `/models`, `/health`, `/v1/chat/completions` are appended in code.
