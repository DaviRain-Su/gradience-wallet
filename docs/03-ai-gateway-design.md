# Phase 3: Gradience AI Gateway — Agent 付费模型网关

> Status: Draft
> Date: 2026-04-07
> Context: 基于 Gradience Wallet 架构，为 AI Agent 提供付费模型调用 + 账户管理 + 策略控制

---

## 1. 产品定位

### 问题
现在的 AI Agent（Claude Code、Codex、Grok DeepSearch、pi-mono 等）要么：
1. **需要用户自管理 API Key** — 不安全，无用量控制，无审计
2. **自带简单的 cost tracking** — 但没有策略、没有预算、没有支付方式

### 解决方案
**Gradience AI Gateway** 是 Gradience Wallet 的上层产品：
- 用户通过 Gradience Wallet 充值/管理预算
- Agent 不需要知道 API Key，通过 Gradience Gateway 调用 LLM
- Gateway 自动处理：身份认证 → 策略评估 → x402/MPP 支付 → 调用 LLM → 审计记录

```
用户/Agent ──→ Gradience AI Gateway ──→ LLM Provider
                   │
                   ├── 策略引擎 (限额/意图/预算)
                   ├── 支付层 (x402/MPP/HSP)
                   ├── 审计日志 (Merkle 锚定)
                   └── 账户管理 (余额/用量/订阅)
```

---

## 2. 核心概念

### 2.1 账户模型

```
User (Passkey 认证)
  └── Gradience Wallet
      ├── Wallet 1 (Agent 钱包)
      │   ├── AI Balance: 500 USDC  ← 预存 LLM 调用额度
      │   ├── Policy: daily_limit=50, per_call_limit=5
      │   ├── Usage: 已用 $12.50 / $500
      │   └── Agent API Key: ows_key_ai_xxx
      │
      └── Wallet 2 (DeFi 钱包 — 与 AI 隔离)
          └── Policy: 链上 DeFi 策略
```

### 2.2 付费模式

| 模式 | 说明 | 适合场景 |
|---|---|---|
| **Prepaid (预付费)** | 用户预先充值 AI Balance, 按用量扣减 | 个人用户, 可控预算 |
| **Pay-per-use (按量付费)** | 每次调用 LLM 时自动 x402/MPP 支付 | 不常用, 低摩擦 |
| **Subscription (订阅)** | 月付固定费用, 享一定额度 (如 $19/月 1000 次调用) | 高频用户, 团队 |
| **Team Pool (团队池)** | Workspace 共享 LLM 预算, 按成员分配 | 企业/团队 |

### 2.3 支持的 LLM Provider

| Provider | 接入方式 | 计费单位 |
|---|---|---|
| OpenAI (GPT-4o, o3) | API Key Gateway 管理 | input/output tokens |
| Anthropic (Claude Sonnet, Opus) | API Key Gateway 管理 | input/output/cache tokens |
| Google (Gemini) | API Key Gateway 管理 | input/output tokens |
| xAI (Grok) | API Key Gateway 管理 | input/output tokens |
| 自部署 (vLLM/Ollama) | 本地, 不计费 | 可选内部计费 (GPU time) |

---

## 3. 架构设计

### 3.1 模块依赖

```
gradience-wallet/ (现有 workspace)
│
├── crates/
│   ├── gradience-core/          ← 现有：策略引擎、钱包、支付
│   │
│   ├── gradience-ai-gateway/    ← 新增：AI 付费网关
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── mod.rs
│   │       ├── provider/        # LLM Provider 适配器
│   │       │   ├── mod.rs
│   │       │   ├── openai.rs
│   │       │   ├── anthropic.rs
│   │       │   ├── google.rs
│   │       │   └── xai.rs
│   │       ├── usage.rs         # 用量追踪 (token 级)
│   │       ├── billing.rs       # 计费引擎 (按模型定价)
│   │       ├── account.rs       # 账户管理 (余额/订阅/充值)
│   │       ├── gateway.rs       # 网关入口 (MCP tool + HTTP API)
│   │       └── pricing.rs       # 定价表 (token → USD)
│   │
│   ├── gradience-mcp/           ← 现有：新增 AI Gateway tool
│   │   └── src/tools/
│   │       ├── llm_generate.rs  # 新增: chat completion tool
│   │       ├── llm_usage.rs     # 新增: 查询用量/余额
│   │       └── llm_models.rs    # 新增: 列出可用模型
│   │
│   └── gradience-api/           ← 现有：新增 /ai/* 路由
│       └── src/routes/
│           └── ai.rs            # 新增: REST API for Web Dashboard
│
└── web/                         # Web Dashboard
    └── src/
        ├── pages/ai-gateway/    # 新增: AI 用量管理页面
        │   ├── overview.tsx     # 总览: 余额/用量/趋势
        │   ├── models.tsx       # 可用模型 + 定价
        │   ├── usage.tsx        # 用量明细 (按 Agent/模型/时间)
        │   └── topup.tsx       # 充值 AI Balance
        └── api/ai.ts            # AI Gateway API client
```

