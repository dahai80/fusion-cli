<div align="center">
  <h1>⚡ Fusion-CLI</h1>
  <p><strong>One CLI, Control All Fusion-MLX Local AI Ecosystem.</strong></p>
  <p><em>A single binary to manage models, inference, knowledge bases, benchmarks, automation, and services — all powered by fusion-mlx.</em></p>

<p>
  <strong>English</strong> | <a href="README_CN.md">中文</a>
</p>
</div>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-orange" alt="Rust">
  <img src="https://img.shields.io/badge/macOS-Apple%20Silicon-brightgreen" alt="macOS">
  <img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License">
  <img src="https://img.shields.io/badge/Backend-fusion--mlx--only-important" alt="fusion-mlx">
  <img src="https://img.shields.io/badge/version-0.3.2-blue" alt="Version">
</p>

---

## 📋 Overview

**Fusion-CLI** is the unified command-line entry point for the entire Fusion-MLX local AI ecosystem. It provides a single binary to manage all aspects of your local AI setup:

- **Model Management** — List, pull, convert, quantize, delete MLX models
- **Inference** — Chat (SSE streaming), single-prompt, code analysis, embeddings (all via fusion-mlx)
- **Knowledge Base** — Create, ingest, query, manage Fusion-KB knowledge bases
- **RAG Service** — Start/stop Fusion-RAG, semantic search, knowledge base listing
- **Benchmarking** — Real speed tests, memory profiling, context stress testing, auto-optimization
- **Service Control** — Start/stop/restart all Fusion ecosystem services with real process management
- **Desktop Automation** — Run Fusion-Desk templates, view history, stop tasks
- **AI Agent** — Natural language with tool calling (sandbox/ask/auto permission tiers)
- **TUI Dashboard** — Interactive terminal dashboard with service monitoring, model list, system stats
- **Shell Completions** — Generate bash/zsh/fish/elvish/powershell completion scripts
- **Init** — One-command environment setup

### Core Design Philosophy

| Principle | Implementation |
|-----------|---------------|
| **One backend only** | 🔒 100% fusion-mlx — no omlx, no ollama, no OpenAI |
| **100% offline** | No telemetry, no phoning home, no cloud APIs |
| **macOS native** | Apple Silicon optimized, single binary, no Python/Node deps |
| **Scriptable** | Every command supports `--format=json` for programmatic output |
| **Ecosystem unified** | One CLI to rule them all: Desk, Code, KB, Bench, Model-Hub |
| **Configurable URLs** | All service URLs read from `~/.fusion/config.toml`, no hardcoding |
| **Connection pooling** | Global `reqwest::Client` with persistent connections across all commands |

---

## 🚀 Quick Start

### Installation

```bash
# Clone
git clone https://github.com/dahai80/fusion-cli.git
cd fusion-cli

# Build (single binary)
cargo build --release

# Install to PATH
cp target/release/fusion /usr/local/bin/

# Initialize environment
fusion init

# Verify
fusion --version
fusion doctor
```

### Basic Usage

```bash
# Check the environment
fusion doctor

# List available models
fusion model list

# Pull a model
fusion model pull llama3-8b-instruct-4bit

# Start a chat (SSE streaming by default)
fusion chat --model=llama3-8b-instruct-4bit

# Run benchmark (real inference)
fusion bench speed --model=llama3-8b-instruct-4bit --runs=3

# Use AI agent
fusion agent "list my models and benchmark the fastest one" --model=llama3-8b-instruct-4bit

# Launch TUI dashboard
fusion dashboard

# Watch service status (auto-refresh every 5s)
fusion service status --watch=5

# JSON output
fusion model list --format=json

# Shell completions
fusion completions zsh
```

---

## 📖 Command Reference

### Global Commands

| Command | Description |
|---------|-------------|
| `fusion version` | Show version info for all ecosystem components |
| `fusion doctor` | Full environment diagnostic (system, MLX, KB, ModelHub, RAG, Desk) |
| `fusion init` | Initialize Fusion environment (dirs, config, health check) |
| `fusion completions <shell>` | Generate shell completion scripts (bash/zsh/fish/elvish/powershell) |
| `fusion dashboard` | Launch interactive TUI dashboard |
| `fusion config list` | View all configuration |
| `fusion config set <key> <value>` | Set a configuration value |
| `fusion log` | View real-time logs |
| `fusion log clear` | Clear all logs |

