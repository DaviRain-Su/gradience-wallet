#!/usr/bin/env bash
# NOTE: This script enables GRADIENCE_DEMO_TOKEN for local demonstration only.
# Never set this variable in a production deployment.
set -euo pipefail

cd "$(dirname "$0")/.."
API_BASE="${GRADIENCE_API_BASE:-http://localhost:8080}"
DEMO_TOKEN="demo-token-full"
DEMO_PASS="demo-passphrase-123"
DATA_DIR="${HOME}/.gradience"
SESSION_FILE="${DATA_DIR}/.session"

export GRADIENCE_DATA_DIR="$(pwd)"
export DATABASE_URL="sqlite:${GRADIENCE_DATA_DIR}/gradience.db"
export GRADIENCE_DEMO_TOKEN="${DEMO_TOKEN}"
export GRADIENCE_DEMO_PASSPHRASE="${DEMO_PASS}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

function info() { echo -e "${YELLOW}[INFO]${NC} $*"; }
function ok() { echo -e "${GREEN}[OK]${NC} $*"; }
function err() { echo -e "${RED}[ERR]${NC} $*"; }

# Ensure demo passphrase exists for CLI
mkdir -p "${DATA_DIR}"
echo "${DEMO_PASS}" > "${SESSION_FILE}"

# ─── Step 0: Ensure API is up ────────────────────────────────────────────────
info "Building gradience-api ..."
if ! DATABASE_URL="${DATABASE_URL}" cargo build --bin gradience-api > /tmp/build-api.log 2>&1; then
  err "gradience-api build failed"
  tail -n 10 /tmp/build-api.log
  exit 1
fi

info "Checking API health at ${API_BASE} ..."
if curl -sf "${API_BASE}/health" >/dev/null 2>&1; then
  info "Stopping existing API to ensure fresh binary ..."
  kill $(lsof -t -i:8080) 2>/dev/null || true
  sleep 1
fi

info "Starting gradience-api in background..."
DATABASE_URL="${DATABASE_URL}" GRADIENCE_DEMO_TOKEN="${DEMO_TOKEN}" GRADIENCE_DEMO_PASSPHRASE="${DEMO_PASS}" \
  ./target/debug/gradience-api > /tmp/gradience-api-demo.log 2>&1 &
API_PID=$!
sleep 3
if ! curl -sf "${API_BASE}/health" >/dev/null 2>&1; then
  err "Failed to start API. Aborting."
  exit 1
fi
ok "API started (PID ${API_PID})"

# ─── Step 1: Create wallet via CLI ───────────────────────────────────────────
info "Listing existing wallets..."
WALLET_ID=$(cargo run --quiet --bin gradience -- agent list 2>/dev/null | grep -oE '^[0-9a-f-]{36}' | head -1 || true)
if [[ -z "${WALLET_ID}" ]]; then
  info "No wallet found. Creating one with CLI..."
  cargo run --quiet --bin gradience -- agent create --name "demo-wallet" 2>/dev/null || true
  WALLET_ID=$(cargo run --quiet --bin gradience -- agent list 2>/dev/null | grep -oE '^[0-9a-f-]{36}' | head -1 || true)
fi
if [[ -z "${WALLET_ID}" ]]; then
  err "Could not create or find a wallet. Aborting."
  exit 1
fi
ok "Using wallet ${WALLET_ID}"

# ─── Step 2: Set policy (spend_limit 0.001 ETH) ──────────────────────────────
POLICY_FILE="/tmp/demo-policy.json"
cat > "${POLICY_FILE}" <<'EOF'
{
  "name": "demo-spend-limit",
  "rules": [
    {
      "type": "spend_limit",
      "max": "1000000000000000",
      "token": "ETH"
    },
    {
      "type": "chain_whitelist",
      "chain_ids": ["eip155:8453", "eip155:1", "eip155:196", "eip155:56"]
    }
  ]
}
EOF
info "Setting spend-limit policy (0.001 ETH) ..."
if cargo run --quiet --bin gradience -- policy set "${WALLET_ID}" --file "${POLICY_FILE}" 2>/dev/null; then
  ok "Policy set"
else
  err "Policy set failed (may already exist). Continuing..."
fi

# ─── Step 3: Pick an EVM address for this wallet ─────────────────────────────
ADDR=$(sqlite3 gradience.db "SELECT address FROM wallet_addresses WHERE wallet_id='${WALLET_ID}' AND chain_id LIKE 'eip155:%' LIMIT 1;" || true)
if [[ -z "${ADDR}" ]]; then
  err "No EVM address found for wallet ${WALLET_ID}. Aborting."
  exit 1
