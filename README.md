# Gradience Wallet

An **Agent Wallet Orchestration Platform** built on the [Open Wallet Standard (OWS)](https://github.com/open-wallet-standard/core), designed for the **HashKey Chain Horizon Hackathon 2026**.

Gradience enables users to create passkey-backed identities, manage multi-chain wallets locally, and delegate fine-grained, policy-gated access to AI agents via a standardized MCP (Model Context Protocol) interface.

---

## Core Features

- **OWS-Native Vault**: Genuine integration with `ows-lib` and `ows-signer` for local mnemonic generation, encrypted wallet storage, and multi-chain signing.
- **Policy Engine**: Multi-layer policy system — Spend limits, Intent analysis, Dynamic risk signals, Time windows, Chain/Contract whitelist.
- **Web UI + Passkey**: Next.js frontend with WebAuthn passkey registration/login, local-first architecture.
- **DEX Integration**: Real 1inch Swap API + Uniswap V3 fallback, executable via Web UI, CLI, and MCP.
- **MCP Server**: JSON-RPC MCP server exposing `sign_transaction`, `get_balance`, `swap`, `pay`, `llm_generate`, `ai_balance`, `ai_models` tools.
- **AI Gateway**: Pre-paid LLM generation with cost tracking and reconciliation.
- **Audit & Integrity**: HMAC-chained audit logs with Merkle tree anchoring for tamper detection.
- **Multi-Platform SDKs**: Node.js (napi-rs) and Python SDKs for external integrations.
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
```

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
│   ├── gradience-core/      # Domain logic: OWS adapter, policy engine, audit, signing, RPC, DEX
│   ├── gradience-cli/       # Command-line wallet (clap)
│   ├── gradience-db/        # SQLite/PostgreSQL layer with sqlx
│   ├── gradience-api/       # Axum REST API server
│   ├── gradience-mcp/       # MCP stdio server and tool handlers
│   └── gradience-sdk-node/  # Node.js NAPI bindings
├── contracts/               # Solidity contracts (Merkle anchor)
├── docs/                    # PRD, architecture, technical spec, tests spec
├── sdk/python/              # Python SDK
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
- [`docs/04-task-breakdown.md`](docs/04-task-breakdown.md) — Hackathon Sprint Plan
- [`docs/05-test-spec.md`](docs/05-test-spec.md) — TDD Test Definitions

---

## Tech Stack

- **Language**: Rust
- **CLI**: `clap`
- **Web**: Next.js + TypeScript + Tailwind CSS
- **DB**: `sqlx` + SQLite (local) / PostgreSQL (cloud)
- **Crypto**: `ows-lib` / `ows-signer` (OWS native), `secp256k1`, `rlp`
- **Networking**: `reqwest`, `axum`
- **MCP**: Custom JSON-RPC stdio server
- **SDKs**: `napi-rs` (Node.js), Python `requests`

---

## Hackathon

- **Event**: HashKey Chain Horizon Hackathon 2026
- **Deadline**: April 15, 2026
- **Status**: Core platform implemented, 51 tests passing, OWS genuine integration complete.

---

## License

MIT (or as specified by the repository owner)
