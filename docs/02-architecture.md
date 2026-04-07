# Phase 2: Architecture — 架构设计

> Project: Gradience Wallet — Agent 钱包编排平台
> Status: Draft
> Date: 2026-04-07
> Input: [01-prd.md](01-prd.md) v2.0
> Dependency: [appendix-policy-engine-draft.md](appendix-policy-engine-draft.md), [appendix-dynamic-policy-intent.md](appendix-dynamic-policy-intent.md)

---

## 1. 系统边界

### 1.1 系统内

| 组件 | 职责 |
|---|---|
| **Web Dashboard** | 用户身份认证后的可视化管理界面 |
| **CLI** | 开发者快速操作的本地命令行工具 |
| **gradience-core** | 核心业务逻辑：钱包、策略、DEX、支付、审计 |
| **API Server** | Web Dashboard 和外部调用的 HTTP 接口 |
| **MCP Server** | Agent 通过 MCP 协议访问钱包的服务 |
| **SDK (NAPI/PyO3)** | 第三方语言集成 Gradience 核心能力 |
| **本地 SQLite 数据库** | 策略、审计、额度追踪等状态存储 |

### 1.2 系统外

| 系统 | 交互方式 | 说明 |
|---|---|---|
| **OWS (ows-core)** | Rust crate 直接依赖 | 加密存储 + 签名执行 |
| **链上 RPC** | JSON-RPC HTTP | 广播交易、查询余额/状态 |
| **Passkey/WebAuthn** | 浏览器 API + `webauthn-rs` 验证 | 身份认证 |
| **x402 服务** | HTTP API | Agent 自动支付 API 费用 |
| **HashKey Chain** | EVM RPC (OP Stack) | Merkle 审计日志锚定 + 部署环境 (Hackathon)
| **MPP (Tempo)** | HTTP + gRPC | 高频微支付 |
| **Forta/Chainalysis** | HTTP/GraphQL API | 动态策略风险信号 |
| **DEX 聚合器** | HTTP API (1inch/Jupiter/Cetus/PancakeSwap/1inch X Layer) | 获取最优交易路由 |
| **SMTP 服务** | HTTP API (Resend/SendGrid) | 邮件通知 |

---

## 2. 模块架构

### 2.1 Cargo Workspace 结构

