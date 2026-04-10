# Gradience Wallet — 完整交付任务清单

> 目标：从当前 ~65% 完成度推进到 100%，所有 PRD 功能全部实现并可用。
> 文档维护者：Droid
> 最后更新：2026-04-10

---

## 最新进展（2026-04-10）

### 已完成
- **本地分发链路打通**：单 binary `gradience` 通过 `rust-embed` 完全内嵌 `web/dist`，不再依赖外部文件；GitHub Actions 自动构建 macOS (Apple Silicon) + Linux x86_64 预编译包；Homebrew `brew tap DaviRain-Su/gradience && brew install gradience` 已可用。
- **终端用户文档**：新增 [`GETTING_STARTED.md`](GETTING_STARTED.md)，覆盖下载、安装、启动、首次设置、常见操作、MCP Agent 配置、FAQ。
- **AI Gateway 真实化**：Anthropic Claude 之外新增 OpenAI 支持（`gpt-4o`、`gpt-4o-mini`）；MCP `llm_generate` 和 REST `POST /api/ai/generate` 均可通过 `provider=openai` 调用；pricing seed 已包含 OpenAI 模型定价。
- **TypeScript SDK 发布就绪**：`@gradience/sdk` 升级至 `v0.1.1`，支持 `provider` 参数；新增 `.github/workflows/npm-publish.yml`，配置 `NPM_TOKEN` secret 后即可手动触发发布。

### 待记录 / 待排期
- **LiFi 协议集成**：作为 DEX 聚合器的增强路径，计划替代或补充现有 1inch + Uniswap 方案，实现单签跨链 swap。
- **OpenAPI 自动生成**：基于 axum 路由自动生成 Swagger / OpenAPI 文档，用于外部开发者集成。
- **Agent-First 钱包架构**：已产出 [`docs/agent-first-wallet.md`](docs/agent-first-wallet.md)，需要按 Phase 1 → Phase 2 → Phase 3 逐步落地。

### 下一步建议
1. 配置 GitHub Secret `NPM_TOKEN` 并触发首次 `@gradience/sdk` 发布。
2. 评估 LiFi API/SDK，撰写技术方案后实现 `dex/lifi.rs`。
3. 引入 `utoipa` 或 ` aide` 到 `gradience-api`，生成 OpenAPI spec。
4. **Agent-First Phase 1**：实现 `agent_sessions` 数据层、AgentSessionService、MCP `approval_id` + `check_approval` 闭环。

---

---

## 执行原则

1. **T00 是阻塞项**：必须先完成安全基线修复，才能做后续功能。
2. **每个任务必须包含**：背景、内容、验收标准、预估工时。
3. **每日收尾**：运行 `cargo test --workspace` 和 Demo 脚本，确保无 regression。
4. **完成标记**：`[completed]`，进行中：`[in_progress]`，阻塞待开始：`[pending]`。

---

## 任务状态总览

| ID | 任务 | 优先级 | 状态 | 预估工时 |
|---|---|---|---|---|
| T00 | 安全基线修复 | P0 | pending | 3h |
| T01 | 策略引擎补全 | P0 | pending | 4h |
| T02 | 策略合并补全 | P0 | pending | 2h |
| T03 | 动态策略真实化 | P0 | pending | 4h |
| T04 | Wallet Manager 补全 | P1 | pending | 3h |
| T05 | DEX 重构（Uniswap API） | P0 | pending | 5h |
| T06 | 多链支持（SVM/Stellar/Stylus） | P1 | pending | 6h |
| T07 | AI Gateway 真实化 | P1 | pending | 4h |
| T08 | x402 真实化 | P1 | pending | 4h |
| T09 | MPP 真实化 | P2 | pending | 4h |
| T10 | 前端策略编辑器 | P0 | pending | 4h |
| T11 | 前端审批流 + Agent 监控 | P1 | pending | 3h |
| T12 | API 补全 | P1 | pending | 3h |
| T13 | MCP 补全 | P1 | pending | 3h |
| T14 | 审计补全 | P1 | pending | 3h |
| T15 | 团队预算 | P2 | pending | 3h |
| T16 | 整合测试 + Demo + 文档 | P0 | pending | 持续 |

---

## T00 — 安全基线修复

**优先级**：P0（阻塞）  
**预估工时**：3h  
**依赖**：无

### 背景
`gradience-api/src/main.rs` 中大量硬编码 `"user-1"`，且多个路由没有校验 wallet/workspace 是否属于当前登录用户。这会导致严重的数据越权问题。

