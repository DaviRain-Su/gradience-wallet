# 附录：策略引擎（Policy Engine）设计草案

> Status: Draft — 预研材料，待 Phase 3 Technical Spec 正式采用
> Date: 2026-04-07
> Source: 会话推导 + OWS v1.2.4 规范对齐

---

## 1. 设计原则

- **充分利用 OWS**：OWS 已原生支持 spending limits、contract allowlists、chain restrictions、time-bound rules、warn/deny 动作。Gradience 只做上层声明式策略管理 + 统一策略模板 + 审计 + 团队同步。
- **声明式 + 可视化**：用户在 Web Dashboard / CLI 看到的是简单 JSON / UI 表单，后台自动翻译成 OWS Policy（executable + config）。
- **性能优先**：所有策略在签名之前（pre-signing）一次性评估，< 10ms。
- **审计闭环**：每一次评估结果必须落库（PolicyResult + reason）。
- **可扩展**：支持自定义规则（未来可插拔风险 API、Agent 行为 profiling）。

---

## 2. Policy 数据模型（JSON Schema）

```json
{
  "$schema": "https://gradience.dev/policy.schema.json",
  "type": "object",
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "name": { "type": "string", "maxLength": 64 },
    "description": { "type": "string" },
    "agentWalletId": { "type": "string" },
    "apiKeyIds": {
      "type": "array",
      "items": { "type": "string" }
    },
    "workspaceId": {
      "type": "string",
      "description": "所属工作空间（团队策略）"
    },
    "rules": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "type": {
            "enum": [
              "spend_limit",
              "daily_limit",
              "monthly_limit",
              "chain_whitelist",
              "contract_whitelist",
              "operation_type",
              "time_window",
              "custom"
            ]
          },
          "config": { "type": "object" },
          "action": { "enum": ["allow", "warn", "deny"] }
        },
        "required": ["type", "config", "action"]
      }
    },
    "priority": {
      "type": "integer",
      "description": "优先级，越小越高 (workspace=0, agent=1)"
    },
    "status": { "enum": ["active", "paused", "deleted"] },
    "version": { "type": "integer" },
    "createdAt": { "type": "string", "format": "date-time" }
  }
}
```

### 规则类型详细 config 示例

| 规则类型 | config 示例 | 说明 |
|---|---|---|
| spend_limit | `{ "maxAmount": "100.0", "token": "USDC", "chainId": "eip155:1", "decimals": 6 }` | 单笔上限 |
| daily_limit | `{ "maxAmount": "1000.0", "token": "USDT", "decimals": 6, "resetHour": 0 }` | 日限额（UTC 重置） |
| monthly_limit | `{ "maxAmount": "5000.0", "token": "ETH", "decimals": 18 }` | 月限额 |
| chain_whitelist | `{ "allowed": ["eip155:1", "solana:5eykt..."] }` | 链白名单（CAIP-2） |
| contract_whitelist | `{ "allowed": ["0x...Uniswap", "JUP...Jupiter"] }` | 合约/程序白名单 |
| operation_type | `{ "allowed": ["transfer", "contractCall"], "blocked": ["signMessage"] }` | 操作类型 |
| time_window | `{ "start": "09:00", "end": "18:00", "tz": "Asia/Tokyo", "days": ["1","2","3","4","5"] }` | 时间窗口 |
| custom | `{ "executable": "./policies/risk-score.js", "params": {...} }` | 自定义（未来集成风险 API） |

---

## 3. 存储层（Database）

### 3.1 策略表

```sql
-- PostgreSQL / SQLite
CREATE TABLE policies (
  id          TEXT PRIMARY KEY,           -- UUID
  name        TEXT NOT NULL,
  description TEXT,
  agent_wallet_id TEXT NOT NULL,
  workspace_id    TEXT,
  rules_json      TEXT NOT NULL,          -- JSON 序列化
  priority        INTEGER DEFAULT 1,
  status          TEXT NOT NULL DEFAULT 'active',
  version         INTEGER DEFAULT 1,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);

CREATE INDEX idx_policies_agent_wallet ON policies(agent_wallet_id);
CREATE INDEX idx_policies_workspace  ON policies(workspace_id);
CREATE INDEX idx_policies_status       ON policies(status);
```

### 3.2 策略评估日志表

