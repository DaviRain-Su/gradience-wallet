#!/bin/bash
set -e

# Gradience Wallet - Local-first launcher
# Spawns API server + Next.js dev server, opens browser when ready

cd "$(dirname "$0")"

export DATABASE_URL="sqlite:./gradience.db?mode=rwc"

echo "[Gradience] Starting local API server..."
cargo run -p gradience-api &
API_PID=$!

echo "[Gradience] Starting web frontend..."
cd web
npm run dev &
WEB_PID=$!

cleanup() {
  echo "[Gradience] Shutting down..."
  kill $WEB_PID $API_PID 2>/dev/null || true
  wait $WEB_PID $API_PID 2>/dev/null || true
}
trap cleanup INT TERM EXIT

echo "[Gradience] Waiting for services to be ready..."
for i in {1..60}; do
  if curl -s http://localhost:3000 >/dev/null 2>&1 && curl -s http://localhost:8080/health >/dev/null 2>&1; then
    echo "[Gradience] Ready! Opening http://localhost:3000"
    if command -v open >/dev/null 2>&1; then
      open http://localhost:3000
    elif command -v xdg-open >/dev/null 2>&1; then
      xdg-open http://localhost:3000
    fi
    break
  fi
  sleep 1
done

wait
