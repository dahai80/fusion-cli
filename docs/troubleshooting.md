---
doc_id: fusion-cli/troubleshooting
product: fusion-cli
class: core
entry: service
doc_type: troubleshooting
title: { zh: "fusion-cli 故障排查", en: "fusion-cli troubleshooting" }
audience: engineer
status: draft
version: 0.4.6
updated: 2026-11-07
owner: @dahai
source_of_truth:
  - src/config/mod.rs
  - src/utils/audit.rs
  - src/utils/crypto.rs
  - src/rbac.rs
  - docs/OPS_RUNBOOK.md
---

# 故障排查 / Troubleshooting

zh：按症状排查。所有命令均可加 `-v` 获取详细日志；日志文件在 `~/.fusion/logs/fusion-cli.log`。

en: Symptom-first. Add `-v` to any command for verbose logs; the log file is `~/.fusion/logs/fusion-cli.log`.

## 1. 命令全部连接失败 / Every command fails to connect

- zh：症状：`fusion model list` 报连接错误。原因：下游服务未启动，或 `mlx.base_url` 指错。诊断：`fusion service status`、`curl http://localhost:11432`（gateway）。修复：`~/fusion/fusion-mlx/start.sh start` 或 `fusion service start`；`fusion net up` 交给 supervisor 批量拉起。
- en: Symptom: connection errors from `fusion model list`. Cause: downstream service down, or wrong `mlx.base_url`. Diagnose: `fusion service status`, `curl http://localhost:11432` against the gateway. Fix: `~/fusion/fusion-mlx/start.sh start` or `fusion service start`; `fusion net up` delegates batch startup to fusion-supervisor.

- zh：约束：CLI 无自动重试；`mlx.failover_urls` 只在主地址不可达时按序切换一次。
- en: Constraint: no automatic retries; `mlx.failover_urls` switches once, in order, when the primary is unreachable.

## 2. RBAC 拒绝（🚫 前缀）/ RBAC denial

- zh：症状：命令输出 `🚫` 且立即退出。原因：`auth.enabled=true` 而 `FUSION_API_KEY` 对应角色不足。诊断：`fusion config roles-list` 查角色表。修复：`fusion config roles-add <role> <key>` 或换用 admin key；mlx.api_key 隐式 admin，永远不会被拒。
- en: Symptom: `🚫` output and immediate exit. Cause: `auth.enabled=true` with an insufficient role for `FUSION_API_KEY`. Diagnose with `fusion config roles-list`. Fix: `fusion config roles-add <role> <key>` or use the admin key; `mlx.api_key` is implicitly admin and never denied.

- zh：审计留痕：被拒命令以 `denied` 写入 `~/.fusion/audit/audit.log`，`fusion audit view` 可回查。
- en: Audit trail: denied commands land as `denied` in the audit log; review with `fusion audit view`.

## 3. 配置损坏 / Corrupted config

- zh：症状：启动时警告 `config.toml parse failed, falling back to defaults`。原因：手编文件引入语法错误。修复：坏文件已自动备份为 `~/.fusion/config.toml.bak.<ts>`，从备份恢复或 `fusion config reset` 后重新 `set`。
- en: Symptom: `config.toml parse failed, falling back to defaults` at startup. Cause: syntax error from hand-editing. Fix: the broken file is auto-backed-up as `~/.fusion/config.toml.bak.<ts>`; restore from it or `fusion config reset` and re-apply with `config set`.

## 4. 密钥解密失败 / Key decryption failure

- zh：症状：`encrypt_secrets=true` 下加载配置报错。原因：`~/.fusion/master.key` 缺失或被替换（AES-256-GCM 认证标签校验失败）。诊断：`fusion doctor`。修复：从备份恢复 master.key；否则删除 `enc:` 前缀字段重新明文写入并重建（旧密文不可恢复，这是认证加密的预期行为）。
- en: Symptom: config load fails with `encrypt_secrets=true`. Cause: `~/.fusion/master.key` missing or replaced (AES-256-GCM auth-tag failure). Diagnose: `fusion doctor`. Fix: restore master.key from backup; otherwise rewrite the `enc:` fields in plaintext and re-enable (old ciphertext is unrecoverable — expected with authenticated encryption).

## 5. 密钥轮换后 401 / 401 after key rotation

- zh：症状：`fusion config rotate-key` 后请求被网关拒。原因：网关侧尚未录入新 key。修复：24h 内旧 key 在 `key_previous` 中保留供回滚；确认 fusion-gateway 双 key 并存配置，见其仓库文档。
- en: Symptom: gateway rejects requests after `fusion config rotate-key`. Cause: the gateway has not ingested the new key yet. Within 24h the old key remains in `key_previous` for rollback; verify the gateway's dual-key coexistence config in its own repo.

