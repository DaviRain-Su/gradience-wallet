#!/bin/bash
set -e

# Quick launcher for MCP client example
# Usage: ./run.sh <wallet-id>

WALLET_ID="${1:-${WALLET_ID}}"
if [ -z "$WALLET_ID" ]; then
  echo "Usage: ./run.sh <wallet-id>"
  echo "Or set WALLET_ID env variable."
  exit 1
fi

export WALLET_ID
export GRADIENCE_ROOT="${GRADIENCE_ROOT:-../..}"

node index.js
