# Gradience Wallet — Remaining Tasks

> 基于已实现功能整理的真正待办清单
> 核心已完成：通用 EVM MPP (7 链)、Conflux Core Rust provider、TON native transfer、Session intent、CI/CD
> 最后更新: 2026-04-09

---

## P0 — 部署与生态注册

- [ ] **部署 MppEscrow 到 6 条 testnet**
  - 命令: `bun run contracts/deploy-mpp-escrow.ts all`
  - 需要: 设置 `ANCHOR_PRIVATE_KEY` 环境变量，确保各测试网有 gas 资金
  - 目标链: Base Sepolia, BSC Testnet, Conflux eSpace Testnet, XLayer Testnet, Arbitrum Sepolia, Polygon Amoy
  - 验收: 将各链合约地址更新至 `contracts/README.md`

- [ ] **提交 awesome-mpp PR**
  - 仓库: https://github.com/mbeato/awesome-mpp
  - 注册 Gradience 为以下链的 MPP payment method:
    - BSC (BNB Chain)
    - Conflux (eSpace + Core)
    - XLayer (OKX)
    - TON
  - 验收: PR merged

---

## P1 — 功能实现

- [ ] **Jetton (TON Token) MPP 支持**
  - 设计文档: `docs/11-jetton-support.md`
  - 主要工作:
    1. `rpc/ton.rs`: 新增 `get_jetton_wallet_address()` 和 `get_jetton_balance()`
    2. `ows/signing.rs`: 实现 `build_jetton_transfer_cell()` TL-B 编码
    3. `payment/mpp_client.rs`: 在 `pay_ton_charge()` 中识别 Jetton Master 地址并走 Jetton 分支
  - 验收: TON MPP charge 能成功支付 testnet jUSDT / jUSDC

---

## P2 — 优化与扩展

- [ ] **TON Mainnet 配置**
  - 当前 AI proxy 和默认配置为 testnet
  - 在 `gradience-ai-proxy/src/handlers.rs` 和生产配置中增加 TON mainnet 选项

- [ ] **提取 `mpp-conflux` 为独立 crate（可选）**
  - 将 Conflux Core Space 的 CIP-37 地址编码 + 签名逻辑独立发布到 crates.io
  - 提升开源生态影响力

---

## 状态速览

| 维度 | 状态 |
|------|------|
| 核心代码 | 64/64 tests passing, CI 就绪 |
| EVM 多链 MPP | 11 链支持 ✅ |
| Conflux Core | 纯 Rust 实现 ✅ |
| TON native | 实现 + 前后端集成 ✅ |
| Session intent | MppEscrow `openSession` 实现 ✅ |
| 待部署 | MppEscrow 6 链 testnet |
| 待实现 | Jetton 支持 |
| 待推广 | awesome-mpp PR |
