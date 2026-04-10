# Agent-First Wallet Architecture

> 目标：将 Gradience 从"人操作、Agent 辅助"升级为"Agent 自主执行、人只设边界"的本地优先多链钱包。

---

## 1. 设计原则

1. **Passkey = Root of Trust**
   - 用户只需记住邮箱 + Passkey
   - Passkey 解锁本地加密 vault，vault 内保存 OWS master seed
   - 一个 seed 可衍生多个 Wallet（多链地址集合）

2. **Wallet = 命名空间 + 资金容器**
   - Wallet 是逻辑概念，下辖各链地址
   - 每个 Wallet 可创建多个 **Agent Session**（代理会话）

3. **Agent Session = 有界能力（Bounded Capability）**
   - 每次给 Agent 授权时，明确指定：链范围、操作类型、合约白名单、支出限额、有效期
   - Agent 在边界内自主执行，超出边界必须停下来等人批准

4. **链自适应签名**
   - 在支持智能合约钱包（AA）的链上，使用 **On-Chain Session Key**（Agent 自己签名 userOp）
   - 在不支持 AA 的链上，回退到 **Policy-Gated EOA**（每笔交易仍回 Gradience 后端签名，但策略放宽时自动放行）

---

## 2. 核心概念

### 2.1 Agent Session 类型

```rust
enum SessionType {
    /// 离线能力令牌：用于 MCP / REST API 调用
    /// 每请求都经过 Gradience 后端，由后端强制执行策略
    CapabilityToken,

    /// 链上会话密钥：用于 ERC-4337 等智能合约钱包
    /// Agent 持有私钥，可直接签名 userOp / 交易
    OnChainSessionKey,
}

struct AgentSession {
    id: String,
    wallet_id: String,
    name: String,
    session_type: SessionType,

    // 边界
    allowed_chains: Vec<String>,        // e.g. ["eip155:8453", "solana:5eykt4UsFv7PfaMu"]
    allowed_actions: Vec<ActionType>,   // Transfer | Swap | Stake | Pay
    spend_limits: Vec<SpendLimit>,      // per_tx / daily / total
    contract_whitelist: Option<Vec<String>>,

    // 时间
    expires_at: DateTime<Utc>,

    // 状态
    status: SessionStatus,              // Active | Revoked | Expired
    remaining_budget: HashMap<String, String>, // token -> raw amount
}
```

### 2.2 两种执行模式对比

| 维度 | Capability Token (Phase 1) | On-Chain Session Key (Phase 2+) |
|------|---------------------------|--------------------------------|
| Agent 是否需要访问 Gradience 后端 | **需要**（每笔调用走 MCP/API） | **不需要**（Agent 本地签名后直接发Bundler/RPC） |
| 签名权在哪 | 本地 vault | 智能合约钱包的 session key |
| 失败/越界行为 | 返回错误，触发 `policy_approval` 审批流 | 链上直接 Revert |
| 适用链 | **所有链**（通用回退） | 仅支持 AA 的链（Base、Ethereum、Arbitrum 等） |
| 实现复杂度 | 低（基于现有 Policy Engine） | 中（需要 Bundler、Paymaster、SessionKeyModule） |

---

## 3. UX 流程

### 创建 Agent Session

1. 用户登录（Passkey 解锁 vault）
2. 进入 `/agents` 页面
3. 点击「创建 Agent 会话」
4. 填写边界：
   - 名称（如 "Hardness DeFi Bot"）
   - 选择 Wallet
   - 选择可用链
   - 设置单日限额 / 单笔限额 / 总预算
   - 选择允许操作（transfer / swap / stake）
   - 合约白名单（可选）
   - 有效期（1h / 24h / 7d / 自定义）
5. 系统根据钱包类型生成凭证：
   - **Capability Token**：生成 JWT/API Key，显示给 Agent 配置
   - **On-Chain Session Key**（仅 AA 钱包）：
     - 生成 Agent 专用 EOA 密钥对
     - 向智能合约钱包发起 `addSessionKey` 交易
     - 用户签名确认后，Agent 私钥展示给 Agent
6. Dashboard 实时监控 Agent 的余额消耗、最近操作、剩余预算

### Agent 执行交易

**Capability Token 路径（当前即可实现）**：
```
Agent (Claude/Cursor)
  → MCP sign_transaction
    → Gradience 后端验证 Session Token
      → Policy Engine 叠加 Wallet Policy + Session Policy
        → Allow → 本地 vault 签名 → 广播 → 返回 txHash ✅
        → Warn → 返回 approval_id → Agent 轮询 check_approval → 人批准后自动恢复
        → Deny → 返回错误 ❌
```

**On-Chain Session Key 路径（Phase 2）**：
```
Agent 本地构造 userOp
  → Agent Session Key 签名 userOp
    → 直接提交 Bundler
      → Smart Account 链上校验 session key 权限
        → 通过 → EntryPoint 执行 → 链上确认 ✅
        → 失败（超限）→ 链上 Revert ❌
```