```sql
CREATE TABLE policy_evaluations (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  policy_id       TEXT NOT NULL,
  api_key_id      TEXT NOT NULL,
  transaction_hash TEXT,
  context_json    TEXT NOT NULL,          -- 交易上下文快照
  result_allow    BOOLEAN NOT NULL,
  result_reason   TEXT,
  evaluated_at    TEXT NOT NULL
);
```

### 3.3 额度追踪表（daily_limit / monthly_limit 需要状态）

```sql
-- 追踪已花费金额，支持日/月限额重置
CREATE TABLE spending_trackers (
  wallet_id   TEXT NOT NULL,
  rule_type   TEXT NOT NULL,              -- 'daily' | 'monthly'
  token       TEXT NOT NULL,
  chain_id    TEXT NOT NULL,              -- CAIP-2
  period      TEXT NOT NULL,              -- '2026-04-07' 或 '2026-04'
  spent_amount TEXT NOT NULL DEFAULT '0', -- 以最小单位（wei/satoshi）存为字符串
  reset_at    TEXT NOT NULL,
  PRIMARY KEY (wallet_id, rule_type, token, chain_id, period)
);
```

### 3.4 warn 待审批表

```sql
-- warn 策略的 pending 审批队列
CREATE TABLE policy_approvals (
  id            TEXT PRIMARY KEY,         -- UUID
  policy_id     TEXT NOT NULL,
  agent_wallet_id TEXT NOT NULL,
  request_json  TEXT NOT NULL,            -- 原始请求
  status        TEXT NOT NULL DEFAULT 'pending',  -- pending | approved | rejected | expired
  approved_by   TEXT,                     -- 审批人（用户 ID）
  approved_at   TEXT,
  expires_at    TEXT NOT NULL             -- 超时自动拒绝
);
```

---

## 4. 核心评估流程

### 4.1 序列图

```
┌────────┐     ┌────────────┐     ┌──────────────┐     ┌───────────┐     ┌───────┐
│ Agent  │────>│ MCP Server │────>│Policy Engine │────>│OWS Vault  │────>│ Chain │
└────────┘     └────────────┘     └──────────────┘     └───────────┘     └───────┘
                                       │
                    1. 加载该 API Key 关联的所有 rules
                    2. 逐条执行规则评估
                    3. 检查 daily/monthly 限额状态
                    4. 返回 PolicyResult[]
                                       │
                    ├── 全部 allow     ──────────────> 继续签名 → 广播
                    │
                    ├── 有 deny        ──< 403 + reasons (中断)
                    │
                    └── 有 warn        ──> 创建 approval 记录
                                          通知用户
                                          等待 approve/reject
                                          ↓
                                          approved ──> 继续
                                          rejected   ──> 403
                                          expired    ──> 403
```

### 4.2 评估逻辑伪代码

```ts
import { PolicyContext, PolicyResult } from '@openwallet-standard/core';

interface EvaluationResult {
  decision: 'allow' | 'deny' | 'warn';
  reasons: string[];
  approvalId?: string;  // warn 时返回
}

export class GradiencePolicyEngine {
  private db: Database;
  private notificationService: NotificationService;

  async evaluate(ctx: PolicyContext): Promise<EvaluationResult> {
    // 1. 加载策略（workspace + agent，按优先级排序）
    const policies = await this.loadPolicies(ctx.apiKeyId);
    const mergedRules = mergePoliciesStrictest(policies); // 合并取最严

    // 2. 逐条评估
    const results: { rule: PolicyRule; result: PolicyResult }[] = [];

    for (const rule of mergedRules) {
      const result = await this.evaluateRule(ctx, rule);
      results.push({ rule, result });

      // deny 短路
      if (!result.allow && rule.action === 'deny') {
        return {
          decision: 'deny',
          reasons: results.filter(r => !r.result.allow).map(r => r.result.reason || '')
        };
      }
    }

    // 3. 如果有 warn
    const warnResults = results.filter(
      r => !r.result.allow && r.rule.action === 'warn'
    );

    if (warnResults.length > 0) {
      const approvalId = await this.createApprovalRequest(ctx, warnResults);
      return { decision: 'warn', reasons: warnResults.map(r => r.result.reason || ''), approvalId };
    }

    return { decision: 'allow', reasons: [] };
  }

  private async evaluateRule(ctx: PolicyContext, rule: PolicyRule): Promise<{ allow: boolean; reason?: string }> {
    switch (rule.type) {
      case 'spend_limit':
        return evaluateSpendLimit(ctx.transaction, rule.config);
      case 'daily_limit':
        return evaluateDailyLimit(ctx, rule.config);
      case 'monthly_limit':
        return evaluateMonthlyLimit(ctx, rule.config);
      case 'chain_whitelist':
        return evaluateChainWhitelist(ctx.chainId, rule.config);
      case 'contract_whitelist':
        return evaluateContractWhitelist(ctx.transaction, rule.config);
      case 'operation_type':
        return evaluateOperationType(ctx.transaction, rule.config);
      case 'time_window':
        return evaluateTimeWindow(ctx.timestamp, rule.config);
      default:
        return { allow: true };
    }
  }
}
```

