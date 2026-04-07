# Phase 4: Task Breakdown — 任务拆解

> Deadline: 2026-04-15 (HashKey Chain Horizon Hackathon)
> Rule: 每个任务 ≤ 4h，必须有验收标准

---

## 执行原则

1. **P0 = Demo 必备** (没它就没法演示)
2. **P1 = Demo 加分** (有它更完整)
3. **P2 = 锦上添花** (时间够就做)
4. **阻塞关系**: 上层任务依赖下层任务完成

---

## Sprint 0: 基建 (Day 1)

### T0.1 — Cargo workspace 骨架搭建
**优先级**: P0  
**预估**: 2h  
**依赖**: 无

**内容**:
- 补全 4 个 crates 的 `Cargo.toml`
- 创建所有模块文件 (`mod.rs`, `lib.rs`, `main.rs`)
- 确保 `cargo check --workspace` 通过（空项目编译通过）

**验收标准**:
- `cargo check --workspace` 0 error
- `cargo build --workspace` 生成 4 个 binary

---

### T0.2 — DB Migration + sqlx prepare
**优先级**: P0  
**预估**: 3h  
**依赖**: T0.1

**内容**:
- 创建 `migrations/001_initial_schema.sql` (含 15 张表)
- 配置 `sqlx migrate run`
- 运行 `cargo sqlx prepare` 生成离线查询

**验收标准**:
- `sqlx migrate run` 成功创建本地 SQLite 数据库
- `.sqlx/query-*.json` 文件存在

---

## Sprint 1: Core (Day 1-2)

### T1.1 — OWS Adapter 最小实现
**优先级**: P0  
**预估**: 4h  
**依赖**: T0.1

**内容**:
- 实现 `OwsAdapter` trait:
  - `init_vault`
  - `create_wallet`
  - `sign_transaction`
- 对接 `ows-core` crate (v1.2.4)
- 实现 VaultHandle 封装

**验收标准**:
- 单元测试: 创建 wallet → 解锁 vault → sign dummy tx → 签名成功
- 单元测试: 错误 passphrase → `OwsError` 返回

---

### T1.2 — Policy Engine (静态规则)
**优先级**: P0  
**预估**: 4h  
**依赖**: T0.2

**内容**:
- `policy::engine::PolicyEngine::evaluate` 实现
- 支持规则:
  - `chain_whitelist`
  - `spend_limit`
  - `daily_limit`
- `merge_policies_strictest` 实现

**验收标准**:
- 测试: 单条 policy allow → `Decision::Allow`
- 测试: chain 不在白名单 → `Decision::Deny`
- 测试: spend > limit → `Decision::Deny`
- 测试: workspace policy + wallet policy 合并后取最严

---

### T1.3 — Wallet Manager + API Key
**优先级**: P0  
**预估**: 3h  
**依赖**: T0.2, T1.1

**内容**:
- `WalletManager`:
  - `create_wallet` (DB 记录 + OWS 派生)
  - `get_wallet_addresses`
  - `list_wallets`
- `ApiKeyService`:
  - `create_api_key` (生成 `ows_key_...`, 存 SHA-256 hash)
  - `verify_api_key`

**验收标准**:
- 测试: 创建 wallet 后 DB 有 wallet + wallet_addresses 记录
- 测试: 创建 API key 后 raw token 只返回一次
- 测试: 用 token hash 能 lookup 到 wallet_id 和 policies

---

## Sprint 2: CLI + MCP (Day 2-3)

### T2.1 — Gradience CLI (核心命令)
**优先级**: P0  
**预估**: 4h  
**依赖**: T1.1, T1.2, T1.3

**内容**:
- `gradience auth login` (交互式 passphrase 输入)
- `gradience agent create --name <name>`
- `gradience agent list`
- `gradience agent balance <id> --chain <chain>`
- `gradience policy set <wallet> --file <json>`
- `gradience policy list <wallet>`

**验收标准**:
- 端到端: CLI 创建 wallet → CLI 设置 policy → CLI 查询余额
- 所有命令返回清晰的 json 或表格输出

---

### T2.2 — MCP Server (sign_tx + get_balance)
**优先级**: P0  
**预估**: 4h  
**依赖**: T2.1

**内容**:
- `gradience-mcp` 二进制启动
- 实现 2 个 MCP tools:
  - `sign_transaction` (调用 PolicyEngine → OWS sign)
  - `get_balance` (查询 wallet balance)
- 工具通过 `OWS_API_TOKEN` 环境变量认证

**验收标准**:
- MCP Inspector / Claude 能连接并调用 tool
- sign_transaction 调用时 policy deny 会返回 403 错误
- sign_transaction 调用成功时返回 signed tx

---

### T2.3 — EVM RPC + 广播
**优先级**: P0  
**预估**: 3h  
**依赖**: T1.1

**内容**:
- `EvmRpcClient` 实现:
  - `eth_getBalance`
  - `eth_sendRawTransaction`
  - `eth_getTransactionReceipt`
- `RpcManager` 按 CAIP-2 前缀路由
- 支持 chains: BNB chain, Xlayer(), Base (8453), BSC (56), HashKey Chain

**验收标准**:
- 测试: 查询 `eip155:8453` 地址余额成功
- 测试: 广播已签名 dummy tx 到 Base testnet 成功 (或返回 expected RPC error)

---

## Sprint 3: 审计 + 锚定 (Day 4)

### T3.1 — Audit Logger (HMAC 链)
**优先级**: P0  
**预估**: 3h  
**依赖**: T0.2