**Global flags**: `--offline`, `--verbose`, `--mlx-ctx`, `--mlx-cache`, `--no-gpu`, `--format=json`

### TUI Dashboard (`fusion dashboard`)

Interactive terminal dashboard with real-time monitoring:

- **Services Tab** — Service status with latency, port, start/stop from dashboard
- **Models Tab** — Loaded MLX models list
- **System Tab** — CPU, memory, temperature, architecture
- **Logs Tab** — Recent log entries

| Key | Action |
|-----|--------|
| `1-4` | Switch tab |
| `↑/↓` or `j/k` | Navigate list |
| `r` | Force refresh |
| `s` | Start selected service |
| `x` | Stop selected service |
| `q` / `Esc` | Quit dashboard |

### Model Management (`fusion model`)

| Command | Description |
|---------|-------------|
| `list` | List all local MLX models |
| `pull <name> [--mirror=URL]` | Download model (ModelHub API or huggingface-cli fallback) |
| `info <name>` | Show model details (size, quantization, params) |
| `delete <name>` | Remove a model |
| `clean` | Clean model cache |
| `convert <source> --quant=4/8/fp16` | Convert third-party model to MLX via mlx_lm |
| `quant <name> --target=4bit` | Re-quantize an existing MLX model via mlx_lm |

### Inference (`fusion chat / run / code / embed`)

| Command | Description |
|---------|-------------|
| `chat --model=<name>` | Interactive terminal chat (SSE streaming) |
| `run --model=<name> --prompt="..."` | Single-prompt inference |
| `code --model=<name> --file=path --task=explain` | Code-specific analysis |
| `embed --text="..." / --dir=path` | Generate embeddings for Fusion-KB |

**Common parameters**: `--ctx`, `--temperature`, `--top-p`, `--no-cache`, `--quiet`, `--timeout`, `--no-stream`

### AI Agent (`fusion agent`)

| Command | Description |
|---------|-------------|
| `agent <prompt>` | Natural language with automatic tool calling |
| `agent <prompt> --model=<name>` | Use specific model |
| `agent <prompt> --permission=sandbox` | Read-only tools only |
| `agent <prompt> --permission=ask` | Ask before dangerous operations (default) |
| `agent <prompt> --permission=auto` | Full auto-pilot |

**Available tools**: `list_models`, `model_info`, `health`, `bench_speed`

### Knowledge Base (`fusion kb`)

| Command | Description |
|---------|-------------|
| `list` | List all knowledge bases |
| `create <name>` | Create a new knowledge base |
| `ingest <name> --path=<dir>` | Import documents into a knowledge base |
| `query <name> --question="..."` | Semantic search and QA |
| `stat <name>` | View knowledge base statistics |
| `clear <name>` | Clear all documents |
| `delete <name>` | Delete a knowledge base |

### RAG Service (`fusion rag`)

| Command | Description |
|---------|-------------|
| `start [--port=11436]` | Start Fusion-RAG service |
| `stop` | Stop Fusion-RAG service |
| `status` | View service status + embedding availability |
| `search <kb_id> --query="..."` | Semantic search via RAG |
| `list` | List all RAG knowledge bases |

### Benchmark (`fusion bench`)

| Command | Description |
|---------|-------------|
| `speed --model=<name> [--tokens=128] [--runs=1]` | Real token generation speed test |
| `mem --model=<name>` | Memory + MLX server stats |
| `ctx --model=<name> --max-ctx=8192 [--step=256]` | Context length stress test (real inference) |
| `auto --model=<name>` | Auto-parameter optimization (real benchmarks) |
| `report --model=<name> --output=report.md` | Export benchmark report with real data |

### Service Control (`fusion service`)

