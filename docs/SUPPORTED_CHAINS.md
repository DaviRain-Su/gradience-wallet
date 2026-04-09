# Supported Chains

Gradience Wallet provides multi-chain support across various blockchain networks for wallet management, MPP payments, and transaction signing.

## MPP (Machine Payments Protocol) Supported Chains

Gradience implements the [Machine Payments Protocol](https://mpp.dev/) for automated agent-to-service micropayments. The following chains are currently supported for MPP charge payments:

### Production Ready (11 Chains)

| Chain | Network | Method | Currency Support | Status |
|-------|---------|--------|------------------|--------|
| **Tempo** | Mainnet | `tempo` | Native TEMPO, ERC-20 | ✅ Production |
| **Base** | Mainnet (8453) | `evm` | Native ETH, ERC-20 (USDC) | ✅ Production |
| **BSC (BNB Chain)** | Mainnet (56) | `evm` | Native BNB, BEP-20 | ✅ Production |
| **Conflux eSpace** | Mainnet (1030) | `evm` | Native CFX, ERC-20 | ✅ Production |
| **Conflux Core** | Mainnet (1029) | `cfx` | Native CFX, CRC-20 | ✅ Production |
| **XLayer (OKX)** | Mainnet (196) | `evm` | Native OKB, ERC-20 | ✅ Production |
| **Arbitrum** | One (42161) | `evm` | Native ETH, ERC-20 | ✅ Production |
| **Polygon** | Mainnet (137) | `evm` | Native MATIC, ERC-20 | ✅ Production |
| **Optimism** | Mainnet (10) | `evm` | Native ETH, ERC-20 | ✅ Production |
| **Solana** | Mainnet | `solana` | Native SOL, SPL tokens | ✅ Production |
| **TON** | Testnet | `ton` | Native TON | ✅ Testnet |

### Session Support (Escrow-based)

Session intent allows pre-funding a session budget for multiple API calls without per-request payments:

| Chain | MppEscrow Contract | Status |
|-------|-------------------|--------|
| Base Sepolia | `0x...` (to deploy) | 🔄 Pending deployment |
| BSC Testnet | `0x...` (to deploy) | 🔄 Pending deployment |
| Conflux eSpace Testnet | `0x...` (to deploy) | 🔄 Pending deployment |

## Wallet Signing Support

Gradience supports account derivation and transaction signing for additional chains beyond MPP:

| Chain | Derivation Path | Signing Support | Notes |
|-------|----------------|-----------------|-------|
| Ethereum | `m/44'/60'/0'/0/0` | ✅ EIP-155 | Full EVM support |
| Bitcoin | `m/44'/0'/0'/0/0` | ⏸️ Planned | BIP-141 SegWit |
| Stellar | `m/44'/148'/0'` | ✅ Ed25519 | Native integration |
| XRPL | `m/44'/144'/0'/0/0` | ✅ secp256k1 | XRP Ledger support |

## Chain-Specific Features

### Conflux Core Space

Conflux Core is NOT EVM-compatible and requires special handling:

- **Address Format**: CIP-37 base32 (`cfx:...` / `cfxtest:...`)
- **Transaction Fields**: `epochHeight`, `storageLimit`, `chainId`
- **RPC Methods**: `cfx_*` instead of `eth_*`
- **Pure Rust Implementation**: No Node.js dependencies

**Example Conflux Core address:**
```
cfx:aak2rra2njvd77ezwjvx04kkds9fzagfe6ku8scz91
```

### TON (The Open Network)

TON uses Ed25519 keypairs and a different transaction structure:

- **Wallet Type**: V4R2 (latest standard wallet contract)
- **Address Format**: Bounceable base64 (`EQ...`, `UQ...`)
- **Transaction Format**: Bag of Cells (BOC) serialization
- **RPC**: TON Center API v2
- **Jetton Support**: Coming soon (TON's token standard)

**Example TON address:**
```
EQD7RMTgzvcyxNNLmK2HdklOvFE8_KNMa-btKZ0dPU1UsqfC
```

### Solana

Solana uses Ed25519 for signing and a unique account model:

- **Derivation**: BIP44 `m/44'/501'/0'/0`
- **SPL Token Support**: Full support for SPL token transfers
- **Recent Blockhash**: Required for transaction validity
- **Multi-instruction**: Batch transfers supported via `build_batch()`

## Network Selection Strategy

When an agent makes an MPP payment, Gradience automatically:

1. **Checks Challenge Method**: Server specifies `evm`, `solana`, `ton`, `cfx`, etc.
2. **Chain Hint**: If `methodDetails.chainId` is present, uses that specific chain
3. **Auto-Routing**: Otherwise, picks the first available configured chain for that method
4. **Cost Optimization**: Future: Select cheapest chain based on gas prices

## Adding New Chains

### EVM-Compatible Chains

To add a new EVM chain to MPP support:

1. Register in `chain.rs`:
   ```rust
   pub fn evm_chain_num(chain: &str) -> Option<u64> {
       match chain {
           "new-chain" => Some(1234),
           // ...
       }
   }
   ```

2. Add to MPP provider config:
   ```rust
   mpp_provider = mpp_provider.with_evm_chain(
       EvmChargeConfig::new(1234, "https://rpc.newchain.io", secret)
   );
   ```

3. Update SDK chain lists in `sdk/typescript/src/types.ts` and `sdk/python/gradience_sdk/client.py`

### Non-EVM Chains

For non-EVM chains (like TON, Solana, Conflux Core):

1. Implement RPC client in `crates/gradience-core/src/rpc/`
2. Implement signing functions in `crates/gradience-core/src/ows/signing.rs`
3. Create MPP charge provider method in `mpp_client.rs`
4. Register in `supports()` and `pay()` dispatch

See `09-ton-integration.md` for detailed TON implementation example.

## Testnet Support

All chains support testnet configurations:

- **EVM Chains**: Set `chain_id` to testnet ID (e.g., Base Sepolia = 84532)
- **Solana**: `https://api.testnet.solana.com`
- **TON**: `GradienceMppProvider::with_ton_mainnet(false)` for testnet
- **Conflux**: network_id = 1 for testnet, 1029 for mainnet

## References

- [MPP Specification](https://mpp.dev/)
- [awesome-mpp Chain Directory](https://github.com/mbeato/awesome-mpp)
- [Conflux CIP-37 Address Format](https://github.com/Conflux-Chain/CIPs/blob/master/CIPs/cip-37.md)
- [TON Documentation](https://ton.org/docs/)
- [EIP-155: Simple replay attack protection](https://eips.ethereum.org/EIPS/eip-155)