```
gradience-wallet/
├── Cargo.toml                    # workspace 定义
├── crates/
│   ├── gradience-core/           # 核心库 —— 所有业务逻辑
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs            # 公共导出
│   │   │   ├── config.rs         # 配置管理
│   │   │   ├── error.rs          # 统一错误类型
│   │   │   ├── identity/         # 身份管理
│   │   │   │   ├── mod.rs
│   │   │   │   ├── passkey.rs    # WebAuthn/Passkey 会话
│   │   │   │   ├── api_key.rs    # API Key 生成/验证
│   │   │   │   └── session.rs    # 认证会话管理
│   │   │   ├── wallet/           # Agent 钱包管理
│   │   │   │   ├── mod.rs
│   │   │   │   ├── hd.rs         # HD 密钥派生
│   │   │   │   ├── manager.rs    # 钱包 CRUD
│   │   │   │   ├── balance.rs    # 多链余额聚合
│   │   │   │   └── lifecycle.rs  # active/suspended/revoked
│   │   │   ├── policy/           # 策略引擎
│   │   │   │   ├── mod.rs
│   │   │   │   ├── engine.rs     # 评估引擎 (核心)
│   │   │   │   ├── static_rules/ # 静态规则
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── spend_limit.rs
│   │   │   │   │   ├── daily_limit.rs
│   │   │   │   │   ├── monthly_limit.rs
│   │   │   │   │   ├── chain_whitelist.rs
│   │   │   │   │   ├── contract_whitelist.rs
│   │   │   │   │   ├── operation_type.rs
│   │   │   │   │   └── time_window.rs
│   │   │   │   ├── dynamic/      # 动态策略
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── reputation.rs
│   │   │   │   │   ├── market_risk.rs
│   │   │   │   │   ├── behavior_profile.rs
│   │   │   │   │   └── signal_cache.rs
│   │   │   │   ├── intent/       # 交易意图分析
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── parser.rs     # Intent 解析
│   │   │   │   │   ├── matcher.rs    # 模板匹配
│   │   │   │   │   └── risk_scorer.rs
│   │   │   │   └── merge.rs      # 策略合并 (取最严)
│   │   │   ├── dex/              # DEX 聚合
│   │   │   │   ├── mod.rs
│   │   │   │   ├── router.rs     # 最优路径选择
│   │   │   │   ├── providers/    # DEX 提供商适配器
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── uniswap.rs      # EVM (Eth/Base/Arb/Polygon)
│   │   │   │   │   ├── pancakeswap.rs  # BNB Chain (BSC)
│   │   │   │   │   ├── jupiter.rs      # Solana
│   │   │   │   │   └── cetus.rs        # Sui
│   │   │   │   └── slippage.rs   # 滑点保护
│   │   │   ├── payment/          # 支付协议
│   │   │   │   ├── mod.rs
│   │   │   │   ├── x402.rs       # x402 适配器
│   │   │   │   ├── mpp.rs        # MPP 适配器 (Tempo/Stripe)
│   │   │   │   ├── hsp.rs          # HSP 支付协议 (HashKey 支付服务)
│   │   │   │   └── budget.rs     # 支付预算管理
│   │   │   ├── audit/            # 审计日志
│   │   │   │   ├── mod.rs
│   │   │   │   ├── logger.rs     # 日志写入
│   │   │   │   ├── exporter.rs   # CSV/JSON 导出
│   │   │   │   └── anchor/       # Merkle 链上锚定 (HashKey Chain, v2.0)
│   │   │   │       ├── merkle.rs       # Merkle tree 实现
│   │   │   │       ├── service.rs      # 调度 + 链上提交
│   │   │   │       └── verifier.rs     # proof 验证
│   │   │   ├── ows/              # OWS 集成隔离层
│   │   │   │   ├── mod.rs
│   │   │   │   ├── adapter.rs    # 对 ows-core 的封装
│   │   │   │   ├── vault.rs      # 存储层封装
│   │   │   │   └── signing.rs    # 签名流程封装
│   │   │   ├── rpc/              # 链上交互
│   │   │   │   ├── mod.rs
│   │   │   │   ├── evm.rs        # EVM (Eth/Base/BSC/X Layer/Arb/OP/Polygon…)
│   │   │   │   ├── svm.rs        # Solana RPC
│   │   │   │   ├── btc.rs        # Bitcoin RPC
│   │   │   │   ├── stellar.rs    # Stellar/Soroban RPC (v1.5/Hackathon 专项)
│   │   │   │   ├── hashkey.rs    # HashKey Chain RPC (EVM, Hackathon)
│   │   │   │   └── multi.rs      # 多链 RPC 管理器 (CAIP-2 路由)
│   │   │   └── team/             # 多租户/团队
│   │   │       ├── mod.rs
│   │   │       ├── workspace.rs  # 工作空间
│   │   │       ├── member.rs     # 成员管理
│   │   │       └── shared_budget.rs
│   │
│   ├── gradience-api/            # REST API Server (Axum)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/           # HTTP 路由
│   │       │   ├── mod.rs
│   │       │   ├── auth.rs       # /auth/*
│   │       │   ├── wallet.rs     # /wallets/*
│   │       │   ├── policy.rs     # /policies/*
│   │       │   ├── dex.rs        # /swap/*
│   │       │   ├── payment.rs    # /payments/*
│   │       │   ├── audit.rs      # /audit/*
│   │       │   ├── team.rs       # /teams/*
│   │       │   └── health.rs     # /health
│   │       ├── middleware/       # HTTP 中间件
│   │       │   ├── mod.rs
│   │       │   ├── auth.rs       # JWT/Session 验证
│   │       │   ├── rate_limit.rs
│   │       │   └── cors.rs
│   │       └── ws.rs             # WebSocket 实时推送
│   │
│   ├── gradience-cli/            # CLI 应用
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       └── commands/
│   │           ├── mod.rs
│   │           ├── auth.rs       # auth login/logout
│   │           ├── agent.rs      # agent create/list/fund/balance
│   │           ├── policy.rs     # policy set/list/test
│   │           ├── dex.rs        # swap/quote
│   │           ├── audit.rs      # audit/export
│   │           └── team.rs       # team invite/list/role
│   │
│   ├── gradience-mcp/            # MCP Server for Agents
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── handler.rs        # MCP tool 实现
│   │       └── tools/
│   │           ├── mod.rs
│   │           ├── sign_tx.rs    # sign_transaction
│   │           ├── sign_msg.rs   # sign_message
│   │           ├── sign_and_send.rs
│   │           ├── get_balance.rs
│   │           ├── swap.rs
│   │           └── pay.rs        # x402/MPP 支付
│   │
│   ├── gradience-skills/         # Agent Skill 定义 (平台适配)
│   │   ├── cursor/
│   │   │   └── skills/
│   │   │       └── gradience-wallet.md    # Cursor: 教 Agent 如何安全使用钱包
│   │   ├── openclaw/
│   │   │   └── skills/
│   │   │       └── gradience-wallet.yaml  # OpenClaw 专用
│   │   ├── claude-code/
│   │   │   └── .claude/
│   │   │       └── skills/
│   │   │           └── gradience-wallet.md # Claude Code 专用
│   │   └── README.md            # 跨平台 Skill 指令规范
│   │
│   ├── gradience-sdk/            # SDK 包
│   │   ├── Cargo.toml            # Rust SDK
│   │   ├── node/                 # NAPI-RS Node.js 绑定
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   └── python/               # PyO3 Python 绑定
│   │       ├── Cargo.toml
│   │       ├── pyproject.toml
│   │       └── src/
│   │
│   └── gradience-db/             # 数据库迁移 + 模型
│
├── web/                          # Web Dashboard (React SPA)
│   ├── package.json
│   ├── src/
│   │   ├── components/          # UI 组件
│   │   ├── pages/               # 页面路由
│   │   ├── hooks/               # React hooks
│   │   └── api/                 # REST client (axios)
│   └── public/                  # Telegram Mini App 复用同一套构建
│       ├── Cargo.toml
│       └── src/
│           ├── migrations/       # SQLx 迁移文件
│           ├── models.rs         # 数据库模型
│           └── queries.rs        # 查询函数
```

### 2.2 动态信号缓存策略

#### 2.2.1 缓存层

| 部署模式 | 缓存实现 | TTL 默认值 |
|---|---|---|
| 本地 (CLI/MCP) | 内存 `HashMap` + 文件持久化 JSON | 5 分钟 |
| 云端 (API Server) | Redis (共享缓存) + 内存二级 | 5 分钟 |

### 2.2.2 信号获取失败 fallback

```
请求动态信号 (e.g. Reputation API)
    │
    ├── 成功 → 写入缓存 → 返回结果
    │
    ├── 超时 (< 500ms) → 返回上次缓存值 (标记 stale)
    │                      → 如果缓存也过期 → fallback_deny (收紧策略)
    │
    └── 错误 → 返回上次缓存值 (标记 error)
                 → 如果缓存也过期 → fallback_deny (收紧策略)
```

**原则**: 信号获取失败时**收紧策略**而非放宽。宁可过度保护，不可放过风险。

---

### 2.3 模块依赖关系

```
gradience-cli ───────┐
gradience-api ───────┤
gradience-mcp ───────┼──▶ gradience-core ───▶ ows-core (Rust crate)
gradience-sdk ───────┤                       └───▶ ring / sodiumoxide
                     │
                     └──▶ gradience-db (SQLite/PostgreSQL)
```

每个可执行组件 (CLI, API, MCP) 都是一个独立的 binary，链接 `gradience-core` 库。SDK 包则暴露 `gradience-core` 的核心 API 到外部语言。

### 2.4 多租户策略合并逻辑

