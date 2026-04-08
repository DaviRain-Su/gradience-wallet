# TON (The Open Network) Integration Plan

> Status: Draft  
> Target: Phase-2 after Solana (post 2026-04-08)  
> Scope: TON mainnet/testnet wallet lifecycle (derive → balance → fund). Swap as Phase-2b.

---

## 1. Objectives & Scope

Bring **TON** into Gradience as the 3rd native chain family (after EVM and Solana), enabling:

1. **Address Derivation**: A TON address is deterministically generated from the same Passkey seed used for EVM/Solana.
2. **Balance Query**: Real-time TON balance via public HTTP API.
3. **Transfer (Fund)**: Sign and broadcast a `WalletV4R2` external message to send TON.
4. **UI Exposure**: Web Dashboard and Telegram Mini App can select "TON" in chain selectors.
5. **Policy Engine**: TON transfers participate in the same policy evaluation flow as EVM/Solana.

**Out of Scope (Phase-2b)**
- TON DEX swaps (DeDust / STON.fi)
- Jetton (SPL-like token) transfers
- TON DNS / NFT interactions

---

## 2. Why TON is Different (Critical Architecture Notes)

TON diverges significantly from EVM and Solana:

| Dimension | EVM | Solana | TON |
|-----------|-----|--------|-----|
| **Account Model** | EOA (keypair) | Program-derived account | **Smart-contract wallet** (WalletV4R2) |
| **Address** | `keccak256(pubkey)[12:]` | Base58(pubkey) | `workchain + hash(StateInit)` |
| **Tx Format** | RLP-encoded tx | Compact array of signatures + message | **Bag of Cells (BoC)** + TL-B |
| **Nonce** | `nonce` per EOA | `recent_blockhash` (global) | **Seqno** per wallet contract |
| **RPC Style** | JSON-RPC (`eth_*`) | Solana JSON-RPC | TON HTTP API / toncenter JSON-RPC |

**Key implication for Gradience**:  
Deriving a TON address requires knowing the **wallet contract version** (we will standardize on `WalletV4R2`) because the address is the hash of `StateInit{code, data}`, where `data` contains the 256-bit Ed25519 public key. You cannot derive a TON address from the pubkey alone without the contract code cell.

---

## 3. Dependency Selection

### Recommended: `ton` crate (ston-fi/ton-rs)

```toml
# In crates/gradience-core/Cargo.toml
[dependencies]
ton = { version = "0.1", default-features = false }
# Do NOT enable tonlibjson feature to avoid C++ tonlib build deps.
```

**What we use from it**
- `ton_core::types::TonAddress` — parse / format bounceable/non-bounceable addresses.
- `ton_core::cell::TonCell` — build and serialize BoC cells.
- `ton::ton_wallet::TonWallet` (if available without tonlibjson) — construct WalletV4R2 external messages.
- `ton_core::traits::tlb::TLB` — serialize custom structs into cells if `TonWallet` is insufficient.

If `TonWallet` is coupled to `tonlibjson`, we will fall back to:
1. Hardcode the WalletV4R2 `code` cell (hex/boc).
2. Manually build `data` cell = `(seqno: u32, pub_key: [u8; 32])`.
3. Build `StateInit{code, data}` and compute the address hash ourselves via `TonCell::hash()`.
4. Build the `ExternalMessage` body (signed by Ed25519 secret key) manually using `TonCell` builders.

### Alternative (if ston-fi proves too heavy): `anychain-ton`

```toml
anychain-ton = "0.1"
```

This wraps `tonlib-rs` and provides a higher-level wallet abstraction, but it brings in `tonlib-sys` (C++ bindings). We prefer to avoid this unless ston-fi proves incomplete.

---

## 4. File-Level Change List

### Backend Core (`crates/gradience-core`)

| File | Change |
|------|--------|
| `Cargo.toml` | Add `ton = { version = "0.1", default-features = false }` |
| `src/chain.rs` | Add `ton:` entries to `resolve_rpc()`, `chain_id_from_name()`, `is_ton_chain()`, `evm_chain_num()` (or new `ton_chain_num()`) |
| `src/ows/signing.rs` | Add `ton_secret_from_seed()` (Ed25519), `ton_address_from_seed()` (WalletV4R2 StateInit), `build_ton_transfer_tx()` |
| `src/ows/local_adapter.rs` | In `derive_account()`, add `chain.starts_with("ton:")` branch |
| `src/ows/local_adapter.rs` | In `broadcast()`, add `chain.starts_with("ton:")` branch to call new `TonRpcClient` |
| `src/rpc/ton.rs` | **New file**: `TonRpcClient` with `get_balance(address)`, `get_seqno(address)`, `send_boc(boc_bytes)` |

### Backend API (`crates/gradience-api`)

