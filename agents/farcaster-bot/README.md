# Gradience Farcaster Bot

An autonomous agent that lives on Farcaster and pays for x402-protected resources on Base Sepolia via Gradience.

## What it does

1. Monitors Farcaster mentions (`@gradience_bot`)
2. Parses `pay <url>` commands
3. Derives a deterministic Base Sepolia private key from Gradience's local adapter
4. Calls the `base-x402` Node bridge to sign and settle EIP-3009 payments
5. Replies with the transaction hash and unlocked content

## Setup

### 1. Neynar Account

- Sign up at https://portal.neynar.com/signup
- Create an API key
- Create a **Signer** for your bot's Farcaster account
- Note down:
  - `NEYNAR_API_KEY`
  - `NEYNAR_SIGNER_UUID`
  - Your bot's `FID` (Farcaster ID)
  - Your bot's username (e.g. `gradience_bot`)

### 2. Environment

```bash
cp .env.example .env
# Edit .env with your Neynar credentials
```

### 3. Install & Run

```bash
npm install
npm run dev
```

## Usage on Farcaster

Post a cast mentioning the bot:

```
@gradience_bot pay http://localhost:4021/weather
```

The bot will reply with the payment result and Base Sepolia tx hash.

## Demo prerequisites

Before testing, make sure the deterministic payer wallet (`WALLET_ID` in `.env`) has **Base Sepolia USDC**.

You can fund it via:
- [CDP Portal Faucet](https://portal.cdp.coinbase.com/products/faucet) (programmatic or UI)
- [Circle Testnet Faucet](https://faucet.circle.com/)

The payer address is derived from `WALLET_ID` + `CHAIN` + BIP-44 path `m/44'/60'/0'/0/0`.