### 4.3 额度限制评估（带状态）

```ts
async function evaluateDailyLimit(
  ctx: PolicyContext,
  config: DailyLimitConfig
): Promise<{ allow: boolean; reason?: string }> {
  const amount = parseAmount(ctx.transaction.value, config.decimals);
  const period = getCurrentPeriod('daily', config.resetHour);

  // 查询当前周期已花费
  const tracker = await db.getSpendingTracker(ctx.walletId, 'daily', config.token, ctx.chainId, period);
  const spent = tracker ? parseAmount(tracker.spentAmount, config.decimals) : 0;

  if (spent + amount > parseFloat(config.maxAmount)) {
    return { allow: false, reason: `超过日限额 ${config.maxAmount} ${config.token} (已用 ${spent})` };
  }

  return { allow: true };
}

// 评估通过后，更新追踪器
async function updateSpendingTracker(
  walletId: string,
  ruleType: 'daily' | 'monthly',
  token: string,
  chainId: string,
  amount: string,
  period: string,
  resetAt: string
): Promise<void> {
  await db.upsert('spending_trackers', {
    wallet_id: walletId,
    rule_type: ruleType,
    token,
    chain_id: chainId,
    period,
    spent_amount: amount,  // 累加
    reset_at: resetAt
  });
}
```

### 4.4 warn 审批状态机

```
┌─────────┐  approve   ┌──────────┐  execute
│ pending  │ ────────> │ approved │ ────────> 继续签名
└────┬─────┘           └──────────┘
     │
     │ reject / expire (30min)
     ↓
┌──────────┐
│ denied   │  ────────> 403 + reason
└──────────┘
```

```ts
async function handleWarnApproval(
  approvalId: string,
  action: 'approve' | 'reject',
  userId: string
): Promise<void> {
  const approval = await db.getApproval(approvalId);

  if (!approval || approval.status !== 'pending') {
    throw new Error('Approval not found or already processed');
  }

  if (new Date(approval.expires_at) < new Date()) {
    approval.status = 'expired';
    await db.updateApproval(approval);
    throw new Error('Approval expired');
  }

  approval.status = action === 'approve' ? 'approved' : 'rejected';
  approval.approved_by = userId;
  approval.approved_at = new Date().toISOString();

  await db.updateApproval(approval);

  // 通知 Agent (通过 WebSocket / callback)
  if (action === 'approved') {
    await notificationService.notifyAgent(approval.agent_wallet_id, 'approved', approvalId);
  }
}
```

---

## 5. 与 OWS 的集成方式

### 5.1 集成架构

```
┌───────────────────────────────────────────────────┐
│  Gradience Policy Engine (应用层)                   │
│  - 声明式 JSON → 规则评估                          │
│  - 状态管理 (spending trackers)                     │
│  - warn 审批队列                                    │
│  - 审计日志                                         │
└─────────────────────┬─────────────────────────────┘
                      │ 调用
┌─────────────────────▼─────────────────────────────┐
│  OWS Vault (执行层)                                │
│  - 本地加密存储                                     │
│  - 原生 Policy 检查 (secondary)                    │
│  - 签名                                             │
└─────────────────────┬─────────────────────────────┘
                      │ 广播
                      ▼
               链上 RPC
```

### 5.2 集成策略

1. **Gradience 做主检查**：所有 MCP Server 请求先经 Gradience Policy Engine 评估，通过才交给 OWS 签名
2. **OWS 做兜底检查**：OWS 内部也注册基础 Policy（deny-all 默认），防止绕过 Gradience 的直接调用
3. **注册方式**：Gradience 提供 policy executable 脚本（`policies/*.js`），安装时注册到 OWS Vault