**策略优先级**：

```
Workspace Policy (priority=0, 最严)
    │ merge (取交集/最小值)
    ▼
Wallet/Agent Policy (priority=1)
    │ merge
    ▼
OWS Default Policy (deny-all 兜底)
```

**合并规则**：

| 规则类型 | 合并策略 | 示例 |
|---|---|---|
| spend_limit | 取最小值 | workspace=1000 + wallet=500 → **500** |
| daily_limit | 取最小值 | workspace=5000 + wallet=2000 → **2000** |
| monthly_limit | 取最小值 | workspace=50000 + wallet=20000 → **20000** |
| chain_whitelist | 取交集 | workspace=[Eth,Sol] + wallet=[Eth,BTC] → **[Eth]** |
| contract_whitelist | 取交集 | workspace=[A,B,C] + wallet=[B,C,D] → **[B,C]** |
| operation_type | 取交集 | workspace=[transfer,swap] + wallet=[swap,stake] → **[swap]** |
| time_window | 取交集 (最窄窗口) | workspace=9-18 + wallet=10-16 → **10-16** |

**实现位置**: `policy/merge.rs` 中的 `merge_policies_strictest()` 函数。

**跨 Agent 预算共享 (v1.5)**:
- Workspace 总预算 → 按 Agent 数量/角色自动分配
- 任一 Agent 使用 = 所有 Agent 剩余预算扣减
- 实现: `team/shared_budget.rs` + `spending_trackers` 表的 workspace_id 维度

---

## 3. OWS 集成层设计

### 3.1 隔离架构

```
外部调用 (API/MCP/CLI/SDK)
    │
    ▼
┌─────────────────────────────────┐
│     gradience-core              │
│  ┌───────────────────────────┐  │
│  │ policy engine (Rust)      │  │
│  │ intent matcher (Rust)     │  │
│  │ dex router (Rust)         │  │
│  └──────┬────────────────────┘  │
│         │ 评估通过               │
│         ▼                       │
│  ┌───────────────────────────┐  │
│  │ ows/adapter.rs            │  │  ← 唯一与 ows-core 交互的点
│  │  - VaultManager           │  │
│  │  - Signer                 │  │
│  │  - PolicyBridge           │  │
│  └──────┬────────────────────┘  │
└─────────┼───────────────────────┘
          │
          ▼
┌─────────────────────────────────┐
│     ows-core (Rust crate)       │
│  - Vault (加密存储)              │
│  - Signing (链签名)              │
│  - Policy Engine (OWS 内置)      │
└─────────────────────────────────┘
```

### 3.2 `ows/adapter.rs` 接口

```rust
/// OWS 适配抽象层 — 唯一与 `ows-core` crate 交互的点
pub trait OwsAdapter {
    /// 初始化/解锁 Vault
    async fn init_vault(&self, passphrase: &str) -> Result<VaultHandle>;

    /// 注册自定义策略可执行文件 (映射 Gradience Policy → OWS Policy)
    async fn register_policy_executable(
        &self,
        vault: &VaultHandle,
        name: &str,
        executable_path: &Path,
        default_action: PolicyAction,
    ) -> Result<String>;  // 返回 OWS policy_id

    /// 附加 API Key 到 Vault 并绑定策略
    async fn attach_api_key_and_policies(
        &self,
        vault: &VaultHandle,
        wallet_id: &WalletId,
        api_key_name: &str,
        policy_ids: Vec<String>,  // OWS 内部 policy_id
    ) -> Result<GradienceApiKey>;  // 包含 raw_key 供一次性返回

    /// 创建新钱包 (通过 OWS 派生)
    async fn create_wallet(
        &self,
        vault: &VaultHandle,
        name: &str,
        derivation_params: DerivationParams,
    ) -> Result<WalletDescriptor>;

    /// 签名 (已确认 Gradience 策略通过后调用)
    async fn sign_transaction(
        &self,
        vault: &VaultHandle,
        wallet_id: &WalletId,
        chain: &ChainId,
        tx: &Transaction,
    ) -> Result<SignedTransaction>;

    /// 广播
    async fn broadcast(
        &self,
        chain: &ChainId,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<TxHash>;
}
```

### 3.3 Gradience Policy → OWS Policy 映射

```
用户定义 (Gradience)                      OWS 内部
┌──────────────────────────────────┐     ┌─────────────────────────────┐
│ policy:                          │     │ OWS Policy (custom exec)    │
│   id: GradPolicy-001             │     │   id: ows-policy-abc        │
│   name: "DeFi Safe"              │────▶│   executable:               │
│   rules: [                       │     │     "policies/gradience-     │
│     {type: spend_limit,          │     │      bridge-eval.sh"         │
│      config: {max: 100 USDC},   │     │   config:                   │
│      action: deny},              │     │     {gradience_policy_id:    │
│     {type: chain_whitelist,     │     │      "GradPolicy-001"}       │
│      config: {chains: [...]},   │     │   action: deny               │
│      action: deny}               │     │                              │
│   ]                              │     │ OWS Policy (custom exec)     │
└──────────────────────────────────┘     │   id: ows-policy-def        │
                                         │   executable: ...           │
                                         └─────────────────────────────┘

Gradience 创建/更新 Policy 时:
  1. 存完整 JSON 到 Gradience DB (policies 表)
  2. 为每个 rule 注册一个 OWS custom executable (bridge 到 Gradience 评估)
  3. 记录映射关系: GradPolicy-001 → [ows-policy-abc, ows-policy-def]
  4. API Key 创建时同时 attach OWS policy_ids
```

**版本锁定**:
- `Cargo.toml`: `ows-core = ">=1.1.0, <1.3.0"` (兼容 minor 级别)
- CI: 每个 PR 运行 `test_ows_compatibility` 套件 (验证 ows-core API 无 breaking change)
- `ows_adapter.rs` 是唯一的 `ows-core` 调用点，升级只需修改此文件
```

### 3.3 策略执行双层模型

```
第一层 (Gradience)    第二层 (OWS 内置)
    │                      │
    │ 1. 声明式策略评估     │
    │    - 静态规则         │
    │    - 动态信号         │  ← Gradience Policy Engine
    │    - Intent 匹配      │
    │                      │
    │ 2. 通过后调用         │
    │    ows-core.sign()   │  ← ows-core 内置策略 (兜底)
    │                      │
    │ 3. 签名结果返回       │  ← 只有两层都通过才签名
    ▼                      ▼
