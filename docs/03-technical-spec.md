# Phase 3: Technical Spec — 技术规格

> Project: Gradience Wallet
> Date: 2026-04-07
> Scope: 全平台 (核心 + AI Gateway + 全链支持)

---

## 1. 数据类型总览

### 1.1 全局常量

```rust
// crates/gradience-core/src/constants.rs

/// USDC atomic decimals (6)
pub const USDC_DECIMALS: u8 = 6;

/// Passphrase minimum length
pub const MIN_PASSPHRASE_LEN: usize = 12;

/// API token prefix
pub const API_KEY_PREFIX: &str = "ows_key_";

/// API token raw length (64 hex chars = 256 bits)
pub const API_KEY_RAW_LEN: usize = 64;

/// Policy executable timeout (seconds)
pub const POLICY_EXEC_TIMEOUT_SECS: u64 = 5;

/// Default audit merkle batch size
pub const DEFAULT_MERKLE_BATCH_SIZE: usize = 1000;

/// Default signal cache TTL (seconds)
pub const SIGNAL_CACHE_TTL_SECS: u64 = 300;

/// Strategy evaluation target latency
pub const EVAL_LATENCY_TARGET_MS: u64 = 10;

/// Warn approval expiration (seconds)
pub const WARN_APPROVAL_EXPIRY_SECS: i64 = 1800; // 30 min

/// MCP cold start target (ms)
pub const MCP_COLD_START_TARGET_MS: u64 = 500;
```

### 1.2 共享核心类型

```rust
// crates/gradience-core/src/types.rs

use serde::{Deserialize, Serialize};

/// CAIP-2 chain identifier, e.g. "eip155:1", "solana:5eykt4..."
pub type ChainId = String;

/// CAIP-10 account identifier, e.g. "eip155:1:0xab16..."
pub type AccountId = String;

/// Wallet UUID v4
pub type WalletId = String;

/// API Key UUID v4
pub type ApiKeyId = String;

/// Policy UUID v4
pub type PolicyId = String;

/// Transaction hash (chain-native hex)
pub type TxHash = String;

/// Address (chain-native encoding)
pub type Address = String;

/// Atomic amount string (avoids float precision issues)
pub type AtomicAmount = String;

/// Derivation path, e.g. "m/44'/60'/0'/0/0"
pub type DerivationPath = String;

/// Unix timestamp in milliseconds
pub type TimestampMs = u64;

/// HTTP URL
pub type Url = String;
```

---

## 2. OWS Adapter (`ows/adapter.rs`)

### 2.1 Trait 定义

```rust
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait OwsAdapter: Send + Sync {
    /// 初始化/解锁 Vault
    /// 
    /// **前置条件**: passphrase.len() >= MIN_PASSPHRASE_LEN
    /// **后置条件**: 返回的 VaultHandle 在内存中，持有一个 mlock 保护的 pool
    async fn init_vault(
        &self,
        passphrase: &str,
    ) -> Result<VaultHandle, GradienceError>;

    /// 注册自定义策略可执行文件
    ///
    /// **前置条件**: 
    /// - executable_path 指向的文件存在且可执行
    /// - vault 已通过 init_vault 解锁
    /// **后置条件**:
    /// - 在 OWS policies/ 目录创建 policy 文件
    /// - 返回 OWS policy_id (UUID v4)
    async fn register_policy_executable(
        &self,
        vault: &VaultHandle,
        name: &str,
        executable_path: &Path,
        default_action: PolicyAction,
    ) -> Result<String, GradienceError>;

    /// 附加 API Key 到 Vault 并绑定策略
    ///
    /// **前置条件**:
    /// - wallet_id 属于 vault
    /// - policy_ids 已注册在 OWS 中
    /// **后置条件**:
    /// - 生成 256-bit random token
    /// - 用 HKDF(token, salt, "ows-api-key-v1") 加密 wallet secret
    /// - 存储 key file 到 ~/.ows/keys/
    /// - 返回 raw_token (ows_key_<64 hex>)，只出现一次
    async fn attach_api_key_and_policies(
        &self,
        vault: &VaultHandle,
        wallet_id: &WalletId,
        api_key_name: &str,
        policy_ids: Vec<String>,
    ) -> Result<GradienceApiKey, GradienceError>;

    /// 创建新钱包
    ///
    /// **前置条件**: vault 已解锁
    /// **后置条件**:
    /// - 生成 BIP-39 mnemonic
    /// - 为 OWS 支持的所有链族派生 account 0
    /// - 保存 wallet file 到 ~/.ows/wallets/
    async fn create_wallet(
        &self,
        vault: &VaultHandle,
        name: &str,
        derivation_params: DerivationParams,
    ) -> Result<WalletDescriptor, GradienceError>;

    /// 签名交易
    ///
    /// **前置条件**:
    /// - 如果是 agent mode: credential 是 ows_key_...，所有 policy 已通过
    /// - 如果是 owner mode: credential 是 passphrase，跳过 policy
    /// **后置条件**:
    /// - 私钥在内存中派生，签名后立即 zeroize
    /// - 返回 SignedTransaction
    async fn sign_transaction(
        &self,
        vault: &VaultHandle,
        wallet_id: &WalletId,
        chain: &ChainId,
        tx: &Transaction,
        credential: &str,
    ) -> Result<SignedTransaction, GradienceError>;

    /// 广播已签名交易
    ///
    /// **前置条件**: signed_tx 是有效签名
    /// **后置条件**: 返回 tx_hash (可能在链上 pending)
    async fn broadcast(
        &self,
        chain: &ChainId,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<TxHash, GradienceError>;
}
```