### 内容
1. 在 `AppState.sessions` 中存储 `user_id`（从 DB 中查询到的真实 user UUID），而不是只有 `username`。
2. 创建一个 `require_wallet_owner(&state, &token, &wallet_id)` 辅助函数，在以下路由调用：
   - `list_wallets`（替换 `"user-1"`）
   - `create_wallet`
   - `wallet_balance`
   - `wallet_addresses`
   - `wallet_portfolio`
   - `wallet_fund`
   - `wallet_sign`
   - `wallet_swap`
   - `wallet_transactions`
   - `wallet_anchor`
   - `create_api_key`
   - `list_api_keys`
   - `create_policy`
3. 创建 `require_workspace_role(&state, &token, &workspace_id, min_role)` 辅助函数，在 workspace 路由中调用。
4. 修复 `RecoverVerify` 返回的 token 也带上 `user_id`。
5. 前端 `dashboard/page.tsx`：将 `parseInt(hex, 16)` 替换为 `BigInt(hex).toString()`，避免大余额精度丢失。
6. `gradience-mcp/src/tools.rs`：修复 `tokio::runtime::Runtime::new()` 在同步函数中的问题，改用 `tokio::task::block_in_place` 或 `#[tokio::main]` 适配。

### 验收标准
- `rg 'user-1' crates/gradience-api/src/main.rs` 返回 0 结果
- `cargo test --workspace` 0 error
- 用不同账号登录，A 用户无法看到/操作 B 用户的钱包
- Dashboard 能正确显示 > 9e15 wei 的余额

---

## T01 — 策略引擎补全

**优先级**：P0  
**预估工时**：4h  
**依赖**：T00

### 背景
`policy/engine.rs` 只评估了 `ChainWhitelist`、`SpendLimit`、`IntentRisk`，其他规则类型被 `_ => {}` 忽略。`DynamicRisk` 还在 API handler 里手动跑，不在 engine 内。

### 内容
1. 在 `Rule` enum 已定义的以下类型上，在 `PolicyEngine::evaluate` 中实现评估逻辑：
   - `ContractWhitelist` — 检查 `tx.to` 是否在允许列表
   - `OperationType` — 结合 `intent` 判断操作类型是否匹配（若无 intent 则 deny）
   - `TimeWindow` — 检查当前时间是否在允许窗口内（考虑 timezone）
   - `MaxTokensPerCall` — AI Gateway 专用，检查 token 消耗
   - `ModelWhitelist` — AI Gateway 专用，检查模型是否在白名单
2. 将 `DynamicRisk` 评估从 `gradience-api/src/main.rs` 移入 `PolicyEngine::evaluate`:
   - 在 `EvalContext` 中传入 `dynamic_signals: Option<DynamicSignals>`
   - 如果 rule 是 `DynamicRisk`，对比缓存中的 Forta/Chainalysis 值
3. 修复 Warn 聚合逻辑：
   - 不应该遇到第一个 Warn 就立即返回
   - 应该收集所有 deny/warn，如果无 deny 但有 warn 则返回 Warn
4. 在 `EvalResult` 中新增 `dynamic_adjustments` 字段（当前缺失）。

### 验收标准
- 单元测试：`ContractWhitelist` deny 不在白名单的合约地址
- 单元测试：`TimeWindow` deny 当前时间不在窗口内
- 单元测试：`OperationType` allow transfer intent，deny swap intent
- 单元测试：`DynamicRisk` 在 engine 内部评估并返回 deny
- 单元测试：多个 rule 混合时，deny > warn > allow 的优先级正确

---

## T02 — 策略合并补全

**优先级**：P0  
**预估工时**：2h  
**依赖**：T01

### 背景
`policy/merge.rs` 中 `merge_policies_strictest` 只实现了 5 种规则，`monthly_limit`、`contract_whitelist`、`operation_type`、`time_window`、`max_tokens` 被留空。

### 内容
1. 补全 `MergedPolicy` 和 `merge_policies_strictest`：
   - `monthly_limit` → min_amount
   - `contract_whitelist` → intersect_vecs
   - `operation_type` → intersect_vecs
   - `time_window` → narrowest_window（需要写一个辅助函数）
   - `max_tokens` → min_u64
2. 确保 `MergedPolicy` 的所有字段都被填充。

### 验收标准
- `merge_policies_strictest` 测试覆盖所有规则类型
- workspace policy + wallet policy 合并后，各规则均取最严值

---

## T03 — 动态策略真实化

**优先级**：P0  
**预估工时**：4h  
**依赖**：T02