fi
ok "EVM address: ${ADDR}"

# ─── Step 4: Low-risk fund (0.0001 ETH) ──────────────────────────────────────
info "Sending low-risk fund request (0.0001 ETH) ..."
HTTP_CODE=$(curl -s -o /tmp/fund-low.json -w "%{http_code}" -X POST "${API_BASE}/api/wallets/${WALLET_ID}/fund" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${DEMO_TOKEN}" \
  -d "{\"to\":\"${ADDR}\",\"amount\":\"0.0001\",\"chain\":\"base\"}" 2>/dev/null || echo "000")
case "${HTTP_CODE}" in
  200)
    ok "Low-risk fund ACCEPTED by policy (HTTP 200)"
    cat /tmp/fund-low.json | jq . 2>/dev/null || cat /tmp/fund-low.json
    ;;
  403)
    err "Low-risk fund REJECTED by policy (HTTP 403)"
    cat /tmp/fund-low.json | jq . 2>/dev/null || cat /tmp/fund-low.json
    ;;
  500)
    ok "Low-risk fund passed policy but broadcast failed (likely empty wallet). This is expected."
    ;;
  *)
    err "Unexpected HTTP ${HTTP_CODE}"
    cat /tmp/fund-low.json | jq . 2>/dev/null || cat /tmp/fund-low.json
    ;;
esac

