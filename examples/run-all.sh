#!/bin/bash
set -e

# Gradience Examples — One-click launcher
# Starts: API + Web UI + Embedded wallet demo server

cd "$(dirname "$0")"

ROOT="$(cd .. && pwd)"

echo "[Examples] Starting Gradience API + Web UI..."
cd "$ROOT"
./start-local.sh &
LOCAL_PID=$!
cd - >/dev/null

echo "[Examples] Starting embedded-wallet demo on :3001..."
npx serve -p 3001 embedded-wallet &
EMBED_PID=$!

cleanup() {
  echo "[Examples] Shutting down all demos..."
  kill $EMBED_PID 2>/dev/null || true
  kill $LOCAL_PID 2>/dev/null || true
  wait $EMBED_PID $LOCAL_PID 2>/dev/null || true
}
trap cleanup INT TERM EXIT

echo ""
echo "=========================================="
echo "  Gradience Demo Matrix Ready"
echo "=========================================="
echo "  Web UI      -> http://localhost:3000"
echo "  API         -> http://localhost:8080"
echo "  Embedded    -> http://localhost:3001"
echo ""
echo "  MCP Client  -> cd examples/mcp-client"
echo "                  WALLET_ID=<id> node index.js"
echo "=========================================="
echo ""

wait