### 3.2 数据模型（新增表）

```sql
-- ========================================
-- AI Gateway — 账户与用量
-- ========================================

-- AI 专属余额 (从钱包分出)
CREATE TABLE ai_balances (
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    token           TEXT NOT NULL DEFAULT 'USDC',
    balance_raw     TEXT NOT NULL DEFAULT '0',     -- 最小单位 (USDC: 6 decimals)
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (wallet_id, token)
);

-- 订阅计划 (v1.5+)
CREATE TABLE ai_subscriptions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id       UUID NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    plan_type       TEXT NOT NULL,                 -- 'pro_unlimited' | 'team_10' | ...
    monthly_quota   BIGINT NOT NULL,               -- 月调用次数/额度
    used_this_month BIGINT NOT NULL DEFAULT 0,
    period_start    TIMESTAMPTZ NOT NULL,
    period_end      TIMESTAMPTZ NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- LLM 调用记录 (细粒度审计)
CREATE TABLE llm_call_logs (
    id              BIGSERIAL PRIMARY KEY,
    wallet_id       UUID NOT NULL REFERENCES wallets(id),
    api_key_id      UUID REFERENCES api_keys(id),
    provider        TEXT NOT NULL,                 -- 'openai' | 'anthropic' | 'google' | 'xai'
    model           TEXT NOT NULL,                 -- 'claude-sonnet-4-20250514'
    input_tokens    BIGINT NOT NULL,
    output_tokens   BIGINT NOT NULL,
    cached_tokens   BIGINT DEFAULT 0,
    cost_raw        TEXT NOT NULL,                 -- 实际花费 (最小单位)
    duration_ms     INTEGER,
    status          TEXT NOT NULL DEFAULT 'success', -- success | denied | budget_exceeded
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 模型定价表 (可动态更新)
CREATE TABLE model_pricing (
    id              BIGSERIAL PRIMARY KEY,
    provider        TEXT NOT NULL,
    model           TEXT NOT NULL,
    input_per_m     BIGINT NOT NULL,               -- 每百万 input tokens 的价格 (原子单位)
    output_per_m    BIGINT NOT NULL,               -- 每百万 output tokens 的价格
    cache_per_m     BIGINT NOT NULL DEFAULT 0,     -- 每百万 cached tokens 的价格
    currency        TEXT NOT NULL DEFAULT 'USDC',  -- 计价货币
    effective_from  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    effective_to    TIMESTAMPTZ,                   -- NULL = 至今有效
    UNIQUE(provider, model, effective_from)
);
```

### 3.3 核心流程

```
Agent 请求 LLM 调用
    │
    │ MCP: llm_generate({
    │   model: "claude-sonnet-4-20250514",
    │   messages: [...],
    │   max_tokens: 4096
    │ })
    ▼
Gradience AI Gateway (gateway.rs)
    │
    ├── 1. 身份验证 (API Key)
    │
    ├── 2. 账户检查
    │   ├── 查询 ai_balances → 余额是否足够?
    │   ├── 查询 spending_tracker → 日/月限额是否超?
    │   └── 查询 ai_subscriptions → 订阅额度是否用完?
    │
    ├── 3. 策略评估 (复用 Gradience Policy Engine)
    │   ├── model_whitelist: 该 Agent 允许调用的模型?
    │   ├── max_daily_cost: 每日预算是否超?
    │   ├── max_tokens_per_call: 单次 token 上限?
    │   └── intent_check: 请求意图是否合法?
    │
    ├── 4. 费用预估 & 支付
    │   ├── 预估费用 = pricing(model) x 预估 tokens
    │   ├── 冻结余额 (预扣)
    │   └── 或 x402 即时支付
    │
    ├── 5. 调用 LLM Provider
    │   └── anthropic_client.complete(prompt, max_tokens)
    │       → 获取 response + 实际 token 用量
    │
    ├── 6. 费用结算
    │   ├── 实际扣费 (多退少补)
    │   └── 记录 llm_call_logs
    │
    ├── 7. 审计 + 通知
    │   ├── 写入 audit_logs
    │   ├── 更新 spending_trackers
    │   └── WebSocket 推送余额更新
    │
    └── 8. 返回 Agent
        └── { content: "...", usage: { input: 1234, output: 567 }, cost: 0.05 USDC }
```