**内容**:
- `AuditLogger::log(ctx, decision)` 实现
- HMAC 链式 hash 计算 (`prev_hash + content`)
- `spending_trackers` 自动更新

**验收标准**:
- 每次 sign_transaction 后 audit_logs 表新增一行
- 审计日志 current_hash 可验证 (重新计算 == 存储值)
- 修改历史记录后 HMAC 验证失败

---

### T3.2 — Merkle Anchor on HashKey Chain
**优先级**: P1  
**预估**: 4h  
**依赖**: T3.1

**内容**:
- Rust `MerkleTree` 实现
- `AuditAnchor` Solidity 合约部署到 HashKey Chain testnet
- AnchorService: 定时批次提交 Merkle root
- CLI `gradience audit verify --log-id <id>`

**验收标准**:
- 测试: 100 条 audit log → build Merkle tree → root 提交链上成功
- 测试: 生成任意 log 的 Merkle proof → 合约 `verifyProof` 返回 true
- HashKey Chain explorer 上可见锚定交易

---

## Sprint 4: AI Gateway (Day 5)

### T4.1 — AI Gateway 最小实现
**优先级**: P1  
**预估**: 4h  
**依赖**: T1.3

**内容**:
- `ai_balances` 表读写
- `llm_generate` 预扣费逻辑
- Anthropic Provider 适配器 (裸 HTTP client)

**验收标准**:
- 测试: topup 100 USDC → balance 查询返回 100 USDC
- 测试: 调用 `llm_generate` 后 balance 按实际 token 扣减
- 测试: balance < cost 时返回 `InsufficientBalance`

---

### T4.2 — MCP AI Tools
**优先级**: P1  
**预估**: 2h  
**依赖**: T4.1, T2.2

**内容**:
- MCP tool: `llm_generate`
- MCP tool: `ai_balance`
- 策略规则: `max_daily_cost_usdc`, `model_whitelist`

**验收标准**:
- Agent 通过 MCP 调用 `llm_generate` → Gateway 计费 → 返回结果
- 超出 daily limit → Gateway 拦截，返回 policy denied

---

## Sprint 5: 集成 + Demo (Day 6-7)

### T5.1 — Demo 脚本
**优先级**: P0  
**预估**: 3h  
**依赖**: T2.1, T2.2, T2.3, T3.1

**内容**:
- 编写 `scripts/demo.sh` (或 Makefile target)
- 完整流程:
  1. `gradience auth login`
  2. `gradience agent create --name demo-agent`
  3. `gradience policy set demo-agent --file demo-policy.json`
  4. 导出 API token
  5. Claude Code / MCP 连接 → 调用 sign_transaction
  6. 展示 Dashboard 或 CLI audit log

**验收标准**:
- Demo 脚本从头到尾可自动化运行
- 每一步有明确的输出和检查点

---

### T5.2 — Web Dashboard 最小页面
**优先级**: P1  
**预估**: 4h  
**依赖**: 无 (纯前端)

**内容**:
- React 页面: `/wallets` (钱包列表)
- React 页面: `/audit` (审计日志时间线)
- React 页面: `/ai` (AI 余额，可选)

**验收标准**:
- 前端能连接本地 gradience-api (mock 或真实)
- Hackathon 演示时可用 Web 页面做 backdrop

---

### T5.3 — Demo 视频 + Pitch Deck
**优先级**: P0  
**预估**: 4h  
**依赖**: T5.1

**内容**:
- 3 分钟演示视频 (录屏)
- Pitch deck (5-7 页):
  1. 问题: Agent 钱包缺少治理
  2. 方案: Passkey + OWS + Policy Engine
  3. Demo 截图
  4. 技术栈: Rust, OWS, HashKey Chain
  5. 商业模式 / 路线图
  6. 团队

**验收标准**:
- 视频 <= 3min
- Deck 有清晰的"为什么选 HashKey"一页

---

## Sprint 6: 提交 + 缓冲 (Day 8)

### T6.1 — 最终整合测试
**优先级**: P0  
**预估**: 2h  
**依赖**: 全部

**内容**:
- 跑通完整 demo 脚本
- 修复 show-stopper bug
- 更新 README (安装/运行说明)

### T6.2 — Hackathon 提交
**优先级**: P0  
**预估**: 1h  
**依赖**: T6.1

**内容**:
- 提交 GitHub 链接
- 上传 Demo 视频
- 填写项目介绍表

---

## 任务依赖图

```
T0.1 ──┬── T1.1 ──┬── T2.1 ──┬── T2.2 ──┬── T3.1 ──┬── T5.1 ──┬── T5.3
       │          │          │          │          │          │
T0.2 ──┴── T1.2 ──┤          │          │          │          └── T6.1 ── T6.2
       │          │          │          │          │
       └── T1.3 ──┴──────────┘          │          │
                                          │          │
T2.3 ────────────────────────────────────┘          │
                                                     │
T3.2 ───────────────────────────────────────────────┤
                                                     │
T4.1 ── T4.2 ───────────────────────────────────────┤
                                                     │
T5.2 ───────────────────────────────────────────────┘
```

---

## 风险缓冲

| 风险 | 应对 |
|---|---|
| OWS v1.2.4 API 变化 | T1.1 优先做，且加版本锁 |
| HashKey testnet faucet 没水 | 用 BSC testnet fallback |
| MCP 集成复杂 | 先保证 CLI 能用，MCP 可以晚一天 |
| 前端来不及 | Dashboard 用 `asciinema` 录 CLI 代替 |
