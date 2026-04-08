# Solana 签名集成实施计划

## 现状分析

- **ows-lib（底层）**：已支持 Solana 的 `ed25519` 签名、`sign_transaction`、`sign_and_send`、以及 `broadcast_solana`（通过 `sendTransaction` RPC base64 提交）。
- **Gradience-core（中间层）**：`local_adapter.rs` 的 `derive_account()` 对 Solana 返回的是 stub 假地址；`broadcast()` 硬编码走 `EvmRpcClient`。
- **上层（CLI / MCP / API）**：`agent.rs`、`dex.rs`、`pay.rs`、`tools.rs` 等全部默认筛选 EVM 地址、拼接 EVM RLP 交易。

## 目标

让 Gradience 的 **Solana 基础交易链路** 真正跑通：
1. `agent create` 生成**真实的 base58 Solana 地址**
2. `agent balance <wallet-id> --chain solana` 查询 SOL 余额（已有，需确认可用）
3. `agent fund <wallet-id> --chain solana ...` 能构造、签名并广播 Solana 转账交易
4. MCP `sign_transaction` / `get_balance` 能正确识别 Solana 链

## 实施范围（MVP）

### Phase 1：核心适配层修复
- **`gradience-core/src/ows/local_adapter.rs`**
  - `derive_account()`：对 `solana:` 链调用 `ows_lib::derive_address()` 生成真实 base58 地址，替代 stub。
  - `broadcast()`：根据 `chain` 参数判断，Solana 链走 `SolanaRpcClient::send_transaction`，EVM 保持现有逻辑。

- **`gradience-core/src/rpc/solana.rs`**
  - 新增 `send_transaction(&self, signed_tx_bytes: &[u8]) -> Result<String>` 方法，调用 `sendTransaction` RPC（base64 encoding）。
  - 新增辅助方法 `get_latest_blockhash()`，用于构造 Solana 转账交易时需要 recent blockhash。

- **`gradience-core/src/ows/signing.rs`**
  - 新增 `solana_transfer_tx(from: &str, to: &str, lamports: u64, recent_blockhash: &[u8; 32]) -> Vec<u8>`，构造未签名的 Solana 系统转账交易（最小可序列化 message）。
  - 由于 Solana 交易序列化较复杂，**首期方案**：直接引入 `solana-sdk` crate 来做 Transaction 构造和序列化。如果依赖太重，再考虑手写 compact array 序列化。

### Phase 2：CLI 层适配
- **`crates/gradience-cli/src/commands/agent.rs`**
  - `balance()`：已支持 Solana，检查 `is_evm` 逻辑是否把 Solana 正确包含进去。
  - `fund()`：当 `chain == "solana"` 时，走 Solana 交易构造 → `ows_lib::sign_and_send` / `local_adapter.sign_transaction + broadcast` 路径，而不是 EVM RLP。

### Phase 3：MCP 层适配
- **`crates/gradience-mcp/src/tools.rs`**
  - `handle_sign_transaction`：当 `chainId` 为 `solana` 或 `solana:mainnet` 时，不再构建 EVM tx，而是构造 Solana transfer tx 或直接透传用户提供的 base64 tx bytes。
  - `handle_get_balance`：确保 Solana 链调用 `SolanaRpcClient`。

### Phase 4：验证
- 运行 `cargo test --workspace` 保证没有回归。
- 手动验证：`gradience agent create --name sol-demo` 生成的地址是真实 base58 → `gradience agent balance <id> --chain solana` → 待发主网测试转账（可用 devnet 替代）。

## 依赖调整

### 新增 crate
在 `crates/gradience-core/Cargo.toml` 添加：
```toml
solana-sdk = "2.0"
bs58 = "0.5"
```

> 备选：如果 `solana-sdk` 编译时间/体积过大，可降级为只用 `solana-program` + `solana-transaction-status`，或手写最小 message 序列化。首期先用 `solana-sdk` 保证正确性。

### 已有可用依赖
- `ed25519-dalek = "2"`（已在 workspace）— 若需要手动签名可用，但我们主要依赖 `ows-lib` 签名。

## 关键改动清单

| 文件 | 改动 |
|:---|:---|
| `crates/gradience-core/Cargo.toml` | 添加 `solana-sdk`, `bs58` |
| `crates/gradience-core/src/ows/local_adapter.rs` | `derive_account` Solana 真实地址；`broadcast` 分支到 SolanaRpcClient |
| `crates/gradience-core/src/rpc/solana.rs` | 新增 `send_transaction`, `get_latest_blockhash` |
| `crates/gradience-core/src/ows/signing.rs` | 新增 `solana_transfer_tx` 构造辅助函数 |
| `crates/gradience-cli/src/commands/agent.rs` | `fund` 支持 `--chain solana` |
| `crates/gradience-mcp/src/tools.rs` | `sign_transaction` / `get_balance` Solana 分支 |

## 风险与回退

- **编译时间**：`solana-sdk` 较重。若无法接受，回退方案是手写 Solana Legacy Message 的最小序列化（~200 行），不引入大依赖。
- **主网测试**：Solana devnet RPC `https://api.devnet.solana.com` 可免费测试转账，建议演示时使用 devnet。