| Command | Description |
|---------|-------------|
| `status [--watch=N]` | View all service statuses (auto-refresh every N seconds) |
| `start [mlx/kb/modelhub/desk/rag/doc/all]` | Start one or all services |
| `stop [mlx/kb/modelhub/desk/rag/doc/all]` | Stop one or all services |
| `restart [mlx/kb/modelhub/desk/rag/doc/all]` | Restart one or all services |
| `log [name] --lines=50` | View service logs |

### Desktop Automation (`fusion desk`)

| Command | Description |
|---------|-------------|
| `list` | List automation templates (API or fallback) |
| `run <name>` | Execute an automation template |
| `history [--limit=20]` | View task execution history |
| `cron <name> --rule="0 21 * * *"` | Schedule a recurring task |
| `stop --task-id=<id>` | Stop a running task |

### Document Service (`fusion doc`)

| Command | Description |
|---------|-------------|
| `start [--port=11449]` | Start Fusion-Doc service |
| `stop` | Stop Fusion-Doc service |
| `status` | View Fusion-Doc service status |
| `log [--lines=50]` | View Fusion-Doc logs |

### Guard Status (`fusion guard`)

Read-only queries to the fusion-guard daemon over UDS (JSON-RPC 2.0, socket `/tmp/fusion-guard.sock`, override with `FUSION_GUARD_SOCK`). No authz decisions, no rule mutation.

| Command | Description |
|---------|-------------|
| `status` | Ping guard daemon — alive, version, rules epoch |
| `rules` | List current guard rule set + epoch |
| `audit [--limit=20]` | Show recent guard audit events |

### Service Orchestration (`fusion net`)

Thin forwarding to the fusion-supervisor daemon over UDS (JSON-RPC 2.0, socket `/tmp/fusion-sv.sock`, override with `FUSION_SV_SOCKET`). Optional token auth via `FUSION_SV_TOKEN` (forwarded in `params.token`). Daemon-down exits with code 3 and a hint to run `fusion-sv daemon`.

| Command | Description |
|---------|-------------|
| `up` | Start all supervised services |
| `down` | Stop all supervised services |
| `status` | List all services with state + port |
| `restart <service>` | Restart a named service |
| `ping` | Probe supervisor daemon (alive check) |

### Memory Service (`fusion memory`)

HTTP client to fusion-memory `fm-server` (port 11435, Bearer auth). Public endpoints (`healthz`, `version`) work without a key; set `fusion config set memory.api-key <key>` for retrieve/commit/delete/etc.

| Command | Description |
|---------|-------------|
| `status` | Service alive + API version |
| `version` | fm-server API version |
| `search <query> [--top-k=5]` | Semantic retrieve of memories |
| `count` | Total memory entries |
| `get <id>` | Fetch a single memory by ID |
| `commit <content> [--scope=...]` | Write a memory |
| `consolidate` | Trigger short→long consolidation |
| `delete <id>` | Delete a memory (confirm enforced) |
| `audit` | Show memory audit log |

### Eval Service (`fusion eval`)

HTTP client to fusion-bench service (port 11467, `/api/v1/*`). Distinct from `fusion bench speed/mem/ctx/auto` (local MLX self-benchmark): `fusion eval` queries the bench server for tasks, suites, results, baselines, and quality gates.

| Command | Description |
|---------|-------------|
| `status` | Service health |
| `resources` | CPU/GPU/memory resources |
| `tasks` | List bench tasks |
| `task <task_id>` | Task detail |
| `suites` | List bench suites |
| `result <task_id>` | Evaluation result |
| `trend` | Results trend |
| `baselines` | List baselines |
| `gates` | List quality gates |

### Cluster & Sync (`fusion cluster` / `fusion sync`)

Manage and sync with a fusion-multi-node Master (port 11452). Both now route to the Master directly (previously hit the MLX gateway by mistake → 404).

| Command | Description |
|---------|-------------|
| `cluster status` | Cluster status |
| `cluster nodes` | List registered nodes |
| `cluster node <id>` | Node detail |
| `cluster remove <id>` | Remove a node |
| `cluster pending` | List pending nodes |
| `cluster approve <id> [--approved-by]` | Approve a pending node |
| `cluster reject <id> [--reason]` | Reject a pending node |
| `cluster routing` | Routing summary |
| `sync manifest <model>` | Fetch a model manifest from Master |
| `sync incremental <model> [--source=...]` | Incremental model sync |