| File | Change |
|------|--------|
| `src/main.rs` | `wallet_balance`: branch for `chain_id.starts_with("ton:")` |
| `src/main.rs` | `wallet_portfolio`: branch for TON (native balance only, token assets Phase-2b) |
| `src/main.rs` | `wallet_fund`: add TON path — find TON address, call `build_ton_transfer_tx`, policy eval, `sign_and_send` |
| `src/main.rs` | `swap_quote` / `wallet_swap`: return `501 Not Implemented` or placeholder for TON (Phase-2b) |

### Frontend

| File | Change |
|------|--------|
| `web/app/dashboard/page.tsx` | Add `"ton"` option to `fundChain` and `swapChain` selectors |
| `web/app/dashboard/page.tsx` | `parseNativeBalance`: handle `chainId.startsWith("ton:")` (divide by 1e9 → TON) |
| `web/app/tg/page.tsx` | Add `"ton"` to `fundChain` selector |
| `web/app/tg/page.tsx` | `formatBalance`: add TON branch |

### CLI

| File | Change |
|------|--------|
| `crates/gradience-cli/src/commands/agent.rs` | `balance` / `fund` support `--chain ton` |
| `crates/gradience-cli/src/cli.rs` | Add `ton` to valid chain args/help text if needed |

### MCP

| File | Change |
|------|--------|
| `crates/gradience-mcp/src/tools.rs` | `handle_sign_transaction` / `handle_sign_and_send` support `ton:` chain prefix |
| `crates/gradience-mcp/src/server.rs` | Tool descriptions updated to mention TON support |

---

## 5. Core Implementation Details

### 5.1 Address Derivation

TON uses Ed25519, same curve as Solana. We can reuse the seed-to-secret pipeline already used for Solana.

```rust
// crates/gradience-core/src/ows/signing.rs

pub fn ton_secret_from_seed(seed: &[u8]) -> [u8; 32] {
    // Same as Solana: use ed25519-dalek SecretKey from seed
    let mut hasher = sha2::Sha512::new();
    hasher.update(seed);
    hasher.update(b"TON"); // chain domain separation
    let hash = hasher.finalize();
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&hash[..32]);
    secret
}
```

Then derive the **WalletV4R2 address**:

```rust
pub fn ton_address_from_seed(seed: &[u8]) -> String {
    let secret = ton_secret_from_seed(seed);
    let public = ed25519_dalek::SigningKey::from_bytes(&secret).verifying_key().to_bytes();

    // WalletV4R2 `data` cell = (seqno: u32, subwallet_id: u32, public_key: [u8;32], plugins: empty dict)
    let data_cell = build_wallet_v4r2_data_cell(0, 0, &public);

    // WalletV4R2 `code` cell = hardcoded BOC hex (official contract)
    let code_cell = TonCell::from_boc_hex(WALLET_V4R2_CODE_BOC).unwrap();

    let state_init = StateInit {
        split_depth: None,
        tick_tock: None,
        code: Some(code_cell.into()),
        data: Some(data_cell.into()),
        library: None,
    };

    let state_init_cell = state_init.to_cell().unwrap();
    let hash = state_init_cell.hash();
    let addr = TonAddress::new(0, &hash); // workchain = 0 (base chain)
    addr.to_base64_urlsafe(false, false) // non-bounceable, non-testnet for display
}
```

> **Important**: `local_adapter.rs` must store the `code` and `data` so that the first external message can carry the `StateInit` for implicit wallet deployment.

### 5.2 Transaction Construction

A TON `WalletV4R2` transfer external message structure:

```
External Message
  ├── dest: wallet_address
  ├── import_fee: 0
  ├── state_init: StateInit (needed until wallet is deployed)
  └── body: SignedInternalMessage
       ├── signature: ed25519(sig)
       ├── wallet_id / subwallet_id
       ├── valid_until
       ├── seqno
       └── actions: InternalMessageInfo[]
            └── (mode, out_msg)
                 └── (bounce, dest, value, body)
```

We need to:
1. Query `seqno` from the contract via RPC.
2. Build the unsigned body cell.
3. Sign the body hash with Ed25519 secret key.
4. Assemble the final external message cell.
5. Serialize to BoC bytes → hex → feed into `ows_lib::sign_transaction` or directly into `sign_and_send`.

`ows_lib::sign_and_send` flow for TON:
- `tx.raw_hex` = `"0x" + hex(BoC bytes of the unsigned external message body)`
- `local_adapter.sign_transaction()` recognizes `chain.starts_with("ton:")`, uses Ed25519 secret to sign the inner body hash, re-assembles the BoC with signature, returns `raw_hex` = signed BoC.
- `local_adapter.broadcast()` recognizes TON, strips `0x`, decodes hex to bytes, calls `TonRpcClient::send_boc(&bytes)`.

