# MPP Migration Specification

## Status
Draft — pending implementation

## Background

Gradience currently maintains **two overlapping payment protocol stacks**:
1. **Legacy x402** — custom Rust implementations for EVM (Base), Stellar, plus Node.js bridges (`bridge/base-x402`, `bridge/stellar-x402`)
2. **MPP (Machine Payments Protocol)** — official Tempo/Stripe open standard with active Rust/TypeScript SDKs (`mpp` crate v0.9.1, `mppx` v0.4.11)

The x402 code is brittle (depends on external Node bridges, has outdated alloy APIs) and is being superseded by the MPP ecosystem. Solana, Stellar, Arbitrum, Base, and 15+ other chains already have MPP integrations or SDKs.

**Goal:** consolidate all payment flows under the open MPP standard, remove custom x402 bridging code, and expose a unified multi-chain MPP experience to developers via Gradience Wallet SDK.

---

## Current State Analysis

### What already works (MPP)
- `crates/gradience-core/Cargo.toml` depends on `mpp = { version = "0.9.1", features = ["tempo", "client"] }`
- `gradience-core/src/payment/mpp_client.rs` wraps `mpp::client::PaymentProvider` and implements automatic HTTP 402 handling
- `GradienceMppProvider` currently supports:
  - `tempo` + `charge` (when a Tempo signer is provided)
  - `gradience` + `session` (stub — requires manual credential attachment)

### What is legacy (to be removed)
- `gradience-core/src/payment/x402.rs`
- `gradience-core/src/payment/base_x402.rs`
- `gradience-core/src/payment/stellar_x402.rs`
- `gradience-core/src/payment/protocol.rs` (legacy protocol enums used primarily by x402)
- `gradience-core/src/payment/mpp_session.rs` (commented out — outdated alloy APIs)
- External bridges:
  - `bridge/base-x402/`
  - `bridge/stellar-x402/`
- CLI command `gradience pay x402...` (`gradience-cli/src/commands/pay.rs`)
- MCP payment tools that call legacy x402 paths (`gradience-mcp/src/tools.rs`)
- Unit tests in `gradience-core/src/tests/payment_tests.rs` targeting x402

### Frontend
- The web frontend (`web/`) currently has **no MPP or x402 integration**.
- The old AI Gateway (`/ai`) was a centralized pre-pay form and has been redirected to `/dashboard`.
- We need to introduce `mppx` (official TypeScript SDK) as the client-side MPP handler.

---

## Target Architecture

### High-level flow

```
┌─────────────────────────────────────────────┐
│  Gradience Web Frontend / Agent SDK          │
│  Uses @wevm/mppx to handle 402 automatically │
└─────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────┐
│  Gradience API / Gateway                     │
│  - Accepts MPP credentials                   │
│  - Verifies payment on-chain                 │
│  - Proxies to OpenAI/Anthropic/etc           │
└─────────────────────────────────────────────┘
                      │
┌─────────────────────────────────────────────┐
│  GradienceMppProvider (Rust)                 │
│  - tempo (charge)                            │
│  - evm (charge)   ← NEW                      │
│  - solana (charge/session) ← NEW             │
│  - gradience (session)                       │
└─────────────────────────────────────────────┘
```

### Backend: `GradienceMppProvider`

We extend `impl PaymentProvider for GradienceMppProvider` to support:

| Method | Intent | Chain | Mechanism |
|--------|--------|-------|-----------|
| `tempo` | `charge` | Tempo/Moderato | Native `mpp::client::TempoProvider` |
| `evm` | `charge` | Base, Arbitrum, etc | Alloy signer + ERC20 `transfer` tx |
| `solana` | `charge` | Solana | `solana-mpp` Rust client (or direct RPC SPL transfer) |
| `solana` | `session` | Solana | `solana-mpp` session open/close |
| `gradience` | `session` | Any | Gradience internal ledger (kept as fallback) |

**Notes:**
- The `mpp` crate’s `PaymentChallenge` and `PaymentCredential` are method-agnostic strings. We map `method = "evm" | "solana"` to our own signers.
- For EVM, we do **not** require ERC-4337 AA. We generate a raw ERC20 `transfer` transaction, sign it with an Alloy `PrivateKeySigner`, and submit it via RPC. The MPP server verifies the tx receipt.
- For Solana, we evaluate whether `mpp-rs` already ships a Solana provider. If not, we implement a lightweight `SolanaMppProvider` using `solana-sdk`/`solana-client` to submit SPL token transfers.

### Frontend: `@wevm/mppx`

We add `mppx` to the web workspace and create a thin wrapper:

```ts
// lib/mpp.ts
import { Mppx } from "mppx";

export function createMppClient(accessKeyCredential: string) {
  return Mppx.create({
    credentials: [accessKeyCredential],
    // mppx handles 402 WWW-Authenticate → payment → retry automatically
  });
}
```

Because the Gradience frontend is a static-exported Next.js app, any **server-side** MPP proxy logic (e.g. calling Anthropic with attached `Authorization` header) runs through our API or a Next.js API Route (if we move away from pure static export).

For the MVP, we keep the flow:
1. Frontend calls Gradience API Gateway endpoint (e.g. `POST /api/ai/mpp-generate`)
2. Gateway forwards to the real MPP-enabled AI provider (e.g. `anthropic.mpp.tempo.xyz`)
3. If the provider returns 402, Gateway relays the challenge back to the client
4. Client (web) uses `mppx` to resolve the challenge locally with the wallet’s Access Key
5. Client retries with `Authorization` header containing the MPP credential

An alternative (simpler for PoC) is to have the **Gateway itself** act as the MPP client, using the Rust `MppClient` with `GradienceMppProvider`. The frontend then only needs to trigger the call and poll for results.