### 背景
`policy/dynamic.rs` 目前是 `mock_fetch_signals`，随机生成 Forta/Chainalysis 值。PRD 要求动态策略基于真实数据。

### 内容
1. 新增 `gradience-core/src/policy/market.rs`：
   - 接入 **CoinGecko API** `https://api.coingecko.com/api/v3/global` 获取市场波动数据
   - 计算 `market_fear_score` (0-100)
2. 新增 `gradience-core/src/policy/forta.rs`：
   - 接入 **Forta Public API** 获取最近 24h 警报数量（或Chainalysis KYT public endpoint）
   - 计算 `threat_score` (0-100)
3. 修改 `dynamic.rs` 的 `RiskSignalCache`：
   - `fetch_signals()` 调用 market.rs + forta.rs
   - 如果 API 失败，fallback 到缓存或保守策略（默认收紧）
   - 环境变量 `USE_MOCK_RISK=1` 保留现有 mock 路径
4. 在 `EvalContext` 和 `EvalResult` 中实现 `DynamicAdjustment`：
   - 例如 market_fear_score > 70 时，动态将所有限额乘以 0.8

### 验收标准
- 启动 API server 后，risk cache 拉取到非随机的真实数值
- `DynamicRisk` rule 基于真实数据正确触发 deny/warn
- 网络断开时，策略评估 graceful fallback（不 panic）
- `cargo test` 通过（含 mock 模式的测试）

---

## T04 — Wallet Manager 补全

**优先级**：P1  
**预估工时**：3h  
**依赖**：T00

### 背景
`wallet/manager.rs` 只有结构定义。Wallet lifecycle（active/suspended/revoked）在 DB 有字段但代码不检查。API key 吊销在签名路径不生效。

### 内容
1. 在 `wallet/manager.rs` 中实现 `WalletService`：
   - `create_wallet` — 封装 DB + OWS 创建流程
   - `suspend_wallet`、`revoke_wallet` — 更新状态
   - `list_wallets_by_owner`
   - `require_status_active` — 任何签名/交易前检查
2. 在 `LocalOwsAdapter::sign_transaction` 和 MCP `sign_transaction` 中调用 `require_status_active`。
3. 在 `ApiKeyService` 中实现真正的吊销检查：
   - `revoke_api_key` 更新 DB `expires_at = NOW()`
   - `verify_key` 检查 `expires_at` 和 DB 状态
   - 所有签名路径在验证 API key 时检查是否 revoked
4. 将 Passkey 恢复邮件从纯 mock 改为可配置：
   - 新增 `EMAIL_PROVIDER=mock|resend` 环境变量
   - mock 模式保持现有 `info!` 日志
   - resend 模式调用 Resend HTTP API

### 验收标准
- 创建钱包后，调用 ` WalletService::require_status_active` 通过
- revoked 状态的钱包无法签名
- revoked 的 API key 无法在 MCP 中通过验证
- `cargo test` 新增 wallet lifecycle 测试

---

## T05 — DEX 重构（接 Uniswap API）

**优先级**：P0  
**预估工时**：5h  
**依赖**：T00

### 背景
当前 DEX `get_quote` 是 mock，Uniswap fallback 只支持 Base，1inch 依赖 API key 且不稳定。PRD 要求真实的 DEX 聚合。

### 内容
1. **接入 Uniswap v3 Quoter 合约**（链上 quote，无需 API key）：
   - `dex/uniswap.rs`：调用 `QuoterV2.quoteExactInputSingle` 获取真实报价
   - 支持 Ethereum、Base、Arbitrum、Optimism、BSC、Polygon
2. **使用 Uniswap v3 SwapRouter02** 作为默认 swap 路径：
   - 替换现有 `encode_exact_input_single` 手动编码为更健壮的 ABI encode
   - 支持多链 router 地址配置
3. **保留 1inch 作为加速路径**：
   - 有 `ONEINCH_API_KEY` 时优先 1inch
   - 无 key 时 fallback 到 Uniswap
4. **接入 PancakeSwap (BSC)**：
   - 新增 `dex/pancakeswap.rs`
   - 调用 PancakeSwap V3 Router
5. **接入 Jupiter (Solana)**：
   - 新增 `dex/jupiter.rs`
   - 调用 `https://quote-api.jup.ag/v6/quote` 和 `swap`
6. **Slippage 保护**：
   - `DexService::build_swap_tx` 增加 `slippage_bps` 参数（默认 50 = 0.5%）
   - 所有 path 根据 slippage 计算 `minAmountOut`
