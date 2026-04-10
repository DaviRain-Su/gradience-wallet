# Getting Started with Gradience Wallet

Gradience is a **local-first, agent-ready wallet**. Your keys and data stay on your own device.

---

## System Requirements

- **macOS** 11+ (Apple Silicon or Intel)
- **Linux** x86_64 (Ubuntu 20.04+, Debian, Fedora, Arch)
- 200 MB free disk space
- Internet connection (for blockchain queries and optional AI features)

---

## 1. Install Gradience

### macOS — Homebrew (recommended)

```bash
brew tap DaviRain-Su/gradience
brew install gradience
```

### macOS / Linux — Download pre-built binary

1. Go to [GitHub Releases](https://github.com/DaviRain-Su/gradience-wallet/releases/latest)
2. Download the file for your platform:
   - **Apple Silicon Mac**: `gradience-aarch64-apple-darwin.tar.gz`
   - **Linux x86_64**: `gradience-x86_64-unknown-linux-gnu.tar.gz`
3. Extract and run:

```bash
# macOS Apple Silicon
curl -L -o gradience.tar.gz https://github.com/DaviRain-Su/gradience-wallet/releases/latest/download/gradience-aarch64-apple-darwin.tar.gz

# Linux x86_64
curl -L -o gradience.tar.gz https://github.com/DaviRain-Su/gradience-wallet/releases/latest/download/gradience-x86_64-unknown-linux-gnu.tar.gz

# Extract and run
tar xzf gradience.tar.gz
./gradience
```

### Build from source (Intel Mac, other platforms)

```bash
git clone https://github.com/DaviRain-Su/gradience-wallet.git
cd gradience-wallet
cargo install --path crates/gradience-cli --bin gradience
```

---

## 2. Start Gradience

Run the binary with no arguments:

```bash
./gradience
```

This will:
1. Start a local API server on `http://localhost:8080`
2. Open your default browser automatically
3. Show the Gradience login page

> The wallet UI is served entirely from your local machine. No data is sent to external servers unless you perform on-chain transactions.

---

## 3. First-time Setup

### Unlock your vault

When you open Gradience for the first time, you will be asked to **set a vault passphrase**.

- This passphrase encrypts your local wallet database.
- **There is no password reset**. If you forget it, you cannot recover your wallets unless you have saved your recovery phrase.
- Choose a strong passphrase (12+ characters) and store it safely.

### Create your first wallet

After unlocking:
1. Go to the **Dashboard** page
2. Enter a wallet name (e.g. "Main Wallet")
3. Click **Create Wallet**
4. Gradience will generate a new multi-chain wallet and show you addresses for Base, Ethereum, Solana, and other supported chains

### View addresses and receive funds

On the Dashboard:
- Click **Addresses** on any wallet card to see your deposit addresses
- Copy the address for the chain you want to receive on

### Check balance

Click **Balance** on a wallet card to see your token balances across chains.

---

## 4. Common Operations

### Send / Transfer

1. Click **Transfer** on a wallet card
2. Enter the destination address, amount, token symbol (e.g. `ETH`), and chain (e.g. `base`)
3. Confirm — the transaction will be signed locally and broadcast to the network

### DEX Swap

1. Click **Swap** on a wallet card
2. Fill in the tokens and amount
3. Get a quote, then execute the swap

### Policy & Permissions

Go to the **Policy** tab to:
- Set spending limits
- Restrict which contracts or chains are allowed
- Require time windows for transactions

---

## 5. Use Gradience with an AI Agent (MCP)

Gradience exposes a **Model Context Protocol (MCP)** server. Any MCP-compatible AI agent (Claude Desktop, Cursor, etc.) can control your wallet within the policy guardrails you set.

### Available MCP tools

- `sign_transaction` — sign and send a transaction
- `get_balance` — read wallet balance
- `swap` — perform a DEX swap
- `pay` — execute a payment
- `llm_generate` — generate AI text through the built-in gateway
- `ai_balance` — check prepaid AI credit

### Example conversation

> **You:** "Swap 0.1 ETH on Base to USDC."
>
> **Agent:** "I'll use the swap tool to convert 0.1 ETH to USDC on Base. The transaction will be signed from your active wallet. Confirm?"
>
> **You:** "Yes."
>
> **Agent:** *submits transaction and returns the tx hash*

---

## 6. Where is my data?

All data stays local:

- **Database**: `~/.gradience/gradience.db`
- **Encrypted vault**: `~/.gradience/vault/`
- **Session file**: `~/.gradience/.session`

You can back up the `~/.gradience` folder to protect your wallets.

---

## 7. Frequently Asked Questions

### What if I forget my passphrase?

There is **no centralized recovery**. Make sure you write down or back up your passphrase. Some platforms may have a recovery phrase backup flow inside the Settings tab.

### Do I need to keep the terminal open?

Yes. `./gradience` runs the local server. Closing the terminal will stop the wallet UI. In the future, a background daemon mode may be added.

### Is there a mobile app?

Currently Gradience is **desktop web only** (macOS and Linux). Mobile support is not available at this time.

### How do I update?

**Homebrew users:**
```bash
brew upgrade gradience
```

**Manual download users:**
Download the latest release from GitHub and replace the binary.

---

## Need Help?

- Open an issue: [github.com/DaviRain-Su/gradience-wallet/issues](https://github.com/DaviRain-Su/gradience-wallet/issues)
- Read the full docs in [`docs/`](docs/)