### 5.3 RPC Client (`TonRpcClient`)

We will target **toncenter HTTP API v2** (public, no API key required for low rate limits).

```rust
pub struct TonRpcClient {
    base_url: String,
    client: reqwest::Client,
}

impl TonRpcClient {
    pub fn new(mainnet: bool) -> Self { ... }

    pub async fn get_balance(&self, address: &str) -> Result<u128> {
        // GET /api/v2/getAddressInformation?address=<addr>
        // response["result"]["balance"] -> string (nanoton)
    }

    pub async fn get_seqno(&self, address: &str) -> Result<u32> {
        // GET /api/v2/getWalletInformation?address=<addr>
        // response["result"]["seqno"] -> number
    }

    pub async fn send_boc(&self, boc_bytes: &[u8]) -> Result<String> {
        // POST /api/v2/sendBoc
        // body: base64(boc_bytes)
        // returns hash of the message
    }
}
```

Testnet endpoint: `https://testnet.toncenter.com/api/v2`  
Mainnet endpoint: `https://toncenter.com/api/v2`

---

## 6. UI Changes

### Web Dashboard

In `WalletCard`:

```tsx
<select value={fundChain} onChange={...}>
  <option value="base">Base</option>
  <option value="solana">Solana</option>
  <option value="ton">TON</option>
</select>
```

When `fundChain === "ton"`:
- placeholder: `TON address (UQ... or EQ...)`
- default amount: `0.01`
- `parseNativeBalance`: divide by `1e9` → `X TON`

### Telegram Mini App

Same pattern: add `<option value="ton">TON</option>` to TG `WalletCard` fund selector and `formatBalance` logic.

---

## 7. Testing & Validation Plan

### Unit / Integration (Rust)

1. **Address Derivation Test**
   - Given a deterministic seed, `ton_address_from_seed()` produces a known TON address (cross-check with official WalletV4R2 tools).
2. **BoC Serialization Test**
   - Construct a dummy transfer message, serialize to BoC, deserialize back, assert fields match.
3. **RPC Client Test**
   - Mock `toncenter` responses for `get_balance`, `get_seqno`, `send_boc`.
4. **End-to-End (devnet)**
   - Create wallet via API/CLI.
   - Query TON balance on testnet (default 0).
   - Fund the wallet from an external testnet faucet.
   - Use `gradience agent fund --chain ton --amount 0.005` to send TON back.
   - Assert balance changes and tx hash is returned.

### Frontend

- `npx next build` must pass with new `<option value="ton">` and balance formatting.
- Manual UI smoke test: switch chain selector to TON, verify placeholder and amount update.

---

## 8. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `ton` crate API incompatible or requires `tonlibjson` | High | Fork/hand-roll WalletV4R2 message builder using `ton_core::cell::TonCell` only. |
| WalletV4R2 `code` BOC changes / wrong hash | High | Use official FunC-compile BOC from ton-blockchain repo, test address derivation against known vectors. |
| TON requires wallet deployment before first use | Medium | Ensure the first `ExternalMessage` **always includes `StateInit`** (code+data). Later messages can omit it. |
| Toncenter rate limits during demo | Medium | Cache `seqno` for short periods; use testnet for live demo; have pre-funded wallet ready as backup. |
| Policy engine treats `seqno` as nonce but TON has no explicit chain-id in tx | Low | Use CAIP-2 `ton:-1` / `ton:0` in policy context; keep tx value in nanoton for spending limits. |

---

## 9. Task Breakdown (Post-Approval)

1. `[ ]` Add `ton` dependency + `rpc/ton.rs` skeleton.
2. `[ ]` Implement `ton_secret_from_seed` + `ton_address_from_seed` + unit test.
3. `[ ]` Implement `build_ton_transfer_tx` + BOC roundtrip test.
4. `[ ]` Wire `local_adapter.rs` (derive + broadcast).
5. `[ ]` Update `gradience-api` (balance, portfolio, fund) for TON branch.
6. `[ ]` Update CLI (`agent balance/fund --chain ton`).
7. `[ ]` Update MCP tools for TON.
8. `[ ]` Update Web Dashboard chain selectors + balance formatting.
9. `[ ]` Update Telegram Mini App chain selectors + balance formatting.
10. `[ ]` Run full test suite + devnet e2e.
11. `[ ]` Update `docs/demo-script.md` with TON fund step.

---

## 10. References

- [TON Docs — Address Derivation](https://docs.ton.org/foundations/addresses/derive)
- [Wallet Contract V4R2 Source](https://github.com/ton-blockchain/wallet-contract/tree/v4r2-stable)
- [ston-fi/ton-rs](https://github.com/ston-fi/ton-rs)
- [Toncenter API Docs](https://toncenter.com/)
- [anychain-ton](https://crates.io/crates/anychain-ton)
