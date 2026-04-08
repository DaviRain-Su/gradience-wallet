# Gradience TypeScript SDK

TypeScript SDK for interacting with the Gradience Wallet API. Works in both Node.js and modern browsers.

## Install

```bash
npm install ./sdk/typescript
```

## Usage

```typescript
import { GradienceClient } from "@gradience/sdk";

const client = new GradienceClient("http://localhost:8080", {
  apiToken: "YOUR_TOKEN",
});

const wallet = await client.createWallet("demo");
console.log(wallet.id);

const balance = await client.getBalance(wallet.id);
console.log(balance);

const quote = await client.swapQuote(wallet.id, {
  fromToken: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  toToken: "0x4200000000000000000000000000000000000006",
  amount: "1000000",
  chain: "base",
});
console.log(quote.to_amount);

const ai = await client.aiGenerate({
  walletId: wallet.id,
  model: "claude-3-5-sonnet",
  prompt: "Summarize DeFi trends",
});
console.log(ai.text);
```

## React Hooks

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

  return (
    <div>
      <h1>{walletId}</h1>
      <pre>{JSON.stringify(balance, null, 2)}</pre>
    </div>
  );
}
```

## EIP-1193 Provider (wagmi / viem compatible)

```ts
import { GradienceProvider } from "@gradience/sdk";

const provider = new GradienceProvider({
  baseUrl: "http://localhost:8080",
  apiToken: "YOUR_TOKEN",
  walletId: "wallet-id",
  chainId: "0x2105", // Base
});

const accounts = await provider.request({ method: "eth_requestAccounts" });
const signedTx = await provider.request({
  method: "eth_sendTransaction",
  params: [{ to: "0x...", value: "1000", data: "0x" }],
});
```

## MCP Client

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

## Build

```bash
npm run build
```

## API Coverage

- **REST Client**: Wallets, Transactions, DEX, AI Gateway, Policies, Audit, Workspaces
- **React Hooks**: `useWallets`, `useWalletBalance`, `useCreateWallet`, `usePolicies`, `useSwapQuote`, `useAiGenerate`
- **EIP-1193 Provider**: `eth_requestAccounts`, `eth_accounts`, `eth_chainId`, `eth_sendTransaction`
- **MCP Client**: `getBalance`, `signTransaction`, `signMessage`, `swap`, `pay`, `llmGenerate`, `aiModels`

See [`docs/06-sdk-guide.md`](../../docs/06-sdk-guide.md) for the full SDK roadmap.

## License

MIT
