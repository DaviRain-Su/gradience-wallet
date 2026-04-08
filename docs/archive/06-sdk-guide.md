# SDK Development Guide & Roadmap

Gradience exposes a REST API on `http://localhost:8080` (default). All official SDKs are thin, idiomatic HTTP wrappers around this API. The heavy lifting — cryptography, policy evaluation, HD derivation — stays inside the Rust core (`gradience-core`).

---

## SDK Philosophy

1. **Rust Core First**: All security-critical logic is implemented in `gradience-core` and exposed through `gradience-api` or `gradience-mcp`.
2. **Thin Language Bindings**: SDKs should be lightweight HTTP clients with typed request/response helpers.
3. **Unified Surface**: Every SDK exposes the same `GradienceClient` class/object with identical method names and argument shapes.
4. **Zero Crypto in SDK**: SDKs never handle private keys or mnemonics. Signing is always delegated to the local OWS vault via the API/MCP.

---

## Existing SDKs

### Python SDK (`sdk/python/`)

**Install**
```bash
pip install ./sdk/python
```

**Usage**
```python
from gradience_sdk import GradienceClient

client = GradienceClient("http://localhost:8080", api_token="YOUR_TOKEN")

wallet = client.create_wallet("demo")
balance = client.get_balance(wallet["id"])
swap = client.swap_quote(wallet["id"], {
    "from_token": "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
    "to_token": "0x4200000000000000000000000000000000000006",
    "amount": "1000000",
    "chain": "base",
})
```

### TypeScript SDK (`sdk/typescript/`)

**Install**
```bash
npm install ./sdk/typescript
```

**Usage**
```typescript
import { GradienceClient } from "@gradience/sdk";

const client = new GradienceClient("http://localhost:8080", {
  apiToken: "YOUR_TOKEN",
});

const wallet = await client.createWallet("demo");
const balance = await client.getBalance(wallet.id);
const swap = await client.swapQuote(wallet.id, {
  fromToken: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  toToken: "0x4200000000000000000000000000000000000006",
  amount: "1000000",
  chain: "base",
});
```

---

## Common API Methods

All SDKs should implement the following methods where the host language idioms permit:

| Method | HTTP | Path | Description |
|--------|------|------|-------------|
| `createWallet(name)` | POST | `/api/wallets` | Create a new OWS wallet |
| `listWallets()` | GET | `/api/wallets` | List user wallets |
| `getBalance(walletId)` | GET | `/api/wallets/:id/balance` | Get wallet balance |
| `fundWallet(walletId, to, amount, chain)` | POST | `/api/wallets/:id/fund` | Fund / transfer |
| `signTransaction(walletId, tx)` | POST | `/api/wallets/:id/sign` | Sign a transaction |
| `sendTransaction(walletId, tx)` | POST | `/api/wallets/:id/swap` or custom | Broadcast via API |
| `listTransactions(walletId)` | GET | `/api/wallets/:id/transactions` | Transaction history |
| `swapQuote(walletId, params)` | GET | `/api/swap/quote` | DEX quote |
| `getAiBalance(walletId)` | GET | `/api/ai/balance/:wallet_id` | AI credit balance |
| `aiGenerate(walletId, model, prompt)` | POST | `/api/ai/generate` | LLM generation |
| `createPolicy(walletId, rules)` | POST | `/api/wallets/:id/policies` | Attach wallet policy |
| `createWorkspacePolicy(workspaceId, rules)` | POST | `/api/workspaces/:id/policies` | Attach workspace policy |
| `listPolicies(walletId)` | GET | `/api/wallets/:id/policies` | List policies |
| `exportAudit(walletId, format)` | GET | `/api/wallets/:id/audit/export` | Export audit logs |
| `createWorkspace(name)` | POST | `/api/workspaces` | Create team workspace |
| `listWorkspaces()` | GET | `/api/workspaces` | List workspaces |

---

## SDK Roadmap

| Language | Status | Notes |
|----------|--------|-------|
| **Python** | ✅ v0.1 shipped | `requests`-based, full method coverage |
| **TypeScript** | ✅ v0.1 shipped | `fetch`-based, Node + Browser support |
| **Go** | ✅ v0.1 skeleton | `net/http`, zero external deps |
| **Java** | ✅ v0.1 skeleton | Gradle + OkHttp + Gson |
| **Ruby** | ✅ v0.1 skeleton | Standard `net/http`, zero gems |
| **Kotlin** | 📋 planned | JVM-compatible extension of Java SDK |

