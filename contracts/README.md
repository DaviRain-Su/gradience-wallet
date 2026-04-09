# Gradience Contracts

Solidity contracts for Gradience Wallet:
- **AuditAnchor** — tamper-proof Merkle root anchoring of audit logs
- **MppEscrow** — MPP session escrow for agent micro-payments (multi-chain)

## Quick Start

### 1. Install dependencies (choose one)

#### Option A: Remix (easiest for developments)
1. Open [Remix IDE](https://remix.ethereum.org)
2. Create file `AuditAnchor.sol` and paste the source
3. Compile with Solidity `0.8.24`
4. Download the `AuditAnchor.json` artifact (contains ABI + Bytecode)
5. Place it at `contracts/out/AuditAnchor.json`

#### Option B: Foundry
```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
cd contracts
forge init --force --no-commit
forge build
```

### 2. Configure environment

```bash
export ANCHOR_RPC_URL="https://hashkeychain-testnet.alt.technology"
export ANCHOR_PRIVATE_KEY="0xYOUR_PRIVATE_KEY"
# optional: export ANCHOR_ARTIFACT="out/AuditAnchor.json"
```

> **Security note:** Never commit private keys. Use a dedicated deployer key with only testnet funds.

### 3. Deploy

```bash
cd contracts
bun install  # or npm install
bun run deploy
```

After deployment succeeds:
```bash
export ANCHOR_CONTRACT_ADDRESS="0xDEPLOYED_CONTRACT_ADDRESS"
```

Then start the API with the env var so the Rust `AnchorService` can broadcast real anchor transactions.

## Contract Overview

- `anchor(root, prevRoot, logStartIndex, logEndIndex, leafCount)` — stores a new Merkle root and links it to the previous anchor.
- `verifyProof(root, leaf, proof)` — on-chain Merkle proof verification using EVM `keccak256`.
- `verifyChainIntegrity(root)` — walks the linked list of anchors to ensure no gaps.
- `getLatestAnchor()` — returns the most recently anchored Merkle root metadata.

## Events

```solidity
event Anchored(
    bytes32 indexed root,
    bytes32 indexed prevRoot,
    uint256 logStartIndex,
    uint256 logEndIndex,
    uint256 leafCount,
    uint256 timestamp,
    address indexed submittedBy
);
```

---

## MppEscrow Multi-Chain Deploy

Deploy the MPP session escrow contract to supported testnets:

```bash
# Deploy to a single chain
bun run deploy-mpp-escrow.ts bsc-testnet

# Deploy to all supported testnets
bun run deploy-mpp-escrow.ts all
```

Supported chains: `bsc-testnet`, `conflux-espace-testnet`, `xlayer-testnet`, `base-sepolia`, `arbitrum-sepolia`, `polygon-amoy`

### Deployed Contracts

| Chain | Chain ID | Contract Address | Tx Hash |
|-------|----------|------------------|---------|
| XLayer Testnet | 1952 | `0x3ca2292b53cbc8f1bff10f3e052eddd7fba86532` | `0xcf651c82fcff709cd0dd71f84bd613745e82fba5abb5dcaf7a6aee5c8c3dd7e2` |
