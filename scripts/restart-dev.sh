#!/usr/bin/env bash
# 一键停止并后台重启：Next 前端 (默认 3000) + Macaca Web API (默认 3001)
#
# 用法：
#   ./scripts/restart-dev.sh
#   BACKEND_PORT=3001 FRONTEND_PORT=3000 ./scripts/restart-dev.sh
#
# 日志默认：/tmp/agent-dev-macaca-web.log 、 /tmp/agent-dev-frontend.log

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BACKEND_PORT="${BACKEND_PORT:-3001}"
FRONTEND_PORT="${FRONTEND_PORT:-3000}"

MACACA_LOG="${MACACA_LOG:-/tmp/agent-dev-macaca-web.log}"
FRONTEND_LOG="${FRONTEND_LOG:-/tmp/agent-dev-frontend.log}"

kill_listeners_on_port() {
  local port="$1"
  local name="$2"
  # macOS / Linux: 占用该 TCP 监听端口的进程
  local pids
  pids="$(lsof -tiTCP:"$port" -sTCP:LISTEN 2>/dev/null || true)"
  if [[ -n "${pids}" ]]; then
    echo "  结束占用 ${name} 端口 ${port} 的进程: ${pids//[$'\n']/ }"
    # shellcheck disable=SC2086
    kill -9 ${pids} 2>/dev/null || true
  else
    echo "  端口 ${port} (${name}) 无监听进程"
  fi
}

# 兜底：命令行里仍挂着 macaca web / next dev 但端口已释放的情况
pkill_by_pattern() {
  local pattern="$1"
  local desc="$2"
  if pgrep -f "$pattern" >/dev/null 2>&1; then
    echo "  pkill: ${desc}"
    pkill -f "$pattern" 2>/dev/null || true
  fi
}

echo "==> 停止旧进程（端口 + 常见残留）"
kill_listeners_on_port "$BACKEND_PORT" "Macaca Web"
kill_listeners_on_port "$FRONTEND_PORT" "Next.js"

# 端口已释放后仍可能残留的命令行（兜底）
pkill_by_pattern '/macaca web' 'macaca web'
pkill_by_pattern 'next dev' 'next dev'

sleep 1

echo ""
echo "==> 启动后端: macaca web (端口 ${BACKEND_PORT})"
cd "$REPO_ROOT/macaca"
: >"$MACACA_LOG"
if [[ -n "${MACACA_BIN:-}" ]]; then
  echo "  使用 MACACA_BIN=$MACACA_BIN"
  nohup "$MACACA_BIN" web --port "$BACKEND_PORT" >>"$MACACA_LOG" 2>&1 &
else
  nohup cargo run --bin macaca -- web --port "$BACKEND_PORT" >>"$MACACA_LOG" 2>&1 &
fi
echo "  PID=$!  日志: $MACACA_LOG"

echo ""
echo "==> 启动前端: next dev (端口 ${FRONTEND_PORT})"
cd "$REPO_ROOT/frontend"
: >"$FRONTEND_LOG"
nohup npm run dev -- -p "$FRONTEND_PORT" >>"$FRONTEND_LOG" 2>&1 &
echo "  PID=$!  日志: $FRONTEND_LOG"

echo ""
echo "==> 完成"
echo "  前端: http://localhost:${FRONTEND_PORT}"
echo "  后端: http://localhost:${BACKEND_PORT}"
echo "  查看日志: tail -f \"$MACACA_LOG\" \"$FRONTEND_LOG\""
