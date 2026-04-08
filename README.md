# Gradience Wallet

An **Agent Wallet Orchestration Platform** built on the [Open Wallet Standard (OWS)](https://github.com/open-wallet-standard/core).

Gradience enables users to create passkey-backed identities, manage multi-chain wallets locally, and delegate fine-grained, policy-gated access to AI agents via a standardized MCP (Model Context Protocol) interface.

---

## Core Features

- **OWS-Native Vault**: Genuine integration with `ows-lib` and `ows-signer` for local mnemonic generation, encrypted wallet storage, and multi-chain signing.
- **Policy Engine**: Multi-layer policy system — Spend limits, Intent analysis, Dynamic risk signals, Time windows, Chain/Contract whitelist.
- **Web UI + Passkey**: Next.js frontend with WebAuthn passkey registration/login, local-first architecture.
- **DEX Integration**: Real 1inch Swap API + Uniswap V3 fallback, executable via Web UI, CLI, and MCP.
- **MCP Server**: JSON-RPC MCP server exposing `sign_transaction`, `sign_message`, `sign_and_send`, `get_balance`, `swap`, `pay`, `llm_generate`, `ai_balance`, `ai_models`, `verify_api_key` tools.
- **AI Gateway**: Real Anthropic Messages API integration with pre-paid balance, cost tracking, and model-whitelist reconciliation.
- **Audit & Integrity**: HMAC-chained audit logs with Merkle tree anchoring for tamper detection.
- **x402 Payments**: Real OWS-signed x402 settlement with ERC-20 transfer on Base/Ethereum.
- **Shared Budget**: Workspace-level team budgets with `shared_budget` policy rules and cross-wallet spending tracking.
- **Multi-Platform SDKs**: Python SDK + TypeScript SDK for external integrations.
- **Telegram Mini App**: TWA wallet UI with bot webhook support.
- **Local-First**: SQLite + local vault; all data stays on your device, fully self-hostable.

---

## Quick Start

### Prerequisites

- Rust 1.80+ / Cargo
- Node.js 18+ / npm

### Build

```bash
cargo build --workspace
```

### Start Web UI (Local-First)

The easiest way to use Gradience is to run both the API server and the web UI locally with a single command.

**Option A — Shell script**
```bash
./start-local.sh
```

**Option B — Rust CLI**
```bash
cargo run --bin gradience -- start
```

Both will:
1. Start the API server on `http://localhost:8080`
2. Start the Next.js dev server on `http://localhost:3000`
3. Open your browser automatically

Then use Passkey to register / log in, create wallets, fund, swap, and anchor transactions through the web UI.

### CLI Usage

```bash
cargo run --bin gradience -- --help

# Create a wallet
cargo run --bin gradience -- agent create --name demo

# Check balance on Base
cargo run --bin gradience -- agent balance <wallet-id> --chain base

# Execute a real DEX swap
cargo run --bin gradience -- dex swap <wallet-id> --from 0x8335... --to 0x4200... --amount 1

# Export audit logs
cargo run --bin gradience -- audit export <wallet-id> --format json
```

### SDK Usage

**Python SDK**
```bash
pip install ./sdk/python
```
```python
from gradience_sdk import GradienceClient

client = GradienceClient("http://localhost:8080", api_token="YOUR_TOKEN")
wallet = client.create_wallet("demo")
balance = client.get_balance(wallet["id"])
```

**TypeScript SDK**
```bash
npm install ./sdk/typescript
```
```typescript
import { GradienceClient } from "@gradience/sdk";

const client = new GradienceClient("http://localhost:8080", { apiToken: "YOUR_TOKEN" });
const wallet = await client.createWallet("demo");
const balance = await client.getBalance(wallet.id);
```

See [`docs/06-sdk-guide.md`](docs/06-sdk-guide.md) for the full SDK development guide.

### Run MCP Server

```bash
cargo run --bin gradience-mcp
```

### Run Tests

```bash
cargo test --workspace
```

---

## Project Structure

```
gradience-wallet/
├── crates/
│   ├── gradience-core/      # Domain logic: OWS adapter, policy engine, audit, signing, RPC, DEX, HD, team
│   ├── gradience-cli/       # Command-line wallet (clap)
│   ├── gradience-db/        # SQLite/PostgreSQL layer with sqlx
│   ├── gradience-api/       # Axum REST API server
│   ├── gradience-mcp/       # MCP stdio server and tool handlers
│   └── gradience-sdk-node/  # Node.js NAPI bindings
├── contracts/               # Solidity contracts (Merkle anchor)
├── docs/                    # PRD, architecture, technical spec, tests spec, SDK guide
├── sdk/
│   ├── python/              # Python SDK
│   ├── typescript/          # TypeScript SDK
│   ├── go/                  # Go SDK
│   ├── java/                # Java SDK
│   └── ruby/                # Ruby SDK
├── web/                     # Next.js web frontend
├── start-local.sh           # One-click local launcher (macOS/Linux)
├── start-local.ps1          # One-click local launcher (Windows)
└── .sqlx/                   # sqlx offline query metadata
```

---

## Architecture

1. **OWS Adapter (`gradience-core`)**: The `LocalOwsAdapter` delegates all wallet creation, signing, and API key management to the official `ows-lib` crate via git dependency.
2. **Database Layer (`gradience-db`)**: 15-table schema covering users, wallets, addresses, policies, API keys, workspaces, audit logs, and payments.
3. **Policy Engine**: Static JSON policy evaluation with strictest-merge semantics for multi-policy overlays.
4. **MCP Gateway**: JSON-RPC 2.0 over stdio, compatible with any MCP host (Claude, Cursor, etc.).

---

## Documentation

- [`docs/01-prd.md`](docs/01-prd.md) — Product Requirements & Roadmap
- [`docs/02-architecture.md`](docs/02-architecture.md) — System Architecture & ADRs
- [`docs/03-technical-spec.md`](docs/03-technical-spec.md) — Interfaces, DB Schema, Algorithms
- [`docs/04-task-breakdown.md`](docs/04-task-breakdown.md) — Development Plan & Milestones
- [`docs/05-test-spec.md`](docs/05-test-spec.md) — TDD Test Definitions
- [`docs/06-sdk-guide.md`](docs/06-sdk-guide.md) — SDK Development Guide & Roadmap

---

## Tech Stack

- **Language**: Rust
- **CLI**: `clap`
- **Web**: Next.js + TypeScript + Tailwind CSS
- **DB**: `sqlx` + SQLite (local) / PostgreSQL (cloud)
- **Crypto**: `ows-lib` / `ows-signer` (OWS native), `secp256k1`, `rlp`
- **Networking**: `reqwest`, `axum`
- **MCP**: Custom JSON-RPC stdio server
- **SDKs**: Python `requests`, TypeScript `fetch`, Go `net/http`, Java OkHttp, Ruby `net/http`, `napi-rs` (Node.js native)

---

## Development Status

Core platform is feature-complete. All backend APIs, MCP tools, frontend pages, policy engine, audit, shared budget, HD derivation, and multi-chain support are implemented. SDKs are available in Python & TypeScript.

---

## License

MIT (or as specified by the repository owner)

---

English | [简体中文](README.zh-CN.md)
