# Changelog

All notable changes to **fusion-cli** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2026-09-01

### Fixed
- `--format=json` no longer leaks banner text before JSON in `cluster`, `memory`,
  and `eval` command groups. All pre-match `println!` banners are now guarded by
  `if !output::is_json_mode()`, so `--format=json` emits pure JSON (verified live:
  `{` on first line). Established by the v0.2.6 JSON-purity pattern; missed on the
  new v0.3.0 command groups, now applied to `cluster.rs`, `memory.rs`,
  `benchsvc.rs` (`sync.rs` was already clean).
- `cluster status` / `cluster nodes` / `cluster routing` / `cluster pending` /
  `sync manifest` / `sync incremental` now reach the fusion-multi-node Master
  directly (port 11452) instead of the MLX gateway. The old routing hit
  `mlx.base_url/api/cluster/status`, which the gateway has no route for (404).
- `cluster` / `sync` commands no longer return `{"detail":"Unauthorized"}` from
  the Master. `BearerAuthMiddleware` (fusion_multi_node/utils/auth.py) requires a
  Bearer `cluster_token` on all routes except `/api/health*`. CLI now attaches
  `Authorization: Bearer <multinode.api_key>` via an `auth_header()` helper
  (mirroring memory.rs); send no header when the key is empty.

### Added
- `multinode.api-key` config key (`~/.fusion/config.toml`, `[multinode]` section,
  default empty). Set with `fusion config set multinode.api-key <FUSION_CLUSTER_TOKEN>`
  to unlock `cluster` / `sync` against a token-protected Master.

