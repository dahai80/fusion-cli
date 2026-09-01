#!/usr/bin/env bash
# 灾备备份: KB / RAG / memory / config / audit 数据。
# 策略: 全量 tar 打包 ~/.fusion 数据目录 + 增量标记 + 保留窗口轮转。
# 用法: ./scripts/backup.sh [保留份数]
#   默认保留 7 份。Cron: 0 2 * * * /path/fusion-cli/scripts/backup.sh 7
set -euo pipefail

KEEP="${1:-7}"
SRC="${HOME}/.fusion"
BACKUP_DIR="${HOME}/.fusion-backup"
TS="$(date +%Y%m%d-%H%M%S)"
OUT="${BACKUP_DIR}/fusion-backup-${TS}.tar.gz"
LOG="${BACKUP_DIR}/backup.log"

mkdir -p "$BACKUP_DIR"
log() { echo "[$(date '+%F %T')] $*" | tee -a "$LOG"; }

if [[ ! -d "$SRC" ]]; then
    log "FATAL: 数据目录 $SRC 不存在 — 无需备份"
    exit 0
fi

log "=== 备份开始: $SRC → $OUT ==="

# 排除运行态 (run/ PID + 临时 models 缓存可单独策略), 核心数据全收。
# 保留: config.toml, kb/, rag/, memory/ (若本地有), audit/, metrics/, cron.json
# 排除: run/ (PID 文件, 重启重建), logs/ (已有滚动, 体积大单独备份)
tar -czf "$OUT" \
    --exclude="$SRC/run" \
    --exclude="$SRC/logs" \
    --exclude="$SRC/models" \
    -C "$HOME" .fusion 2>>"$LOG" || {
    log "FATAL: tar 失败"
    exit 1
}

SIZE="$(du -h "$OUT" | cut -f1)"
log "备份完成: $OUT ($SIZE)"

# 完整性校验: tar 能列内容。
if ! tar -tzf "$OUT" >/dev/null 2>&1; then
    log "FATAL: 备份完整性校验失败 — $OUT 可能损坏"
    exit 2
fi
log "完整性校验通过"

# 保留窗口轮转: 超过 KEEP 份删最旧。
COUNT="$(ls -1 "${BACKUP_DIR}"/fusion-backup-*.tar.gz 2>/dev/null | wc -l | tr -d ' ')"
if [ "$COUNT" -gt "$KEEP" ]; then
    REMOVE=$(( COUNT - KEEP ))
    log "轮转: 保留 $KEEP 份, 删除 $REMOVE 份旧备份"
    ls -1t "${BACKUP_DIR}"/fusion-backup-*.tar.gz | tail -n "$REMOVE" | while read -r old; do
        rm -f "$old"
        log "  删除: $old"
    done
fi

# 单独备份日志 (压缩, 不进主包避免膨胀)。
if [[ -d "$SRC/logs" ]]; then
    LOG_OUT="${BACKUP_DIR}/fusion-logs-${TS}.tar.gz"
    tar -czf "$LOG_OUT" -C "$SRC" logs 2>>"$LOG" || log "WARN: 日志备份失败 (非致命)"
    log "日志备份: $LOG_OUT"
fi

log "=== 备份结束 ==="