7. **Quote 真实化**：
   - `get_quote` 不再返回 mock，而是调用当前最优 provider 返回真实价格

### 验收标准
- `cargo test`：Uniswap quote 在 Base 测试网/mainnet 成功返回
- 至少一次真实的 Base 链 swap 获得 tx hash
- `slippage_bps` 参数在 API/CLI/MCP 中可配置
- Jupiter quote 在 Solana devnet/mainnet 成功返回（可先 mock 链连接测试 schema）

---

## T06 — 多链支持（SVM / Stellar / Stylus）

**优先级**：P1  
**预估工时**：6h  
**依赖**：T05

### 背景
PRD 承诺多链地址支持（EVM、Solana、BTC、SUI 等），但目前只有 EVM RPC。Architecture 还写了 Stellar 和 Arbitrum Stylus 专项支持。

### 内容
1. **CAIP-2 路由**：
   - 新建 `rpc/multi.rs`：根据 `chain_id` 前缀路由到对应 client
2. **Solana RPC (`rpc/svm.rs`)**：
   - `get_balance(address)` → `getBalance`
   - `broadcast(signed_tx)` → `sendTransaction`
   - `get_token_accounts_by_owner` 获取 SPL token 余额
3. **Stellar RPC (`rpc/stellar.rs`)**：
   - Horizon `accounts/{address}` 获取余额
   - Soroban-RPC `sendTransaction` 广播
   - x402 on Stellar 需要的 auth entry signing 支持
4. **Arbitrum Stylus 支持**：
   - Stylus 使用 WASM 合约但地址格式仍是 EVM 兼容（ED25519 签名可用）
   - 新增 `rpc/stylus.rs`：Arbitrum Sepolia/Stylus testnet RPC
   - `ows/adapter.rs` 或 `ows/local_adapter.rs` 中支持 Stylus 地址派生
   - 部署一个最小 Stylus 合约（或复用已有示例）并演示调用
5. **OWS 本地适配器**：
   - 确保 `local_adapter.rs` 的 `create_wallet` 能为 Solana/Stellar 派生地址
   - 若 `ows_lib` 天然不支持，在 adapter 层做补充派生

### 验收标准
- `RpcManager` 能根据 `eip155:8453`、`solana:5eykt4...`、`stellar:pubnet`、`eip155:421614` 路由到正确 client
- Solana 地址能正确显示余额
- Stellar 能成功广播一笔测试交易
- Stylus 合约交互有一次成功的 tx hash

---

## T07 — AI Gateway 真实化

**优先级**：P1  
**预估工时**：4h  
**依赖**：T00

### 背景
`ai/gateway.rs` 的 `llm_generate` 返回 mock string。PRD 要求真实 LLM 接入。

### 内容
1. 新增 `gradience-core/src/ai/anthropic.rs`：
   - HTTP client 调用 `https://api.anthropic.com/v1/messages`
   - 支持 `claude-3-5-sonnet-20241022`
   - 返回真实 content + input/output tokens
2. 新增 `gradience-core/src/ai/openai.rs`：
   - HTTP client 调用 `https://api.openai.com/v1/chat/completions`
   - 支持 `gpt-4o`
3. 修改 `AiGatewayService::llm_generate`：
   - 根据 `provider` 参数路由到 anthropic.rs 或 openai.rs
   - 失败时 fallback 到 mock（保留开发体验）
4. 修复 `status` 字符串不一致：
   - `LlmResponse` 和 DB `llm_call_logs` 的 enum 统一为 `success | denied | budget_exceeded`
5. 策略引擎中 `ModelWhitelist` 和 `MaxTokensPerCall` 生效后，AI Gateway 需要读取合并后的策略进行拦截（与 T01 联动）。

### 验收标准
- 设置 `ANTHROPIC_API_KEY` 后，调用 `llm_generate` 返回真实 Claude 回复
- token 计数和 cost 计算准确
- balance 不足时返回 `budget_exceeded`
- `cargo test` 通过（mock 模式测试保留）

---

## T08 — x402 真实化

**优先级**：P1  
**预估工时**：4h  
**依赖**：T06（Stellar 支持）

### 背景
`payment/x402.rs` 有结构定义和 ERC-20 settle，但签名是 dummy，没有真实 facilitator，也没有 HTTP 402 协商。

### 内容
1. 实现真实的 x402 facilitator 客户端：
   - `X402Client::negotiate(requirement)` — 向服务端发送 payment requirement
   - `X402Client::verify_receipt(response)` — 验证服务端返回的 receipt
