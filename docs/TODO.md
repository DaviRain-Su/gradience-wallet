# Gradience Wallet — Remaining Tasks

> 基于已实现功能整理的真正待办清单
> 核心已完成：通用 EVM MPP (7 链)、Conflux Core Rust provider、TON native transfer、Session intent、CI/CD
> 最后更新: 2026-04-09

---

## P0 — 部署与生态注册

- [x] **部署 MppEscrow 到 XLayer testnet**
  - 钱包: `0x067aBc270C4638869Cd347530Be34cBdD93D0EA1`
  - 合约地址: `0x3ca2292b53cbc8f1bff10f3e052eddd7fba86532`
  - 交易哈希: `0xcf651c82fcff709cd0dd71f84bd613745e82fba5abb5dcaf7a6aee5c8c3dd7e2`
  - 已更新至 `contracts/README.md`

- [ ] **部署 MppEscrow 到其他 5 条 testnet**
  - 目标链: Base Sepolia, BSC Testnet, Conflux eSpace Testnet, Arbitrum Sepolia, Polygon Amoy
  - 需要: 获取各测试网的 gas 资金和私钥
  - 验收: 将所有合约地址补充到 `contracts/README.md`

- [x] **提交 awesome-mpp PR**
  - 仓库: https://github.com/mbeato/awesome-mpp
  - PR: https://github.com/mbeato/awesome-mpp/pull/5
  - 注册 Gradience 为以下链的 MPP payment method:
    - BSC (BNB Chain)
    - Conflux (eSpace + Core)
    - XLayer (OKX)
    - TON
  - 验收: PR created, pending merge

---

## P1 — 功能实现

- [x] **Jetton (TON Token) MPP 支持**
  - 设计文档: `docs/11-jetton-support.md`
  - 完成工作:
    1. `rpc/ton.rs`: 新增 `run_get_method()`、`get_jetton_wallet_address()`、`get_jetton_balance()`
    2. `ows/signing.rs`: 实现 `JettonTransferBody` TL-B Cell 编码 + `build_jetton_transfer_tx()`
    3. `payment/mpp_client.rs`: `pay_ton_charge()` 自动识别 Jetton Master 地址并走 Jetton 分支
    4. 单元测试覆盖 Jetton body 编码 和 完整交易 BOC 构建
  - 验收: Jetton transfer 编码通过单元测试，待真实 testnet 资金端到端验证

---

## P2 — 优化与扩展

- [x] **TON Mainnet 配置**
  - `gradience-ai-proxy/src/handlers.rs` 已切换为 `with_ton_mainnet(true)`
  - 生产部署需同步更新环境配置

- [ ] **提取 `mpp-conflux` 为独立 crate（可选）**
  - 将 Conflux Core Space 的 CIP-37 地址编码 + 签名逻辑独立发布到 crates.io
  - 提升开源生态影响力

---

## 状态速览

| 维度 | 状态 |
|------|------|
| 核心代码 | 72/72 tests passing, CI 就绪 |
| EVM 多链 MPP | 11 链支持 ✅ |
| Conflux Core | 纯 Rust 实现 ✅ |
| TON native | 实现 + 前后端集成 ✅ |
| TON Jetton | 编码实现 ✅ (待 testnet 资金 e2e) |
| Session intent | MppEscrow `openSession` 实现 ✅ |
| Policy Engine | 2 个 pre-existing 测试失败已修复 ✅ |
| 待部署 | MppEscrow 6 链 testnet |
| 待合并 | awesome-mpp PR |
| 待排期 | `mpp-conflux` 独立 crate (可选) |