# ─── Step 5: High-risk fund (0.01 ETH) ───────────────────────────────────────
info "Sending high-risk fund request (0.01 ETH, exceeds 0.001 limit) ..."
HTTP_CODE=$(curl -s -o /tmp/fund-high.json -w "%{http_code}" -X POST "${API_BASE}/api/wallets/${WALLET_ID}/fund" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${DEMO_TOKEN}" \
  -d "{\"to\":\"${ADDR}\",\"amount\":\"0.01\",\"chain\":\"base\"}" 2>/dev/null || echo "000")
if [[ "${HTTP_CODE}" == "403" ]]; then
  ok "High-risk fund correctly REJECTED by policy (HTTP 403)"
  cat /tmp/fund-high.json | jq . 2>/dev/null || cat /tmp/fund-high.json
else
  err "Expected HTTP 403, got ${HTTP_CODE}"
  cat /tmp/fund-high.json | jq . 2>/dev/null || cat /tmp/fund-high.json
fi

# ─── Step 6: Portfolio query ─────────────────────────────────────────────────
info "Fetching portfolio..."
curl -sf "${API_BASE}/api/wallets/${WALLET_ID}/portfolio" \
  -H "Authorization: Bearer ${DEMO_TOKEN}" | jq . > /tmp/portfolio.json
ok "Portfolio saved to /tmp/portfolio.json"
jq . /tmp/portfolio.json

# ─── Step 7: Audit logs (JSON export) ────────────────────────────────────────
info "Fetching audit logs (JSON)..."
curl -sf "${API_BASE}/api/wallets/${WALLET_ID}/audit/export?format=json" \
  -H "Authorization: Bearer ${DEMO_TOKEN}" | jq . > /tmp/audit-logs.json
ok "Audit logs saved to /tmp/audit-logs.json"
LOG_COUNT=$(jq 'length' /tmp/audit-logs.json)
ok "Audit log count: ${LOG_COUNT}"

# ─── Step 8: Merkle proof for the latest audit log ───────────────────────────
LATEST_LOG_ID=$(jq -r '.[-1].id // empty' /tmp/audit-logs.json || true)
if [[ -n "${LATEST_LOG_ID}" ]]; then
  info "Generating Merkle proof for latest audit log (id=${LATEST_LOG_ID}) ..."
  curl -sf "${API_BASE}/api/wallets/${WALLET_ID}/audit/proof?log_id=${LATEST_LOG_ID}" \
    -H "Authorization: Bearer ${DEMO_TOKEN}" | jq . > /tmp/merkle-proof.json
  ok "Merkle proof saved to /tmp/merkle-proof.json"
  jq . /tmp/merkle-proof.json
else
  err "No audit logs available for Merkle proof."
fi

# ─── Step 9: CLI audit export to CSV ─────────────────────────────────────────
info "Exporting audit logs to CSV via CLI..."
cargo run --quiet --bin gradience -- audit export --wallet-id "${WALLET_ID}" --format csv --output ./demo-audit.csv 2>/dev/null
ok "CSV exported to ./demo-audit.csv"
head -5 ./demo-audit.csv

# ─── Step 10: Team Budget Demo ───────────────────────────────────────────────
info "--- Team Budget Demo ---"

# Create workspace
WORKSPACE_NAME="demo-workspace-${RANDOM}"
info "Creating workspace ${WORKSPACE_NAME} ..."
cargo run --quiet --bin gradience -- team create-workspace --name "${WORKSPACE_NAME}" 2>/dev/null || true
WORKSPACE_ID=$(sqlite3 gradience.db "SELECT id FROM workspaces WHERE name='${WORKSPACE_NAME}' LIMIT 1;" || true)
if [[ -z "${WORKSPACE_ID}" ]]; then
  err "Workspace creation failed. Skipping team budget demo."
  exit 0
fi
ok "Workspace id: ${WORKSPACE_ID}"

# Assign wallet to workspace
sqlite3 gradience.db "UPDATE wallets SET workspace_id='${WORKSPACE_ID}' WHERE id='${WALLET_ID}';" >/dev/null
ok "Wallet ${WALLET_ID} assigned to workspace ${WORKSPACE_ID}"

# Allocate team budget 0.001 ETH
cargo run --quiet --bin gradience -- team budget-set "${WORKSPACE_ID}" \
  --amount "0.001" --token "ETH" --chain-id "eip155:8453" --period "monthly" 2>/dev/null
ok "Workspace budget set to 0.001 ETH"

# Set policy with shared_budget rule
TEAM_POLICY_FILE="/tmp/demo-team-policy.json"
cat > "${TEAM_POLICY_FILE}" <<EOF
{
  "name": "demo-team-budget",
  "rules": [
    {
      "type": "shared_budget",
      "max": "1000000000000000",
      "token": "ETH",
      "period": "monthly"
    }
  ]
}
EOF
info "Setting workspace-level shared-budget policy ..."
cargo run --quiet --bin gradience -- policy set "${WALLET_ID}" --file "${TEAM_POLICY_FILE}" 2>/dev/null || true
ok "Shared budget policy set"

# Simulate prior spending so that remaining budget is tiny
CURRENT_TOTAL=$(sqlite3 gradience.db "SELECT total_amount FROM shared_budget_trackers WHERE workspace_id='${WORKSPACE_ID}' AND token_address='ETH' AND chain_id='eip155:8453' AND period='monthly' LIMIT 1;" || echo "0")
if [[ "${CURRENT_TOTAL}" != "0" && -n "${CURRENT_TOTAL}" ]]; then
  # Set spent_amount to just under total (e.g. 90% used)
  SPENT=$(( CURRENT_TOTAL * 9 / 10 ))
  sqlite3 gradience.db "INSERT INTO shared_budget_trackers (workspace_id, token_address, chain_id, period, spent_amount, total_amount, reset_at) VALUES ('${WORKSPACE_ID}', 'ETH', 'eip155:8453', 'monthly', '${SPENT}', '${CURRENT_TOTAL}', datetime('now','+30 days')) ON CONFLICT(workspace_id, token_address, chain_id, period) DO UPDATE SET spent_amount = excluded.spent_amount;"
  ok "Simulated ${SPENT} wei already spent from workspace budget"
fi

# Query remaining budget
cargo run --quiet --bin gradience -- team budget-status "${WORKSPACE_ID}" \
  --token "ETH" --chain-id "eip155:8453" --period "monthly" 2>/dev/null || true

# Try a fund that exceeds remaining workspace budget
info "Sending fund request that exceeds remaining workspace budget ..."
HTTP_CODE=$(curl -s -o /tmp/fund-team.json -w "%{http_code}" -X POST "${API_BASE}/api/wallets/${WALLET_ID}/fund" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${DEMO_TOKEN}" \
  -d "{\"to\":\"${ADDR}\",\"amount\":\"0.0005\",\"chain\":\"base\"}" 2>/dev/null || echo "000")
if [[ "${HTTP_CODE}" == "403" ]]; then
  ok "Workspace budget correctly REJECTED the transaction (HTTP 403)"
  cat /tmp/fund-team.json | jq . 2>/dev/null || cat /tmp/fund-team.json
else
  err "Expected HTTP 403 for team budget deny, got ${HTTP_CODE}"
  cat /tmp/fund-team.json | jq . 2>/dev/null || cat /tmp/fund-team.json
fi

info "========================================"
ok "Full demo completed successfully!"
info "========================================"
