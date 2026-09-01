# Compliance & Legal Review Checklist

> #5 enterprise gap: `cargo deny` covers license/audit/bans at build time, but
> enterprise procurement needs a documented compliance posture. This is the
> checklist + evidence bundle for legal/security review.

## License posture

`cargo deny check licenses` enforces a whitelist at CI time (deny.toml). The
runtime license inventory:

- **fusion-cli**: dual-licensed — see `LICENSE` at repo root.
- **All Rust dependencies**: must pass `cargo deny check licenses` (CI gate,
  green on v0.4.1). License types accepted: MIT, Apache-2.0, BSD-2/3-Clause,
  ISC, Unicode-DFS. No GPL/AGPL/copyleft (blocked by deny.toml).

**Evidence for legal**: run + attach:
```bash
cargo deny check licenses --format json > license-report.json
```

## Security audit

### Automated (CI, every push)
- `cargo audit` via `cargo deny check advisories` — RustSec vulnerability DB.
  Green on v0.4.1. Known accepted risk: `proc-macro-error2` future-incompat
  (transitive, documented in deny.toml, RUSTSEC-2026-0173 accepted).
- `cargo clippy --all-targets -- -D warnings` — lint, warnings = errors.
- `cargo deny check bans sources` — banned crates + source-origin enforcement.

### Manual review checklist (security team sign-off)

- [ ] **Secrets**: `fusion init` generates random `/dev/urandom` api_key,
      config chmod 0600. No hardcoded secrets in source (grep verified).
- [ ] **Path traversal**: all user-controlled URL segments pass
      `validate_path_segment` (kb/rag/desk/modelhub/bench). Verified by tests.
- [ ] **Audit chain**: `audit.log` SHA-256 hash chain, `fusion audit verify`
      detects tamper. Append-only.
- [ ] **Auth**: gateway requires `Authorization: Bearer <api_key>` for MLX;
      CLI RBAC (#46) gates at entry. (Cross-tenant isolation pending
      fusion-gateway#150 — flag to legal as in-progress.)
- [ ] **Error visibility**: no bare `unwrap_or_default` swallowing errors;
      `json_or_error` surfaces backend errors. No silent failures.
- [ ] **Child process safety**: external commands (huggingface-cli, mlx_lm)
      use `kill_on_drop(true)` — timeouts actually kill, no orphans.
- [ ] **Dependencies pinned**: `Cargo.lock` committed + staged in releases
      (#53) — reproducible builds, no supply-chain drift.

**Evidence bundle** (attach to security review):
```bash
cargo deny check advisories bans licenses sources --format json > deny-report.json
cargo audit > cargo-audit.txt 2>&1 || true
cargo tree > dependency-tree.txt
```

## Data handling

- **Local-first**: fusion-cli is local-only. All inference, KB, RAG, memory,
  audit data stays on-host (`~/.fusion/`). No telemetry, no cloud calls.
  HTTP only to `127.0.0.1` ecosystem services.
- **PII**: KB/RAG may contain user-ingested PII. Backup (`scripts/backup.sh`)
  should target encrypted storage in prod — ops responsibility.
- **Audit log**: records `actor/command/outcome/duration/detail` with
  credential auto-redaction. Retention per `BACKUP_DR.md`.

## Compliance gaps (transparent — flag to legal)

| Gap | Status | Mitigation |
|-----|--------|------------|
| Cross-tenant isolation | fusion-gateway#150 open | CLI RBAC gates identity; backend scoping pending upstream. Do NOT deploy multi-tenant until #150 resolved. |
| Server-side HA | fusion-mlx#754 open | Single-instance = no HA. Acceptable for single-tenant; P1 for multi-tenant. |
| Formal pen-test | not done | Recommend external pen-test before enterprise contract sign. |

## Sign-off

| Reviewer | Item | Date | Pass |
|----------|------|------|------|
| Legal | License posture | _____ | ☐ |
| Security | Audit checklist | _____ | ☐ |
| Security | Pen-test | _____ | ☐ |
| Eng lead | Compliance gaps accepted | _____ | ☐ |
