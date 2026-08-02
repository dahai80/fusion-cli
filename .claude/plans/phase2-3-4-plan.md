# Phase 2/3/4 Implementation Plan — Fusion-CLI V0.2

## Current State (after Phase 1)

- `src/service/` — 6 modules (mlx, kb, modelhub, rag, desk, health) with typed APIs
- `src/cmd/` — 13 command modules, most using service layer; bench/desk/model still have simulated operations
- `src/config/` — FusionConfig with 7 sections, 12 config keys
- SSE streaming stub exists (`chat_completion_stream`) but not consumed by cmd/chat
- `cargo build/clippy/test` all green
- CI workflow + Makefile in place

## Phase 2: SSE Streaming + Real Bench + Real Model Operations

**Goal**: Replace all simulated operations with real API calls; add SSE streaming to chat.

### 2.1 SSE Streaming for chat/run/code (P0)

**Files**: `src/service/mlx.rs`, `src/cmd/chat.rs`, `Cargo.toml`

- Add `eventsource-stream = "0.2"` to Cargo.toml (futures-util already present)
- Implement `mlx::chat_completion_stream()` consumer in `cmd/chat.rs`:
  - Parse SSE events, extract `delta.content`, print token-by-token
  - Add `--stream` / `--no-stream` flag (default: stream for chat, no-stream for run)
  - Fallback to non-streaming if SSE fails
- `handle_run()`: Add `--stream` flag support
- `handle_code()`: Add `--stream` flag support

### 2.2 Real Bench Implementation (P1)

**Files**: `src/service/mlx.rs`, `src/cmd/bench.rs`

- Add `mlx::generate_tokens()` — calls `/v1/completions` with `max_tokens` and times the response
- `bench_speed()`: Call `mlx::generate_tokens()` instead of simulated loop; compute real tok/s
- `bench_mem()`: Call `mlx::get_server_stats()` (already exists, currently dead_code) for real memory data
- `bench_ctx()`: Call `mlx::generate_tokens()` with increasing prompt lengths until failure
- `bench_auto()`: Run speed test with different ctx values, pick best
- `bench_report()`: Collect real data from speed/mem/ctx, generate report

### 2.3 Real Model Pull (P1)

**Files**: `src/service/modelhub.rs`, `src/cmd/model.rs`

- Add `modelhub::download_model()` — streams download with progress callback
- `pull_model()`: Call modelhub API to get download URL, stream-download with indicatif progress bar
- `convert_model()`: Shell out to `python -m mlx_lm.convert` via fusion-mlx
- `quantize_model()`: Shell out to `python -m mlx_lm.convert --quantize` via fusion-mlx

### 2.4 Real Desk Operations (P2)

**Files**: `src/service/desk.rs`, `src/cmd/desk.rs`

- Add `desk::list_templates()`, `desk::get_history()`, `desk::stop_task()`
- `desk_list()`: Call `desk::list_templates()` instead of hardcoded array
- `desk_run()`: Call `desk::run_task()` (already exists, currently dead_code)
- `desk_history()`: Call `desk::get_history()` instead of hardcoded array
- `desk_stop()`: Call `desk::stop_task()`

### 2.5 `--format=json` Output Mode (P2)

**Files**: `src/main.rs`, `src/cmd/*.rs`

- Add global `--format <text|json|quiet>` arg to Cli struct
- Pass format flag through to each command handler
- Implement `JsonOutput` helper for structured JSON output
- Apply to: model list, model info, service status, bench results

## Phase 3: Agent Core + Tools + Context

**Goal**: Add Agent mode with tool-calling loop.

### 3.1 Tool Registry (`src/tools/`)

**New files**: `src/tools/mod.rs`, `src/tools/registry.rs`, `src/tools/builtin.rs`

- `ToolDef` struct: name, description, parameter schema (JSON Schema), tier, handler
- `ToolRegistry`: HashMap<String, ToolDef>, register/lookup/execute
- Builtin tools (20+):
  - `service_health`, `service_start`, `service_stop`
  - `model_list`, `model_info`, `model_pull`
  - `kb_list`, `kb_query`, `kb_ingest`
  - `rag_search`, `rag_list`
  - `bench_speed`, `bench_mem`
  - `desk_list`, `desk_run`
  - `config_get`, `config_set`
  - `fs_read`, `fs_glob`, `fs_grep`, `shell_exec`

### 3.2 Agent Loop (`src/agent/`)

**New files**: `src/agent/mod.rs`, `src/agent/context.rs`, `src/agent/planner.rs`

- `AgentRunner`: model, context, tools, max_iterations
- Standard loop: user input → LLM call → parse tool_calls → execute → feed result → repeat
- Tool call format: OpenAI-compatible function calling
- `handle_agent()`: REPL loop with prompt, dispatch to AgentRunner
- `handle_plan()`: Single LLM call that outputs a plan without executing

### 3.3 Context Manager (`src/agent/context.rs`)

- Load `FUSION.md` from project root or `~/.fusion/FUSION.md`
- Load `.fusion/rules/*.md` if present
- Build system message from: tool schemas + project context + service status
- Context compression: truncate old messages when approaching token limit

### 3.4 Permission Model

- Tier1 (auto-approve): read operations — health checks, list, info, get
- Tier2 (confirm): write operations — pull, delete, start, stop, set, ingest, run
- Tier3 (force-confirm): destructive — clean, reset, stop all
- `check_permission()` in agent loop: prompt user for Tier2/3

### 3.5 Config Enhancements

**File**: `src/config/mod.rs`

- Add `AgentConfig` section: default_model, max_iterations, auto_approve_tier1, stream_output
- Add `GatewayConfig` section: enabled, base_url
- Add `fusion.mode` key: cmd | agent | plan

## Phase 4: TUI Dashboard + Gateway + Polish

**Goal**: Full TUI dashboard, gateway integration, shell completions.

### 4.1 TUI Dashboard (`fusion dashboard`)

**New files**: `src/cmd/dashboard.rs`, `src/render/`

- Add `ratatui = "0.29"` + `crossterm = "0.28"` to Cargo.toml
- Service status panel with auto-refresh
- Resource monitor (CPU, memory, model slots)
- Log tail panel
- Key bindings: q=quit, r=refresh, s=start, x=stop

### 4.2 `fusion init` — Project Context Generator

**New file**: `src/cmd/init.rs`

- Scan project directory: detect language, build system, test framework
- Generate `FUSION.md` with project structure, build commands, key paths
- Interactive: save/edit/cancel

### 4.3 Gateway Integration

**New file**: `src/service/gateway.rs`

- `gateway::discover()` — GET `http://localhost:11432/services` for service registry
- Fallback: if gateway unavailable, use config.toml URLs
- `ServiceUrls::resolve()` — try gateway first, fallback to config

### 4.4 Shell Completions

**File**: `src/main.rs`, `Cargo.toml`

- Enable `clap_complete` dependency
- `fusion completions <shell>` subcommand generating zsh/bash/fish completions

### 4.5 Watch Mode + Diff Review (P2)

- `fusion service status --watch --interval=5s`: periodic refresh
- Agent diff review: show proposed file changes, accept/reject

## Execution Order

1. **Phase 2** (streaming + real ops) — immediate, ~8 tasks
2. **Phase 3** (agent core) — after Phase 2, ~6 tasks
3. **Phase 4** (TUI + gateway) — after Phase 3, ~5 tasks

## Version Milestones

- V0.2.0: After Phase 2 complete (SSE + real bench/model/desk)
- V0.3.0: After Phase 3 complete (Agent mode + tools)
- V0.4.0: After Phase 4 complete (TUI + gateway + completions)
