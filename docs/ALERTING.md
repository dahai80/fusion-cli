# Alerting — Prometheus Rules & SLO

> #5 enterprise gap: metrics are produced (`fusion metrics export` → Prometheus
> 0.0.4 exposition), but there is no alert-rule set or SLO definition. This doc
> specifies the alert rules to load into Prometheus + the collection wiring.

## Collection Wiring

`fusion` is a local CLI, not a long-running server, so it cannot expose an
`/metrics` HTTP endpoint. Use the **node_exporter textfile collector** pattern:

1. Cron the export to a `.prom` file:
   ```cron
   */1 * * * * fusion metrics export > /var/lib/node_exporter/textfile/fusion.prom 2>>/var/log/fusion-export.err
   ```
2. node_exporter with `--collector.textfile.directory=/var/lib/node_exporter/textfile`
   scrapes the file and exposes it on its `/metrics`.
3. Prometheus scrapes node_exporter as usual.

For multi-node: each node runs `fusion` locally and exports its own counters;
Prometheus labels by `instance` (node_exporter's), so per-node CLI activity is
distinguishable. CLI counters are **per-process cumulative** — they reset when
the host reboots, so alert on rates (`increase()`), not raw gauges.

## Metric Reference

| Metric | Type | Meaning |
|--------|------|---------|
| `fusion_cli_requests_total` | counter | total inference requests |
| `fusion_cli_request_errors_total` | counter | total request errors |
| `fusion_cli_model_pulls_total` | counter | model pull ops |
| `fusion_cli_kb_ingests_total` | counter | KB ingest ops |
| `fusion_cli_bench_runs_total` | counter | benchmark runs |
| `fusion_cli_service_ops_total` | counter | ecosystem service ops |
| `fusion_cli_latency_ms_bucket` | histogram | op latency (ms), buckets le=50/200/500/2000/+Inf |

## Alert Rules

Prometheus rules (load via `rule_files` in prometheus.yml). Thresholds are
starting points — tune to the deployment's traffic.

```yaml
groups:
  - name: fusion-cli
    rules:
      # High error rate — SLO: error rate < 5% over 5m.
      - alert: FusionCliHighErrorRate
        expr: |
          sum(rate(fusion_cli_request_errors_total[5m]))
          / clamp_min(sum(rate(fusion_cli_requests_total[5m])), 1)
          > 0.05
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "fusion-cli error rate > 5% on {{ $labels.instance }}"
          description: "Over the last 5m, >5% of CLI requests errored. Check backend health (fusion doctor) and audit log (fusion audit verify)."

      # P95 latency breach — SLO: P95 < 2000ms.
      - alert: FusionCliHighLatencyP95
        expr: |
          histogram_quantile(0.95,
            sum(rate(fusion_cli_latency_ms_bucket[5m])) by (le, instance)
          ) > 2000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "fusion-cli P95 latency > 2s on {{ $labels.instance }}"
          description: "P95 inference latency exceeds the 2s SLO. Likely backend saturation — check backpressure circuit breaker state (fusion doctor)."

      # Circuit breaker likely open — no successful requests for 5m while errors climb.
      - alert: FusionCliBreakerLikelyOpen
        expr: |
          sum(rate(fusion_cli_requests_total[5m])) == 0
          and sum(increase(fusion_cli_request_errors_total[5m])) > 10
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "fusion-cli circuit breaker likely open on {{ $labels.instance }}"
          description: "No successful requests + rising errors for 5m — the #52 backpressure breaker is likely Open (backend down). Verify fusion-mlx is running."

      # Audit chain broken — surfaced via a separate check (fusion audit verify exit code),
      # exported as a custom metric by a node_exporter textfile script, not built-in here.
      # See "Audit integrity" below.
```

## Audit Integrity (separate probe)

The audit hash chain (#51) is verified by `fusion audit verify` (exit 0 = intact,
1 = broken). Export its result as a gauge via a small cron probe:

```cron
*/5 * * * * fusion audit verify >/dev/null 2>&1 && echo "fusion_cli_audit_chain_intact 1" > /var/lib/node_exporter/textfile/fusion_audit.prom || echo "fusion_cli_audit_chain_intact 0" > /var/lib/node_exporter/textfile/fusion_audit.prom
```

```yaml
      - alert: FusionCliAuditChainBroken
        expr: fusion_cli_audit_chain_intact == 0
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "fusion-cli audit chain BROKEN on {{ $labels.instance }}"
          description: "audit.log tamper detected (hash chain broken). Investigate immediately — run `fusion audit view` for the break point."
```

## SLO Summary

| SLO | Target | Alert |
|-----|--------|-------|
| Availability (error rate) | < 5% / 5m | FusionCliHighErrorRate |
| Latency P95 | < 2000ms / 5m | FusionCliHighLatencyP95 |
| Backend health | requests succeeding | FusionCliBreakerLikelyOpen |
| Audit integrity | chain intact | FusionCliAuditChainBroken |

## Out of scope

- Alertmanager routing/notification config (deployment-specific).
- Per-tenant metrics (requires upstream cross-tenant isolation — fusion-gateway#150).