### 2.2 关联类型

```rust
/// Vault 内存句柄 (OWS 隔离)
pub struct VaultHandle {
    _opaque: (), // 不暴露内部结构
}

/// OWS 策略动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    Deny,
    // v2.0+ Warn,
}

/// 交易数据 (序列化后)
#[derive(Debug, Clone)]
pub struct Transaction {
    pub to: Option<Address>,
    pub value: AtomicAmount,
    pub data: Vec<u8>,
    pub raw_hex: String,
}

/// 签名后交易
#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub raw_hex: String,
    pub chain_id: ChainId,
}

/// 派生参数
#[derive(Debug, Clone)]
pub struct DerivationParams {
    pub account_index: u32,
}

impl Default for DerivationParams {
    fn default() -> Self {
        Self { account_index: 0 }
    }
}
```

### 2.3 错误码映射

| Error | 原因 | HTTP 映射 |
|---|---|---|
| `OwsPolicyDenied` | OWS 内置策略拒绝 | 403 |
| `OwsInvalidCredential` | credential 格式错误 | 401 |
| `OwsWalletNotFound` | wallet_id 不存在 | 404 |
| `OwsKeyExpired` | API key 已过期 | 401 |
| `OwsExecutableTimeout` | policy executable 超时 | 500 |
| `OwsBroadcastFailure` | 链上广播失败 | 502 |

---

## 3. 策略引擎 (`policy/engine.rs`)

### 3.1 数据结构

```rust
/// 策略定义 (数据库存储格式)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub name: String,
    pub wallet_id: WalletId,
    pub workspace_id: Option<String>,
    pub rules: Vec<Rule>,
    pub priority: i32, // 0=workspace, 1=wallet/agent
    pub status: PolicyStatus,
    pub version: i32,
    pub created_at: String, // ISO-8601
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyStatus {
    Active,
    Paused,
    Deleted,
}

/// 单条规则
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Rule {
    SpendLimit { max: AtomicAmount, token: String },
    DailyLimit { max: AtomicAmount, token: String },
    MonthlyLimit { max: AtomicAmount, token: String },
    ChainWhitelist { chain_ids: Vec<ChainId> },
    ContractWhitelist { contracts: Vec<Address> },
    OperationType { allowed: Vec<String> },
    TimeWindow { start_hour: u8, end_hour: u8, timezone: String },
    MaxTokensPerCall { limit: u64 },
    ModelWhitelist { models: Vec<String> },
    Custom { config: serde_json::Value },
}

/// 评估上下文
#[derive(Debug, Clone)]
pub struct EvalContext {
    pub wallet_id: WalletId,
    pub api_key_id: ApiKeyId,
    pub chain_id: ChainId,
    pub transaction: Transaction,
    pub intent: Option<Intent>,
    pub timestamp_ms: TimestampMs,
}

/// 交易意图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub intent_type: String, // "swap" | "transfer" | "pay" | "stake"
    pub from_token: Option<String>,
    pub to_token: Option<String>,
    pub estimated_value_usd: Option<f64>,
    pub target_protocol: Option<String>,
    pub risk_score: Option<f64>, // 0.0 - 1.0
}

/// 评估结果
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub decision: Decision,
    pub reasons: Vec<String>,
    pub matched_intent: Option<Intent>,
    pub dynamic_adjustments: Vec<DynamicAdjustment>,
    pub required_approvals: Vec<ApprovalRequirement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
    Warn,
}

/// 动态策略调整
#[derive(Debug, Clone)]
pub struct DynamicAdjustment {
    pub source: String,      // "reputation" | "market_risk" | "behavior"
    pub multiplier: f64,     // e.g. 0.8 = tighten by 20%
    pub reason: String,
}

/// 审批要求
#[derive(Debug, Clone)]
pub struct ApprovalRequirement {
    pub approval_type: String,
    pub message: String,
    pub expires_at_ms: TimestampMs,
}
```