**Decision for Phase 1:** use Gateway-as-MppClient so the web frontend stays simple, and expose a REST endpoint that handles all MPP negotiation server-side.

---

## Task Breakdown

### Phase 1: Remove Legacy x402 Debt (1 day)
1. **Delete files**
   - `crates/gradience-core/src/payment/x402.rs`
   - `crates/gradience-core/src/payment/base_x402.rs`
   - `crates/gradience-core/src/payment/stellar_x402.rs`
   - `crates/gradience-core/src/payment/mpp_session.rs` (already commented)
2. **Simplify `protocol.rs`**
   - Remove `PaymentProtocol::X402` variant and all x402-specific types
   - Keep only `PaymentProtocol::Mpp`, `PaymentProtocol::Hsp`, etc.
3. **Update `mod.rs`**
   - Remove `pub mod x402;`, `pub mod base_x402;`, `pub mod stellar_x402;`, `pub mod mpp_session;`
4. **Update CLI (`gradience-cli`)**
   - Remove `x402` subcommand from `cli.rs` and `main.rs`
   - Rewrite `commands/pay.rs` to use `MppClient` instead of legacy x402 bridges
   - Remove all `bridge/...` references
5. **Update MCP (`gradience-mcp`)**
   - Rewrite payment tool logic in `tools.rs` to route through `gradience_core::payment::mpp_client::MppClient`
6. **Update tests**
   - Delete or rewrite `gradience-core/src/tests/payment_tests.rs` to test `MppService` and `GradienceMppProvider`
7. **Delete external bridges (optional but recommended)**
   - `bridge/base-x402/`
   - `bridge/stellar-x402/`
   - *Caution:* verify no other crate depends on them first

### Phase 2: Extend `GradienceMppProvider` (2-3 days)
1. **EVM Charge Provider**
   - Add `evm_signer: Option<PrivateKeySigner>` and `evm_rpc: String` fields
   - Implement `method == "evm" && intent == "charge"` in `pay(...)`
   - Parse `ChargeRequest` from challenge, build ERC20 `transfer` calldata, sign and broadcast via Alloy
   - Return `PaymentCredential` containing tx hash or signature proof
2. **Solana Charge Provider**
   - Evaluate `mpp-rs` for built-in Solana support
   - If absent, add new dependency `solana-sdk` / `solana-client` (or `solana-mpp` Rust wrapper if available)
   - Implement `method == "solana" && intent == "charge"`
   - Build SPL token transfer, sign with Solana keypair, submit via RPC
3. **Payment Router Integration**
   - Hook `PaymentRouter::select_route(...)` into the EVM/Solana paths so we can auto-select the cheapest chain when the challenge does not dictate one
4. **Session Support (future)**
   - Keep `gradience` session as internal ledger
   - Solana `session` intent can reuse `solana-mpp` session primitives later

### Phase 3: Gateway API & Frontend (2 days)
1. **New API endpoint**
   - `POST /api/ai/mpp-generate`
   - Body: `{ wallet_id, provider, model, prompt, max_usd, preferred_chain? }`
   - Use `MppClient` + `GradienceMppProvider` to call MPP-enabled AI provider
   - Stream or return the AI response
2. **Frontend page**
   - Restore `/ai` page as a lightweight MPP AI Gateway UI
   - Show provider list (OpenAI, Anthropic, etc.)
   - Let user select chain preference or leave as "auto"
   - Call `/api/ai/mpp-generate`
3. **Add `mppx` to frontend**
   - `npm install mppx@^0.4.11`
   - Create wrapper in `web/lib/mpp.ts`
   - *Note:* if we use Gateway-as-MppClient, `mppx` may only be needed for advanced users who want direct-browser MPP; the basic UI does not require it.

### Phase 4: Testing & Build (1 day)
1. Run `cargo check` for all crates
2. Run `npm run build` in `web/`
3. Add integration test for `GradienceMppProvider` against a mock MPP server
4. Add unit tests for EVM/Solana transaction builders (with mocked RPC)

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| Deleting `bridge/*` breaks unrelated build scripts | Search `Cargo.toml`, `package.json`, CI config for `base-x402`/`stellar-x402` references before removal |
| `mpp-rs` lacks Solana provider | Fall back to direct `solana-sdk` RPC implementation; keep provider interface generic |
| Alloy version mismatch between `mpp` 0.9.1 and our `alloy` dependency | Pin compatible versions; `mpp` 0.9.1 is very recent (2026-04-07) so should align |
| Frontend Next.js static export cannot run server-side Mppx | Use Gateway-as-MppClient pattern; `mppx` only loaded if we later move to SSR/API Routes |
| MCP tools still need synchronous/blocking payment | Keep `block_on_async` wrapper around `MppClient::send` |

---

## Success Criteria

- [ ] `cargo check` passes with zero x402 references
- [ ] CLI `gradience pay` command uses MPP instead of x402 bridges
- [ ] MCP payment tool uses MPP instead of x402 bridges
- [ ] `GradienceMppProvider` supports `tempo`, `evm`, and `solana` charge methods
- [ ] A test MPP AI provider call succeeds end-to-end on at least one chain (Tempo or Base)
- [ ] Frontend `/ai` page can trigger an MPP-backed AI generation via Gateway

---

## Related Links

- MPP protocol: https://mpp.dev
- MPP spec (IETF): https://paymentauth.org
- `mpp-rs` (Rust SDK): https://github.com/tempoxyz/mpp-rs
- `mppx` (TypeScript SDK): https://github.com/wevm/mppx
- `solana-mpp`: https://github.com/sendaifun/solana-mpp
- `awesome-mpp` registry: https://github.com/mbeato/awesome-mpp
