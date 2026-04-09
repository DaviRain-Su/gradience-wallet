# Gradience Wallet: 后续任务清单

> 基于 2026-04-09 项目全面 Review 后整理
> 核心目标：将 Tempo MPP 支付推向更多未被官方覆盖的链
>
> **最后更新**: 2026-04-09
> **已完成**: Phase A + B + C1 + D (核心) + E1 + E2 + F (全部)
> - Phase A: 修复 (chain_id, x402, build_batch)
> - Phase B: 通用 EVM 多链 (7 条链)
> - Phase C1: Conflux Core Space pure Rust provider
> - Phase D: TON MPP provider (native TON transfers)
> - Phase E1: MppEscrow 多链部署脚本
> - Phase E2: Session intent 支持 (openSession)
> - Phase F1-F2: SDK + 前端更新
> - Phase F3: 集成测试 + CI/CD (GitHub Actions)
>
> **支持的链**: Tempo, Base, BSC, Conflux eSpace, Conflux Core, XLayer, Arbitrum, Polygon, Optimism, Solana, TON (11 chains)
>
> **待完成**: MppEscrow 合约部署到测试网，awesome-mpp PR，Jetton 支持

---

## 一、背景分析

### MPP 生态现状（截至 2026-04-09）

| 链 | 官方/社区 MPP SDK | 状态 | Gradience 已支持 |
|---|---|---|---|
| Tempo | mpp-rs (官方) | 完善 | Yes |
| Solana | @solana/mpp, solana-mpp, mpp-solana 等 6+ 个 SDK | 完善 | Yes |
| Stellar | stellar-mpp-sdk (官方) | 完善 | No (有 RPC, 无 MPP charge) |
| Sui | @t2000/mpp-sui | 社区 | No |
| TON | mpp-ton (TesseraeVentures) | 社区 | No (有 RPC, 无 MPP charge) |
| MultiversX | mppx-multiversx | 社区 | No |
| XRP Ledger | xrpl-mpp-sdk | 社区 | No |
| Algorand | algorand-mpp-sdk | 社区 | No |
| Avalanche | mpp-avalanche | 社区 | No |
| Lightning | 3+ SDK | 社区 | No |
| **Conflux** | **无** | **空白** | 有 RPC, 无 MPP |
| **BSC (BNB Chain)** | **无** | **空白** | 有 RPC, 无 MPP |
| **XLayer (OKX)** | **无** | **空白** | 有 RPC, 无 MPP |
| **Arbitrum** | **无 (独立 SDK)** | **空白** | 无 |
| **Polygon** | **无 (独立 SDK)** | **空白** | 无 |
| **Optimism** | **无 (独立 SDK)** | **空白** | 无 |

### 核心机会

Conflux、BSC、XLayer 这三条链在 MPP 生态中完全空白。我们的 Gradience 已经有这些链的 RPC 支持和钱包派生能力（`chain.rs` + `ows-lib`），只需要补上 MPP charge provider 实现即可成为这些链的首个 MPP 支付方案。

同时，对于 Arbitrum、Polygon、Optimism 等 EVM L2，虽然理论上可以复用 EVM charge provider，但当前 `GradienceMppProvider` 的 chain_id 解析是硬编码的字符串匹配，无法自动适配新链。

---

## 二、任务清单

### Phase A: 修复现有问题 (1-2 天)

#### A1 — 修复 EVM chain_id 解析硬编码
**优先级**: P0
**预估**: 2h

**问题**: `mpp_client.rs` 中的 chain_id 解析通过 `self.evm_rpc.contains("base")` 等字符串匹配 RPC URL，非常脆弱。

**方案**: 新增 `evm_chain_id: Option<u64>` 字段到 `GradienceMppProvider`，使用 `chain.rs` 中已有的 `evm_chain_num()` 函数统一解析。

**验收**:
- `GradienceMppProvider` 支持显式指定 chain_id
- 无需通过 RPC URL 猜测链

---

#### A2 — 清理残留 x402 引用
**优先级**: P0
**预估**: 1h

**问题**: `payment_tests.rs` 中仍有 x402 测试断言 `PaymentProtocol::from_str("x402")` 等。

**方案**: 清理所有 x402 相关测试代码和注释。

**验收**:
- `rg x402 crates/` 返回 0 结果

---

#### A3 — MppService.build_batch() 实现真正的 multi-transfer 编码
**优先级**: P1
**预估**: 4h

**问题**: `mpp.rs` 中的 `build_batch()` 只是把请求 JSON 序列化，没有编码真正的 multi-transfer calldata。

**方案**:
- EVM: 编码 Multicall3 或逐笔 ERC20 transfer calldata
- Solana: 编码多条 SPL Token transfer instruction