### 3.2 PolicyEngine Trait

```rust
#[async_trait]
pub trait PolicyEngine: Send + Sync {
    /// 评估请求
    ///
    /// **算法**:
    /// 1. 加载 wallet 的所有 policy (workspace + wallet)
    /// 2. merge_policies_strictest 合并
    /// 3. 如果 intent 存在:
    ///    a. IntentMatcher::match(intent, templates)
    ///    b. 不匹配模板且 template.required → deny
    /// 4. 查询动态信号缓存
    ///    a. Reputation score
    ///    b. Market risk index
    ///    c. Behavior profile
    /// 5. 应用动态调整 (multiplier)
    /// 6. 逐条静态规则评估
    ///    a. spend_limit: 查询 spending_tracker
    ///    b. chain_whitelist: chain_id 匹配
    ///    c. contract_whitelist: tx.to 匹配
    ///    d. time_window: 当前时间匹配
    /// 7. 汇总结果:
    ///    - 任何 deny → Decision::Deny
    ///    - 任何 warn 且无 deny → Decision::Warn
    ///    - 全部 allow → Decision::Allow
    ///
    /// **时间复杂度**:
    /// - 纯静态规则: O(n_rules) < 1ms
    /// - 含动态信号: O(n_rules + cache_lookup) < 10ms
    async fn evaluate(
        &self,
        ctx: EvalContext,
    ) -> Result<EvalResult, GradienceError>;

    /// 合并策略 (取最严)
    fn merge(&self,
        workspace_policy: Option<&Policy>,
        wallet_policies: Vec<&Policy>,
    ) -> MergedPolicy;
}

/// 合并后的策略 (内存中只读)
#[derive(Debug, Clone)]
pub struct MergedPolicy {
    pub spend_limit: Option<AtomicAmount>,     // min
    pub daily_limit: Option<AtomicAmount>,      // min
    pub monthly_limit: Option<AtomicAmount>,    // min
    pub chain_whitelist: Option<Vec<ChainId>>,   // intersection
    pub contract_whitelist: Option<Vec<Address>>, // intersection
    pub operation_type: Option<Vec<String>>,       // intersection
    pub time_window: Option<TimeWindowRule>,      // narrowest
    pub max_tokens: Option<u64>,
    pub model_whitelist: Option<Vec<String>>,
}
```

### 3.3 merge_policies_strictest 算法

```rust
/// 合并策略 — 每种规则取最严
pub fn merge_policies_strictest(
    workspace: Option<&Policy>,
    agents: Vec<&Policy>,
) -> MergedPolicy {
    let mut merged = MergedPolicy::default();
    let all: Vec<&Policy> = workspace.into_iter().chain(agents.into_iter()).collect();

    // Spend limit: 取最小值
    merged.spend_limit = min_amount(
        all.iter().filter_map(|p| p.spend_limit())
    );

    // Daily limit: 取最小值
    merged.daily_limit = min_amount(
        all.iter().filter_map(|p| p.daily_limit())
    );

    // Monthly limit: 取最小值
    merged.monthly_limit = min_amount(
        all.iter().filter_map(|p| p.monthly_limit())
    );

    // Chain whitelist: 取交集
    merged.chain_whitelist = intersect_vecs(
        all.iter().filter_map(|p| p.chain_whitelist())
    );

    // Contract whitelist: 取交集
    merged.contract_whitelist = intersect_vecs(
        all.iter().filter_map(|p| p.contract_whitelist())
    );

    // Operation type: 取交集
    merged.operation_type = intersect_vecs(
        all.iter().filter_map(|p| p.operation_type())
    );

    // Time window: 取最窄窗口
    merged.time_window = narrowest_window(
        all.iter().filter_map(|p| p.time_window())
    );

    // Max tokens: 取最小值
    merged.max_tokens = min_u64(
        all.iter().filter_map(|p| p.max_tokens())
    );

    // Model whitelist: 取交集
    merged.model_whitelist = intersect_vecs(
        all.iter().filter_map(|p| p.model_whitelist())
    );

    merged
}
```