2. 替换 `settle_payment` 中的 `sig = "dummy-signature-for-demo"` 为真实 OWS 签名：
   - 对 EVM：用 `ows_lib::sign_message` 签名 x402 payload
   - 对 Stellar：用 Soroban auth entry signing
3. 新增 HTTP 402 middleware 抽象层：
   - `payment/x402_middleware.rs`：Agent 调用外部 API 时自动拦截 402 响应并触发钱包支付
4. server-sponsored fees 支持：
   - x402 requirement 中标记 `sponsored: true`，钱包不付 gas

### 验收标准
- 能向一个 x402 测试服务端发起支付并获得成功响应
- EVM 上 settle 的交易有真实 tx hash
- Stellar 版本的 auth entry 签名通过验证

---

## T09 — MPP 真实化

**优先级**：P2  
**预估工时**：4h  
**依赖**：无

### 背景
`payment/mpp.rs` 只有结构定义和 JSON 序列化，没有真实 Tempo/Stripe 集成。

### 内容
1. 调研 Tempo/Stripe MPP 当前可用的 SDK 或 API
2. 实现 `MppClient`：
   - `create_session(wallet_id, recipient, amount)` — 创建支付 session
   - `authorize_session(session_id)` — 用户/策略审批后授权
   - `stream_payment(session_id, chunk_amount)` — 流式微支付
3. 将 MPP 集成到 MCP `pay` tool 中：
   - 根据金额/频率自动选择 x402 或 MPP
4. Stellar 路径：
   - 使用 Stellar Asset Contract (SAC) 进行 USDC path payment
   - server-sponsored fees

### 验收标准
- 能创建一个 MPP session 并成功发送一笔或分多笔支付
- MCP `pay` tool 根据策略选择 x402 或 MPP

---

## T10 — 前端策略编辑器

**优先级**：P0  
**预估工时**：4h  
**依赖**：T01

### 背景
Web Dashboard 完全没有策略管理页面。这是 Demo 时最核心的缺失。

### 内容
1. 新建 `web/app/policies/page.tsx`：
   - 列出当前用户所有 wallet 的策略
   - 支持创建新策略：表单包含
     - 策略名称
     - chain_whitelist（多选框：Base, Ethereum, Solana, BSC, Arbitrum…）
     - spend_limit（输入框 + token 选择）
     - daily_limit / monthly_limit
     - contract_whitelist（文本域，每行一个地址）
     - operation_type（多选：transfer, swap, stake, pay）
     - time_window（开始时间、结束时间、时区）
2. 在 `web/app/dashboard/page.tsx` 的 `WalletCard` 上显示策略摘要：
   - "Chain: Base | Limit: 0.01 ETH"
3. 调用现有 `POST /api/wallets/:id/policies` API

### 验收标准
- 用户在 Web 上能创建并保存策略
- Dashboard 正确显示策略摘要
- 策略 deny 的交易在 Dashboard 上有明确提示

---

## T11 — 前端审批流 + Agent 监控

**优先级**：P1  
**预估工时**：3h  
**依赖**：T10

### 内容
1. **审批流完善**：
   - `web/app/approvals/page.tsx`：
     - 将 `request_json` 解析为人类可读文本
     - pending 用黄色 badge，approved 绿色，rejected 红色
   - `dashboard/page.tsx`：顶部显示 pending approval 数量 badge
   - approved 的 warn 交易自动重试（提示用户或后台轮询）
2. **Agent 监控页面**：
   - 新建 `web/app/agents/page.tsx`
   - 展示每个 wallet 的 API Key（= Agent 凭证）
   - 显示最近 5 条 audit log（sign / ai_generate / swap）
   - 显示 Agent 状态（活跃：24h 内有活动 / 静默）

### 验收标准
- 打开 `/approvals` 能直观看到请求内容
- Dashboard 顶部有 pending 数量
- `/agents` 能列出所有 Agent 和最近活动

---

## T12 — API 补全

**优先级**：P1  
**预估工时**：3h  
**依赖**：T05, T07

### 背景
Architecture 中列出的多个 REST 路由缺失。

### 内容
1. 新增路由：
   - `POST /swap/quote` → 调用 `DexService::get_quote`
   - `GET /ai/models` → 返回支持的 provider/model 列表及定价
   - `POST /payments` → 发起 x402/MPP 支付
   - `GET /payments/:id` → 查询支付状态
2. 提取中间件模块：
   - `gradience-api/src/middleware/auth.rs` — JWT/Session 验证 Tower middleware
   - `gradience-api/src/middleware/rate_limit.rs` — 基础限流
   - `gradience-api/src/middleware/cors.rs` — 迁移现有的 inline CORS