### 3.4 定价引擎

```rust
/// 模型定价 (原子单位: USDC = 10^-6)
struct ModelPricing {
    provider: String,
    model: String,
    input_per_million: u64,    // 每百万 input tokens 价格 (USDC 最小单位)
    output_per_million: u64,   // 每百万 output tokens
    cached_per_million: u64,   // 每百万 cached tokens
}

impl ModelPricing {
    /// 计算调用费用
    fn calculate(&self, input_tokens: u64, output_tokens: u64, cached_tokens: u64) -> u64 {
        let input_cost = (input_tokens * self.input_per_million) / 1_000_000;
        let output_cost = (output_tokens * self.output_per_million) / 1_000_000;
        let cached_cost = (cached_tokens * self.cached_per_million) / 1_000_000;
        input_cost + output_cost + cached_cost
    }
}

// 当前定价 (2026-04, 示例)
// Claude Sonnet 4:   input=$3/M,  output=$15/M,  cached=$0.30/M
// Claude Opus 4:     input=$15/M, output=$75/M,   cached=$1.50/M
// GPT-4o:            input=$2.5/M, output=$10/M,  cached=$1.25/M
// Gemini 2.5 Pro:    input=$2.5/M, output=$10/M,  cached=$0.625/M
```

---

## 4. 与 pi-mono 的集成

### 4.1 方案：Gradience Gateway 作为 LLM Provider 代理

pi-mono 的 `@mariozechner/pi-ai` 通过 models.json 配置 LLM Provider。我们添加一个 **Gradience Gateway Provider**：

```json
// models.json 新增
{
  "id": "gradience-gateway",
  "name": "Gradience AI Gateway",
  "provider": "gradience",
  "models": [
    {
      "id": "claude-sonnet-4-20250514",
      "name": "Claude Sonnet 4 (via Gradience)",
      "contextWindow": 200000,
      "pricing": { "input": 3.0, "output": 15.0, "currency": "USDC" }
    },
    {
      "id": "gpt-4o-2026-03",
      "name": "GPT-4o (via Gradience)",
      "contextWindow": 128000,
      "pricing": { "input": 2.5, "output": 10.0, "currency": "USDC" }
    }
  ]
}
```

### 4.2 Agent Extension（pi-mono 插件）

```typescript
// extensions/gradience-payment.ts
import { Extension, AgentContext } from '@gradience/pi-agent-core';

export class GradiencePaymentExtension implements Extension {
  name = 'gradience-payment';

  async onBeforeModelCall(ctx: AgentContext) {
    // 1. 检查 Gradience AI 余额
    const balance = await fetchGradienceBalance(ctx.walletId);
    if (balance.remaining < ctx.estimatedCost) {
      throw new Error(`Insufficient AI balance: ${balance.remaining} USDC`);
    }

    // 2. 检查策略 (每日限额、模型白名单等)
    const policy = await fetchGradiencePolicy(ctx.walletId);
    if (!policy.allows(ctx.model, ctx.estimatedTokens)) {
      throw new Error(`Policy denied: ${policy.denyReason}`);
    }

    // 3. 冻结费用
    await gradienceFreeze(ctx.walletId, ctx.estimatedCost);
  }

  async onAfterModelCall(ctx: AgentContext, result: ModelResult) {
    // 4. 实际结算 (多退少补)
    actualCost = gradience.calculate(result.actualTokens);
    await gradienceSettle(ctx.walletId, actualCost);

    // 5. 通知用户 (如果余额低于阈值)
    if (balanceAfter < alertThreshold) {
      await gradienceNotify(ctx.userId, 'low_balance', { remaining: balanceAfter });
    }
  }
}
```