---

## 4. 数据库模型 (SQL 精确)

### 4.1 完整建表语句

```sql
-- ========================================
-- 用户与身份
-- ========================================

CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT UNIQUE NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status      TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'suspended'))
);

CREATE TABLE passkey_credentials (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    credential_id   BYTEA UNIQUE NOT NULL,         -- raw credential ID
    credential_pk   BYTEA NOT NULL,                -- public key
    counter         BIGINT NOT NULL DEFAULT 0,
    transports      TEXT[] NOT NULL DEFAULT '{}',
    device_name     TEXT,
    last_used_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_passkey_user ON passkey_credentials(user_id);

-- ========================================
-- 团队/工作空间
-- ========================================

CREATE TABLE workspaces (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    owner_id        UUID NOT NULL REFERENCES users(id),
    plan            TEXT NOT NULL DEFAULT 'free'
        CHECK (plan IN ('free', 'pro', 'team', 'enterprise')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_workspaces_owner ON workspaces(owner_id);

CREATE TABLE workspace_members (
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role            TEXT NOT NULL DEFAULT 'member'
        CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
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
    status          TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'suspended', 'revoked')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_wallets_owner ON wallets(owner_id);
CREATE INDEX idx_wallets_workspace ON wallets(workspace_id);

CREATE TABLE wallet_addresses (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    chain_id        TEXT NOT NULL,
    address         TEXT NOT NULL,
    derivation_path TEXT NOT NULL,
    UNIQUE(wallet_id, chain_id)
);

CREATE INDEX idx_wallet_addresses_wallet ON wallet_addresses(wallet_id);

CREATE TABLE api_keys (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    key_hash        BYTEA NOT NULL,        -- SHA-256
    permissions     TEXT[] NOT NULL DEFAULT '{sign, read}',
    expires_at      TIMESTAMPTZ,
    last_used_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_wallet ON api_keys(wallet_id);
CREATE INDEX idx_api_keys_hash ON api_keys(key_hash);

-- ========================================
-- 策略
-- ========================================

CREATE TABLE policies (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name            TEXT NOT NULL,
    wallet_id       UUID REFERENCES wallets(id) ON DELETE CASCADE,
    workspace_id    UUID REFERENCES workspaces(id) ON DELETE CASCADE,
    rules_json      JSONB NOT NULL,
    priority        INTEGER NOT NULL DEFAULT 1
        CHECK (priority IN (0, 1)),
    status          TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'paused', 'deleted')),
    version         INTEGER NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 约束: wallet_id 和 workspace_id 至少有一个非空
    CONSTRAINT chk_policy_scope CHECK (
        wallet_id IS NOT NULL OR workspace_id IS NOT NULL
    )
);

CREATE INDEX idx_policies_wallet ON policies(wallet_id, status);
CREATE INDEX idx_policies_workspace ON policies(workspace_id, status);

-- ========================================
-- 额度追踪 (含跨 Agent 预算共享)
-- ========================================

CREATE TABLE spending_trackers (
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    workspace_id    UUID,                    -- 跨 Agent 预算共享
    rule_type       TEXT NOT NULL
        CHECK (rule_type IN ('daily', 'monthly')),
    token_address   TEXT NOT NULL,
    chain_id        TEXT NOT NULL,
    period          TEXT NOT NULL,           -- "2026-04-07" 或 "2026-04"
    spent_amount    TEXT NOT NULL DEFAULT '0',
    reset_at        TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (wallet_id, rule_type, token_address, chain_id, period)
);

CREATE INDEX idx_spending_workspace ON spending_trackers(workspace_id, period);

-- ========================================
-- Warn 审批队列
-- ========================================

CREATE TABLE policy_approvals (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id       UUID NOT NULL REFERENCES policies(id),
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    request_json    JSONB NOT NULL,
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'expired')),
    approved_by     UUID REFERENCES users(id),
    approved_at     TIMESTAMPTZ,
    expires_at      TIMESTAMPTZ NOT NULL
        DEFAULT (NOW() + INTERVAL '30 minutes'),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_approvals_wallet ON policy_approvals(wallet_id, status);
CREATE INDEX idx_approvals_pending ON policy_approvals(status, expires_at)
    WHERE status = 'pending';

-- ========================================
-- 审计日志
-- ========================================

CREATE TABLE audit_logs (
    id              BIGSERIAL PRIMARY KEY,
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    api_key_id      UUID REFERENCES api_keys(id),
    action          TEXT NOT NULL,
    context_json    JSONB NOT NULL,
    intent_json     JSONB,
    decision        TEXT NOT NULL
        CHECK (decision IN ('allowed', 'denied', 'warned')),
    decision_reason TEXT,
    dynamic_factors JSONB,
    tx_hash         TEXT,
    anchor_tx_hash  TEXT,                    -- v2.0 链上锚定
    anchor_root     TEXT,
    prev_hash       TEXT NOT NULL,             -- HMAC 链式
    current_hash    TEXT NOT NULL,             -- HMAC(key, content+prev)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_wallet_time ON audit_logs(wallet_id, created_at DESC);
CREATE INDEX idx_audit_anchor ON audit_logs(anchor_root)
    WHERE anchor_root IS NOT NULL;

-- ========================================
-- 锚定批次
-- ========================================

CREATE TABLE anchor_batches (
    id              BIGSERIAL PRIMARY KEY,
    root            TEXT NOT NULL,
    prev_root       TEXT,
    log_start_index BIGINT NOT NULL,
    log_end_index   BIGINT NOT NULL,
    leaf_count      INTEGER NOT NULL,
    tx_hash         TEXT NOT NULL,
    block_number    BIGINT,
    anchored_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_anchor_root ON anchor_batches(root);

-- ========================================
-- 支付记录
-- ========================================

CREATE TABLE payment_records (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    protocol        TEXT NOT NULL
        CHECK (protocol IN ('x402', 'mpp', 'hsp')),
    amount          TEXT NOT NULL,
    token           TEXT NOT NULL,
    recipient       TEXT NOT NULL,
    status          TEXT NOT NULL
        CHECK (status IN ('pending', 'completed', 'failed')),
    tx_hash         TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ========================================
-- AI Gateway (v1.5+)
-- ========================================

CREATE TABLE ai_balances (
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    token           TEXT NOT NULL DEFAULT 'USDC',
    balance_raw     TEXT NOT NULL DEFAULT '0',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (wallet_id, token)
);

CREATE TABLE llm_call_logs (
    id              BIGSERIAL PRIMARY KEY,
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    api_key_id      UUID REFERENCES api_keys(id),
    provider        TEXT NOT NULL,
    model           TEXT NOT NULL,
    input_tokens    BIGINT NOT NULL,
    output_tokens   BIGINT NOT NULL,
    cached_tokens   BIGINT DEFAULT 0,
    cost_raw        TEXT NOT NULL,
    duration_ms     INTEGER,
    status          TEXT NOT NULL DEFAULT 'success'
        CHECK (status IN ('success', 'denied', 'budget_exceeded')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_llm_wallet ON llm_call_logs(wallet_id, created_at DESC);

CREATE TABLE model_pricing (
    id              BIGSERIAL PRIMARY KEY,
    provider        TEXT NOT NULL,
    model           TEXT NOT NULL,
    input_per_m     BIGINT NOT NULL,
    output_per_m    BIGINT NOT NULL,
    cache_per_m     BIGINT NOT NULL DEFAULT 0,
    currency        TEXT NOT NULL DEFAULT 'USDC',
    effective_from  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    effective_to    TIMESTAMPTZ,
    UNIQUE(provider, model, effective_from)
);
```