---

## 4. 安全模型

| 风险场景 | 保护机制 |
|---------|---------|
| Agent Session Key 泄露 | **时间 + 预算双限制**：损失被限定在剩余预算和有效期内；用户可随时 Revoke |
| Agent 发起恶意合约调用 | **合约白名单**：只允许与预批准合约交互 |
| Agent 超额支出 | **多层级 spend limit**：per-tx / daily / total，取最严值 |
| Gradience 后端被绕过 | **AA 路径**：链上智能合约强制校验，后端不存在即可执行 |
| Passkey / Vault 泄露 | 与当前相同，需要 vault passphrase；建议用户备份 recovery phrase |

---

## 5. 分阶段实现路线图

### Phase 1 — Capability Token + 闭环审批（1-2 周）
> 目标：在当前 EOA 架构下，让 Agent 能"设定边界后自动跑"。

| 任务 | 内容 |
|------|------|
| DB Schema | 新建 `agent_sessions`、`agent_session_limits`、`agent_session_usage` 表 |
| Domain Model | 实现 `AgentSessionService`：创建、校验、消耗预算、吊销 |
| 替换 API Keys | 将现有的 `api_keys` 升级为 `AgentSession` 的前身，支持边界和预算 |
| Policy Engine 增强 | `EvalContext` 增加 `session_id`，策略合并时叠加 Session Policy |
| MCP 闭环 | `sign_transaction` Warn 时创建 `policy_approval` 并返回 `approval_id`；新增 `check_approval` tool；Agent 可轮询恢复 |
| 前端 Agents 页 | `/agents` 创建/列表/吊销 Agent Session；显示实时消耗 |

### Phase 2 — EVM Smart Account + Session Keys（2-4 周）
> 目标：在 Base/Ethereum 等链上，Agent 可以真正不经过 Gradience 后端独立完成交易。

| 任务 | 内容 |
|------|------|
| AA 选型 | 采用 **ERC-6900 Modular Account** 或 **Coinbase Smart Wallet** 兼容架构 |
| Rust 模块 | 新建 `gradience-core/src/aa/`：
| | - `SmartAccountFactory`：从 seed 计算 counterfactual address |
| | - `SessionKeyModule`：封装 `addSessionKey` / `revokeSessionKey` |
| | - `UserOpBuilder`：构造、hash、签名 user operation |
| Bundler 集成 | 接入 **Pimlico** 或 **Alchemy** Bundler RPC（Base mainnet/testnet） |
| Paymaster（可选） | 支持 ERC-20 Paymaster，让 Agent 用 USDC 支付 gas |
| Wallet 升级UI | 在 Wallet 设置中增加「升级为 Smart Account」按钮 |
| Agent Session 扩展 | 创建 session 时检测 wallet 是否为 Smart Account，若是则走 On-Chain Session Key 路径 |

### Phase 3 — Solana & TON 可编程钱包（1-2 个月）
> 目标：把 Session Key 模型扩展到非 EVM 链。

| 任务 | 内容 |
|------|------|
| Solana | 调研 **Squads**、**Solflare委托** 或 SPL 程序级权限控制；实现 agent sub-key 签名 + 程序校验 |
| TON | 实现子钱包（sub-wallet）或智能合约钱包的受限权限模块 |
| 跨链统一抽象 | `gradience-core/src/aa/multi/`：统一 `AgentSigner` trait，链无关的 session 创建接口 |

---

## 6. 与现有代码的关系

| 现有模块 | 改造方式 |
|---------|---------|
| `gradience-core/src/ai/gateway.rs` | Agent Session 的 `llm_generate` 调用也应走 session 限额 |
| `gradience-core/src/policy/engine.rs` | 增加 `session_policy` 叠加逻辑 |
| `gradience-mcp/src/tools.rs` | `sign_transaction` 支持 `approval_id` + `check_approval` |
| `gradience-api/src/handlers.rs` | 新增 `/api/agents/sessions` CRUD 路由 |
| `web/app/ai/page.tsx` | 可扩展为 `/agents` 管理入口 |
| `gradience-ai-proxy` | AI Proxy 的 `verify_api_key` 应校验 Agent Session 边界 |

---

## 7. 快速验证 checklist

- [ ] 创建 Agent Session（Capability Token）后，Agent 可在限额内自动 swap
- [ ] 超出单笔限额时，交易被 Warn，返回 `approval_id`
- [ ] 用户在 Dashboard 点 Approve 后，Agent `check_approval` 成功，交易自动执行
- [ ] Smart Account 部署后，Agent Session Key 可独立签名 userOp 并发上 Base
- [ ] Agent Session 被 Revoke 后，新的交易立即失败