### 4.3 用户启动方式

```bash
# 用户先配置 Gradience Wallet
gradience auth login
gradience ai topup --amount 100 --token USDC  # 充值 $100 到 AI Balance

# 然后启动 pi-mono，挂载 Gradience 扩展
pi --extension gradience-payment --wallet my-agent-wallet

# 所有 LLM 调用自动走:
# Gradience 策略 → 余额检查 → 支付 → LLM → 审计
```

---

## 5. MCP Tools 新增

```typescript
// gradience-mcp 新增 AI Gateway tools

{
  "name": "llm_generate",
  "description": "调用 LLM 模型生成回复 (自动计费和策略评估)",
  "inputSchema": {
    "provider": "string",        // "anthropic" | "openai" | "google"
    "model": "string",           // "claude-sonnet-4-20250514"
    "messages": [object],        // Chat format
    "max_tokens": "number?",     // 默认 4096
    "temperature": "number?"
  },
  "returns": {
    "content": "string",
    "usage": { "input": "number", "output": "number", "total_tokens": "number" },
    "cost": { "amount": "string", "currency": "string" }
  }
}

{
  "name": "ai_balance",
  "description": "查询 AI 调用余额和用量",
  "inputSchema": { "wallet_id": "string?" },
  "returns": {
    "balance": "string",
    "daily_used": "string",
    "monthly_used": "string",
    "daily_limit": "string",
    "monthly_limit": "string"
  }
}

{
  "name": "ai_models",
  "description": "列出可用模型及定价",
  "returns": {
    "models": [{
      "provider": "string",
      "model": "string",
      "name": "string",
      "pricing": { "input_per_m": "string", "output_per_m": "string" }
    }]
  }
}
```

---

## 6. Web Dashboard 页面

### 6.1 AI 用量总览

```
┌─────────────────────────────────────────────────┐
│  AI Balance                                    │
│                                                 │
│  $487.50 / $500.00 USDC                        │
│  ████████████████████████████████████░░░░ 97%   │
│                                                 │
│  今日已用: $12.50 / $50.00    本月: $125.00     │
│  预计剩余天数: 38 天 (按当前速度)                 │
│                                                 │
│  [充值] [设置限额] [订阅升级]                    │
└─────────────────────────────────────────────────┘

┌─────────────────┬─────────────────┬─────────────────┐
│   Claude Sonnet  │    GPT-4o       │   Claude Opus    │
│   1,234 调用     │    456 调用     │   89 调用        │
│   $87.50         │    $23.10      │   $14.40        │
│   ████████░░     │    ███░░░░░    │   ██░░░░░░░     │
└─────────────────┴─────────────────┴─────────────────┘
```

### 6.2 用量明细

```
┌─────────────────────────────────────────────────┐
│  最近 LLM 调用                                  │
│                                                 │
│  时间          | 模型              | Token   | 费用    │
│  04-07 15:23   | claude-sonnet-4   | 2.1k+   | $0.045  │
│  04-07 15:21   | claude-sonnet-4   | 1.8k+   | $0.039  │
│  04-07 14:55   | gpt-4o            | 3.2k+   | $0.040  │
│  ...                                              │
│                                                 │
│  [导出 CSV] [按月汇总]                          │
└─────────────────────────────────────────────────┘
```

---

## 7. 策略引擎集成

AI Gateway 复用 Gradience 已有的策略引擎，新增规则类型：

```json
// AI 专属策略
{
  "id": "ai-model-policy",
  "rules": [
    {
      "type": "model_whitelist",
      "models": ["claude-sonnet-4-20250514", "gpt-4o-2026-03"]
    },
    {
      "type": "max_daily_cost_usdc",
      "limit": 50000000  // 50 USDC (6 decimals)
    },
    {
      "type": "max_tokens_per_call",
      "limit": 8192
    },
    {
      "type": "max_monthly_calls",
      "limit": 5000
    }
  ],
  "action": "deny"
}
```

### 智能限额调整 (动态策略 v1.5+)

```
如果 Agent 行为正常 (通过 Intent Analysis) → 提升限额 (自动信任)
如果检测到异常 (短时间内大量调用 / 不寻常模型组合) → 收紧限额 + warn 用户
如果余额 < 10% 阈值 → 自动通知 + 触发 warn 审批
```