### 5.3 OWS 映射

每个 Gradience Policy → 1 N 个 OWS Policy：

```
Gradience Policy:
  id: "policy-uuid"
  rules: [spend_limit, chain_whitelist]
  →

OWS Policy 1:
  executable: "policies/spend-limit.js"
  config: { maxAmount: "100", token: "USDC" }
  action: "deny"

OWS Policy 2:
  executable: "policies/chain-whitelist.js"
  config: { allowed: ["eip155:1", "solana:..."] }
  action: "deny"
```

---

## 6. 策略优先级与合并

### 6.1 优先级

```
Workspace Policy (priority=0, 最严)
    ↓
Agent Policy (priority=1, 合并取最严)
    ↓
OWS Default Policy (兜底deny)
```

### 6.2 合并规则

- **同类型规则合并取最严**：
  - spend_limit: 取最小值
  - daily_limit: 取最小值
  - chain_whitelist: 取交集
  - contract_whitelist: 取交集
- **deny 优先于 allow**
- **warn 不阻塞 allow，但触发审批流**

```ts
function mergePoliciesStrictest(policies: Policy[]): PolicyRule[] {
  const merged = new Map<string, PolicyRule>();

  for (const policy of policies) {
    for (const rule of policy.rules) {
      const existing = merged.get(rule.type);
      if (!existing) {
        merged.set(rule.type, rule);
        continue;
      }
      // 取更严格的
      merged.set(rule.type, mergeRulesStrictest(existing, rule));
    }
  }
  return Array.from(merged.values());
}
```

---

## 7. 代币精度处理

不同链/token 精度不同，统一**存储为最小单位的字符串**，评估时按需转换：

| Token | 精度 | 最小单位 | 示例 (1.5 token) |
|---|---|---|---|
| USDC (EVM) | 6 | 1/10^6 | "1500000" |
| ETH | 18 | wei | "1500000000000000000" |
| BTC | 8 | satoshi | "150000000" |
| SOL | 9 | lamport | "1500000000" |

```ts
function parseAmount(rawValue: string, decimals: number): number {
  return parseFloat(rawValue) / Math.pow(10, decimals);
}

function formatAmount(amount: number, decimals: number): string {
  return (amount * Math.pow(10, decimals)).toFixed(0);
}
```

---

## 8. 通知机制

warn 策略触发时，通知用户：

| 渠道 | 实现 | 延迟 |
|---|---|---|
| **Dashboard In-App** | WebSocket 推送到前端 | < 1s |
| **邮件** | SMTP (Resend/SendGrid) | ~30s |
| **Telegram Bot** | Telegram Bot API | ~5s |
| **Webhook** (自定义) | POST 到用户指定 URL | ~1s |

---

## 9. v0.1 Alpha 最小实现范围（2-3 周）

1. 支持 `spend_limit` + `chain_whitelist` + `operation_type` 三种规则
2. 实现 Policy JSON Schema + 验证
3. 集成 OWS policy evaluation
4. 记录 evaluation log 到 SQLite
5. CLI `policy set / list / test` 命令
6. 单元测试覆盖 95% 规则逻辑

**明确排除**（后续版本）：
- daily_limit / monthly_limit（需要状态管理，v0.2 加）
- time_window（v0.2 加）
- warn 审批流（v0.2 加）
- 团队策略（v1.5 加）
- 自定义规则（v1.0 加）

---

## 10. 风险 & 后续扩展

| 风险 | 缓解措施 |
|---|---|
| OWS 标准小版本升级 → policy executable API 变动 | 用 `owsAdapter.ts` 抽象层隔离 |
| high-frequency 评估性能 | 规则缓存 + 并行评估 + < 10ms 基准测试 |
| 时区处理错误 | UTC 统一存储，前端用户时区展示 |

### 后续扩展路线图

- **v1.0**：自定义规则（集成 Chainalysis / Forta 风险 API）
- **v1.5**：Agent 行为 profiling、跨 Agent 预算共享
- **v2.0**：规则版本控制 + 审批流（Owner 手动 review warn 交易）

---

*本文为预研草案，待 Phase 3 Technical Spec 阶段正式采用并精确到每个字段、接口、状态转换。*
