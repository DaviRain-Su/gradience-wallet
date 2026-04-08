#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
API_BASE="${GRADIENCE_API_BASE:-http://localhost:8080}"
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo "======================================"
echo "Real API Quick Verification"
echo "======================================"
echo ""

# --- Anthropic AI Gateway ---
echo "[1] Anthropic AI Gateway"
if [ -z "${ANTHROPIC_API_KEY:-}" ]; then
  echo -e "${YELLOW}SKIP${NC} ANTHROPIC_API_KEY not set. Export it to test real LLM responses."
else
  echo "ANTHROPIC_API_KEY detected. Testing via /api/ai/generate ..."
  # Note: this endpoint requires auth. For a quick manual test, use curl with a valid Bearer token.
  echo "  curl -X POST $API_BASE/api/ai/generate \\"
  echo "    -H \"Authorization: Bearer <YOUR_TOKEN>\" \\"
  echo "    -H \"Content-Type: application/json\" \\"
  echo "    -d '{\"wallet_id\":\"<WALLET_ID>\",\"provider\":\"anthropic\",\"model\":\"claude-3-5-sonnet-20241022\",\"prompt\":\"Say hello in one word\"}'"
  echo ""
fi

# --- 1inch DEX Quote ---
echo "[2] 1inch DEX Quote"
if [ -z "${ONEINCH_API_KEY:-}" ]; then
  echo -e "${YELLOW}SKIP${NC} ONEINCH_API_KEY not set. Export it to test real 1inch quotes."
else
  echo "ONEINCH_API_KEY detected. Testing /api/swap/quote ..."
  STATUS=$(curl -s -o /tmp/1inch-quote.json -w "%{http_code}" -X POST "$API_BASE/api/swap/quote" \
    -H "Content-Type: application/json" \
    -d '{
      "chain": "base",
      "from_token": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
      "to_token": "0x4200000000000000000000000000000000000006",
      "amount": "1000000"
    }')
  if [ "$STATUS" = "200" ]; then
    PROVIDER=$(cat /tmp/1inch-quote.json | python3 -c "import sys,json; print(json.load(sys.stdin).get('provider','unknown'))" 2>/dev/null || echo "unknown")
    if [ "$PROVIDER" = "1inch" ]; then
      echo -e "${GREEN}PASS${NC} 1inch quote returned successfully"
      cat /tmp/1inch-quote.json
      echo ""
    else
      echo -e "${YELLOW}WARN${NC} Quote succeeded but provider='$PROVIDER' (expected '1inch')"
      cat /tmp/1inch-quote.json
      echo ""
    fi
  else
    echo -e "${RED}FAIL${NC} 1inch quote HTTP $STATUS"
    cat /tmp/1inch-quote.json
    echo ""
  fi
fi

# --- On-chain Uniswap V3 Quoter (no API key needed) ---
echo "[3] Uniswap V3 Quoter (public RPC fallback)"
STATUS=$(curl -s -o /tmp/uni-quote.json -w "%{http_code}" -X POST "$API_BASE/api/swap/quote" \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "base",
    "from_token": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    "to_token": "0x4200000000000000000000000000000000000006",
    "amount": "1000000"
  }')
if [ "$STATUS" = "200" ]; then
  PROVIDER=$(cat /tmp/uni-quote.json | python3 -c "import sys,json; print(json.load(sys.stdin).get('provider','unknown'))" 2>/dev/null || echo "unknown")
  echo -e "${GREEN}PASS${NC} Public quote endpoint returns 200 (provider=$PROVIDER)"
else
  echo -e "${RED}FAIL${NC} Public quote endpoint HTTP $STATUS"
  cat /tmp/uni-quote.json
fi

echo ""
echo "======================================"
echo "To enable real APIs before the demo:"
echo "  export ANTHROPIC_API_KEY='sk-ant-...'"
echo "  export ONEINCH_API_KEY='...'"
echo "======================================"
