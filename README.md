# Gradience Wallet

An **Agent Wallet Orchestration Platform** built on the [Open Wallet Standard (OWS)](https://github.com/open-wallet-standard/core), designed for the **HashKey Chain Horizon Hackathon 2026**.

Gradience enables users to create passkey-backed identities, manage multi-chain wallets locally, and delegate fine-grained, policy-gated access to AI agents via a standardized MCP (Model Context Protocol) interface.

---

## Core Features

- **OWS-Native Vault**: Genuine integration with `ows-lib` and `ows-signer` for local mnemonic generation, encrypted wallet storage, and multi-chain signing.
- **Policy Engine**: Dual-layer policy system — Gradience smart evaluation + OWS native policy enforcement.
- **API Key Access**: Issue scoped API keys (`ows_key_...`) for agents, with HKDF-based key derivation and built-in key revocation.
- **MCP Server**: A stdio JSON-RPC MCP server exposing `sign_transaction` and `get_balance` tools.
- **EVM RPC Client**: Direct integration with EVM-compatible chains (Base, HashKey Chain, Ethereum).
- **Audit & Integrity**: HMAC-chained audit logs with Merkle tree anchoring for tamper detection.
- **Hybrid Deployment**: Local SQLite for personal use, ready for PostgreSQL cloud sync.

---

## Quick Start

### Prerequisites

- Rust 1.80+ / Cargo
- `sqlx-cli` (optional, for database migrations)

### Build

```bash
cargo build --workspace
```

### Run CLI

```bash
cargo run --bin gradience -- --help

# Create a wallet
cargo run --bin gradience -- agent create --name demo

# Check balance on Base
cargo run --bin gradience -- agent balance <wallet-id> --chain base
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
│   ├── gradience-core/      # Domain logic: OWS adapter, policy engine, audit, signing, RPC
│   ├── gradience-cli/       # Command-line wallet (clap)
│   ├── gradience-db/        # SQLite/PostgreSQL layer with sqlx
│   └── gradience-mcp/       # MCP stdio server and tool handlers
├── contracts/               # Solidity contracts (Merkle anchor)
├── docs/                    # PRD, architecture, technical spec, tests spec
├── web/                     # Web frontend (WIP)
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
- **DB**: `sqlx` + SQLite (local) / PostgreSQL (cloud)
- **Crypto**: `ows-lib` / `ows-signer` (OWS native), `secp256k1`, `rlp`
- **Networking**: `reqwest`, `axum` (future gateway)
- **MCP**: Custom JSON-RPC stdio server

---

## Hackathon

- **Event**: HashKey Chain Horizon Hackathon 2026
- **Deadline**: April 15, 2026
- **Status**: Core platform implemented, 51 tests passing, OWS genuine integration complete.

---

## License

MIT (or as specified by the repository owner)
