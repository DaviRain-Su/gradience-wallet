# Jetton (TON Token) Support Design

> 设计文档：为 Gradience MPP provider 添加 Jetton (TON 的 FT 标准) 支持

## 背景

当前 TON MPP provider 只支持 native TON 转账。Jetton 是 TON 的可替代代币标准（类似 ERC-20），需要与 Jetton Master 和 Jetton Wallet 合约交互。

### Jetton 架构

```
┌─────────────────┐
│ Jetton Master   │  ← Token contract (USDT, USDC, etc.)
└────────┬────────┘
         │ owns
         ↓
┌─────────────────┐
│ Jetton Wallet   │  ← User's token balance container
│  (per user)     │
└─────────────────┘
```

每个用户对于每个 Jetton 都有一个独立的 Jetton Wallet 合约地址。

## 核心挑战

### 1. Jetton Wallet 地址解析

要发送 Jetton，需要：
1. 知道 Jetton Master 合约地址（例如 jUSDT: `EQD...`）
2. 调用 Jetton Master 的 `get_wallet_address(owner_address)` 方法
3. 获得发送者的 Jetton Wallet 地址

### 2. Jetton Transfer Message 构建

Jetton 转账通过向**发送者的 Jetton Wallet** 发送特定消息：

```
transfer {
  query_id: u64           // 任意查询 ID
  amount: Coins           // 转账金额（Jetton 单位）
  destination: Address    // 接收者地址
  response_destination: Address  // 响应目标（通常是发送者）
  custom_payload: ?Cell   // 可选自定义数据
  forward_ton_amount: Coins  // 附加的 TON（用于 notification）
  forward_payload: ?Cell  // 转发给接收者的数据
}
```

OP code: `0x0f8a7ea5`

### 3. TL-B 序列化

TON 使用 Type Language - Binary (TL-B) 进行数据序列化，需要正确编码 Cell 结构。

## 实现计划

### Phase 1: Jetton RPC 查询

在 `crates/gradience-core/src/rpc/ton.rs` 中添加：

```rust
impl TonRpcClient {
    /// Get Jetton wallet address for a given owner and Jetton master
    pub async fn get_jetton_wallet_address(
        &self,
        jetton_master: &str,
        owner_address: &str,
    ) -> Result<String> {
        // Call get_method "get_wallet_address" on Jetton Master
        // Parse stack response to extract wallet address
    }

    /// Get Jetton wallet balance
    pub async fn get_jetton_balance(
        &self,
        jetton_wallet: &str,
    ) -> Result<u128> {
        // Call get_method "get_wallet_data" on Jetton Wallet
        // Parse balance from stack
    }
}
```

### Phase 2: Jetton Transfer 构建

在 `crates/gradience-core/src/ows/signing.rs` 中添加：

```rust
pub fn build_jetton_transfer_tx(
    seed: &[u8],
    jetton_master: &str,
    recipient: &str,
    amount_jetton: u128,
    timeout_sec: u32,
) -> Result<String> {
    // 1. Derive sender TON address from seed
    let sender_address = ton_address_from_seed(seed)?;

    // 2. Get sender's Jetton Wallet address (needs async RPC call)
    //    Challenge: signing.rs is sync, RPC is async
    //    Solution: Pass jetton_wallet address as parameter

    // 3. Build Cell with transfer message
    let transfer_cell = build_jetton_transfer_cell(
        query_id: random(),
        amount_jetton,
        recipient,
        sender_address,  // response_destination
        0,               // forward_ton_amount
        None,            // forward_payload
    );

    // 4. Build wallet V4R2 transaction to jetton_wallet
    let tx = build_ton_v4r2_tx(
        seed,
        to: jetton_wallet,
        amount_ton: 50_000_000,  // ~0.05 TON for gas
        payload: transfer_cell,
        timeout_sec,
    );

    Ok(tx.to_boc_base64())
}

fn build_jetton_transfer_cell(
    query_id: u64,
    amount: u128,
    destination: &str,
    response_destination: &str,
    forward_ton_amount: u64,
    forward_payload: Option<Cell>,
) -> Cell {
    // TL-B encoding:
    // transfer#0f8a7ea5 query_id:uint64 amount:Coins destination:MsgAddress
    //   response_destination:MsgAddress custom_payload:(Maybe ^Cell)
    //   forward_ton_amount:Coins forward_payload:(Either Cell ^Cell)
    //   = InternalMsgBody;

    let mut builder = CellBuilder::new();
    builder.store_uint(0x0f8a7ea5, 32);  // OP code
    builder.store_uint(query_id, 64);
    builder.store_coins(amount);
    builder.store_address(parse_ton_address(destination)?);
    builder.store_address(parse_ton_address(response_destination)?);
    builder.store_maybe_ref(None);  // custom_payload
    builder.store_coins(forward_ton_amount);
    builder.store_maybe_ref(forward_payload);
    builder.build()
}
```

### Phase 3: MPP Provider 集成

在 `crates/gradience-core/src/payment/mpp_client.rs` 中更新：

```rust
async fn pay_ton_charge(
    &self,
    charge_req: &ChargeRequest,
    challenge_echo: ChallengeEcho,
) -> Result<PaymentCredential, MppError> {
    let seed = self.ton_seed.as_ref()?;
    let recipient = charge_req.recipient.as_deref()?;
    let amount = charge_req.amount.parse::<u64>()?;
    let currency = &charge_req.currency;

    // Check if Jetton transfer
    let is_jetton = !currency.is_empty()
        && currency != "TON"
        && currency.starts_with("EQ");  // Jetton master address

    let tx_boc = if is_jetton {
        // 1. Get sender's Jetton wallet address
        let rpc = TonRpcClient::new(self.ton_mainnet);
        let sender_addr = ton_address_from_seed(seed)?;
        let jetton_wallet = rpc
            .get_jetton_wallet_address(currency, &sender_addr)
            .await?;

        // 2. Build Jetton transfer tx
        build_jetton_transfer_with_wallet(
            seed,
            &jetton_wallet,
            recipient,
            amount,
            60,
        )?
    } else {
        // Native TON transfer
        build_ton_transfer_tx(seed, recipient, amount, 60)?
    };

    let tx_hash = rpc.send_boc(&tx_boc).await?;
    let payload = PaymentPayload::hash(tx_hash);
    Ok(PaymentCredential::new(challenge_echo, payload))
}
```

