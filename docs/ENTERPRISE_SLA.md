# Enterprise SLA & Support Process (Template)

> #4 enterprise gap: SLA is a commercial contract between vendor and customer,
> not code. This is a **template** — fill the bracketed values per customer
> agreement. Engineering signs the technical feasibility, legal/ sales close
> the contract.

> ⚠️ This SLA assumes single-tenant / single-node deployment. Multi-tenant HA
> SLA targets are **not achievable** until fusion-mlx#754 + fusion-gateway#150
> resolve. Do not sign an HA SLA against a single-instance deployment.

## Service Definition

**Service**: Fusion-CLI local AI inference + knowledge base + agent tooling,
served via the `fusion` binary on a customer-designated Apple Silicon host.

**Coverage**: inference (`chat`/`run`/`embed`), KB CRUD + query, model
management, audit trail, metrics. Excludes: customer-supplied hardware
failures, network outages outside the host, model content quality.

## Uptime / Availability

| Tier | Target | Measurement | Credit on breach |
|------|--------|-------------|------------------|
| Standard | 99.0% / month | `fusion doctor` success rate, sampled every 1min | 10% monthly fee |
| Premium | 99.5% / month | same | 25% monthly fee |

Availability = (total minutes − unavailable minutes) / total minutes.
"Unavailable" = `fusion doctor` reports MLX down for > 5 consecutive min.

**Exclusions**: scheduled maintenance (notified ≥ 48h ahead), customer-caused
(config errors, model OOM from oversize context), force majeure.

## Latency SLO

| Metric | Target | Window |
|--------|--------|--------|
| P95 inference latency | < 2000 ms | rolling 5 min |
| Error rate | < 5% | rolling 5 min |

Measured via fusion-cli metrics (`fusion_cli_latency_ms_bucket`,
`fusion_cli_request_errors_total`). Alert rules in `ALERTING.md`.

**Note**: latency SLO depends on loaded model size + context length. Baseline
must be established per deployment via `scripts/load_test.sh` before signing.

## Support tiers

| Tier | Response time | Coverage | Channel |
|------|---------------|----------|---------|
| Standard | P1 < 4h business, P2 < 1 business day | business hours | email / ticket |
| Premium | P0 < 1h 24/7, P1 < 1h, P2 < 4h | 24/7 | PagerDuty + dedicated engineer |

Priority definitions:
- **P0**: full outage or audit tamper — service unusable / data integrity risk.
- **P1**: single capability down or SLO breach — degraded but partially usable.
- **P2**: non-critical bug, no SLO impact.

## Issue escalation path

```
Customer reports ──> Support tier-1 (triage, ≤30min)
                         │
                    cannot resolve in <SLA window>
                         ▼
                   Engineering on-call (PagerDuty)
                         │
                    needs code fix
                         ▼
                   GitHub issue + hotfix branch
                         │
                    verified + released
                         ▼
                   customer notified + postmortem
```

## Remediation credits

If SLA breached in a billing month, customer entitled to credit (table above),
applied to next invoice. Max aggregate credit = 1 month fee / month. Credit is
not a refund; customer must request within 30 days of breach.

## Change management

- CLI version upgrades: customer notified ≥ 7 days ahead via release channel
  (canary → stable). Customer may pin to a stable tag.
- Breaking changes: per SemVer, major version bump; migration guide provided.
- Emergency security patch: may bypass notice window; documented in CHANGELOG.

## Customer responsibilities

- Maintain the host (power, OS updates, disk space for `~/.fusion/models`).
- Run backups (`scripts/backup.sh`) or authorize vendor-managed backup.
- Provide model loading + context within documented limits.
- Notify vendor of incidents within 1h for P0/P1.

## Limitations (must disclose to customer)

- **Single-instance**: no server-side HA until fusion-mlx#754. A host failure
  = service down until host restored. Premium 24/7 P0 covers *response*, not
  instant recovery if hardware dead.
- **Multi-tenant**: not supported until fusion-gateway#150. One deployment =
  one tenant. Multi-tenant customers need separate deployments per tenant.
- **Model content**: vendor does not warrant model output accuracy, safety, or
  bias — that's the model provider's domain.

## Sign-off

| Party | Role | Name | Date |
|-------|------|------|------|
| Vendor | Eng lead (technical feasibility) | _____ | _____ |
| Vendor | Legal/Sales (contract) | _____ | _____ |
| Customer | Authorized signatory | _____ | _____ |
