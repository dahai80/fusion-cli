# Operations Runbook

> #6 enterprise gap: deployment topology + ops SOP. This is the on-call
> operator's reference for deploying, operating, and troubleshooting
> fusion-cli in production.

## Deployment Topology

### Single-node (small team / single-tenant)

```
┌─────────────────────────────────────────────┐
│  Host (Apple Silicon Mac)                    │
│                                             │
│  fusion-cli ──HTTP──> fusion-mlx :11434     │
│     │                   (inference engine)   │
│     ├──> fusion-kb :11434 (direct)          │
│     ├──> fusion-gateway :11432 (MLX route)  │
│     ├──> fusion-memory :11435               │
│     └──> ecosystem services (direct ports)  │
│                                             │
│  ~/.fusion/  config, kb, rag, audit, metrics│
└─────────────────────────────────────────────┘
```

Deploy steps:
1. Install binary from GitHub Release (see RELEASE_STRATEGY.md channel).
2. `fusion init` — generates config + random api_key, chmod 0600.
3. `~/claude-home/fusion-mlx/start.sh start` — start inference engine.
4. `fusion doctor` — verify all services healthy.
5. Load a model: `fusion model pull <model>` then load via mlx.
6. Smoke: `fusion chat -m <model> -p "hi"`.

### Multi-node (enterprise — pending upstream HA)

Multi-node HA requires fusion-mlx#754 (model replicas) + fusion-gateway#150
(load-balance + tenant isolation). Topology target:

```
                ┌──────────────┐
  clients ──--> │  gateway     │ ── tenant scoping + LB
                │  :11432      │
                └───┬───┬───┬──┘
                    │   │   │
            ┌───────┘   │   └───────┐
            ▼           ▼           ▼
       mlx-node-1   mlx-node-2   mlx-node-N   (same model loaded)
```

**Status**: topology documented; deployment BLOCKED on #754 + #150. Do not
deploy multi-node until both resolve — CLI's `pick_alive_mlx_base` failover
has nothing to fail over to in a single-instance setup.

## Daily Operations

### Health check
```bash
fusion doctor                          # all services + config validation
fusion service status                  # per-service status + latency
fusion audit verify                    # audit chain integrity (exit 0 = ok)
```

### Start / stop inference
```bash
~/claude-home/fusion-mlx/start.sh start
~/claude-home/fusion-mlx/start.sh stop
~/claude-home/fusion-mlx/start.sh status   # PID, port, mem, loaded models
```

### Backups
```bash
./scripts/backup.sh 7                  # nightly via cron, keep 7
```
See `BACKUP_DR.md` for restore.

### Metrics / monitoring
```bash
fusion metrics view                    # human snapshot
fusion metrics export                  # Prometheus exposition (cron to .prom)
```
Monitoring stack: `deploy/prometheus-alertmanager/` (see `ALERTING.md`).

## Troubleshooting (SOP)

### Symptom: `fusion doctor` reports unhealthy service

1. `fusion service status` — identify which service down.
2. Check that service's port: `lsof -i :<port>` or `curl localhost:<port>/health`.
3. Restart: `fusion service restart <service>` (polls health until exit).
4. If still down: check logs `fusion service log <service>` + `~/.fusion/logs/`.
5. If MLX down: `~/claude-home/fusion-mlx/start.sh doctor` then `start`.

### Symptom: high error rate alert (FusionCliHighErrorRate)

1. Check backend: `fusion doctor` + `~/claude-home/fusion-mlx/start.sh status`.
2. Check circuit breaker: if `FusionCliBreakerLikelyOpen` also firing → backend
   is down. Restart MLX.
3. Check audit log for error pattern: `fusion audit view | grep error`.
4. If backend healthy but errors persist → check `~/.fusion/logs/fusion-cli.log`
   for `json_or_error` surfaces.

### Symptom: P95 latency alert (FusionCliHighLatencyP95)

1. Check MLX load: `~/claude-home/fusion-mlx/start.sh status` (memory, models).
2. Check if model OOM-ing / swapping — reduce context or unload other models.
3. Run bench: `fusion bench speed -m <model>` — compare to baseline.
4. If saturated → scale out (needs #754 multi-instance) or throttle input.

### Symptom: audit chain broken alert (FusionCliAuditChainBroken)

**P0 — potential tamper.**
1. `fusion audit view` — find break point (first record failing verify).
2. Do NOT restore over it — preserve evidence: `mv ~/.fusion/audit ~/.fusion/audit.tamper-$(date +%s)`.
3. Investigate who/what edited audit.log (file mtime, access logs).
4. Restore audit from backup AFTER forensics: see `BACKUP_DR.md`.
5. Escalate to security lead.

### Symptom: `fusion chat` hangs / times out

1. Verify MLX alive: `~/claude-home/fusion-mlx/start.sh status`.
2. Verify model loaded: `fusion model list`.
3. Check gateway (if enabled): `curl -H "Authorization: Bearer <key>" http://localhost:11432/v1/models`.
4. Check backpressure breaker state in logs — if Open, backend was down;
   breaker recovers after cooldown.

## On-call escalation

| Severity | Response | Channel |
|----------|----------|---------|
| P0 (audit tamper, full outage) | immediate, 24/7 | PagerDuty (alertmanager critical-pager) |
| P1 (single service down, SLO breach) | < 1h business | Slack #fusion-alerts |
| P2 (degraded, non-critical) | next business day | Slack #fusion-alerts |

## Maintenance windows

- Model updates: `fusion model pull` + reload — brief inference interruption,
  schedule off-peak.
- CLI upgrades: swap binary + `fusion doctor` — no state migration unless
  `CURRENT_CONFIG_VERSION` changed (auto-migrates via `migrate_config`).
- Backup: nightly cron, no service interruption.

## Pre-production checklist

- [ ] `fusion init` run, config 0600, api_key random
- [ ] `fusion doctor` green
- [ ] Model loaded + `fusion chat` round-trip ok
- [ ] `fusion audit verify` exit 0
- [ ] Backup cron scheduled (`scripts/backup.sh`)
- [ ] Metrics export cron scheduled (`fusion metrics export > .prom`)
- [ ] Prometheus + Alertmanager deployed (`deploy/prometheus-alertmanager/`)
- [ ] Alert channels wired (alertmanager.yml — PagerDuty/Slack)
- [ ] Load test passed (`scripts/load_test.sh`, SLO met)
- [ ] License + security review signed (`COMPLIANCE.md`)
- [ ] SLA agreed with customer (`ENTERPRISE_SLA.md`)
- [ ] DR drill passed (restore from backup, `BACKUP_DR.md`)
