# Backup & Disaster Recovery

> #3 enterprise gap: backup/DR is an ops responsibility, but the executable
> pieces live here so ops runs them directly. Companion to `scripts/backup.sh`.

## What to back up

`~/.fusion/` holds all persistent fusion-cli state:

| Path | Contents | Backup | Rationale |
|------|----------|--------|-----------|
| `config.toml` | service URLs, api keys | ✅ full | re-deploy needs it; keys non-recoverable |
| `kb/` | knowledge base data | ✅ full | user ingested content, expensive to rebuild |
| `rag/` | RAG indexes | ✅ full | derived but rebuild is slow |
| `audit/audit.log` | tamper-evident audit chain | ✅ full | compliance / forensic record |
| `metrics/metrics.json` | counters snapshot | ✅ full | observability history |
| `cron.json` | scheduled desk tasks | ✅ full | user scheduled jobs |
| `logs/` | runtime logs | ✅ separate | large, rotating — own archive |
| `run/` | PID files | ❌ exclude | rebuilt on restart |
| `models/` | model weights | ❌ exclude | re-pull from hub; huge; separate strategy |

## Backup procedure

`scripts/backup.sh` does a full tar of the above (excluding `run/`, `logs/`,
`models/`), verifies integrity, and rotates to keep N copies. Logs get a
separate compressed archive.

```bash
# Manual
./scripts/backup.sh 7

# Cron — nightly 02:00, keep 7 days
0 2 * * * /path/to/fusion-cli/scripts/backup.sh 7 >> /var/log/fusion-backup.cron 2>&1
```

Backups land in `~/.fusion-backup/`. For off-site, rsync that dir to remote
storage (S3/NAS) — out of this script's scope, ops wires the sync.

## Restore procedure

1. Stop fusion-mlx + any fusion CLI activity:
   ```bash
   ~/claude-home/fusion-mlx/start.sh stop
   ```
2. Move current state aside (don't delete — forensic value if restore is from
   a tamper event):
   ```bash
   mv ~/.fusion ~/.fusion.corrupt-$(date +%s)
   ```
3. Restore from backup:
   ```bash
   tar -xzf ~/.fusion-backup/fusion-backup-<TS>.tar.gz -C ~
   ```
4. Verify audit chain integrity post-restore:
   ```bash
   fusion audit verify   # must exit 0
   ```
5. Restart fusion-mlx, health check:
   ```bash
   ~/claude-home/fusion-mlx/start.sh start
   fusion doctor
   ```

## Recovery Time / Point Objectives (template — fill per deployment)

| Objective | Target | Notes |
|-----------|--------|-------|
| RPO (data loss tolerance) | 24h | nightly backup; reduce with more frequent cron for tighter RPO |
| RTO (recovery time) | 30min | tar restore + restart; model re-pull adds time if `models/` not backed up |
| Backup retention | 7 days | `KEEP` arg to backup.sh; extend for compliance |

## DR drill

Quarterly: pick a backup, restore to a clean host, run `fusion doctor` +
`fusion audit verify` + one `fusion chat` round-trip. Record pass/fail. A drill
that fails is a P1 — fix the backup before it's needed for real.

## Out of scope

- Off-site replication (ops wires rsync/S3 sync).
- `models/` backup strategy (re-pull is cheaper; large-file storage is a
  separate infra decision).
- Multi-node consistent snapshots (needs upstream HA — fusion-mlx#754).