3. WebSocket/SSE：
   - 新增 `GET /ws` WebSocket 路由
   - 推送 audit log 更新和 approval 通知

### 验收标准
- `/swap/quote` 返回真实 quote JSON
- `/ai/models` 返回模型列表
- auth middleware 能正确拦截无 token 请求
- WebSocket 能连接并收到测试消息

---

## T13 — MCP 补全

**优先级**：P1  
**预估工时**：3h  
**依赖**：T00, T12

### 内容
1. 新增 MCP tools：
   - `sign_message` — 对任意消息做 OWS 签名
   - `sign_and_send` — 签名并自动广播（ convenience wrapper）
2. API key 校验：
   - MCP tools 在使用 API Key 模式时，校验 token hash 和权限
   - 拒绝 revoked/expired key
3. Warn 重试机制：
   - `sign_transaction` 返回 warn 时，给 agent 一个 `approval_id`
   - 新增 `check_approval` tool，agent 可轮询直到 approved/denied
   - approved 后自动继续签名流程

### 验收标准
- MCP Inspector 能调用 `sign_message`
- 使用 revoked API key 调用任何 tool 返回 401
- warn 交易的 approval 状态可被 agent 查询并自动完成

---

## T14 — 审计补全

**优先级**：P1  
**预估工时**：3h  
**依赖**：无

### 内容
1. **统一 logger**：
   - 删除 `audit/logger.rs` 中的内存 logger（或标记为 deprecated）
   - 所有审计写入统一走 `audit/service.rs` 的 `log_wallet_action`
2. **审计导出**：
   - CLI `gradience audit export --format csv --output ./audit.csv`
   - CLI `gradience audit export --format json --output ./audit.json`
3. **Merkle proof API**：
   - `GET /api/wallets/:id/audit/proof?log_id=123` — 返回该 log 的 Merkle proof
4. **HMAC secret 可配置**：
   - 从 `AUDIT_SECRET` 环境变量读取，不再硬编码

### 验收标准
- `cargo test` 通过
- `gradience audit export --format csv` 生成有效 CSV
- `/api/wallets/:id/audit/proof` 返回可验证的 proof
- 修改 `AUDIT_SECRET` 后旧链验证失败、新链验证成功

---

## T15 — 团队预算

**优先级**：P2  
**预估工时**：3h  
**依赖**：T00, T02

### 内容
1. 新建 `crates/gradience-core/src/team/shared_budget.rs`：
   - `SharedBudgetService`：
     - `allocate_workspace_budget(workspace_id, total_amount, token)`
     - `deduct_from_workspace(workspace_id, amount)` — 任何 Agent 消费时扣减 workspace 总预算
     - `get_remaining_budget(workspace_id)`
2. 修改 `spending_trackers` 表逻辑：
   - 当 wallet 属于 workspace 时，同时更新 wallet + workspace 的 tracker
3. 在 policy engine 中：
   - workspace policy 的 `daily_limit` / `monthly_limit` 实际作用于 workspace 级别 tracker

### 验收标准
- workspace 总预算被任一 Agent 消费后，其他 Agent 可见余额减少
- 超出 workspace 预算时，policy deny

---

## T16 — 整合测试 + Demo + 文档

**优先级**：P0  
**预估工时**：持续  
**依赖**：全部

### 内容
1. **每日运行**：
   - `cargo test --workspace`
   - `cargo clippy --workspace`
   - `cargo fmt --check`
2. **Demo 脚本**：`scripts/demo.sh`
   - Passkey 注册 → 登录 → 解锁
   - 创建 wallet → 查看多链地址
   - Web 上设置 policy
   - 创建 API Key
   - MCP 调用 sign_transaction（allow 和 deny 两条路径）
   - Dashboard 查看 audit log
   - 手动触发 anchor → HashKey Chain 上验证
3. **性能测试**：
   - 策略引擎纯静态评估 benchmark（目标 < 10ms）
4. **文档更新**：
   - README：更新功能列表和运行说明
   - docs/02-architecture.md：补充 Stylus、SVM、Stellar 支持说明
   - docs/04-task-breakdown.md：标记已完成任务

### 验收标准
- Demo 脚本从头到尾无 500 错误
- 策略引擎 latency benchmark 有数据
- 所有变更文档化

---

## Agent-First Wallet 专项

基于 [`docs/agent-first-wallet.md`](docs/agent-first-wallet.md) 的分阶段设计。

