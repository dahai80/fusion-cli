#!/usr/bin/env bash
# 企业级规模化压测: 并发 / 长跑 / SLO 验证。
# 依赖: fusion CLI 已装 + fusion-mlx 已起 + 已加载模型。
# 用法: ./scripts/load_test.sh [并发数] [持续秒数] [模型名]
#   默认: 10 并发, 60s, 自动取首个已加载模型。
set -euo pipefail

CONCURRENCY="${1:-10}"
DURATION="${2:-60}"
MODEL="${3:-}"
LOG_DIR="${HOME}/.fusion/loadtest"
mkdir -p "$LOG_DIR"
TS="$(date +%Y%m%d-%H%M%S)"
RUN_LOG="$LOG_DIR/run-$TS.log"
SUMMARY="$LOG_DIR/summary-$TS.json"

log() { echo "[$(date '+%H:%M:%S')] $*" | tee -a "$RUN_LOG"; }

log "=== fusion-cli 企业压测 开始 ==="
log "并发: $CONCURRENCY  持续: ${DURATION}s  模型: ${MODEL:-auto}"

# 健康前置: mlx 必须活着。
if ! fusion doctor >/dev/null 2>&1; then
    log "FATAL: fusion doctor 失败 — fusion-mlx 未运行或生态不健康"
    exit 1
fi

# 取首个已加载模型 (若未指定)。
if [[ -z "$MODEL" ]]; then
    MODEL="$(fusion model list --format=json 2>/dev/null | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d[0]["id"] if d else "")' 2>/dev/null || true)"
    if [[ -z "$MODEL" ]]; then
        log "FATAL: 无已加载模型 — 先加载模型再压测"
        exit 1
    fi
    log "自动选模型: $MODEL"
fi

# 启动 metrics 基线快照 (压测前)。
fusion metrics export > "$LOG_DIR/baseline-$TS.prom" 2>/dev/null || true

# 压测: 每并发 worker 在 DURATION 内循环发 chat 请求。
WORKER_DIR="$LOG_DIR/workers-$TS"
mkdir -p "$WORKER_DIR"
log "启动 $CONCURRENCY 个 worker, 目录: $WORKER_DIR"

END_AT=$(( $(date +%s) + DURATION ))
worker() {
    local id="$1"
    local f="$WORKER_DIR/w$id.log"
    local reqs=0 errs=0 lat_sum=0
    while [ "$(date +%s)" -lt "$END_AT" ]; do
        local t0 t1 ms
        t0="$(python3 -c 'import time;print(int(time.time()*1000))')"
        if fusion chat -m "$MODEL" -p "Reply with only: ok" --no-stream >/dev/null 2>>"$f.err"; then
            t1="$(python3 -c 'import time;print(int(time.time()*1000))')"
            ms=$(( t1 - t0 ))
            lat_sum=$(( lat_sum + ms ))
            reqs=$(( reqs + 1 ))
        else
            errs=$(( errs + 1 ))
        fi
    done
    local avg=0
    [ "$reqs" -gt 0 ] && avg=$(( lat_sum / reqs ))
    echo "{\"worker\":$id,\"requests\":$reqs,\"errors\":$errs,\"avg_latency_ms\":$avg}" >> "$f.json"
}

pids=()
for i in $(seq 1 "$CONCURRENCY"); do
    worker "$i" &
    pids+=($!)
done

# 等所有 worker 跑完 DURATION。
for p in "${pids[@]}"; do
    wait "$p"
done

# 汇总。
python3 - "$WORKER_DIR" "$SUMMARY" "$CONCURRENCY" "$DURATION" "$TS" <<'PY'
import sys, json, glob, os
wdir, summary, conc, dur, ts = sys.argv[1:6]
tot_req=tot_err=tot_lat=0; n=0
for f in glob.glob(os.path.join(wdir, "*.json")):
    d=json.load(open(f)); tot_req+=d["requests"]; tot_err+=d["errors"]; tot_lat+=d["avg_latency_ms"]*d["requests"]; n+=1
avg_lat = (tot_lat/tot_req) if tot_req else 0
err_rate = (tot_err/(tot_req+tot_err)*100) if (tot_req+tot_err) else 0
rps = tot_req/int(dur) if int(dur) else 0
out={"run_ts":ts,"concurrency":int(conc),"duration_s":int(dur),
     "total_requests":tot_req,"total_errors":tot_err,
     "error_rate_pct":round(err_rate,2),"avg_latency_ms":round(avg_lat,2),
     "requests_per_sec":round(rps,2),
     "slo_error_rate_pass": err_rate < 5.0,
     "slo_p95_target_ms": 2000}
json.dump(out, open(summary,"w"), indent=2)
print(json.dumps(out, indent=2))
PY

log "汇总写入: $SUMMARY"
log "压测后 metrics:"
fusion metrics export > "$LOG_DIR/after-$TS.prom" 2>/dev/null || true

# SLO 判定。
ERR_PCT="$(python3 -c 'import json;print(json.load(open("'"$SUMMARY"'"))["error_rate_pct"])')"
PASS="$(python3 -c 'import json;print(json.load(open("'"$SUMMARY"'"))["slo_error_rate_pass"])')"

log "=== 压测结束 ==="
log "总请求 / 错误率 / RPS / 平均延迟: 见 $SUMMARY"
log "SLO 错误率 < 5%: $PASS (实测 ${ERR_PCT}%)"

if [ "$PASS" = "True" ]; then
    log "✅ SLO 通过"
    exit 0
else
    log "❌ SLO 未达标 — 检查后端容量/熔断器状态"
    exit 2
fi
