# x402 Payment Example

A zero-build HTML page that demonstrates on-chain payments through Gradience Wallet.

## Features
- Select a wallet to pay from
- Enter recipient address and amount
- Choose chain (Base / Ethereum)
- Execute payment and display transaction hash

## Run
1. Start the Gradience API:
   ```bash
   ./start-local.sh
   ```
2. Log in via the main web UI (`http://localhost:3000`) and copy your JWT token from browser localStorage (`gradience_token`).
3. Open `index.html` in your browser.
4. Paste the API base URL and JWT token, then send a payment.