| ID | 任务 | 优先级 | 状态 | 预估工时 |
|---|---|---|---|---|
| A01 | Agent Session 数据层 | P0 | pending | 2h |
| A02 | AgentSessionService 核心逻辑 | P0 | pending | 3h |
| A03 | Policy Engine 叠加 Session Policy | P0 | pending | 2h |
| A04 | MCP 闭环审批（approval_id + check_approval） | P0 | pending | 3h |
| A05 | 前端 Agents 页面 | P1 | pending | 3h |
| A06 | EVM Smart Account (ERC-4337) 基础 | P1 | pending | 5h |
| A07 | On-Chain Session Key 模块 | P1 | pending | 4h |
| A08 | Bundler 集成（Base/Ethereum） | P1 | pending | 3h |
| A09 | Solana 可编程钱包 / 委托 | P2 | pending | 4h |

### A01 — Agent Session 数据层
**优先级**：P0（阻塞后续所有 Agent-first 功能）  
**预估工时**：2h  
**依赖**：无

#### 内容
1. 新增 migration：
   - `agent_sessions`：核心会话表
   - `agent_session_limits`：每种 limit 类型一条记录（per_tx / daily / total）
   - `agent_session_usage`：按日/按总会话记录已消耗金额
2. 更新 `gradience-db/src/models.rs` 增加对应 structs。
3. 更新 `gradience-db/src/queries.rs` 增加 CRUD queries：
   - `create_agent_session`
   - `get_agent_session_by_id`
   - `list_agent_sessions_by_wallet`
   - `revoke_agent_session`
   - `deduct_agent_session_budget`

#### 验收标准
- `cargo test --workspace` 通过
- 新表可正常插入、查询、更新

---

### A02 — AgentSessionService 核心逻辑
**优先级**：P0  
**预估工时**：3h  
**依赖**：A01

#### 内容
1. 新建 `gradience-core/src/agent/session.rs`：
   - `AgentSessionService` struct
   - `create_session(wallet_id, name, session_type, boundaries)`
   - `validate_session(session_id, intended_action, chain_id, amount)`
   - `consume_budget(session_id, token, amount_raw)`
   - `revoke_session(session_id)`
2. 对 `SessionType::CapabilityToken` 生成随机的 secure token（类似 JWT 或高熵随机字符串）。
3. 对 `SessionType::OnChainSessionKey` 生成 EOA 密钥对（`alloy::signers::local::PrivateKeySigner`）。

#### 验收标准
- 单元测试：创建 session → 校验通过 → 消耗预算 → 校验失败（超限）
- 单元测试：revoke 后的 session 校验失败

---

### A03 — Policy Engine 叠加 Session Policy
**优先级**：P0  
**预估工时**：2h  
**依赖**：A02

#### 内容
1. 扩展 `EvalContext`：
   ```rust
   pub struct EvalContext {
       // ... existing fields ...
       pub session_id: Option<String>,
   }
   ```
2. `PolicyEngine::evaluate`：如果 `session_id` 存在，读取 `agent_session_limits` 并与 Wallet/Workspace policy 进行 **strictest-merge**。
3. Session 的 `allowed_chains`、`contract_whitelist`、`spend_limits` 都映射为临时 `Policy` rules 参与评估。

#### 验收标准
- 单元测试：Wallet policy allow 但 Session policy deny → 最终结果 deny
- 单元测试：Wallet policy daily_limit = 1 ETH，Session daily_limit = 0.1 ETH → 取 0.1 ETH

---

### A04 — MCP 闭环审批（approval_id + check_approval）
**优先级**：P0  
**预估工时**：3h  
**依赖**：A03

#### 内容
1. 修改 `gradience-mcp/src/tools.rs` `handle_sign_transaction`：
   - `Decision::Warn` 时，调用 `gradience_db::queries::create_policy_approval(...)` 生成审批记录
   - 返回 JSON 中包含 `approval_id`
2. 新增 MCP tool `check_approval`：
   - 参数：`approval_id`
   - 返回：`pending | approved | rejected`
3. 新增 MCP tool `resume_sign_transaction`（或复用 `sign_transaction` 传入 `approval_id`）：
   - 查询 approval 状态为 approved 后，继续执行本地签名并返回 txHash

#### 验收标准
- MCP Inspector 测试：触发 Warn → 返回 approval_id → Dashboard 批准 → MCP `check_approval` 返回 approved → 交易成功签名

---

### A05 — 前端 Agents 页面
**优先级**：P1  
**预估工时**：3h  
**依赖**：A01, A02