### 4.2 数据库 Rust 模型

```rust
// crates/gradience-db/src/models.rs

use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, sqlx::FromRow)]
pub struct WalletRow {
    pub id: uuid::Uuid,
    pub name: String,
    pub owner_id: uuid::Uuid,
    pub workspace_id: Option<uuid::Uuid>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: i64,
    pub wallet_id: uuid::Uuid,
    pub api_key_id: Option<uuid::Uuid>,
    pub action: String,
    pub context_json: Value,
    pub intent_json: Option<Value>,
    pub decision: String,
    pub tx_hash: Option<String>,
    pub prev_hash: String,
    pub current_hash: String,
    pub created_at: DateTime<Utc>,
}
```

---

## 5. MCP Server Tools (`gradience-mcp`)

### 5.1 Tool 清单

```rust
pub enum GradienceTool {
    SignTransaction,
    SignMessage,
    SignAndSend,
    GetBalance,
    Swap,
    Pay,
    LlmGenerate,      // AI Gateway
    AiBalance,        // AI Gateway
    AiModels,         // AI Gateway
}
```

### 5.2 `sign_transaction`

```json
{
  "name": "sign_transaction",
  "description": "Sign a blockchain transaction. Policy evaluation is enforced automatically.",
  "inputSchema": {
    "type": "object",
    "required": ["walletId", "chainId", "transaction"],
    "properties": {
      "walletId": { "type": "string", "format": "uuid" },
      "chainId": { "type": "string", "description": "CAIP-2 chain ID, e.g. eip155:8453" },
      "transaction": {
        "type": "object",
        "required": ["value", "data"],
        "properties": {
          "to": { "type": "string", "description": "Recipient address" },
          "value": { "type": "string", "description": "Value in wei/lamports/etc." },
          "data": { "type": "string", "description": "Hex-encoded calldata" }
        }
      },
      "intent": {
        "type": "object",
        "properties": {
          "intentType": { "type": "string", "enum": ["swap", "transfer", "pay", "stake"] },
          "fromToken": { "type": "string" },
          "toToken": { "type": "string" },
          "estimatedValueUsd": { "type": "number" }
        }
      },
      "simulate": { "type": "boolean", "default": true }
    }
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "signature": { "type": "string" },
      "txHash": { "type": "string" },
      "decision": { "type": "string", "enum": ["allowed", "denied", "warned"] },
      "cost": { "type": "string", "description": "Optional model usage cost" }
    }
  },
  "errors": {
    "POLICY_DENIED": "Transaction blocked by active policy",
    "INSUFFICIENT_BALANCE": "Wallet balance too low for transaction",
    "INVALID_CHAIN": "Unsupported chain identifier",
    "SIGNATURE_FAILED": "Internal signing error"
  }
}
```

