# Gradience Examples & Playgrounds

This directory contains standalone examples and interactive playgrounds for Gradience Wallet.

Each example is designed to be run independently with minimal setup.

---

## Examples

### 1. `basic-wallet/`
A pure HTML+Tailwind standalone page demonstrating:
- Listing wallets
- Creating a wallet
- Viewing balances across chains
- Sending a quick fund transfer

**How to run:**
```bash
# Start the Gradience API first
./start-local.sh
# Then open the example in your browser
open basic-wallet/index.html
```

---

### 2. `mcp-client/`
A minimal Node.js MCP client that spawns `gradience-mcp` via stdio and demonstrates:
- Protocol initialization
- Tool discovery (`tools/list`)
- Typed tool invocation (`tools/call` for `get_balance`)

**How to run:**
```bash
cd mcp-client
npm install  # no real deps, just sets up package
WALLET_ID=<your-wallet-id> node index.js
```

Or from the repo root:
```bash
cd examples/mcp-client
WALLET_ID=your-wallet-id GRADIENCE_ROOT=../.. node index.js
```

---

### 3. `x402-payment/`
A pure HTML+Tailwind standalone page demonstrating:
- Selecting a source wallet
- Entering recipient & amount
- Executing an on-chain payment through Gradience API

**How to run:**
```bash
# Start the Gradience API first
./start-local.sh
# Then open the example in your browser
open x402-payment/index.html
```

---

### 4. `embedded-wallet/`
A third-party dApp demo that embeds Gradience Wallet in an iframe and communicates over `postMessage`:
- Connect to wallet
- Query balance
- Request transaction signatures with in-iframe user confirmation

**How to run:**
```bash
# Start the Gradience stack first
./start-local.sh
# Serve this example from any port
cd embedded-wallet && npx serve -p 3001
# Then open http://localhost:3001
```

---

## Quick Start (All Demos)

### macOS / Linux
```bash
cd examples
./run-all.sh
```

### Windows
```powershell
cd examples
.\run-all.ps1
```

This starts:
- Gradience API (`http://localhost:8080`)
- Gradience Web UI (`http://localhost:3000`)
- Embedded wallet demo (`http://localhost:3001`)

Then open `basic-wallet/index.html` and `x402-payment/index.html` directly in your browser.

---

## Demo Matrix

| Example            | Stack        | Best For                              |
|--------------------|--------------|---------------------------------------|
| `basic-wallet`     | HTML + JS    | Development live demo, visual impact    |
| `mcp-client`       | Node.js      | Showing MCP interoperability          |
| `x402-payment`     | HTML + JS    | Demonstrating real on-chain payments  |
| `embedded-wallet`  | HTML + iframe| dApp integration & wallet embedding   |