### Tests
- 63 → 69 unit tests (+6 multinode service URL/payload-shape tests, +1 config
  default-`api_key`-empty test). Gates green: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings` (exit 0), `cargo test`.

## [0.3.0] - 2026-09-01

### Added
- `fusion memory` subcommand group — HTTP client to fusion-memory `fm-server`
  (port 11435, Bearer auth). Routes source: `crates/fm-server/src/http.rs`.
  - `fusion memory status` — alive + API version (`/healthz`, `/v1/memory/version`).
  - `fusion memory version` — API version (public endpoint).
  - `fusion memory search <query> [--top-k=N]` — semantic retrieve (`/v1/memory/retrieve`).
  - `fusion memory count` — total entries (`/v1/memory/count`).
  - `fusion memory get <id>` — fetch one (`/v1/memory/:id`).
  - `fusion memory commit <content> [--scope=...]` — write (`/v1/memory/commit`).
  - `fusion memory consolidate` — short→long (`/v1/memory/consolidate`).
  - `fusion memory delete <id>` — delete with `confirm:true` (`/v1/memory/delete`).
  - `fusion memory audit` — audit log (`/v1/memory/audit`).
  - Auth: Bearer from `memory.api-key` (config). Default empty → `auth_header()`
    returns `None`, only public endpoints (healthz, version) work unauthenticated.
    Set `fusion config set memory.api-key <key>` for commit/retrieve/delete/etc.
- `fusion eval` subcommand group — HTTP client to fusion-bench service
  (port 11467, `/api/v1/*`). Distinct from `fusion bench speed/mem/ctx/auto`
  (local MLX self-benchmark): `fusion eval` queries the bench server.
  - `fusion eval status` — service health (`/api/v1/system/health`).
  - `fusion eval resources` — CPU/GPU/mem (`/api/v1/system/resources`).
  - `fusion eval tasks` / `eval task <id>` — task list/detail (`/api/v1/tasks`).
  - `fusion eval suites` — suite list (`/api/v1/suites`).
  - `fusion eval result <task_id>` — result (`/api/v1/results/:id`).
  - `fusion eval trend` — results trend (`/api/v1/results/trend`).
  - `fusion eval baselines` / `eval gates` — baselines + quality gates.
- `fusion cluster` rewritten — now hits fusion-multi-node Master (11452) via
  `service/multinode.rs` (was wrongly routing to the MLX gateway → 404).
  - `fusion cluster status` — `/api/cluster/status`.
  - `fusion cluster nodes` / `node <id>` / `remove <id>` — `/api/nodes*`.
  - `fusion cluster pending` / `approve <id> [--approved-by]` / `reject <id> [--reason]`.
  - `fusion cluster routing` — `/api/routing/summary`.
- `fusion sync` rewritten — same fix, routes to Master 11452 not gateway:
  - `fusion sync manifest <model>` — `/api/models/:name/manifest`.
  - `fusion sync incremental <model> [--source=...]` — `POST /api/sync/incremental`.
- Config: new sections `[memory]` (base_url, api_key), `[bench]` (base_url),
  `[multinode]` (base_url). Default ports 11435/11467/11452. New config keys:
  `memory.base-url`, `memory.api-key`, `bench.base-url`, `multinode.base-url`.
- `service/health.rs`: `check_all_with_latency` now probes Memory/Bench/MultiNode.
- 13 new unit tests (config +3, memory +6, benchsvc +4). Test count 51 → 63.

### Changed
- `fusion sync`/`fusion cluster` no longer route through the MLX gateway — bug
  fix: old `get_base_url()` returned `ServiceUrls.mlx` (gateway 11432) then
  appended `/api/cluster/*`, hitting routes the gateway does not serve (404).
  Real routes live on the multi-node Master at 11452.

## [0.2.8] - 2026-09-01

### Added
- `fusion net` subcommand group (closes #13): thin forwarding to the
  fusion-supervisor daemon over UDS (JSON-RPC 2.0, newline-framed, 5s timeout).
  Socket defaults to `/tmp/fusion-sv.sock`, overridable via `FUSION_SV_SOCKET`.
  - `fusion net up` → `up` → start all supervised services.
  - `fusion net down` → `down` → stop all supervised services.
  - `fusion net status` → `status` → service table (name, state, port).
  - `fusion net restart <service>` → `restart` → restart a named service.
  - `fusion net ping` → `ping` → daemon alive check.
  - Optional token auth via `FUSION_SV_TOKEN` (forwarded in `params.token`,
    matching supervisor's optional token check).
  - Daemon-down exits with code 3 + `fusion-sv daemon` hint (matches `fusion-sv` CLI).
- `service/sv.rs`: sync UDS client mirroring `service/guard.rs` pattern, with a
  typed `SvError` (DaemonDown / Rpc / Other) so the command layer can map
  daemon-down to exit 3 vs rpc-error to exit 1. Envelope mirrors
  fusion-supervisor `src/rpc/schema.rs` (`id: i64`, `params: Value`).
- 13 unit tests for sv wire framing (JSON-RPC envelope, result/error parsing,
  status array, restart params, token injection pure fn, socket-path resolution,
  error classification) — race-free, no env mutation. Test count 38 → 51.

## [0.2.7] - 2026-08-29

### Added
- `fusion guard` subcommand group (closes #9): read-only queries to the
  fusion-guard daemon over UDS (JSON-RPC 2.0, newline-framed, 2s timeout).
  Socket defaults to `/tmp/fusion-guard.sock`, overridable via `FUSION_GUARD_SOCK`.
  - `fusion guard status` → `guard.ping` → alive, version, rules epoch.
  - `fusion guard rules` → `guard.rule.list` → current rule set + epoch.
  - `fusion guard audit --limit=N` → `guard.audit.list` → recent audit events.
  - No authz decisions, no rule mutation (PRD §11 boundary respected).
- `service/guard.rs`: sync UDS client mirroring the `fg-pyo3` `UdsClient` pattern
  (`std::os::unix::net::UnixStream`, `BufReader` newline framing).
- 6 unit tests for guard wire framing (JSON-RPC envelope, result/error parsing,
  socket-path resolution) — no socket, network, or env mutation needed. Test count 32 → 38.

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
- Gateway `/stats` reverse-proxy support (upstream PR #35 merged, closes issue #34):
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

[0.3.0]: https://github.com/dahai80/fusion-cli/releases/tag/v0.3.0
[0.2.6]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.6
[0.2.5]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.5
[0.2.4]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.4
[0.2.3]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.3
[0.2.2]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.2
[0.2.1]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.1
[0.2.0]: https://github.com/dahai80/fusion-cli/releases/tag/v0.2.0
[0.1.1]: https://github.com/dahai80/fusion-cli/releases/tag/v0.1.1
[0.1.0]: https://github.com/dahai80/fusion-cli/releases/tag/v0.1.0