```

OWS 内置策略作为兜底：即使 Gradience 层被绕过（直接调用 ows-core），OWS 仍有 deny-all 默认策略。

---

## 4. 数据模型

### 4.1 数据库选型

| 部署模式 | 数据库 | 说明 |
|---|---|---|
| 本地 (CLI/MCP) | SQLite (via `rusqlite` + `sqlx`) | 零配置，单文件 |
| 云端 (API Server) | PostgreSQL (via `sqlx`) | 多租户，并发安全 |

代码通过 `sqlx` 抽象，统一接口，编译时验证 SQL。

### 4.2 核心表结构

```sql
-- ========================================
-- 用户身份
-- ========================================
CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT UNIQUE NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status      TEXT NOT NULL DEFAULT 'active'  -- active | suspended
);

-- Passkey 凭证
CREATE TABLE passkey_credentials (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   BYTEA UNIQUE NOT NULL,          -- Raw credential ID
    credential_pk   BYTEA NOT NULL,                 -- Public key
    counter         BIGINT NOT NULL DEFAULT 0,      -- Signature counter
    transports      TEXT[] NOT NULL DEFAULT '{}',   -- [usb, nfc, ble, internal]
    device_name     TEXT,
    last_used_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ========================================
-- 团队/工作空间
-- ========================================
CREATE TABLE workspaces (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    owner_id        UUID NOT NULL REFERENCES users(id),
    plan            TEXT NOT NULL DEFAULT 'free',   -- free | pro | team | enterprise
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE workspace_members (
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member', -- owner | admin | member | viewer
    invited_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_id, user_id)
);

-- ========================================
-- Agent 钱包
-- ========================================
CREATE TABLE wallets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    owner_id        UUID NOT NULL REFERENCES users(id),
    workspace_id    UUID REFERENCES workspaces(id),
    status          TEXT NOT NULL DEFAULT 'active', -- active | suspended | revoked
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 钱包在多链上的地址
CREATE TABLE wallet_addresses (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    chain_id        TEXT NOT NULL,          -- CAIP-2, e.g. "eip155:1"
    address         TEXT NOT NULL,          -- 链上地址
    derivation_path TEXT NOT NULL,          -- BIP-44 path
    UNIQUE(wallet_id, chain_id)
);

-- API Key (Agent 访问凭证)
CREATE TABLE api_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    key_hash        BYTEA NOT NULL,         -- SHA-256 of raw key
    permissions     TEXT[] NOT NULL DEFAULT '{sign, read}',
    expires_at      TIMESTAMPTZ,
    last_used_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ========================================
-- 策略
-- ========================================
CREATE TABLE policies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    workspace_id    UUID REFERENCES workspaces(id),
    rules_json      JSONB NOT NULL,         -- 策略规则 JSON
    priority        INTEGER NOT NULL DEFAULT 1,  -- 0 = workspace, 1 = wallet
    status          TEXT NOT NULL DEFAULT 'active', -- active | paused | deleted
    version         INTEGER NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 额度追踪 (daily_limit / monthly_limit)
CREATE TABLE spending_trackers (
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    workspace_id    UUID,                     -- 跨 Agent 预算共享 (v1.5)
    rule_type       TEXT NOT NULL,          -- daily | monthly
    token_address   TEXT NOT NULL,
    chain_id        TEXT NOT NULL,
    period          TEXT NOT NULL,          -- "2026-04-07" 或 "2026-04"
    spent_amount    TEXT NOT NULL DEFAULT '0',  -- 最小单位字符串
    reset_at        TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (wallet_id, rule_type, token_address, chain_id, period)
);

-- Warn 审批队列
CREATE TABLE policy_approvals (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id       UUID NOT NULL REFERENCES policies(id),
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    request_json    JSONB NOT NULL,         -- 原始交易请求
    status          TEXT NOT NULL DEFAULT 'pending', -- pending | approved | rejected | expired
    approved_by     UUID REFERENCES users(id),
    approved_at     TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ NOT NULL DEFAULT (NOW() + INTERVAL '30 minutes'),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ========================================
-- 审计日志
-- ========================================
CREATE TABLE audit_logs (
    id              BIGSERIAL PRIMARY KEY,
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    api_key_id      UUID REFERENCES api_keys(id),
    action          TEXT NOT NULL,          -- sign_tx | sign_msg | swap | pay
    context_json    JSONB NOT NULL,         -- 请求上下文
    intent_json     JSONB,                  -- 交易意图 (v1.5+)
    decision        TEXT NOT NULL,          -- allowed | denied | warned
    decision_reason TEXT,
    dynamic_factors JSONB,                  -- 动态信号快照 (v1.5+)
    tx_hash         TEXT,                   -- 链上交易哈希
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 支付记录 (支付也受 spending_trackers / budget.rs 限额控制)
CREATE TABLE payment_records (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    protocol        TEXT NOT NULL,          -- x402 | mpp
    amount          TEXT NOT NULL,          -- 最小单位字符串
    token           TEXT NOT NULL,
    recipient       TEXT NOT NULL,
    status          TEXT NOT NULL,          -- pending | completed | failed
    tx_hash         TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### 4.3 索引策略

```sql
-- 高频查询优化
CREATE INDEX idx_policies_wallet ON policies(wallet_id, status);
CREATE INDEX idx_policies_workspace ON policies(workspace_id, status);
CREATE INDEX idx_audit_logs_wallet_time ON audit_logs(wallet_id, created_at DESC);
CREATE INDEX idx_api_keys_wallet ON api_keys(wallet_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);
CREATE INDEX idx_workspaces_owner ON workspaces(owner_id);
CREATE INDEX idx_passkey_user ON passkey_credentials(user_id);
```

---

## 5. 产品前端入口

Gradience Wallet 面向用户的访问方式有多端，但核心操作统一：

### 5.0 入口矩阵

### 5.0 入口矩阵

| 入口 | 技术 | 场景 | 覆盖 |
|---|---|---|---|
| **Web Dashboard** | React SPA + Tailwind | 完整管理：钱包/策略/Agent/API Key/审计 | 所有用户 (主要入口) |
| **Telegram Mini App** | React SPA + @twa-dev/sdk | 轻量操作：快速支付/审批/余额/Bot 通知 | Telegram 10 亿用户 |
| **CLI** | `clap` Rust | 高级用户/开发调试/CI 集成 | 开发者/运维 |
| **Agent Skill** | MCP + 平台 Skill 文件 | Agent 自动使用钱包 (Claude Code/Cursor/OpenClaw) | AI Agent 自身 |

**核心流程**：
```
用户登录 Web/Tg Mini App (Passkey 认证)
    │
    ├── 创建 Master Wallet (一条 mnemonic → 10 链地址)
    ├── 配置策略 (限额/链白名单/合约白名单/意图分析)
    ├── 创建 Agent API Key (每个 Agent 一个 Key + 绑定 Policy)
    │
    └── 分发 API Key Token 给 Agent
        │
        ├── 方式 1: 配置 Agent MCP → Agent 自动连接 Gradience MCP Server
        ├── 方式 2: 配置 Agent Skill → Agent 读取 Skill 指令 + MCP tools
        └── 方式 3: 环境变量 OWS_PASSPHRASE → Agent 直接调用 OWS CLI

Agent 持有 Token 后的操作流:
    Agent 发起签名/支付请求
    │
    ▼
Gradience MCP Server (或 OWS CLI)
    │  → 用 Token 查 API Key 文件
    │  → 加载 Key 绑定的所有 Policy
    │  → Policy Engine 评估 (AND 逻辑，任一 deny 则拒绝)
    ├── allow → OWS 解密签名 → 链上广播
    ├── deny  → 返回 POLICY_DENIED error (私钥从未接触)
    └── warn  → 通知用户审批 (Web/Tg 弹窗)
```

---

### 5.1 本地部署 (MVP: CLI + MCP)

```
用户设备 (macOS/Linux/Windows)
├── gradience-cli          # 单二进制，~15MB
├── gradience-mcp          # 单二进制，~15MB (后台常驻)
├── ~/.gradience/
│   ├── vault.encrypted    # OWS 加密存储
│   ├── database.sqlite    # SQLite 数据库
│   └── config.toml        # 用户配置
└── 系统托盘 / 后台进程
```

用户运行：
```bash
# 安装 (下载预编译二进制)
curl -fsSL https://gradience.dev/install.sh | sh

# 使用
gradience auth login
gradience agent create --name "trading-bot"
gradience-mcp serve  # MCP 服务器启动，供 Claude Code 等 Agent 使用
```

### 5.2 云端部署 (v1.0+)

```
                    ┌─────────────────────────┐
                    │   CDN / Vercel (前端)    │
                    │   Web Dashboard (SPA)    │
                    └───────────┬─────────────┘
                                │
                    ┌───────────▼─────────────┐
                    │  Load Balancer (AWS)    │
                    └───────────┬─────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          ▼                     ▼                     ▼
    ┌───────────┐        ┌───────────┐        ┌───────────┐
    │gradience- │        │gradience- │        │gradience- │
    │api:1      │        │api:2      │        │api:3      │
    │(Axum)     │        │(Axum)     │        │(Axum)     │
    └─────┬─────┘        └─────┬─────┘        └─────┬─────┘
          │                    │                    │
          └────────────────────┼────────────────────┘
                               │
                    ┌──────────▼─────────┐
                    │  PostgreSQL (RDS)  │
                    └────────────────────┘

    每个用户的 Vault 仍存储在本地 (用户设备)
    云端不存私钥，只存策略、审计、元数据
```

### 5.3 企业私有部署 (v2.0)

```
企业 VPC
├── gradience-api (Docker, 多副本)
├── PostgreSQL (RDS/自建)
├── Redis (缓存 + WebSocket)
├── gradience-mcp (多副本, 供企业 Agent 使用)
├── HSM (可选, YubiHSM / AWS CloudHSM)
└── Vault 存储在服务器 (可选, 企业自管风险)
```

### 5.4 混合部署模式（推荐）

| 用户类型 | 身份认证 | Vault 位置 | MCP 位置 | 策略/审计存储 | 适用场景 |
|---|---|---|---|---|---|
| **个人 (免费/Pro)** | Passkey (浏览器) | 本地设备 | 本地 MCP | 本地 SQLite | 个人 DeFi, Agent 实验 |
| **团队 (Team)** | Passkey + 邮箱邀请 | 本地设备 | 云端 MCP (共享) | 云端 PostgreSQL | 小团队协作, 共享 Agent |
| **企业 (Enterprise)** | SSO (SAML/OIDC) | 可选 HSM | 企业自托管 MCP | 企业自建 PostgreSQL + Redis | 合规要求高, 大规模 Agent 池 |

**云端不存私钥原则不变** — 即使是企业部署，Vault 也由企业自己的安全基础设施管理（HSM/安全存储），Gradience 只提供策略管理和审计功能。个人用户的 Vault 始终在本地设备。

> **签名始终在用户/企业控制的 Vault 中完成。** 云端 API Server 可以接收策略评估请求并转发签名请求，但私钥解密和签名操作永远在本地 MCP/Vault 中执行。

### 5.5 Telegram 集成（Mini App + Bot）

Telegram 拥有 10 亿+ 用户，Mini Apps 是最轻量、最自然的 Gradience 前端入口。

**架构映射**：
| Telegram 组件 | 对应 Gradience 模块 | 说明 |
|---|---|---|
| **Telegram Mini App (TMA)** | Web Dashboard (React SPA) | 近乎零改动复用，只需加 @twa-dev/sdk |
| **Telegram Bot** | gradience-mcp (MCP tools) | 命令式交互，映射到 CLI 命令 |
| **Telegram Passkey** | Passkey/WebAuthn 身份认证 | Telegram 原生支持生物识别/PIN |
| **Bot 通知** | WebSocket `ws.rs` | 实时推送审批/warn/交易状态 |

**Mini App 实现**：
```
Telegram 用户点击 Bot 按钮
    │
    ▼
Telegram 打开 WebView (Mini App)
    │  → 加载 https://app.gradience.io (React SPA)
    │  → @twa-dev/sdk 获取 tg_user_id + 主题
    │  → Telegram Passkey 身份认证
    ▼
React Dashboard (完整钱包管理界面)
    │  → 调用 gradience-api (REST)
    │  → 策略引擎 / 意图分析 / 审计
    │  → WebSocket 实时推送
    ▼
用户执行支付/swap/sign → 策略评估 → OWS 签名 → 链上广播
```

**Bot 快捷命令**：
```
/policy      — 查看当前策略
/wallet      — 打开 Mini App（完整钱包界面）
/pay         — 快捷支付 (x402/MPP)
/approve     — 审批 warn 请求
/audit       — 最近审计日志摘要
/agent       — Agent 状态 / Reputation
```

**Hackathon 加分**：Telegram Mini App 是最直观的 Demo 展示方式，用户无需离开 Telegram 即可完成完整 Agent 钱包操作流程。

---
### 5.6 BNB Chain (BSC) 专项支持

BNB Chain 是完全 EVM 兼容链（Chain ID: 56, 测试网: 97），与 Ethereum/Base/Arbitrum 使用同一套 RPC 接口。支持 BSC 几乎零工作量：

| 组件 | 说明 |
|---|---|
| **RPC** | 复用 `rpc/evm.rs`，新增 Chain Config (`eip155:56`, `https://bsc-dataseed.bnbchain.org`) |
| **地址派生** | BIP-44: `m/44'/60'/...` (与 Ethereum 相同路径) |
| **签名** | ECDSA secp256k1，完全复用 OWS EVM signer |
| **DEX** | PancakeSwap (`pancakeswap.rs`) — BSC 最主流 AMM |
| **Token 标准** | BEP-20 ↔ ERC-20 完全兼容 |
| **区块时间** | ~3s, 低 Gas — 适合 Agent 高频操作 |
| **策略引擎** | 合约白名单、限额规则完全复用 EVM 逻辑 |

**PancakeSwap DEX Adapter**:
- Swap 路由：通过 PancakeSwap V3 Router 合约获取最优路径
- 报价 API：兼容已有的 DEX aggregator 接口 (`get_quote()`, `swap()`)
- 支持 BNB/WBNB、BEP-20 token pairs

HashKey Chain + BNB Chain + X Layer + Telegram Mini App 可同时作为 Hackathon 多链+多端演示环境 — 在 BSC 上跑通支付/DeFi (PancakeSwap)，在 X Layer 上跑通 x402 Agentic Payment (OKX Onchain OS)，在 HSK 上跑通 Merkle 审计锚定 (AuditAnchor 合约)，展示完整的"策略保护 + 合规可证明" Agent 钱包平台。


### 5.7 X Layer (OKX 生态) 专项支持

X Layer 是 OKX 基于 Polygon CDK + AggLayer 构建的 zkEVM L2，完全 EVM 兼容，是 OKX Agentic Wallet 和 Onchain OS 的主战场。

| 组件 | 说明 |
|---|---|
| **RPC** | 复用 `rpc/evm.rs`，新增 Chain Config (`eip155:196`, `https://xlayerrpc.okx.com`) |
| **地址派生** | BIP-44: `m/44'/60'/...` (与 Ethereum 相同) |
| **签名** | ECDSA secp256k1，完全复用 OWS EVM signer |
| **x402 原生支持** | X Layer 深度集成 x402 (Onchain OS 原生支付协议) |
| **Agentic 生态** | OKX Agentic Wallet + Onchain OS Skills — Gradience MCP 可直接对接 |
| **区块时间** | < 1s, 近零 Gas — 极适合 Agent 高频小额支付 |
| **策略引擎** | 合约白名单、限额规则完全复用 EVM 逻辑 |

**Hackathon 匹配** (Build X, 截止 4 月 15 日):
- 60K USDT 奖金池，Agentic Commerce 赛道
- Gradience 定位："带智能 Policy 的 Agent 钱包编排平台" + x402 + Onchain OS
- 最小 Demo: OWS Vault → X Layer 签名 → x402 支付 → 策略引擎评估 → 展示完整 Agent 安全支付闭环
- 差异化优势: 多 Agent 编排 + Reputation 动态策略 + 审计 log (OKX 官方 Agentic Wallet 缺少治理层)

---
### 5.8 Stellar 专项支持（Hackathon 方向）

OWS 的 signer trait 已支持多链扩展。Stellar 接入要点：

| 组件 | 说明 |
|---|---|
| **签名** | Soroban Authorization Entry signing (与 EVM `signTypedData` 语义相似，OWS 已支持) |
| **x402 支付** | Stellar 原生 facilitator + server-sponsored fees + one-way-channel contract |
| **MPP** | Tempo 通过 CAP-38 (AMM) 支持 Stellar 路径支付 |
| **CAIP-2** | `stellar:pubnet` / `stellar:testnet` |
| **RPC** | Soroban-RPC (stellar/horizon) + OWS stellar adapter |

Hackathon 最小可打 Demo 路径：OWS Vault → Stellar 签名 → 链上交易 → Policy Engine 审计，展示"Agent 在策略保护下执行链上操作"的完整闭环。

---

## 6. 安全架构

### 6.1 密钥生命周期

```
                    ┌──────────────┐
                    │  用户 Passkey │
                    │  (WebAuthn)   │
                    └──────┬───────┘
                           │ 认证
                    ┌──────▼───────┐
                    │  Session Key │ (内存中, 会话生命周期)
                    └──────┬───────┘
                           │ 派生
    ┌──────────────────────▼──────────────────────┐
    │              gradience-core                 │
    │  ┌────────────────────────────────────────┐ │
    │  │ master_key (HKDF from Passkey binding) │ │
    │  │  - 存在内存 (mlock)                     │ │
    │  │  - 会话结束清零 (zeroize)               │ │
    │  └──┬─────────────────────────────────────┘ │
    │     │ HD 派生                               │
    │  ┌──▼─────────────────────────────────────┐ │
    │  │ agent_wallet_keys (BIP-32 派生)        │ │
    │  │  - 每个 Agent 独立密钥                 │ │
    │  │  - 按需加载, 用后清零                   │ │
    │  └──┬─────────────────────────────────────┘ │
    │     │ 签名                                  │
    │  ┌──▼─────────────────────────────────────┐ │
    │  │ signed_tx                              │ │
    │  └────────────────────────────────────────┘ │
    └─────────────────────────────────────────────┘
```

### 6.2 存储加密

| 数据类型 | 加密方式 | 位置 |
|---|---|---|
| OWS Vault | AES-256-GCM, passphrase 派生密钥 | 本地文件 (~/.gradience/vault.encrypted) |
| SQLite 数据库 | SQLCipher (可选), 或依赖文件系统加密 | 本地文件 (~/.gradience/database.sqlite) |
| API Key (数据库) | SHA-256 哈希 (不可逆) | 数据库中 |
| 审计日志 | 明文 (需可读), 可选签名防篡改 | 本地/云端数据库 |
| Session | AES-256-GCM, 短期密钥 | 内存 + Cookie |

### 6.3 内存安全

```rust
use zeroize::Zeroize;

struct Session {
    master_key: Vec<u8>,  // 标记为 Zeroize
    // ...
}

impl Drop for Session {
    fn drop(&mut self) {
        self.master_key.zeroize();  // 会话结束时清零
    }
}
```

使用 `mlock` 防止敏感内存被交换到磁盘：

```rust
#[cfg(unix)]
fn lock_memory(data: &[u8]) -> Result<()> {
    unsafe {
        libc::mlock(data.as_ptr() as *const _, data.len());
    }
    Ok(())
}
```

### 6.4 攻击面分析

| 攻击向量 | 缓解措施 |
|---|---|
| 磁盘窃取攻击 | Vault AES-256-GCM 加密，passphrase 不在磁盘 |
| 内存转储攻击 | mlock + zeroize，密钥不留痕迹 |
| Passkey 设备丢失 | 多设备绑定 + 邮箱恢复 |
| API Key 泄露 | 哈希存储 + 可按 Key 吊销 + 限速 |
| MCP SSRF | 严格校验目标地址, 不允许任意 URL |
| 策略绕过 | 双层检查 (Gradience + OWS), deny-all 兜底 |
| 重放攻击 | 请求 nonce + 时间戳窗口 |
| 审计日志篡改 | HMAC 链式签名 + v2.0 可选链上 Merkle 锚定 |

### 6.5 审计日志防篡改 (HMAC 链式签名)

```
audit_logs 表每一行携带 HMAC 指纹:
┌────┬─────────┬──────────┬──────────────────────────┐
│ id │ content │  prev_hash │ current_hash            │
├────┼─────────┼────────────┼─────────────────────────┤
│ 1  │ {...}   │ 0x000...   │ HMAC(key, content+prev) │
│ 2  │ {...}   │ 0xABC...   │ HMAC(key, content+prev) │
│ 3  │ {...}   │ 0xDEF...   │ HMAC(key, content+prev) │
└────┴─────────┴────────────┴─────────────────────────┘

验证完整性: 从任意行回溯, 重新计算 HMAC 链。单条被改 → 后续全部不匹配。
```

**v2.0 扩展**: 周期性将 audit_logs 的 Merkle root 锚定到 **HashKey Chain** (EVM)，提供不可篡改的时间戳证明。详见 [appendix-merkle-anchor-design.md](appendix-merkle-anchor-design.md)。

```solidity
// HashKey Chain AuditAnchor 合约核心
function anchor(bytes32 root, bytes32 prevRoot, uint256 start, uint256 end, uint256 count) external;
function verifyProof(bytes32 root, bytes32 leaf, bytes32[] calldata proof) external pure returns (bool);
```

---

## 7. 关键数据流

### 7.1 Agent 交易签名完整流程

```
Agent (Claude Code / Cursor)
    │  MCP 调用 sign_transaction
    │  { "walletId": "...", "chainId": "eip155:1",
    │    "tx": { ... }, "intent": { "type": "swap", ... } }
    ▼
gradience-mcp
    │  1. 验证 API Key (SHA-256 比对)
    │  2. 加载 Wallet 信息
    ▼
PolicyEngine::evaluate()
    │  3. 加载 Wallet 的 Policies (workspace + wallet)
    │  4. 合并策略 (取最严)
    │  5. 如果 intent 存在:
    │  │  a. IntentMatcher::match(intent, templates)
    │  │  b. 不匹配 → deny
    │  6. 动态信号 (v1.5+):
    │  │  a. 读取缓存的 Reputation / 市场风险
    │  │  b. 计算 adjustment (multiplier, override)
    │  7. 逐条静态规则评估:
    │  │  a. spend_limit: 查询 spending_tracker, 累加对比
    │  │  b. chain_whitelist: 检查 chain_id 是否在允许列表
    │  │  c. contract_whitelist: 检查目标合约
    │  │  d. operation_type: 检查操作类型
    │  │  e. time_window: 检查当前时间
    │  8. 结果:
    │  │  - 全部 allow → continue
    │  │  - 有 deny → 403 + reasons
    │  │  - 有 warn → 创建 approval, 返回 pending
    ▼
OwsAdapter::sign_transaction()
    │  9. 解锁 OWS Vault (内存中)
    │  10. 查找派生密钥
    │  11. 执行签名 (ring / ows-core)
    │  12. 清零内存 (zeroize)
    ▼
RpcManager::broadcast()
    │  13. 选择 RPC 端点
    │  14. 发送签名交易
    │  15. 返回 tx_hash
    ▼
AuditLogger::log()
    │  16. 写入 audit_logs (context, intent, decision, tx_hash)
    │  17. 更新 spending_tracker
    │  18. WebSocket 推送 Dashboard 更新
    ▼
Agent 收到 tx_hash
```

### 7.2 warn 审批流程

```
触发 warn 策略
    │
    ├── 创建 policy_approvals 记录
    │
    ├── 通知用户
    │   ├── WebSocket → Dashboard 实时弹窗
    │   ├── 邮件 (SMTP)
    │   └── Telegram Bot (可选)
    │
    ├── 用户操作 (Dashboard)
    │   ├── Approve → 更新 status = 'approved'
    │   │   └── 继续签名流程 (从步骤 9 开始)
    │   ├── Reject  → 更新 status = 'rejected'
    │   │   └── 返回 403 给 Agent
    │   └── 超时 (30min) → 自动 expired
    │       └── 返回 403 给 Agent
```

---

## 8. 架构决策记录 (ADR)

### ADR-001: 后端全 Rust 栈

**决定**: 后端核心、CLI、MCP Server 全部使用 Rust。

**背景**: OWS 核心 (`ows-core`) 是纯 Rust 实现。当前 ows-core v1.2.4 (2026-04)，已验证 policy executable 注册与 auth-entry signing 兼容性。若用 Node.js 需通过 NAPI 调用 Rust，引入 FFI 序列化开销、跨语言调试复杂度、两套包管理。

**替代方案**:
- Node.js + NAPI (NAPI 绑定到 ows-core)
- Go (需自己移植 OWS 逻辑或 CGo)

**选择 Rust 的理由**:
1. 直接依赖 `ows-core` crate，无 FFI
2. 内存安全 (`zeroize` / `mlock`) 对钱包场景至关重要
3. 单二进制部署，用户无需安装 Node.js
4. 策略 < 10ms 的性能承诺在 Rust 下更易保证
5. NAPI-RS / PyO3 仍可导出 Node.js / Python SDK

**后果**: Rust 学习曲线陡峭，但团队已有 Rust 经验。

### ADR-002: SQLite (本地) + PostgreSQL (云) 双模式

**决定**: 本地部署使用 SQLite，云端部署使用 PostgreSQL，通过 `sqlx` 统一接口。

**理由**:
- 本地模式：零配置，单文件，适合 CLI 用户
- 云端模式：PostgreSQL 支持并发、多租户、备份
- `sqlx` 提供编译时 SQL 验证 + 统一 Rust API

### ADR-003: 不发 Token

**决定**: Gradience 不发行自有 Token，采用 SaaS 订阅 + 微手续费的纯现金流模式。

**理由**:
- Token 带来监管风险和投机行为
- SaaS 模式清晰、可持续
- 避免"做市 / 流动性"等非核心业务负担

### ADR-004: 策略引擎双层执行

**决定**: Gradience Policy Engine 作为前置评估，OWS 内置 Policy 作为兜底。

**理由**:
- Gradience 层提供智能评估 (动态、Intent)
- OWS 层提供安全兜底 (deny-all 默认)
- 即使 Gradience 被绕过，私钥仍受保护

### ADR-005: 本地优先 + 可选云端

**决定**: 用户私钥永远存储在本地设备，云端只存策略、审计、元数据。

**理由**:
- "不托管用户资金" — 合规底线
- 降低云端安全压力
- 企业用户可选择私有部署

### ADR-006: 动态信号的隐私保护与数据源去中心化倾向

**决定**: 动态策略优先使用去中心化/开放数据源，对中心化 API (Chainalysis 等) 设置严格隐私边界。

**背景**: 动态策略依赖外部信号 (Reputation, 市场风险, 行为分析)。若过度依赖中心化服务，存在单点故障、隐私泄露、审查风险。

**数据源优先级**:
1. **内置 (零外部依赖)**: Agent 历史交易行为 profiling (本地计算)
2. **开放数据源**: Forta (去中心化威胁检测)、链上公开数据 (MEV 指数可从公共 RPC 计算)
3. **中心化 API (可选, 需用户明确开启)**: Chainalysis KYT(需钱包地址上传)

**隐私保护**:
- 上传到外部服务的数据**仅包含必要信息** (tx hash, 钱包地址)
- 不包含用户身份、email、策略配置等敏感信息
- 企业用户可部署本地 Forta node，避免外部调用

**降级策略**:
- 中心化 API 不可用时 → 降级到开源数据源
- 所有外部源不可用时 → 回退到纯静态策略 (保守模式)

---

## 9. 非功能性需求

| 指标 | 目标 | 说明 |
|---|---|---|
| **策略评估延迟** | < 10ms | 从 MCP 请求到评估结果 |
| **签名延迟** | < 50ms | 策略通过后的纯签名时间 |
| **CLI 启动时间** | < 200ms | 从输入命令到输出 |
| **内存占用** | < 100MB (CLI/MCP 常驻) | 后台常驻服务的上限 |
| **二进制大小** | < 20MB (压缩后) | 下载友好 |
| **并发 (API Server)** | 1000 req/s (单实例) | 标准云实例 |
| **审计日志写入** | 不阻塞主流程 | 异步写入 (tokio::spawn) |
| **策略评估吞吐量** | 10,000 eval/s (纯静态，单核) / 2,000+ eval/s (含动态信号) | 高频 Agent 场景
| **MCP 冷启动** | < 500ms | 服务启动 + DB 连接 + Vault 初始化 |
| **DB 迁移安全性** | 零停机 | sqlx 只增不改, 新字段需 NOT NULL DEFAULT, 迁移失败自动 rollback |
| **锚定延迟** | < 30s 从日志写入到链上确认 | HashKey Chain 确认快 (~2s block time)

---

*验收通过后进入 Phase 3: Technical Spec*
