# Changelog

All notable changes to **fusion-cli** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.5] - 2026-09-01

Enterprise production-readiness round 2 — added audit trail, observability
metrics, and consolidated architecture debt (A1). 95 → 101 tests. All gates green.

### Added — Compliance & Observability
- **Audit trail** (`~/.fusion/audit/audit.log`): append-only JSONL recording
  `ts/actor/command/outcome/duration_ms/detail` for every invocation. Credential
  fields (api_key/token/secret/password) auto-redacted; detail capped at 2 KB.
  New `fusion audit view -c N` / `fusion audit path` subcommands (read-only).
- **Observability metrics** (`~/.fusion/metrics/metrics.json`): process-level
  counters (requests, errors, model pulls, KB ingests, bench runs, service ops)
  + latency histogram buckets (0-50/50-200/200-500/500-2000/2000+ ms). New
  `fusion metrics view` / `fusion metrics json` / `fusion metrics path`
  subcommands. JSON export enables external Prometheus exporter wiring.
- Both audit + metrics hooked into the central `async_main` dispatch loop —
  no per-command instrumentation needed.

### Architecture — A1 ServiceUrls Consolidation
- Removed drift-prone duplicate `base_url()`/`stats_url()` helpers in
  `service/mlx.rs` that each called `from_config()` independently of the
  same function's `service_urls()` (redundant config reads + duplicated
  trim-`/v1` logic). Centralized MLX root-path trimming into a new
  `ServiceUrls::mlx_base()` accessor; `mlx_api()` now delegates to it. Every
  mlx.rs function now resolves `ServiceUrls` exactly once per call.

### Fixed — Product Audit Functional (G-series)
- **G1 (P2)**: `kb ingest` now explicitly prints "Files staged in KB dir (not
  yet vectorized)" so users know the op is file staging, not real indexing.
  Full vectorization routing to fusion-rag's `/bases/{kb_id}/documents/ingest`
  tracked in issue #18 (requires running fusion-rag + embedding model).
- **G2 (P2)**: `Agent` subcommand help text now reads "AI 只读助手 (带只读工具
  调用)" — accurate positioning, no over-promise of write/orchestration ability.
- **G3 (P3)**: `desk cron` non-persistence already explicitly labeled (unchanged,
  audit confirmed non-concealed).

### Tests
- 95 → 101 (audit redaction + metrics bucketing + metrics snapshot).

## [0.3.4] - 2026-09-01

Product-readiness audit remediation — fixed all P0–P3 findings from the
six-dimension product audit (`audit/fusion-cli-audit-result-product-0901.md`).
91 → 95 tests. Build, fmt, clippy (`-D warnings`) all green.

### Security (P0)
- **`fusion init` no longer writes a world-readable 0644 config with a hardcoded
  default `fg-admin-key`.** Init now generates a random 32-byte hex API key from
  `/dev/urandom` and writes the config via `save_config` (chmod 0600). Existing
  configs keep their key but `doctor` now flags open permissions (P0-1).
- **Upgrade no longer silently drops user config.** All config sub-structs
  (`model`/`kb`/`mlx`/`modelhub`/`rag`/`desk`/`doc`/`memory`/`bench`/`multinode`/`log`)
  now use `#[serde(default)]` per-field with `default = "fn"` helpers plus a per-struct
  `Default` impl, so a v0.2.x config missing newer sections parses and fills defaults
  instead of failing. A `config_version` field + `migrate_config()` tracks schema versions
  (current `0.3.4`). Malformed TOML is backed up to `config.toml.bak.<unix_ts>` and
  surfaced via `eprintln` + `tracing::error` rather than silently falling back (P0-2).