### 5.3 `get_balance`

```json
{
  "name": "get_balance",
  "description": "Get token balances for a wallet on a specific chain.",
  "inputSchema": {
    "type": "object",
    "required": ["walletId", "chainId"],
    "properties": {
      "walletId": { "type": "string" },
      "chainId": { "type": "string" },
      "tokenAddresses": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Optional list of token contract addresses"
      }
    }
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "native": { "type": "string" },
      "tokens": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "address": { "type": "string" },
            "symbol": { "type": "string" },
            "balance": { "type": "string" }
          }
        }
      }
    }
  }
}
```

### 5.4 `llm_generate` (AI Gateway)

```json
{
  "name": "llm_generate",
  "description": "Generate text with an LLM through Gradience AI Gateway. Auto-billed against AI balance.",
  "inputSchema": {
    "type": "object",
    "required": ["provider", "model", "messages"],
    "properties": {
      "provider": {
        "type": "string",
        "enum": ["anthropic", "openai", "google", "xai"]
      },
      "model": { "type": "string" },
      "messages": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "role": { "type": "string", "enum": ["system", "user", "assistant"] },
            "content": { "type": "string" }
          }
        }
      },
      "maxTokens": { "type": "integer", "default": 4096 },
      "temperature": { "type": "number", "minimum": 0, "maximum": 2 }
    }
  },
  "outputSchema": {
    "type": "object",
    "properties": {
      "content": { "type": "string" },
      "usage": {
        "type": "object",
        "properties": {
          "input": { "type": "integer" },
          "output": { "type": "integer" },
          "cached": { "type": "integer" }
        }
      },
      "cost": {
        "type": "object",
        "properties": {
          "amount": { "type": "string" },
          "currency": { "type": "string" }
        }
      }
    }
  }
}
```

---

## 6. CLI 命令 (`gradience-cli`)

### 6.1 命令树

```rust
#[derive(clap::Parser)]
#[command(name = "gradience")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(clap::Subcommand)]
pub enum Commands {
    Auth(AuthCmd),
    Agent(AgentCmd),
    Policy(PolicyCmd),
    Dex(DexCmd),
    Audit(AuditCmd),
    Team(TeamCmd),
    Ai(AiCmd),
    Mcp(McpCmd),
}
```

### 6.2 详细命令