**验收**:
- 返回的 `Vec<u8>` 是有效的链上交易数据
- 单元测试验证编码格式正确

---

#### A4 — FaceID auto-unlock 前端落地
**优先级**: P1
**预估**: 3h

**问题**: `spec-passkey-faceid.md` 中设计的 `autoUnlock()` 流程尚未在 Dashboard 页面实现。

**方案**: 在 `web/app/dashboard/page.tsx` 中集成 `SecureVault.retrieveKey()` 调用，Capacitor 原生环境下自动尝试生物识别解锁。

**验收**:
- iOS/Android 原生 App 打开后自动弹出 FaceID/指纹
- Web 端不受影响，继续使用 passphrase

---

### Phase B: 通用 EVM MPP Provider (2-3 天)

#### B1 — 抽象 EvmMppChargeProvider
**优先级**: P0
**预估**: 4h

**目标**: 从当前 `GradienceMppProvider` 中 EVM charge 逻辑提取为通用组件，支持任意 EVM 链。

**设计**:
```rust
pub struct EvmChargeConfig {
    pub chain_id: u64,
    pub rpc_url: String,
    pub secret: [u8; 32],
    pub gas_limit_native: u64,   // 21000
    pub gas_limit_erc20: u64,    // 65000
}
```

**内容**:
- 将 `mpp_client.rs` 中 `"evm" charge` 分支重构为独立的 `EvmChargeProvider`
- 支持通过 `EvmChargeConfig` 实例化任意 EVM 链
- `GradienceMppProvider` 改为持有 `Vec<EvmChargeConfig>` 支持多链

**验收**:
- 同一个 Provider 可以同时处理 Base、BSC、XLayer 的 charge
- `supports("evm", "charge")` 根据可用的 config 动态返回

---

#### B2 — 注册 BSC (BNB Chain) MPP charge
**优先级**: P0
**预估**: 2h

**内容**:
- 配置 BSC mainnet (chain_id=56) 和 testnet (chain_id=97)
- RPC: `https://bsc-dataseed.binance.org`
- 验证 USDT/USDC ERC20 transfer 可以通过 MPP charge 完成
- 部署 `MppEscrow.sol` 到 BSC testnet（session 支持）

**验收**:
- 端到端测试: MPP 402 challenge (method=evm, chain=56) -> BSC 上 USDT transfer -> credential 返回

---

#### B3 — 注册 Conflux eSpace MPP charge
**优先级**: P0
**预估**: 3h

**内容**:
- Conflux eSpace (chain_id=1030) 是 EVM 兼容的，可以直接复用 `EvmChargeProvider`
- RPC: `https://evm.confluxrpc.com`
- 确认 Conflux eSpace 的 ERC20 transfer gas 参数
- 部署 `MppEscrow.sol` 到 Conflux eSpace testnet
- **额外**: Conflux Core Space (cfx:1029) 使用不同的地址格式 (cfx:xxx)，需要单独处理

**验收**:
- Conflux eSpace MPP charge 端到端通过
- 在 awesome-mpp 提 PR 注册为新的 payment method

---

#### B4 — 注册 XLayer (OKX) MPP charge
**优先级**: P0
**预估**: 2h

**内容**:
- XLayer mainnet (chain_id=196)
- RPC: `https://rpc.xlayer.tech`
- 复用 `EvmChargeProvider`
- 验证 gas 参数和 token 合约地址

**验收**:
- XLayer MPP charge 端到端通过

---

#### B5 — 扩展更多 EVM L2 (Arbitrum, Polygon, Optimism)
**优先级**: P1
**预估**: 3h

**内容**:
- 批量注册:
  - Arbitrum One (chain_id=42161)
  - Polygon (chain_id=137)
  - Optimism (chain_id=10)
- 这些链本身 EVM 兼容，但需要：
  - 在 `chain.rs` 补充 RPC 和 chain_id 映射
  - 在前端 `chains.ts` 确认已有（已有）
  - 验证 gas 参数差异（L2 fee 模型不同）

**验收**:
- 三条 L2 均可通过 `EvmChargeProvider` 完成 MPP charge

---

### Phase C: Conflux Core Space MPP Provider (3-4 天)

#### C1 — Conflux Core Space 签名适配
**优先级**: P1
**预估**: 6h

**背景**: Conflux Core Space 不是标准 EVM，使用 `cfx:` 前缀地址格式和不同的交易结构（多了 `epochHeight`, `storageLimit` 字段）。

**内容**:
- 新建 `ConfluxCoreChargeProvider` 实现 `PaymentProvider`
- 使用 Conflux Core RPC: `eth_getBalance` -> `cfx_getBalance` 等
- 交易构建: 需要 `epochNumber`, `storageLimit` 等额外字段
- 签名: 复用 secp256k1，但地址编码不同

