---
doc_id: fusion-cli/config-reference
product: fusion-cli
class: core
entry: service
doc_type: config-reference
title: { zh: "fusion-cli 配置参考", en: "fusion-cli config reference" }
audience: engineer
status: draft
version: 0.4.6
updated: 2026-11-07
owner: "@dahai"
source_of_truth:
  - src/config/mod.rs
  - src/utils/audit.rs
  - src/utils/metrics.rs
  - src/utils/logger.rs
  - src/utils/crypto.rs
---

# 配置参考 / Config Reference

zh：唯一配置文件是 `~/.fusion/config.toml`，权限收敛为 0600。所有键都有代码内默认值（`src/config/mod.rs`），缺段不报错。解析失败时坏文件备份为 `config.toml.bak.<ts>` 并回落默认值。配置版本字段 `config_version` 当前为 `0.4.2`。

en: The single config file is `~/.fusion/config.toml`, permissions tightened to 0600. Every key has an in-code default (`src/config/mod.rs`); missing sections are tolerated. On parse failure the broken file is backed up as `config.toml.bak.<ts>` and defaults apply. The `config_version` field is currently `0.4.2`.

## `[model]`

| Key | Default | 说明 / Meaning |
|---|---|---|
| `default_path` | `~/.fusion/models` | 默认模型目录 / Default model directory |

## `[mlx]`

| Key | Default | 说明 / Meaning |
|---|---|---|
| `base_url` | `http://localhost:11432` | fusion-gateway 地址（勿直连 :11434）/ gateway URL; do not hit :11434 directly |
| `api_key` | 源码内开发默认值，见 `default_mlx_api_key()` / dev default in source | 建议 `fusion config rotate-key` 轮换 / rotate it |
| `default_ctx` | 4096 | 默认上下文长度 / Default context length |
| `enable_cache` | true | KV Cache / KV cache |
| `cache_size` | `4GB` | 缓存容量 / Cache size |
| `max_batch_size` | 8 | 批大小上限 / Batch cap |
| `failover_urls` | `[]` | HA 备用网关列表，按序尝试 / Ordered failover gateway list |
| `key_rotated_at` | `""` | 上次轮换时间（RFC3339）；doctor 对超过 90 天告警 / Last rotation; doctor warns past 90 days |
| `key_previous` | `""` | 旧 key，24h 宽限回滚用 / Previous key, 24h grace rollback |

## 生态服务端点 / Service endpoints

| Section | Key | Default |
|---|---|---|
| `[kb]` | `default_path` | `~/.fusion/kb` |
| `[kb]` | `base_url` | `http://localhost:11434` |
| `[modelhub]` | `base_url` | `http://localhost:11444` |
| `[rag]` | `base_url` + `api_key` | `http://localhost:11436` |
| `[desk]` | `base_url` | `http://localhost:9000` |
| `[doc]` | `base_url` | `http://localhost:11449` |
| `[memory]` | `base_url` + `api_key` | `http://localhost:11435` |
| `[bench]` | `base_url` | `http://localhost:11467` |
| `[multinode]` | `base_url` + `api_key` | `http://localhost:11452` |

zh：端口以仓库根 `architecture/port-registry.yaml` 为准；`[kb]` 的 11434 指向 fusion-mlx 嵌入端点。

en: Ports are governed by the repo-root `architecture/port-registry.yaml`; `[kb]` on 11434 points at the fusion-mlx embedding endpoint.

## `[auth]`（RBAC + 加密）

| Key | Default | 说明 / Meaning |
|---|---|---|
| `enabled` | false | true 时按 `FUSION_API_KEY` 查 `keys` 表做命令级门禁 / Gate commands via `FUSION_API_KEY` |
| `keys` | `[]` | `[{key, role}]`，role ∈ reader/operator/admin / Role-key table |
| `encrypt_secrets` | false | true 时 4 个 api_key 以 AES-256-GCM 加密落盘（`enc:` 前缀）/ Encrypt api_key fields at rest |

zh：`mlx.api_key` 隐式 admin，owner 永不锁死。密钥管理命令：`fusion config roles-add/roles-remove/roles-enable`、`fusion config rotate-key`。

en: `mlx.api_key` is implicitly admin, so the owner is never locked out. Key management: `fusion config roles-add/roles-remove/roles-enable` and `fusion config rotate-key`.

## `[log]` / `[backpressure]`

| Key | Default | 说明 / Meaning |
|---|---|---|
| `[log] level` | `info` | 日志级别 / Log level |
| `[backpressure]` | 缺省 None（用代码默认） | 限流熔断配置；`src/utils/backpressure.rs` / Rate-limit and circuit-breaker config |

## 数据文件位置 / Data file locations

| Path | 内容 / Content |
|---|---|
| `~/.fusion/config.toml` | 配置（0600）/ Config |
| `~/.fusion/master.key` | AES-256-GCM 主密钥（32B，0600）/ Master key for encryption |
| `~/.fusion/audit/audit.log` | 审计日志（SHA-256 hash chain）/ Audit log with hash chain |
| `~/.fusion/metrics/metrics.json` | metrics 快照（累计值，覆盖写）/ Cumulative metrics snapshot |
| `~/.fusion/logs/fusion-cli.log` | CLI 日志 / CLI log file |
| `~/.fusion/cron.json` | desk 定时任务持久化 / Persisted desk cron jobs |
| `~/.fusion/models` | 本地模型 / Local models |

## 环境变量 / Environment variables

| Var | 设置方 / Set by | 说明 / Meaning |
|---|---|---|
| `FUSION_API_KEY` | 用户 / user | RBAC 门禁查角色用 / Looked up by the RBAC gate |
| `FUSION_WORKSPACE_ROOT` | 用户 / user | 覆盖 workspace 根（默认 `~/fusion`）/ Override workspace root |
| `FUSION_OFFLINE` | CLI 内部 / internal | 由 `--offline` 推导 / Derived from `--offline` |
| `FUSION_OUTPUT_FORMAT` | CLI 内部 / internal | 由 `--format json` 推导 / Derived from `--format json` |

## 修改方式 / How to change values

zh：推荐用命令而非手编文件：`fusion config list` / `get <KEY>` / `set <KEY> <VALUE>` / `reset`。手编后保存时 CLI 会自动收敛文件权限并（若 `encrypt_secrets=true`）重新加密密钥字段。

en: Prefer commands over hand-editing: `fusion config list` / `get <KEY>` / `set <KEY> <VALUE>` / `reset`. After a hand edit, saving via the CLI re-tightens permissions and (if `encrypt_secrets=true`) re-encrypts key fields.

## 约束 / Constraints

- zh：加密只覆盖 `mlx/rag/memory/multinode` 四个 `api_key` 字段；其余段永远明文。
- en: Encryption covers only the `mlx/rag/memory/multinode` `api_key` fields; every other section stays plaintext.
- zh：`key_previous` 回滚窗口固定 24h，网关侧需双 key 并存配合（属上游 fusion-gateway 职责）。
- en: The `key_previous` rollback window is fixed at 24h; dual-key coexistence on the gateway side is an upstream fusion-gateway responsibility.
