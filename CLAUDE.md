# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build --release          # Binary: target/release/fusion
cargo run -- <subcommand>      # Run in dev mode
cargo test                     # 139 tests + 3 ignored, all in #[cfg(test)] blocks (no tests/ dir)
cargo check --all-targets      # Type-check incl. tests/benches
cargo fmt --all -- --check     # Format check (CI enforces)
cargo clippy --all-targets -- -D warnings   # Lint, warnings = errors (CI enforces)
make check                     # clippy + fmt + test (Makefile shortcuts: build/release/lint/clean)
```

Run a single test: `cargo test <test_name>` (tests are inline `mod tests` blocks — grep `mod tests` to find them).

CI (`.github/workflows/ci.yml`, pushes to `main`/`port-standardization` + PRs to `main`): check → test → fmt → clippy → build release, plus a multi-OS build matrix producing artifacts. Releases: `release.yml` on `v*` tags (stable), `release-canary.yml` for canary builds; `v*-beta.*` tags publish as prerelease. Dependency policy locked by `deny.toml` (cargo-deny) — new transitive dep versions can fail CI.

## Architecture

Rust 2024 edition CLI binary using clap derive macros. Single entry point dispatches to subcommand handlers. **Single-machine focus**: `cluster`/`sync` commands and the multinode service client were removed; `MultinodeConfig` remains in config for backward compatibility only.

```
main.rs          → Cli struct (global args) + Commands enum → dispatch.
                   Env vars (FUSION_OUTPUT_FORMAT/FUSION_OFFLINE) set in sync main() BEFORE tokio runtime
                   starts — Rust 2024 marks set_var unsafe for exactly this; do not move it later.
cmd/             → One file per subcommand group, each with its own *Commands enum + handle_*()
service/         → Unified HTTP service layer: global reqwest::Client, cached config, typed APIs
tui/             → TUI dashboard (ratatui + crossterm): event loop, app state, rendering, data fetcher
agent/           → AI Agent engine: loop, context, permissions, tool calling
tools/           → Tool registry with built-in tools (list_models, model_info, health, bench_speed)
config/          → FusionConfig: TOML config at ~/.fusion/config.toml (CURRENT_CONFIG_VERSION = "0.4.2")
rbac.rs          → Role-based access control
utils/           → logger, output (JSON), audit (tamper-evident hash chain, flock-guarded),
                   crypto (AES-256-GCM for config secrets), metrics, backpressure, cron_store
```

### Key Design Constraint: ServiceUrls

All service URLs flow through `service/mod.rs` → `ServiceUrls::from_config()` (config is cached; call `invalidate_config_cache()` after writing config). MLX inference routes through the **gateway** at `http://localhost:11432` (OpenAI-compatible `/v1/*`, requires `Authorization: Bearer <mlx.api_key>`, default key `fg-admin-key`) with HA failover via `mlx_base_candidates()` (backup gateway list). `mlx_api()` normalizes the base URL — strips trailing `/` and any `/v1` suffix, then appends `/v1` — idempotent against trailing-slash or double-`/v1` bugs. All other ecosystem services (kb/modelhub/rag/desk/doc/memory/bench) hit their direct local ports, NOT the gateway. This CLI is fusion-mlx only, no other inference backends.

Non-streaming chat (`chat_completion`) transparently aggregates SSE if the gateway returns a stream for a non-stream request: concatenates `delta.content` across `data:` chunks and extracts `usage`.

### Subcommand Groups

Standalone: `version`, `doctor`, `init`, `completions`, `dashboard` (TUI)
Management: `config`, `audit` (compliance read-only trail), `metrics` (observability snapshot), `log`
Model + inference: `model` (list/pull/info/delete/clean/convert/quant), `chat`/`run`/`code`/`embed` (SSE streaming via eventsource-stream)
Knowledge: `kb` (CRUD + ingest/query), `rag` (start/stop/status/search/list)
Bench: `bench` (speed/mem/ctx/auto via MLX API), `eval` (fusion-bench HTTP service :11467)
Ecosystem: `service`, `doc`, `desk`, `guard` (fusion-guard UDS JSON-RPC, read-only), `net` (fusion-supervisor UDS: up/down/status/restart/ping), `memory` (fusion-memory fm-server :11435)
`agent` — AI agent with read-only tool calling (prompt, model, permission tier sandbox/ask/auto)

Global args: `--offline`, `--verbose`, `--mlx-ctx`, `--mlx-cache`, `--no-gpu`, `--format` (json output must be pure JSON — no banner/log leaks to stdout; see `utils/output.rs`)

### Audit Trail

`utils/audit.rs` appends hash-chained records to `~/.fusion/audit/audit.log`. `record()` holds a cross-process `flock(LOCK_EX)` across read-last-hash + append — concurrent writers forked the chain before this lock existed; any change to record/verify must preserve the lock and `fusion audit verify` must exit non-zero on a broken chain.

### TUI Dashboard

`fusion dashboard`: ratatui + crossterm. `tui/mod.rs` event loop, `tui/app.rs` state machine, `tui/ui.rs` rendering, `tui/service_fetcher.rs` background fetch (health+latency, models, sysinfo, logs). 4 tabs (Services/Models/System/Logs), keys 1-4 tabs, ↑↓ nav, r refresh, s start, x stop, q quit. Auto-refresh every 2s.

## Config & Data Paths

- Config: `~/.fusion/config.toml` (sections: auth, model, kb, mlx, modelhub, rag, desk, doc, memory, bench, multinode(legacy), log, gateway). Secrets AES-256-GCM encrypted (`utils/crypto.rs`).
- Models: `~/.fusion/models/`; KB data: `~/.fusion/kb/`
- Logs: `~/.fusion/logs/` and `~/.fusion/fusion-cli.log`; audit: `~/.fusion/audit/`
- RAG binary: `~/.fusion/bin/`, PID files in `~/.fusion/run/`

## Service URLs

| Service | URL | Config Key |
|---------|-----|------------|
| fusion-mlx (via gateway) | `http://localhost:11432` (+ `/v1/*` appended) | `mlx.base_url` |
| MLX API key | `fg-admin-key` (default) | `mlx.api_key` |
| Model-Hub | `http://localhost:11444` | `modelhub.base_url` |
| Fusion-KB | `http://localhost:11434` | `kb.base_url` |
| Fusion-RAG | `http://localhost:11436` | `rag.base_url` |
| Fusion-Desk | `http://localhost:9000` | `desk.base_url` |
| Fusion-Doc | `http://localhost:11449` | `doc.base_url` |
| Fusion-Memory | `http://localhost:11435` | `memory.base_url` |
| Fusion-Bench | `http://localhost:11467` | `bench.base_url` |
| Gateway | `http://localhost:11432` | `gateway.base_url` |

Port assignments are governed repo-wide by `~/fusion/architecture/port-registry.yaml` — changing a port means editing that registry too, not just this repo.

## Ops & Release

- `scripts/backup.sh` (config/KB backup + DR drill), `scripts/load_test.sh` (RPS/load smoke via `$FUSION` binary)
- `deploy/prometheus-alertmanager/` — monitoring stack (prometheus.yml, alertmanager.yml, `rules/fusion-alerts.yml`)
- `docs/` — ALERTING.md, BACKUP_DR.md, COMPLIANCE.md, ENTERPRISE_SLA.md, OPS_RUNBOOK.md, RELEASE_STRATEGY.md
- Version bumps: Cargo.toml `version` + CHANGELOG.md + README badges; only bump `CURRENT_CONFIG_VERSION` when config schema changes