## 6. audit verify 报链断裂 / audit verify reports a broken chain

- zh：症状：`fusion audit verify` 失败。原因：审计文件被手工编辑或磁盘损坏。修复：审计日志只追加不修改；确认为外部篡改后，隔离文件并走事件流程（见 `docs/COMPLIANCE.md`）。不要在断链后继续追加——校验会持续失败。
- en: Symptom: `fusion audit verify` fails. Cause: the audit file was hand-edited or damaged on disk. Fix: the audit log is append-only; on confirmed external tampering, quarantine the file and follow the incident process (`docs/COMPLIANCE.md`). Do not keep appending after a break — verification keeps failing.

## 7. JSON 输出不生效 / JSON output not applied

- zh：症状：`--format json` 仍输出文本。原因：标志拼写错误或被子命令覆盖。诊断：`fusion run -m <model> -p "hi" --format json`；该值经 `FUSION_OUTPUT_FORMAT` 在运行时启动前写入，正常应立即生效。错误消息始终走 stderr，不属此问题。
- en: Symptom: text still printed with `--format json`. Cause: flag typo or subcommand override. Verify with `fusion run -m <model> -p "hi" --format json`; the value is applied via `FUSION_OUTPUT_FORMAT` before runtime startup. Error messages always go to stderr and are not affected.

## 8. 模型拉取被拒 / Model pull refused

- zh：症状：`fusion model pull` 报外网访问被禁。原因：`--offline` 默认开启。修复：`fusion model pull <name>` 需要外网时加 `--offline=false`，并建议 `HF_MIRROR=https://hf-mirror.com`。纯本地操作（list/delete/clean）无需关闭离线。
- en: Symptom: `fusion model pull` refuses external access. Cause: `--offline` is on by default. Fix: add `--offline=false` for external pulls and prefer `HF_MIRROR=https://hf-mirror.com`. Local-only operations (list/delete/clean) never need offline off.

## 9. doctor --prod 失败 / doctor --prod fails

- zh：症状：预检非零退出。原因：prod profile 缺必需配置 key。诊断：输出会列出缺失项；用 `fusion config set` 补齐后重跑。
- en: Symptom: non-zero exit from the prod preflight. Cause: missing required keys in the prod profile. The output lists them; fill each with `fusion config set` and rerun.

## 10. dashboard 无法启动 / dashboard does not start

- zh：症状：TUI 闪退或乱码。原因：无 TTY（管道/CI）或终端尺寸过小。修复：交互终端内运行；自动化场景改用 `fusion service status --format json`。
- en: Symptom: TUI exits at once or renders garbage. Cause: no TTY (pipe/CI) or a terminal window that is too small. Run interactively; for automation use `fusion service status --format json`.

## 11. metrics 为空 / metrics empty

- zh：症状：`fusion metrics view` 看不到数据。原因：`~/.fusion/metrics/metrics.json` 尚未生成（无命令执行过）或快照落盘失败（旁路，仅记 tracing）。诊断：`fusion metrics path` 确认路径后先跑任意命令再查。
- en: Symptom: no data in `fusion metrics view`. Cause: `~/.fusion/metrics/metrics.json` not yet written (no command ran) or the sideband flush failed (logged via tracing only). Confirm the path with `fusion metrics path`, run any command, then re-check.

## 12. 定时任务丢失 / Cron jobs missing

- zh：症状：desk 定时任务不执行。诊断：`fusion desk cron-list` 对照 `~/.fusion/cron.json`；删除用 `fusion desk cron-rm`。文件被删即任务丢失，无独立持久层——约束：请勿手工编辑该文件。
- en: Symptom: desk cron jobs never fire. Diagnose: compare `fusion desk cron-list` with `~/.fusion/cron.json`; remove entries with `fusion desk cron-rm`. Deleting the file loses the jobs; there is no separate persistence layer — do not hand-edit it.

## 升级排查 / Version mismatches

- zh：症状：`fusion --version` 与 CHANGELOG 不一致。原因：`target/release/fusion` 是旧构建。修复：`cargo build --release` 重新构建并替换 PATH 中的二进制。
- en: Symptom: `fusion --version` disagrees with CHANGELOG. Cause: a stale `target/release/fusion`. Fix: rebuild with `cargo build --release` and replace the binary on PATH.