**验收**:
- Conflux Core Space MPP charge 端到端通过

---

#### C2 — 发布 `mpp-conflux` SDK
**优先级**: P1
**预估**: 4h

**目标**: 将 Conflux MPP 支持提取为独立的开源 SDK，发布到 crates.io 和 npm。

**内容**:
- Rust crate: `mpp-conflux` (Conflux eSpace + Core Space charge provider)
- TypeScript package: `@gradience/mpp-conflux` (mppx 插件)
- 提交 PR 到 `awesome-mpp` 注册

**验收**:
- `cargo publish` 成功
- `npm publish` 成功
- awesome-mpp PR merged

---

### Phase D: TON MPP Provider (2-3 天)

#### D1 — 评估 mpp-ton 现状
**优先级**: P2
**预估**: 2h

**内容**:
- Review `TesseraeVentures/mpp-ton` 的实现成熟度
- 评估是否可以直接集成，还是需要自己实现
- 如果可用，在 `GradienceMppProvider` 中添加 TON charge 分支

---

#### D2 — TON MPP charge 实现
**优先级**: P2
**预估**: 6h

**内容**:
- 复用项目中已有的 `rpc/ton.rs` TON RPC client
- 实现 TON native transfer + Jetton (TON 的 ERC20 等价) transfer
- 构建签名逻辑（ed25519）

**验收**:
- TON MPP charge 端到端通过

---

### Phase E: MppEscrow 多链部署 + Session 支持 (2 天)

#### E1 — MppEscrow 合约多链部署
**优先级**: P1
**预估**: 3h

**内容**:
- 将 `contracts/MppEscrow.sol` 部署到：
  - BSC testnet
  - Conflux eSpace testnet
  - XLayer testnet
  - Arbitrum Sepolia
  - Polygon Amoy
- 更新 `contracts/deploy.ts` 支持多链配置

**验收**:
- 每条链上合约地址记录在 `contracts/README.md`

---

#### E2 — Session intent 支持
**优先级**: P2
**预估**: 4h

**内容**:
- 在 `GradienceMppProvider` 中实现 `intent == "session"` 对 EVM 链的支持
- 调用 `MppEscrow.openSession()` 和 `redeemVoucher()`
- 集成到 `MppClient` 的 402 处理流程中

**验收**:
- Session-based MPP 支付在 BSC/Conflux 上端到端通过

---

### Phase F: 前端 + SDK + 测试 (2-3 天)

#### F1 — 前端链选择器增强
**优先级**: P1
**预估**: 3h

**内容**:
- `/ai` 页面的链选择器增加 BSC, Conflux, XLayer 等新链
- 显示每条链的 gas 费估算
- 支持 "auto" 模式自动选最便宜的链

---

#### F2 — Python/TypeScript SDK 更新
**优先级**: P1
**预估**: 3h

**内容**:
- `sdk/python` 和 `sdk/typescript` 增加 MPP charge 方法
- 暴露链选择和 provider 配置

---

#### F3 — 集成测试
**优先级**: P0
**预估**: 4h

**内容**:
- Mock MPP server 测试所有链的 charge flow
- 端到端测试: CLI -> MppClient -> mock 402 -> charge -> credential
- CI 集成

**验收**:
- `cargo test --workspace` 全部通过
- 覆盖 Tempo, EVM (Base/BSC/Conflux/XLayer), Solana 所有 charge path

---

## 三、优先级排序

```
Phase A (修复) ────> Phase B (通用 EVM) ────> Phase C (Conflux Core)
                                          ├──> Phase E (Session)
                                          └──> Phase F (前端/SDK/测试)
                     Phase D (TON) 可并行
```

**最小可行版本 (1 周)**:
- Phase A + Phase B (B1-B4) = 通用 EVM MPP Provider + BSC/Conflux/XLayer
- 这就够把 Tempo MPP 推到 3 条全新的链上，形成差异化

**完整版本 (2-3 周)**:
- 加上 Phase C (Conflux Core) + Phase D (TON) + Phase E + Phase F

---

## 四、交付物

1. **开源 SDK**: `mpp-conflux` (Rust + TypeScript) — 首个 Conflux MPP 支付实现
2. **开源 SDK**: `mpp-bsc` (如果社区需求足够大可以独立发布)
3. **awesome-mpp PR**: 注册 Gradience 作为 BSC/Conflux/XLayer 的 MPP payment method
4. **MppEscrow 部署**: 多链合约地址列表
5. **博客/文档**: 「How to bring MPP to any EVM chain」 技术文章