```rust
// --- auth ---
#[derive(clap::Subcommand)]
pub enum AuthCmd {
    Login,
    Logout,
    Status,
}

// --- agent (wallet) ---
#[derive(clap::Subcommand)]
pub enum AgentCmd {
    Create { #[arg(long)] name: String },
    List,
    Fund { wallet_id: String, amount: String, #[arg(long)] chain: String },
    Balance { wallet_id: String, #[arg(long)] chain: String },
}

// --- policy ---
#[derive(clap::Subcommand)]
pub enum PolicyCmd {
    Set {
        wallet_id: String,
        #[arg(long)] file: String,
    },
    List { wallet_id: String },
    Test {
        wallet_id: String,
        #[arg(long)] tx_file: String,
    },
}

// --- audit ---
#[derive(clap::Subcommand)]
pub enum AuditCmd {
    Export {
        #[arg(long)] wallet_id: Option<String>,
        #[arg(long)] format: String, // json | csv
        #[arg(long)] output: String,
    },
    Verify {
        #[arg(long)] log_id: i64,
    },
}

// --- ai (gateway) ---
#[derive(clap::Subcommand)]
pub enum AiCmd {
    Topup {
        #[arg(long)] wallet_id: String,
        #[arg(long)] amount: String,
        #[arg(long)] token: String,
    },
    Balance { wallet_id: String },
    Models,
}

// --- mcp ---
#[derive(clap::Subcommand)]
pub enum McpCmd {
    Serve {
        #[arg(long, default_value = "3000")] port: u16,
    },
}
```

---

## 7. RPC 模块 (`rpc/`)

### 7.1 EVM RPC (`rpc/evm.rs`)

```rust
/// EVM 兼容链通用 RPC 客户端
/// 支持: Ethereum, Base, Arbitrum, Optimism, BSC, Polygon, Avalanche, X Layer, HashKey Chain
pub struct EvmRpcClient {
    chain_id: String,
    rpc_url: String,
    client: reqwest::Client,
}

impl EvmRpcClient {
    pub async fn get_balance(
        &self,
        address: &str,
        block: BlockTag,
    ) -> Result<AtomicAmount, GradienceError>;

    pub async fn estimate_gas(
        &self,
        tx: &EvmTransaction,
    ) -> Result<u64, GradienceError>;

    pub async fn send_raw_transaction(
        &self,
        signed_tx: &str,
    ) -> Result<TxHash, GradienceError>;

    pub async fn get_transaction_receipt(
        &self,
        tx_hash: &str,
    ) -> Result<Option<EvmReceipt>, GradienceError>;
}

#[derive(Debug, Clone)]
pub struct EvmTransaction {
    pub from: Address,
    pub to: Option<Address>,
    pub value: AtomicAmount,
    pub data: Vec<u8>,
    pub gas_limit: u64,
    pub max_fee_per_gas: AtomicAmount,
    pub max_priority_fee_per_gas: AtomicAmount,
    pub nonce: u64,
    pub chain_id: u64,
}
```

### 7.2 CAIP-2 路由 (`rpc/multi.rs`)

```rust
pub struct RpcManager {
    clients: HashMap<ChainId, Box<dyn ChainRpcClient>>,
}

#[async_trait]
pub trait ChainRpcClient: Send + Sync {
    async fn get_balance(&self, address: &str) -> Result<AtomicAmount, GradienceError>;
    async fn broadcast(&self, signed_tx: &SignedTransaction) -> Result<TxHash, GradienceError>;
}

impl RpcManager {
    pub fn new(configs: Vec<ChainConfig>) -> Self {
        // 根据 chain_id 前缀路由:
        // "eip155:*"     → EvmRpcClient
        // "solana:*"     → SolanaRpcClient
        // "bip122:*"     → BitcoinRpcClient
        // "stellar:*"    → StellarRpcClient
        // "tron:*"       → TronRpcClient
        // "sui:*"        → SuiRpcClient
        // "cosmos:*"     → CosmosRpcClient
        // "xrpl:*"       → XrplRpcClient
        // "spark:*"      → SparkRpcClient
        // "fil:*"        → FilecoinRpcClient
    }
}
```

---

## 8. 审计 + Merkle 锚定 (`audit/`)

### 8.1 HMAC 链式签名