## 依赖库

### Option 1: 使用 `ton` crate (当前)

当前项目使用 `ton = "0.1"` crate，但该 crate 功能有限。

### Option 2: 使用 `tonlib` (推荐)

[tonlib-rs](https://github.com/ston-fi/tonlib-rs) 提供完整的 TON SDK，包括：
- Jetton 标准支持
- TL-B 序列化工具
- 完整的 RPC 客户端

```toml
[dependencies]
tonlib = "0.18"
```

### Option 3: 手动实现 (最灵活)

使用现有的 `tlb-ton` crate 手动编码 Jetton transfer Cell。

**推荐**: 先用 Option 3 (手动实现) 验证概念，后续可迁移到 Option 2。

## 测试策略

### 1. Jetton Wallet 地址查询测试

```rust
#[tokio::test]
async fn test_get_jetton_wallet_address() {
    let rpc = TonRpcClient::new(false);  // testnet
    let owner = "EQ...";  // 测试地址
    let jetton_master = "kQD...";  // 测试 Jetton (e.g. jUSDT testnet)

    let jetton_wallet = rpc
        .get_jetton_wallet_address(jetton_master, owner)
        .await
        .unwrap();

    assert!(jetton_wallet.starts_with("EQ") || jetton_wallet.starts_with("UQ"));
}
```

### 2. Jetton Transfer Cell 编码测试

```rust
#[test]
fn test_jetton_transfer_cell_encoding() {
    let cell = build_jetton_transfer_cell(
        123,  // query_id
        1_000_000,  // amount
        "EQRecipient...",
        "EQSender...",
        0,
        None,
    );

    // Verify OP code is present
    let boc = cell.to_boc();
    assert_eq!(&boc[0..4], &[0x0f, 0x8a, 0x7e, 0xa5]);
}
```

### 3. 端到端 Jetton 转账测试（需要 testnet 资金）

```rust
#[tokio::test]
#[ignore]  // 需要手动运行，需要真实 testnet 资金
async fn test_jetton_transfer_e2e() {
    let seed = [/* test seed */];
    let jetton_master = "kQD...";  // testnet jUSDT
    let recipient = "EQ...";
    let amount = 100;  // 0.1 USDT (假设 6 decimals)

    let tx_boc = build_jetton_transfer_tx(
        &seed,
        jetton_master,
        recipient,
        amount,
        60,
    ).unwrap();

    let rpc = TonRpcClient::new(false);
    let tx_hash = rpc.send_boc(&tx_boc).await.unwrap();

    println!("Jetton transfer tx: {}", tx_hash);
}
```

## 已知的 Jetton 示例

### Testnet Jettons

- **jUSDT (testnet)**: `kQD2vDT0RZTGDTjDXXKF6qOFNnEGhCN-XZKhJW3dL8b8m9l4`
- **jUSDC (testnet)**: `kQAiboDEv_qRrcEdrYsNVmEP4T0-6gcE0Tb330m1WdTL9r_M`

### Mainnet Jettons

- **jUSDT (mainnet)**: `EQCxE6mUtQJKFnGfaROTKOt1lZbDiiX1kCixRv7Nw2Id_sDs`
- **jUSDC (mainnet)**: `EQBynBO23ywHy_CgarY9NK9FTz0yDsG82PtcbSTQgGoXwiuA`

## 实现优先级

### P0 (Must Have)

- [ ] RPC: `get_jetton_wallet_address()` 方法
- [ ] Signing: `build_jetton_transfer_cell()` TL-B 编码
- [ ] MPP: `pay_ton_charge()` 中添加 Jetton 分支判断

### P1 (Should Have)

- [ ] 完整的 Jetton transfer 端到端测试
- [ ] Jetton balance 查询 RPC 方法
- [ ] 错误处理：insufficient Jetton balance, invalid Jetton master

### P2 (Nice to Have)

- [ ] 自动 Jetton decimals 查询
- [ ] Jetton metadata 缓存（symbol, name, decimals）
- [ ] Multi-Jetton 批量转账支持

## 参考资料

- [TEP-74: Jetton Standard](https://github.com/ton-blockchain/TEPs/blob/master/text/0074-jettons-standard.md)
- [TON TL-B Schemas](https://github.com/ton-blockchain/ton/blob/master/crypto/block/block.tlb)
- [tonlib-rs Documentation](https://docs.rs/tonlib/)
- [TON Center API v2](https://toncenter.com/api/v2/)
- [Jetton Transfer Example (Python)](https://github.com/ton-blockchain/tonlib-python/blob/master/example/jetton.py)

## 时间估算

- **Phase 1** (RPC 查询): 2-3 小时
- **Phase 2** (Cell 编码): 4-6 小时
- **Phase 3** (MPP 集成): 2-3 小时
- **测试与调试**: 4-6 小时

**总计**: ~12-18 小时开发时间

## 当前状态

- ✅ Native TON transfers working
- 🔄 Jetton support - design phase
- ⏸️ Implementation - pending prioritization

---

> 最后更新: 2026-04-09
