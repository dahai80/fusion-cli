<div align="center">
  <h1>⚡ Fusion-CLI</h1>
  <p><strong>One CLI, Control All Fusion-MLX Local AI Ecosystem.</strong></p>
  <p><em>A single binary to manage models, inference, knowledge bases, benchmarks, automation, and services — all powered by fusion-mlx.</em></p>
</div>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.80%2B-orange" alt="Rust">
  <img src="https://img.shields.io/badge/macOS-Apple%20Silicon-brightgreen" alt="macOS">
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="License">
  <img src="https://img.shields.io/badge/Backend-fusion--mlx--only-important" alt="fusion-mlx">
  <img src="https://img.shields.io/badge/status-beta-yellow" alt="Beta">
</p>

---

## 📋 Overview

**Fusion-CLI** is the unified command-line entry point for the entire Fusion-MLX local AI ecosystem. It provides a single binary to manage all aspects of your local AI setup:

- **Model Management** — List, pull, convert, quantize, delete MLX models
- **Inference** — Chat, single-prompt, code analysis, embeddings (all via fusion-mlx)
- **Knowledge Base** — Create, ingest, query, manage Fusion-KB knowledge bases
- **Benchmarking** — Speed tests, memory profiling, context stress testing, auto-optimization
- **Service Control** — Start/stop/restart all Fusion ecosystem services
- **Desktop Automation** — Run Fusion-Desk templates from the terminal
- **Configuration** — Global settings, environment diagnostics, log management

### Core Design Philosophy

| Principle | Implementation |
|-----------|---------------|
| **One backend only** | 🔒 100% fusion-mlx — no omlx, no ollama, no OpenAI |
| **100% offline** | No telemetry, no phoning home, no cloud APIs |
| **macOS native** | Apple Silicon optimized, single binary, no Python/Node deps |
| **Scriptable** | Every command supports `--quiet` for shell scripts and cron jobs |
| **Ecosystem unified** | One CLI to rule them all: Desk, Code, KB, Bench, Model-Hub |

### Ecosystem Position

```
fusion-mlx (inference engine, Metal, KV Cache, quantization)
        ↓
Model-Hub / KB / Bench (data & evaluation layer)
        ↓
Fusion-CLI (UNIFIED COMMAND ENTRY POINT)
        ↓
Desk / Code / Doc / Agent-Studio (application layer)
```

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

# Start a chat
fusion chat --model=llama3-8b-instruct-4bit

# Run benchmark
fusion bench speed --model=llama3-8b-instruct-4bit

# Start all services
fusion service start all

# Check service status
fusion service status
```

---

## 📖 Command Reference

### Global Commands

| Command | Description |
|---------|-------------|
| `fusion version` | Show version info for all ecosystem components |
| `fusion doctor` | Full environment diagnostic (system, MLX, dependencies) |
| `fusion config list` | View all configuration |
| `fusion config set <key> <value>` | Set a configuration value |
| `fusion log` | View real-time logs |
| `fusion log clear` | Clear all logs |

### Model Management (`fusion model`)

| Command | Description |
|---------|-------------|
| `list` | List all local MLX models |
| `pull <name>` | Download a model from Model-Hub |
| `info <name>` | Show model details (size, quantization, params) |
| `delete <name>` | Remove a model |
| `clean` | Clean model cache |
| `convert <path> --quant=4/8/fp16` | Convert third-party model to MLX |
| `quant <name> --target=4bit` | Re-quantize an existing MLX model |

### Inference (`fusion chat / run / code / embed`)

| Command | Description |
|---------|-------------|
| `chat --model=<name>` | Interactive terminal chat |
| `run --model=<name> --prompt="..."` | Single-prompt inference |
| `code --model=<name> --file=path --task=explain` | Code-specific analysis |
| `embed --text="..." / --dir=path` | Generate embeddings for Fusion-KB |

**Common parameters**: `--ctx`, `--temperature`, `--top-p`, `--no-cache`, `--quiet`, `--timeout`

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

### Benchmark (`fusion bench`)

| Command | Description |
|---------|-------------|
| `speed --model=<name>` | Token generation speed test |
| `mem --model=<name>` | Memory usage profiling |
| `ctx --model=<name> --max-ctx=8192` | Context length stress test |
| `auto --model=<name>` | Auto-parameter optimization |
| `report --model=<name> --output=report.md` | Export benchmark report |

### Service Control (`fusion service`)

| Command | Description |
|---------|-------------|
| `status` | View all service statuses |
| `start [mlx/kb/modelhub/desk/all]` | Start one or all services |
| `stop [mlx/kb/modelhub/desk/all]` | Stop one or all services |
| `restart [mlx/kb/modelhub/desk/all]` | Restart one or all services |
| `log [name] --lines=50` | View service logs |

### Desktop Automation (`fusion desk`)

| Command | Description |
|---------|-------------|
| `list` | List all automation templates |
| `run <name>` | Execute an automation template |
| `history` | View task execution history |
| `cron <name> --rule="0 21 * * *"` | Schedule a recurring task |
| `stop [task-id]` | Stop a running task |

---

## 🔧 Architecture

```
src/
├── main.rs              # Entry point, CLI framework (clap)
├── cmd/                 # Command implementations
│   ├── mod.rs
│   ├── version.rs       # fusion version
│   ├── doctor.rs        # fusion doctor
│   ├── log.rs           # fusion log
│   ├── model.rs         # fusion model (list/pull/info/delete/clean/convert/quant)
│   ├── chat.rs          # fusion chat/run/code/embed
│   ├── kb.rs            # fusion kb
│   ├── bench.rs         # fusion bench
│   ├── service.rs       # fusion service
│   └── desk.rs          # fusion desk
├── config/              # Global configuration management
│   └── mod.rs
├── mlx_bind/            # fusion-mlx binding layer (core)
│   └── mod.rs           # HTTP client to fusion-mlx API
├── ecosystem/           # Model-Hub / KB / Desk integration
│   └── mod.rs
├── system/              # Process, system info, scheduling
│   └── mod.rs
└── utils/               # Logging, utilities
    └── mod.rs
    └── logger.rs
```

### Binding Layer: fusion-mlx Only

The `mlx_bind` module is the **only** inference interface. It communicates exclusively with fusion-mlx via HTTP (OpenAI-compatible API at `localhost:11434`). The codebase has a compile-time assertion that prevents any other backend from being used.

```rust
// mlx_bind/mod.rs — hard-coded to fusion-mlx only
const MLX_BASE_URL: &str = "http://localhost:11434/v1";

pub fn assert_fusion_mlx_only() -> Result<()> {
    assert!(
        MLX_BASE_URL.contains("localhost:11434"),
        "Fusion-CLI only supports fusion-mlx"
    );
    Ok(())
}
```

---

## 🔒 Security

- **100% Offline** — Zero network requests to external services
- **No Telemetry** — No analytics, no phoning home, no update checks
- **Local Only** — All models, data, and vectors stay on your machine
- **No Third-Party Backends** — Hard-coded to fusion-mlx only

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

### V0.2 (Planned)
- [ ] Batch task pipeline
- [ ] Script template marketplace
- [ ] Multi-model comparison benchmark
- [ ] Knowledge base migration / backup

### V0.3 (Future)
- [ ] Distributed node management
- [ ] One-click ecosystem deployment
- [ ] Full CI/CD local AI pipeline

---

## 📄 License

MIT License. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <strong>Fusion-CLI — One CLI, Control All Fusion-MLX Local AI Ecosystem.</strong>
</p>
<p align="center">
  <sub>Built with ❤️ and Rust 🦀</sub>
</p>