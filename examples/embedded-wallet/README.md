# Embedded Wallet Example

A third-party dApp demo that embeds the Gradience Wallet inside an iframe and communicates via `postMessage`.

## Features
- Connect to wallet
- Query balance
- Request transaction signature (with user confirmation inside the iframe)

## Prerequisites
- Gradience Web UI running on `http://localhost:3000`
- You are logged in to Gradience (so the embed page can access the session)

## Run
1. Start the full Gradience stack:
   ```bash
   ./start-local.sh
   ```
2. Open `index.html` in your browser (can be served from any origin, e.g. `npx serve -p 3001`).
3. The iframe loads `http://localhost:3000/embed`.
4. Click buttons to interact with the embedded wallet.
