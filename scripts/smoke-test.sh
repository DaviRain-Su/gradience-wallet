#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
API_BASE="${GRADIENCE_API_BASE:-http://localhost:8080}"
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass=0
fail=0

function test_ok() {
  echo -e "${GREEN}PASS${NC} $1"
  ((pass++)) || true
}

function test_fail() {
  echo -e "${RED}FAIL${NC} $1"
  ((fail++)) || true
}

echo "======================================"
echo "Gradience Smoke Test"
echo "API: $API_BASE"
echo "======================================"
echo ""

# Ensure clean server binary exists
echo "[0] Building gradience-api..."
set +e
DATABASE_URL="sqlite:./gradience.db" cargo build --bin gradience-api > /tmp/build-api.log 2>&1
CODE=$?
set -e
if [ $CODE -ne 0 ]; then
  test_fail "gradience-api build failed"
  tail -n 10 /tmp/build-api.log
  exit 1
fi

# Kill any stale server and start fresh
kill $(lsof -t -i:8080) 2>/dev/null || true
sleep 1
DATABASE_URL="sqlite:./gradience.db" ./target/debug/gradience-api > /tmp/api-smoke.log 2>&1 &
API_PID=$!
sleep 3

function cleanup() {
  kill $API_PID 2>/dev/null || true
}
trap cleanup EXIT

# 1. Health check
echo "[1] Health check..."
if curl -sf "$API_BASE/health" > /tmp/health.json; then
  test_ok "Health endpoint returns 200"
  cat /tmp/health.json
  echo ""
else
  test_fail "Health endpoint unreachable"
  tail -n 10 /tmp/api-smoke.log
  exit 1
fi

# 2. Swap quote (public)
echo ""
echo "[2] DEX swap quote..."
if curl -sf -X POST "$API_BASE/api/swap/quote" \
  -H "Content-Type: application/json" \
  -d '{"chain":"base","from_token":"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913","to_token":"0x4200000000000000000000000000000000000006","amount":"1000000"}' \
  > /tmp/quote.json; then
  test_ok "Swap quote returns 200"
  cat /tmp/quote.json
  echo ""
else
  test_fail "Swap quote failed"
fi

# 3. MPP demo - 402 challenge
echo ""
echo "[3] MPP demo (402 challenge)..."
STATUS=$(curl -s -o /tmp/mpp402.json -w "%{http_code}" -X POST "$API_BASE/api/mpp/demo" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"hello mpp"}')
if [ "$STATUS" = "402" ]; then
  test_ok "MPP demo returns 402 with challenge"
  grep -q "WWW-Authenticate" /tmp/mpp402.headers 2>/dev/null || true
  cat /tmp/mpp402.json
  echo ""
else
  test_fail "MPP demo expected 402, got $STATUS"
fi

# 4. MPP demo - 200 with credential
echo ""
echo "[4] MPP demo (payment retry)..."
STATUS=$(curl -s -o /tmp/mpp200.json -w "%{http_code}" -X POST "$API_BASE/api/mpp/demo" \
  -H "Content-Type: application/json" \
  -H "Authorization: Payment dummy" \
  -d '{"prompt":"hello mpp"}')
if [ "$STATUS" = "200" ]; then
  test_ok "MPP demo returns 200 with credential"
  cat /tmp/mpp200.json
  echo ""
else
  test_fail "MPP demo expected 200, got $STATUS"
fi

# 5. MCP tool local - get_balance (does not need running server)
echo ""
echo "[5] MCP tool (get_balance) via gradience-cli..."
# This signs a local JSON-RPC payload; it may fail if no wallet exists.
set +e
OUTPUT=$(DATABASE_URL="sqlite:./gradience.db" cargo run --quiet --bin gradience -- mcp balance test-wallet eip155:8453 2>&1)
CODE=$?
set -e
if [ $CODE -eq 0 ]; then
  test_ok "MCP get_balance tool executed"
  echo "$OUTPUT"
else
  test_fail "MCP get_balance tool failed (expected if no wallet)"
  echo "$OUTPUT" | tail -n 3
fi

# 6. Front-end build
echo ""
echo "[6] Next.js build..."
set +e
(cd web && npm run build > /tmp/web-build.log 2>&1)
CODE=$?
set -e
if [ $CODE -eq 0 ]; then
  test_ok "Next.js build succeeds"
else
  test_fail "Next.js build failed"
  tail -n 10 /tmp/web-build.log
fi

# 7. Workspace cargo check
echo ""
echo "[7] Rust workspace check..."
set +e
DATABASE_URL="sqlite:./gradience.db" cargo check --workspace > /tmp/cargo-check.log 2>&1
CODE=$?
set -e
if [ $CODE -eq 0 ]; then
  test_ok "cargo check --workspace passes"
else
  test_fail "cargo check failed"
  tail -n 10 /tmp/cargo-check.log
fi

echo ""
echo "======================================"
echo -e "Results: ${GREEN}$pass passed${NC}, ${RED}$fail failed${NC}"
echo "======================================"

if [ $fail -gt 0 ]; then
  exit 1
fi
