# Basic Wallet Example

A zero-build HTML page that interacts with the Gradience Wallet API.

## Features
- List wallets and view balances
- Create a new wallet
- Send a quick fund transfer

## Run
1. Start the Gradience API:
   ```bash
   ./start-local.sh
   ```
2. Log in via the main web UI (`http://localhost:3000`) and copy your JWT token from browser localStorage (`gradience_token`).
3. Open `index.html` in your browser.
4. Paste the API base URL and JWT token, then interact with your wallets.