```rust
/// 计算 audit log 的 HMAC 链式 hash
/// current_hash = HMAC(secret_key, prev_hash || serialize(log_entry))
pub fn compute_audit_hash(
    secret_key: &[u8; 32],
    prev_hash: ,str,
    entry: &AuditLogEntry,
) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret_key).unwrap();
    mac.update(prev_hash.as_bytes());
    mac.update(serde_json::to_vec(entry).unwrap().as_slice());
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}
```

### 8.2 Merkle Tree

```rust
pub struct MerkleTree {
    pub leaves: Vec<[u8; 32]>,
    pub layers: Vec<Vec<[u8; 32]>>,
    pub root: [u8; 32],
}

impl MerkleTree {
    pub fn new(leaves: Vec<[u8; 32]>) -> Self;
    pub fn generate_proof(&self,
        leaf_index: usize,
    ) -> Option<(Vec<[u8; 32]>, [u8; 32])>;
}
```

### 8.3 锚定服务接口

```rust
#[async_trait]
pub trait AnchorService: Send + Sync {
    /// 提交 root 到 HashKey Chain
    async fn submit_root(
        &self,
        root: [u8; 32],
        log_start: i64,
        log_end: i64,
        leaf_count: usize,
    ) -> Result<AnchorReceipt, GradienceError>;
}
```

---

## 9. REST API (`gradience-api`)

### 9.1 路由设计

| Method | Path | 说明 |
|---|---|---|
| POST | `/auth/register` | Passkey 注册开始 |
| POST | `/auth/login` | Passkey 认证开始 |
| POST | `/auth/verify` | 验证 Passkey assertion |
| GET | `/wallets` | 列出用户钱包 |
| POST | `/wallets` | 创建钱包 |
| GET | `/wallets/:id` | 钱包详情 |
| GET | `/wallets/:id/balance` | 余额查询 |
| GET | `/wallets/:id/audit` | 审计日志 |
| POST | `/policies` | 创建策略 |
| GET | `/policies/:id` | 策略详情 |
| POST | `/swap/quote` | DEX 报价 |
| POST | `/swap` | 执行 swap |
| POST | `/payments` | 发起支付 |
| GET | `/payments/:id` | 支付状态 |
| GET | `/ai/balance` | AI 余额 |
| GET | `/ai/models` | 模型列表 |
| POST | `/ai/generate` | LLM 生成 |
| GET | `/ws` | WebSocket 连接 |

### 9.2 状态码映射

| 状态码 | 使用场景 |
|---|---|
| 200 | 正常返回 |
| 201 | 创建成功 (wallet, policy) |
| 400 | 参数错误 |
| 401 | 认证失败 |
| 403 | 策略拒绝 |
| 404 | 资源不存在 |
| 409 | 冲突 (重复创建) |
| 429 | 限流 |
| 500 | 内部错误 |
| 502 | 链上广播失败 |

---

## 10. 边界条件

1. **钱包创建时 mnemonic 重复** — 概率可忽略 (2^256)，无需处理
2. **API Key 创建后未保存 raw_token** — 无法恢复，必须重新创建
3. **Policy executable 超时 (5s)** — 默认 deny，记录 timeout 原因
4. **动态信号缓存过期且 API 不可用** — fallback_deny (收紧策略)
5. **链上广播成功但 receipt 获取超时** — 记录 pending 状态，后台轮询
6. **spending tracker 并发写入冲突** — 数据库事务 + optimistic locking
7. **Merkle anchor 链上 Gas 不足** — 跳过该批次，标记为 pending_anchor
8. **用户同时修改 workspace policy 和 wallet policy** — 后写入者生效 (last-write-wins)
9. **MCP tool 传入非法 chain_id** — 400 InvalidChain 在进入 OWS 前拦截
10. **审计日志表达到 1TB** — 自动按 month 分表，查询带时间范围

---

## 验收标准

- [x] OWS adapter trait 定义完整 (8 个方法，含前/后置条件)
- [x] PolicyEngine trait + merge 算法精确到伪代码
- [x] 数据库 schema 包含全部 15 张表 + 索引 + 约束
- [x] MCP tools 定义了 sign_tx / get_balance / llm_generate (含 input/output schema)
- [x] CLI 命令树覆盖 auth / agent / policy / audit / ai / mcp
- [x] EVM RPC client 接口定义完整
- [x] Merkle tree + HMAC 链式签名算法定义
- [x] REST API 状态码映射 + 路由表
- [x] 10 个边界条件明确

**本技术规格通过后可进入 Phase 4: Task Breakdown**