---

## 🔧 Architecture

```
src/
├── main.rs              # Entry point, CLI framework (clap)
├── cmd/                 # Command implementations
│   ├── mod.rs
│   ├── version.rs       # fusion version
│   ├── doctor.rs        # fusion doctor
│   ├── init.rs          # fusion init (V0.2)
│   ├── completions.rs   # fusion completions (V0.2)
│   ├── dashboard.rs     # fusion dashboard (V0.2.1)
│   ├── log.rs           # fusion log
│   ├── model.rs         # fusion model (real pull/convert/quant)
│   ├── chat.rs          # fusion chat/run/code/embed (SSE streaming)
│   ├── kb.rs            # fusion kb
│   ├── bench.rs         # fusion bench (real benchmarks)
│   ├── service.rs       # fusion service (watch mode V0.2.1)
│   ├── rag.rs           # fusion rag
│   ├── doc.rs           # fusion doc (start/stop/status/log)
│   ├── desk.rs          # fusion desk (real API calls)
│   ├── sync.rs          # fusion sync (model sync via multi-node Master)
│   ├── guard.rs         # fusion guard (UDS JSON-RPC status/rules/audit)
│   ├── net.rs           # fusion net (forward to fusion-supervisor UDS)
│   ├── memory.rs        # fusion memory (fm-server HTTP client)
│   ├── benchsvc.rs      # fusion eval (fusion-bench HTTP client)
│   └── cluster.rs       # fusion cluster (multi-node Master)
├── service/             # Unified service layer
│   ├── mod.rs           # Global reqwest::Client + ServiceUrls + check_url()
│   ├── mlx.rs           # MLX inference client (chat, stream, embed, bench, health)
│   ├── kb.rs            # Fusion-KB client (list, query, health)
│   ├── modelhub.rs      # Model-Hub client (list, search, download, health)
│   ├── rag.rs           # Fusion-RAG client (search, health, list KBs)
│   ├── desk.rs          # Fusion-Desk client (templates, tasks, history, stop, health)
│   ├── doc.rs           # Fusion-Doc client (health check, status detail)
│   ├── gateway.rs       # Gateway client (service discovery) (V0.2.1)
│   ├── guard.rs         # Guard UDS client (JSON-RPC ping, rule.list, audit.list)
│   ├── sv.rs            # Supervisor UDS client (JSON-RPC ping/status/up/down/restart)
│   ├── memory.rs        # fm-server client (retrieve/commit/delete/audit, Bearer)
│   ├── benchsvc.rs      # fusion-bench client (tasks/suites/results/gates/baselines)
│   ├── multinode.rs     # multi-node Master client (nodes/cluster/routing/sync)
│   └── health.rs        # Unified health check (check_all, check_all_with_latency)
├── tui/                 # TUI dashboard (V0.2.1)
│   ├── mod.rs           # Event loop + terminal setup
│   ├── app.rs           # App state machine (tabs, selection, data)
│   ├── ui.rs            # ratatui rendering (services, models, system, logs)
│   └── service_fetcher.rs # Background data fetching (health, models, system info)
├── agent/               # AI Agent engine (V0.2)
│   ├── mod.rs           # Agent loop with tool calling
│   ├── context.rs       # Context manager (message history, trimming)
│   ├── loop_engine.rs   # Loop statistics tracking
│   └── permission.rs    # Permission tiers (sandbox/ask/auto)
├── tools/               # Tool registry (V0.2)
│   └── mod.rs           # ToolExecutor + built-in tools
├── config/              # Global configuration management
│   └── mod.rs           # FusionConfig: TOML at ~/.fusion/config.toml
└── utils/               # Logging, utilities
    ├── mod.rs
    ├── logger.rs
    └── output.rs        # JSON output support (V0.2)
```

### Service URL Configuration

fusion-mlx inference routes through the gateway (`http://localhost:11432`, OpenAI-compatible `/v1/*` endpoints, requires `mlx.api_key`). All other ecosystem services connect to their direct local ports by default; the gateway currently proxies only the mlx inference API.