#### 内容
1. 新建 `web/app/agents/page.tsx`：
   - 列出当前 Wallet 下的所有 Agent Sessions
   - 显示状态、剩余预算、有效期
   - 提供 Revoke 按钮
2. 新建 `web/app/agents/new/page.tsx`：
   - 表单：名称、选择链、操作类型、限额、有效期
   - 创建成功后展示 token / agent private key（一次性复制）
3. Dashboard 顶部增加 quick link 到 Agents 页面。

#### 验收标准
- 用户能在 Web UI 完成一个 Agent Session 的创建和 Revoke
- 创建成功后 token/key 能被 Agent 使用

---

### A06 — EVM Smart Account (ERC-4337) 基础
**优先级**：P1  
**预估工时**：5h  
**依赖**：A02

#### 内容
1. 新建 `gradience-core/src/aa/mod.rs`：
   - 引入 `alloy` 的 ERC-4337 类型（`PackedUserOperation`, `UserOperation`）
   - 若依赖太重，评估使用 `silius` 或 `ethers-rs` 的 AA 扩展
2. `SmartAccountFactory`：
   - 给定 deployer key，计算 SimpleAccount / Modular Account 的 counterfactual address
3. `AccountDeployer`：
   - 构造并签名 `initCode`，通过 Bundler 发起部署
4. 在 Wallet 表中增加 `is_smart_account` 标识和 `account_factory` 字段。

#### 验收标准
- 单元测试（或 integration test）：给定 seed 计算出确定的 Smart Account 地址
- 使用测试网 Bundler 能成功部署一个 Account（可手动验证 tx hash）

---

### A07 — On-Chain Session Key 模块
**优先级**：P1  
**预估工时**：4h  
**依赖**：A06

#### 内容
1. `SessionKeyValidator`：
   - 封装对 ERC-7579 / ERC-6900 session key module 的调用
   - `encode_add_session_key(agent_pubkey, valid_until, allowed_targets, spend_limit)`
   - `encode_revoke_session_key(agent_pubkey)`
2. `AgentSessionService::create_session`：
   - 当 `session_type == OnChainSessionKey` 且 wallet 是 Smart Account 时，自动构造并签名 `addSessionKey` userOp
   - 提交到 Bundler，等待链上确认后再把 Agent 私钥展示给用户

#### 验收标准
- 成功在 Base Sepolia（或类似测试网）上为一个 Smart Account 添加 session key
- Session key 可以签名 userOp 并被 EntryPoint 接受

---

### A08 — Bundler 集成（Base / Ethereum）
**优先级**：P1  
**预估工时**：3h  
**依赖**：A07

#### 内容
1. `BundlerClient`：
   - 支持 `eth_sendUserOperation`、`eth_getUserOperationReceipt`
2. 默认配置：
   - Base mainnet: `https://bundler.base.org` 或 Pimlico endpoint
   - Base Sepolia: 对应测试网 bundler
3. Paymaster（可选 Phase 2.5）：
   - 接入 Pimlico Verifying Paymaster，允许 Agent 用 USDC 付 gas

#### 验收标准
- `cargo test` 包含一个 integration test，向 Base Sepolia Bundler 发送 userOp 并拿到 tx receipt

---

### A09 — Solana 可编程钱包 / 委托
**优先级**：P2  
**预估工时**：4h  
**依赖**：A08

#### 内容
1. 调研 Solana 上适合 Agent 权限模型的方案：
   - **Squads** 多签程序的 delegate
   - **Solana Smart Wallet**（如 Castle / Snowflake）
   - 或最简单的 **sub-wallet 预授权**：从主钱包派生子密钥对，用户预存 SOL/SPL，Agent 操作子钱包
2. 若选择 sub-wallet：
   - `SolanaAgentWallet`：derive sub-key from wallet seed
   - 用户 approve 时把子钱包地址加入白名单并预存资金
   - Agent 用子钱包私钥直接签名 Solana tx，但无法接触主钱包资金
3. 将 Solana Session 消耗的预算统一汇总到 `agent_session_usage`。

#### 验收标准
- Agent 能使用 Solana sub-wallet 完成一次 SPL token transfer
- 子钱包超限时自动停止（通过 off-chain budget tracking）

---

## 快速导航

- **当前阻塞项**：T00（安全基线）
- **CEO 最关注**：T10（策略编辑器，产品核心卖点）+ T05（DEX 真实 swap）+ T03（动态策略真实数据）
- **技术债务**：T14（统一 logger）+ T01/T02（策略引擎完整补全）
- **Development 加分**：T06（Stylus/Stellar/SVM）+ T08（x402 真实化）