---

## 8. 经济模型

### 8.1 Gradience 如何盈利

| 收入来源 | 说明 | 费率 |
|---|---|---|
| **SaaS 订阅** | Pro ($19/月) / Team ($99/月) 含 AI Gateway 功能 | 固定 |
| **LLM 加价** | 从 LLM Provider 获批量折扣, Gateway 收取 5-15% 服务费 | margin |
| **充值手续费** | 法币→加密货币充值收取 1-2% | 一次性 |
| **企业 API** | 按调用量计费 (超出订阅后) | 按量 |

### 8.2 用户成本示例

```
场景: 每天使用 Claude Sonnet 4 进行 Coding (约 2,000 调用/月)

直接购买:
  - Anthropic API (5M context): ~$0.045/call × 2000 = ~$90/月

通过 Gradience AI Gateway:
  - 实际 LLM 费用: ~$90/月 (按实际 token)
  - Gradience 服务费: ~$9.00/月 (10%)
  - SaaS Pro 订阅: $19/月 (含策略引擎/Dashboard/审计)
  - 总计: ~$118/月

用户获得的价值:
  ✅ 统一钱包管理 (不是每个 Agent 单独充 API)
  ✅ 策略控制 (Agent 超限额自动拦截)
  ✅ 完整审计 (每个调用可追溯 + 链上锚定)
  ✅ 多 Agent 共享预算 (Workspace)
  ✅ Telegram/Mini App 随时查看
```

---

## 9. Hackathon 匹配

| 比赛 | 赛道 | 如何展示 |
|---|---|---|
| **HashKey Horizon** | PayFi | "Agent LLM 支付" — x402 自动支付 LLM 费用 + HSP 结算 |
| **OKX Build X** | Agentic Commerce | "AI Agent 付费调用 + Onchain OS 集成" |
| **Stellar Agents** | Agent Safety | "Agent 在策略保护下调用 LLM，防止超支/滥用" |

### 最小 Demo 路径
1. 用户在 Dashboard 充值 $100 AI Balance
2. Agent (pi-mono 或 Claude Code) 通过 MCP 调用 `llm_generate`
3. Gateway 显示: 余额检查 → 策略通过 → 预估费用 → 调用 Claude → 实际扣费
4. Dashboard 实时更新余额和用量
5. Bot 通知: "AI 余额低于 $20，是否需要充值？"

---

## 10. 实施计划 (Phase 3 → Phase 4)

| Phase | 模块 | 工作量 | 依赖 |
|---|---|---|---|
| **Phase 3.0** | DB schema + 定价表 + 余额管理 | 2 天 | 现有 DB migration infra |
| **Phase 3.1** | LLM Provider 适配器 (anthic/openai/gemini) | 3 天 | 无 |
| **Phase 3.2** | AI Gateway (计费 + 策略集成) | 3 天 | Phase 3.0 + 3.1 |
| **Phase 3.3** | MCP Tools (llm_generate / ai_balance / ai_models) | 2 天 | Phase 3.2 |
| **Phase 3.4** | Web Dashboard AI 页面 | 2 天 | REST API ready |
| **Phase 3.5** | pi-mono Extension 集成 | 1 天 | Phase 3.3 |
| **Phase 3 Demo** | 端到端 Demo (充值 → 调用 → 审计 → 通知) | 1 天 | 全 Phase 完成 |

**总计**: ~14 天可出可用版本。

---

## 11. 安全考虑

| 风险 | 缓解 |
|---|---|
| **API Key 泄露** | Gateway 统一保管，Agent 无需接触 |
| **超支** | 多层策略 (daily/monthly/subscription 限额) |
| **定价不准** | 预估 vs 实际费用，支持退款 (多退) |
| **Provider 故障** | Fallback 模型 + 不扣费 |
| **Token 价格波动** | USDC 计价 (稳定币)，避免 ETH/SOL 波动 |

---

*本文档为 Phase 3 设计草案。核心思路：**复用 Gradience Wallet 已建的策略引擎 + 支付 + 审计基础设施，向上扩展为 AI Agent 的付费调用网关。** Agent 不需要知道自己的 LLM 调用花了多少钱——Gradience 帮你管、帮你付、帮你审计。*