### TypeScript SDK Advanced Features

The TypeScript SDK goes beyond a simple HTTP wrapper and provides three advanced layers:

#### 1. React Hooks (`sdk/typescript/src/react/hooks.ts`)

Zero-config React hooks for common wallet operations:

```tsx
import { useWallets, useWalletBalance, useSwapQuote } from "@gradience/sdk";

function WalletDashboard() {
  const { wallets, loading } = useWallets({
    baseUrl: "http://localhost:8080",
    apiToken: "YOUR_TOKEN",
  });
  const walletId = wallets?.[0]?.id;
  const { balance } = useWalletBalance(
    { baseUrl: "http://localhost:8080", apiToken: "YOUR_TOKEN" },
    walletId
  );
  // ...
}
```

Available hooks: `useWallets`, `useWalletBalance`, `useCreateWallet`, `usePolicies`, `useCreatePolicy`, `useSwapQuote`, `useAiGenerate`.

#### 2. EIP-1193 Provider (`sdk/typescript/src/provider.ts`)

Expose a Gradience wallet as a standard Ethereum Provider, compatible with **wagmi**, **viem**, and any EIP-1193 consumer:

```ts
import { GradienceProvider } from "@gradience/sdk";

const provider = new GradienceProvider({
  baseUrl: "http://localhost:8080",
  apiToken: "YOUR_TOKEN",
  walletId: "wallet-id",
  chainId: "0x2105",
});

const accounts = await provider.request({ method: "eth_requestAccounts" });
const signedTx = await provider.request({
  method: "eth_sendTransaction",
  params: [{ to: "0x...", value: "1000" }],
});
```

Supported methods: `eth_requestAccounts`, `eth_accounts`, `eth_chainId`, `eth_sendTransaction`.

#### 3. MCP Client (`sdk/typescript/src/mcp.ts`)

Direct MCP stdio/server integration using `@modelcontextprotocol/sdk`:

```ts
import { GradienceMcpClient } from "@gradience/sdk";

const mcp = await GradienceMcpClient.fromStdio("cargo", [
  "run",
  "--bin",
  "gradience-mcp",
]);

const balance = await mcp.getBalance("wallet-id", "base");
const quote = await mcp.swap("wallet-id", {
  from_token: "0x...",
  to_token: "0x...",
  amount: "1000000",
});
```

This allows LLM agents and MCP hosts to interact with Gradience wallets natively.

---

### Future SDK Priorities

1. **Browser + Node compatibility** via standard `fetch` and `AbortController`.
2. **Typed responses** via auto-generated OpenAPI/JSON-Schema types (or hand-written interfaces).
3. **Streaming support** (WebSocket `pendingApprovals` count).
4. **Wagmi Connector** (`gradienceWallet` connector for one-line wagmi integration).

### Adding a New SDK

1. Create `sdk/<language>/`.
2. Implement a `GradienceClient` equivalent.
3. Provide a `README.md` with install and usage examples.
4. Add a minimal test suite that runs against a local `gradience-api` instance (`cargo run --bin gradience-api`).
5. Update this guide and the root `README.md`.

---

## Error Handling Convention

All SDKs should raise/throw a single `GradienceError` (or language equivalent) with:

- `message: string` — human readable
- `statusCode?: number` — HTTP status when available
- `body?: any` — raw response body for debugging

Example (TypeScript):
```typescript
try {
  await client.createWallet("");
} catch (err) {
  if (err instanceof GradienceError) {
    console.error(err.statusCode, err.message);
  }
}
```

---

## Authentication Pattern

1. User registers / logs in via Web UI (passkey or email+password).
2. Web UI receives a JWT (`token`).
3. SDK is initialized with this token:
   ```typescript
   const client = new GradienceClient(baseUrl, { apiToken: token });
   ```
4. Every request sends `Authorization: Bearer <token>`.
5. API keys (`/api/wallets/:id/api-keys`) are a separate machine-to-machine credential and can also be used as `apiToken`.

---

## Local Development Flow

```bash
# 1. Start the Rust API
cargo run --bin gradience-api

# 2. In another terminal, run SDK tests
cd sdk/python && pytest
# or
cd sdk/typescript && npm test
```

---

## Related

- [`gradience-api`](../crates/gradience-api) — REST API source
- [`gradience-core`](../crates/gradience-core) — Rust domain logic
- [`README.md`](../README.md) — Project overview
