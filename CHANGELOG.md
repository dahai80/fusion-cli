# Changelog

All notable changes to **fusion-cli** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.6] - 2026-08-07

### Fixed
- Wire `--format=json` into `version`, `model`, `run`, `embed`, `service`, and `bench`
  subcommands so JSON mode emits pure JSON (no progress-text leak).
- `version` command now uses `env!("CARGO_PKG_VERSION")` instead of a placeholder.

### Added
- Unit-test coverage expanded from 4 to 32 tests covering the critical path with
  zero network dependency (suitable for CI):
  - `aggregate_sse_to_response` (SSE delta concatenation + usage capture, empty stream).
  - `extract_tool_calls` (single/multiple/unclosed/non-kv ```tool block parsing).
  - `ToolExecutor::is_known` / `list_tools` (registry membership + sorting).
  - `ServiceUrls::mlx_api` (v1 append, idempotency, trailing-slash handling).
  - `ServiceUrls::mlx_auth_header` (Bearer scheme, empty-key).
  - `FusionConfig::default` (gateway port, api key, ctx/cache, direct service ports,
    gateway=None, log level).
  - `is_json_mode` (env var set/unset/case-insensitive/non-json).
- `CHANGELOG.md`.

### Changed
- Fixed `mlx_api()` trailing-slash bug: a `mlx.base_url` ending in `/` (e.g.
  `http://localhost:11432/v1/`) no longer produces a doubled `/v1/v1` path.
  Same fix applied to `stats_url()` and `health_check()` base trimming.

### Added
- `fusion doctor` now warns when `mlx.base_url` points at the gateway (:11432)
  but fusion-mlx is unreachable, with a one-line remediation command to switch
  back to the direct MLX port — closes the "config defaults depend on gateway" gap.

## [0.2.5] - 2026-08-07

### Fixed
- Gateway authentication and SSE aggregation: gateway returns `data:` chunks even
  for `stream:false`; `aggregate_sse_to_response()` now concatenates
  `choices[].delta.content` and captures `usage` into a single `InferenceResponse`.
- Service URL routing: all MLX inference routes through `ServiceUrls::from_config()`;
  direct services (kb/modelhub/rag/desk/doc) keep their own ports, not the gateway.
- MLX health check path corrected to `/health` (not `/v1/models`).

### Added
- Gateway `/stats` reverse-proxy support (upstream PR #35, closes issue #34):
  `fusion bench mem` now reports full server memory stats through the gateway.

## [0.2.4] - 2026-08-06

### Changed
- Migrated `mlx.base_url` default to gateway port `:11432` (was direct `:11434`).
- Added Apache License 2.0 (closes #8).
- Added `README_CN.md` for bilingual (Chinese/English) documentation.

## [0.2.3] - 2026-08-06

### Fixed
- `version` command reads `CARGO_PKG_VERSION` (v0.2.3).
- MLX health check uses `/health` instead of `/v1/models`.

### Added
- `fusion doc` subcommand (closes #6): start/stop/status/log for the document service.

## [0.2.2] - 2026-08-05

### Fixed
- CI pipeline: clippy clean, patch version 0.2.1 baseline.

## [0.2.1] - 2026-08-05

### Added
- TUI dashboard (ratatui + crossterm): 4 tabs (Services, Models, System, Logs).
- Gateway service discovery with fallback to `config.toml` URLs.
- `service --watch` continuous refresh mode.

## [0.2.0] - 2026-08-04

### Added
- Real operations: ModelHub API + `huggingface-cli` fallback + `mlx_lm` shell-out
  for model list/pull/info/delete/clean/convert/quant.
- SSE streaming chat/run/code/embed inference via `eventsource-stream`.
- Knowledge base CRUD + ingest/query.
- Speed/mem/ctx/auto/report benchmarking against the real MLX API.
- AI agent with tool calling (`prompt`, `model`, permission tiers).
- `init`, `completions`, `sync`, `cluster`, `desk` subcommands.

### Changed
- Unified HTTP service layer with a global `reqwest::Client` pool.
- Port standardization across all ecosystem services.

## [0.1.1] - 2026-08-03

### Changed
- Phase 1 refactor: unified service layer, port standardization, CI/lint baseline.

## [0.1.0] - 2026-08-01

### Added
- Initial `fusion` single-binary CLI (Rust 2024 edition, clap derive).

[0.2.6]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.6
[0.2.5]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.5
[0.2.4]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.4
[0.2.3]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.3
[0.2.2]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.2
[0.2.1]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.1
[0.2.0]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.0
[0.1.1]: https://github.com/dahai80/fusion-cli/releases/tag/v0.1.1
[0.1.0]: https://github.com/dahai80/fusion-cli/releases/tag/v0.1.0