| Service | Default URL | Config Key |
|---------|-------------|------------|
| fusion-mlx | `http://localhost:11432` | `mlx.base_url` |
| fusion-mlx API key | `fg-admin-key` | `mlx.api_key` |
| Fusion-KB | `http://localhost:11434` | `kb.base_url` |
| Model-Hub | `http://localhost:11444` | `modelhub.base_url` |
| Fusion-RAG | `http://localhost:11436` | `rag.base_url` |
| Fusion-Desk | `http://localhost:9000` | `desk.base_url` |
| Fusion-Doc | `http://localhost:11449` | `doc.base_url` |
| Fusion-Memory | `http://localhost:11435` | `memory.base_url` |
| Fusion-Memory API key | _(no default)_ | `memory.api_key` |
| Fusion-Bench | `http://localhost:11467` | `bench.base_url` |
| Fusion-MultiNode | `http://localhost:11452` | `multinode.base_url` |
| Gateway | `http://localhost:11432` | `gateway.base_url` |

---

## 🛣️ Roadmap

### V0.1 (MVP) ✅
- [x] Global commands: version, doctor, config, log
- [x] Model management: list, pull, info, delete, clean, convert, quant
- [x] Inference: chat, run, code, embed (all via fusion-mlx)
- [x] Knowledge base: list, create, ingest, query, stat, clear, delete
- [x] Benchmark: speed, mem, ctx, auto, report
- [x] Service control: status, start, stop, restart, log
- [x] Desktop automation: list, run, history, cron, stop
- [x] Rust single binary, no runtime dependencies

### V0.2 ✅
- [x] Unified service layer (`src/service/`) replacing dead `ecosystem/`, `mlx_bind/`, `system/`
- [x] Global `reqwest::Client` connection pooling
- [x] Configurable service URLs from `config.toml`
- [x] Typed service APIs (mlx, kb, modelhub, rag, desk, health)
- [x] SSE streaming support (`chat_completion_stream`)
- [x] Real MLX start/stop via `~/claude-home/fusion-mlx/start.sh`
- [x] RAG service integration (start/stop/status/search/list)
- [x] Real bench benchmarks via MLX API (`generate_tokens`)
- [x] Real model pull (ModelHub API + huggingface-cli fallback)
- [x] Real model convert/quantize via `mlx_lm.convert`
- [x] Real desk operations (API calls, history, stop)
- [x] Agent mode (natural language → tool calling with permission tiers)
- [x] Tool registry (list_models, model_info, health, bench_speed)
- [x] `--format=json` global output mode
- [x] `fusion init` one-command setup
- [x] `fusion completions` shell completion generation
- [x] Context manager with message trimming
- [x] Loop statistics tracking

### V0.2.1 ✅
- [x] TUI dashboard (`fusion dashboard`) with ratatui — 4 tabs: Services/Models/System/Logs
- [x] Gateway integration (`src/service/gateway.rs`) — service discovery with fallback
- [x] Service health with latency detection (`check_all_with_latency`)
- [x] Watch mode (`fusion service status --watch=N`) — auto-refresh every N seconds
- [x] Dashboard service start/stop from TUI (s/x keys)

### V0.3 ✅
- [x] `fusion memory` — fusion-memory fm-server client (status/version/search/count/get/commit/consolidate/delete/audit)
- [x] `fusion eval` — fusion-bench HTTP service client (status/resources/tasks/suites/results/trend/baselines/gates)
- [x] `fusion cluster`/`fusion sync` rewritten to route to multi-node Master (11452), not the gateway
- [x] Health checks probe Memory/Bench/MultiNode
- [x] Config sections `[memory]`/`[bench]`/`[multinode]`

### V0.4 (Future)
- [ ] Distributed node management
- [ ] One-click ecosystem deployment
- [ ] Full CI/CD local AI pipeline
- [ ] Agent auto-repair loop
- [ ] MCP protocol support

---

## 📄 License

Apache License 2.0. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Fusion-CLI — One CLI, Control All Fusion-MLX Local AI Ecosystem.</strong>
</p>
<p align="center">
  <sub>Built with ❤️ and Rust 🦀</sub>
</p>