### Fixed (P1)
- **Path-traversal injection on URL path segments.** Added a shared
  `service::validate_path_segment` rejecting `/` and `\`, applied to kb names,
  RAG `kb_id`, KB service `kb_id`, ModelHub model name, desk `task_id`,
  bench `task_id`, memory `id`, and multinode `node_id`/`model_name` (S5/A14).
- **Bare `resp.json()` swallowed HTTP errors as opaque serde failures.** Unified
  15+ sites across `service/{kb,rag,doc,benchsvc,modelhub,desk}.rs` through
  `json_or_error(resp, service)`, which bails with service name + status + body
  snippet on non-2xx. Replaced `unwrap_or_default()` silent swallows in
  `desk::list_templates`/`get_history` and `modelhub::search` (P1-3).
- **Orphaned child processes on timeout.** `model pull/convert/quant` external
  commands now set `kill_on_drop(true)` so a timeout actually SIGKILLs the
  `huggingface-cli`/`mlx_lm` child instead of leaving it consuming bandwidth/disk (P1-4).
- **`parent().unwrap()` panic risk in `rag start`** replaced with `ok_or_else`
  error propagation (P1-4).
- **Persistent file logging.** Added `tracing-appender` writing to
  `~/.fusion/fusion-cli.log` alongside the console layer; terminal output stays
  timestamp-free while the file layer carries timestamps for post-mortem audit.
  Falls back to console-only if the log dir is unwritable (P1-6).
- **`service restart` raced on a hardcoded 1s sleep.** Now polls
  `health_check` until the service is confirmed down (up to 5s) before
  starting, avoiding port-in-use failures (P1-7).
- **`doctor` now covers the full ecosystem.** Added `fusion-memory`,
  `fusion-bench`, `fusion-multi-node`, and `Fusion-Doc` health checks, plus a
  config validation block: parse success, `config_version` freshness, and
  `0600` permission check (P1-8).
- **Dead gateway subsystem removed.** `GatewayConfig` struct, `gateway` field,
  and `default_gateway_url()` deleted; MLX port routing is independent and
  unchanged (P1-9).

### Fixed (P2/P3)
- **`kb ingest` reported attempted count as success.** Metadata `document_count`
  now reflects the actual count of successfully copied files, and a warning lists
  failures (F6).
- **`bench` `System::new_all()` enumerated all processes for a memory-only read.**
  Switched to `System::new()` + `refresh_memory()` at both bench sites (P3-4).

## [0.3.3] - 2026-09-01

### Security
- **`model pull` path traversal fully sealed.** HF repo id (e.g. `mlx-community/Qwen2-7B`)
  is sanitized to a flat local name via `replace('/', "_")` *before* `validate_model_name`,
  then a `canonicalize().starts_with(models_dir)` prefix check blocks symlink escape. The
  download still uses the original repo id for the upstream fetch (P0-1, audit A3).
- **`config.toml` permissions restricted to 0600** on save (chmod via
  `PermissionsExt`), since the file holds the gateway API key (`mlx.api_key`). Write/parse
  errors now `tracing::error!` instead of silently returning defaults (audit R10).

### Fixed
- **Agent context unbounded growth → context-window overflow.** `add_assistant_message`
  and `add_system_message` (tool results can reach tens of KB) previously never trimmed;
  10 tool-dense turns blew past `max_position_embeddings` → 400/413 with no fallback. Now
  every add calls `trim()`: a message-count cap (`max_turns*2`) plus a token-budget cap
  (75% of `mlx.default_ctx`) dropping oldest messages first (audit A5).
- **SSE non-streaming response buffered whole body into memory.** `chat_completion`
  aggregated by `resp.text().await` then line-parsed — long generations held the entire
  token stream as a resident `String`, RSS climbing linearly, defeating streaming. Rewritten
  to `resp.bytes_stream()` incremental parsing; memory = content string only (audit R1).
- **`service start/stop` honesty.** Each start/stop function returns `bool` (true = CLI
  managed the process, false = manual launcher needed). The `all` branch splits results
  into `started`/`manual_required` and prints an honest summary — never claims `✅ All
  started` when services are no-ops (audit A1, hardens v0.3.2 A1).
- **Bench JSON purity + correctness.** `bench_mem`/`bench_ctx`/`bench_auto`/`bench_report`
  now guard all banners with `is_json_mode()` and emit structured JSON payloads; progress
  bars removed from the JSON path. `bench_auto` actually exercises context length via
  `chat_completion` before `generate_tokens`. `bench_ctx` adds a 500ms settle sleep between
  steps; `bench_speed` 1s between runs (audit R2, E12, R8).
- **`check_url` typed retry.** `check_url_verbose` distinguishes connection-refused /
  timeout / invalid-request / HTTP-status / other, and `check_url_with_retry` returns
  false immediately on 4xx (no retry) with exponential backoff on transient errors
  (audit R3/R4).
- **TUI system-info CPU/IO churn.** `fetch_system_info` reused a single `System` instance
  (Lazy<Mutex<System>>) calling only `refresh_cpu_usage`+`refresh_memory` — no per-tick
  `System::new_all()`/`refresh_all()` process enumeration (audit R9).
- **Config cache invalidation.** `ServiceUrls::from_config()` now reads an mtime-cached
  config (reloads only on file change); `save_config` invalidates the cache after write
  (audit A4).
- **Env-var race before tokio workers.** `main` is now sync, sets `FUSION_OUTPUT_FORMAT` /
  `FUSION_OFFLINE` before building the multi-thread runtime — no `set_var` after workers
  start (audit R6).
- **`check_named` covers all 9 services** (mlx/kb/modelhub/rag/desk/doc/memory/bench/
  multinode) with per-service health paths and latency timing (audit E8).
- **`extract_port` IPv6-safe** via `reqwest::Url::port()` — old `split(':').next_back()`
  misparsed `[::1]:11434` and path-bearing URLs (audit E7).
- **`convert_model` script-existence check** uses the convert script if present, else
  falls back to `python3 -m mlx_lm.convert`; no silent invocation of a missing script
  (audit E11).

### Changed
- `service::mod::check_url` delegates to `check_url_verbose().0`; the verbose variant
  returns `(bool, String)` reason for retry logic.
- Dead code removed: `OutputFormat` enum + `JsonPrinter` (`utils/output.rs`), `check_all`
  (`service/health.rs`), `ContextManager::clear`. `agent::loop_engine::LoopStats` is now
  wired into the agent loop (was scaffold) — logs `Turns/Tools/Tokens` at loop end
  (audit E2).

### Tests
- Unit test count: 71 → 91 (+20). Added risk-path coverage the audit flagged as absent:
  - `validate_model_name` path-traversal rejection (`../etc/passwd`, `org/../../sensitive`,
    `..`, `.`, `/`, `\`, `.hidden`) and acceptance of safe ids (audit E1).
  - `pull_model` safe-name derivation neutralizes `/` traversal (audit E1).
  - `extract_port` IPv4/IPv6/path/invalid (audit E7).
  - `ContextManager` trim by message-count, by token-budget, across assistant/system
    messages, empty context (audit A5).
- Gates green: `cargo fmt --all -- --check` (exit 0), `cargo clippy --all-targets --
  -D warnings` (exit 0), `cargo test` (91 passed, 0 failed).

## [0.3.2] - 2026-09-01

### Security
- **Permission system now enforced (was non-functional).** `agent::permission::confirm`
  actually prompts and honors the tiered tool list — previously all tools auto-allowed
  regardless of the "dangerous" classification (P0-1). Default-to-deny on prompt error.
- **Path traversal in model delete/info/convert/quantize blocked.** `validate_model_name`
  rejects names containing path separators/back-segments, preventing `../` escape out of
  the models directory (P0-2).

### Fixed
- Fake service stops removed: `service stop kb/modelhub/desk/doc` now print honest
  "manual stop required" hints instead of a false `✅` (P1-1).
- `service log <name>` matching fixed — only tail files whose name starts with the
  service prefix, no longer grabs unrelated logs (P1-2).
- `desk run --params` now forwards params to the service; `desk cron` honestly reports
  it does not persist schedules (no placebo) (P1-3).
- `--ctx` is a documented legacy alias for `--max-tokens` (generation cap), not context
  window size; `effective_max_tokens()` resolves both (P1-5).
- `net` subcommand error path honors `--format=json` (P1-6).
- `--format=json` purity extended to all remaining handlers: `model` (pull/info/delete/
  clean/convert/quant), `desk` (list/run/history/cron/stop), `service` (start/stop/log),
  `rag` (start/stop/status/search/list), `chat` (rejects interactive REPL in JSON mode)
  (P2-2).
- `--version` now driven by `env!("CARGO_PKG_VERSION")` — single source of truth (P2-3).
- `--offline` now gates external-network commands (`model pull/convert`) via the
  `FUSION_OFFLINE` env var, failing fast with a clear message instead of hanging on DNS
  (P2-4).
- Dead `service::gateway` module deleted — gateway routing already lives in
  `ServiceUrls::mlx_api()` (P2-1).

### Changed
- Health checks now probe all 9 services **concurrently** (`futures::join_all`); worst-case
  blocking drops from 18s to 2s (P3-1).
- All service modules route HTTP responses through `json_or_error()`: non-2xx bodies are
  surfaced as `"<service> HTTP <status>: <body>"` instead of opaque serde parse errors
  (P3-2). Applied to `mlx`, `memory`, `multinode`, `desk`.
- SSE aggregator logs malformed chunks (`info!`) instead of silently dropping them, and
  propagates the real `finish_reason` (e.g. `"length"` on truncation) instead of a
  hardcoded `"stop"` (P3-3, P3-7).
- `bench_speed` tool bails on a non-integer `tokens` arg instead of silently defaulting
  to 128 (P3-4).
- External process calls (`huggingface-cli download`, `mlx_lm.convert`) now run with
  timeouts (60min download / 30min convert) via `tokio::time::timeout`, preventing
  indefinite hangs on network stall or full disk (P3-5).
- `embed --dir` now recurses into subdirectories and enforces a 512KB total cap, bailing
  with a clear message instead of building an unbounded payload (P3-6).
- `memory` and `multinode` URL path segments (`id`, `node_id`, `model_name`) validated
  against `/` and `\` to prevent path injection (P3-8).

### Tests
- Unit test count: 69 → 71 (added `aggregate_sse` finish_reason propagation + default).

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
