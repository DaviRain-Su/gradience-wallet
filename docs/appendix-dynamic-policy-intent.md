# 附录：动态策略与交易意图设计草案

> Status: Draft — 预研材料，v1.5+ 版本规划
> Date: 2026-04-07
> Dependency: `appendix-policy-engine-draft.md`（静态策略引擎）

---

## 1. 设计目标

策略引擎不只是静态守门人，而是**智能治理中枢**：

- **静态策略**（v0.1-v1.0）：固定限额、白名单、时间窗口 → 守门人
- **动态策略 + 交易意图**（v1.5+）：根据上下文实时调整 + 理解 Agent 想干什么 → 智能治理

解决核心问题：
1. Agent 的历史信用应该影响其权限（高信用 → 更宽松）
2. 市场环境变化应该影响策略（高风险 → 收紧）
3. 用户需要知道 Agent "想做什么"，而不仅是 "发了什么 TX"

---

## 2. 动态策略（Dynamic Policies）

### 2.1 核心思路

策略不再是写死的 JSON 值，而是**可根据外部信号实时调整**的规则集。利用 OWS 的 `custom executable` 能力实现。

### 2.2 信号来源

| 信号来源 | 示例数据 | 如何影响策略 |
|---|---|---|
| **Gradience Reputation** | Agent 历史胜率、Judge 分数、任务完成率 | 高 Reputation → 自动放宽限额 20% |
| **市场风险（外部 API）** | Forta/Chainalysis 风险分、波动率、MEV 指数 | 高风险 → 收紧滑点 + 提高审批门槛 |
| **Agent 行为 Profiling** | 最近 7 天交易频率、平均金额 | 异常行为 → 临时降级为 warn 模式 |
| **团队/全局预算** | Workspace 剩余月度预算 | 接近上限 → 全 Agent 限额同步下调 |
| **实时链上数据** | Gas 价格、流动性深度 | 高 Gas → 延迟非紧急交易 |

### 2.3 JSON Schema 扩展

在原有 `rules` 数组新增规则类型 `dynamic_rule`：

```json
{
  "type": "dynamic_rule",
  "config": {
    "name": "reputation_boost",
    "sources": ["gradience_reputation", "market_volatility"],
    "adjustment": {
      "spend_limit_multiplier": 1.2,
      "action_override": "warn"
    },
    "fallback": { "action": "deny" },
    "refresh_interval": 300
  },
  "action": "allow"
}
```

### 2.4 评估流程升级

```
PolicyEngine.evaluate()
    ├── 1. 加载静态 rules
    ├── 2. 加载动态 rules
    │       └── 调用信号适配器
    │           ├── Gradience Reputation API
    │           ├── Forta API (风险信号)
    │           └── 链上数据 RPC
    ├── 3. 计算调整后规则 (multiplier, override)
    ├── 4. 合并后逐条评估
    └── 5. 记录动态决策到 policy_evaluations
            └── 附加 dynamic_factors JSON
```

### 2.5 数据库扩展

```sql
-- 信号缓存表
CREATE TABLE policy_signals (
  signal_type   TEXT NOT NULL,          -- 'reputation' | 'market_risk' | 'gas_price'
  subject_id    TEXT NOT NULL,          -- agent_wallet_id 或 chain_id
  value_json    TEXT NOT NULL,
  fetched_at    TEXT NOT NULL,
  expires_at    TEXT NOT NULL,
  PRIMARY KEY (signal_type, subject_id)
);

-- 策略评估日志扩展字段
-- policy_evaluations 表新增
ALTER TABLE policy_evaluations ADD COLUMN dynamic_factors TEXT;
-- dynamic_factors JSON 示例:
-- {"reputation_score": 85, "market_risk": "low", "gas_gwei": 25}
```

### 2.6 v1.5 最小实现

1. 先支持 **Reputation** 一个信号源
2. 仅实现 `spend_limit_multiplier` 一种调整类型
3. fallback = deny（信号获取失败则拒绝）
4. 刷新间隔固定 5 分钟

---

## 3. 交易意图（Transaction Intent）

### 3.1 核心思路

Agent 提交**结构化意图**（Intent）而非裸交易。策略引擎先解析意图是否符合用户/Agent 的交易策略模板，再做 Policy 检查。

### 3.2 Intent 数据结构

新增到 PolicyContext：

```ts
interface TransactionIntent {
  type: "swap" | "transfer" | "bridge" | "stake" | "custom_strategy";
  description?: string;           // 自然语言描述（可选）
  params: {
    fromToken: string;            // CAIP-19 资产标识符
    toToken?: string;
    amount: string;               // 最小单位字符串
    targetPrice?: string;         // 限价单
    strategyTag?: "dca" | "arbitrage" | "rebalance" | "yield";
    expectedSlippage?: number;    // 0-100 (bps)
    bridgeRoute?: string;         // 跨链路由标识
  };
  simulationResult?: {            // 预执行结果（OWS 已支持）
    expectedValue: string;
    riskScore: number;            // 0-100
    gasEstimate: string;
  };
}
```

### 3.3 解析 & 检查流程

```
Agent MCP 请求 signTransaction(tx, intent?)
    │
    ├── 有 intent → 意图检查
    │   ├── 1. 匹配交易策略模板
    │   │   └── 该 Agent 是否被允许做 swap/bridge?
    │   │   └── 参数范围是否在模板内?
    │   ├── 2. 风险评分
    │   │   ├── simulation 结果
    │   │   └── 动态信号 (Reputation, 市场风险)
    │   └── 3. intentRiskScore → 输入动态策略引擎
    │
    └── 无 intent → 降级处理
        └── 仅做裸 TX 策略检查（向后兼容，但记录降级标记）
```

### 3.4 交易策略模板

用户在 Dashboard 预设允许的 Intent 类型和参数范围：

```json
{
  "id": "template-dca-eth-usdc",
  "name": "ETH-USDC DCA",
  "allowedIntents": [{
    "type": "swap",
    "params": {
      "fromToken": "eip155:1/erc20:0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
      "toToken": "eip155:1/erc20:0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
      "maxAmount": "1000000000",       // 1000 USDC (6 decimals)
      "maxSlippage": 50,                // 0.5%
      "strategyTag": "dca"
    }
  }]
}
```

### 3.5 意图匹配引擎

```ts
interface IntentMatchResult {
  matched: boolean;
  templateId?: string;
  reason?: string;
  riskScore: number;  // 0-100
}

class IntentMatcher {
  match(intent: TransactionIntent, templates: StrategyTemplate[]): IntentMatchResult {
    for (const template of templates) {
      for (const allowed of template.allowedIntents) {
        if (this.isIntentCompatible(intent, allowed)) {
          return {
            matched: true,
            templateId: template.id,
            riskScore: this.calculateRiskScore(intent, allowed)
          };
        }
      }
    }
    return {
      matched: false,
      reason: `Intent type "${intent.type}" does not match any allowed strategy template`,
      riskScore: 100
    };
  }

  private isIntentCompatible(intent: TransactionIntent, allowed: AllowedIntent): boolean {
    if (intent.type !== allowed.type) return false;
    if (intent.params.fromToken !== allowed.params.fromToken) return false;
    if (intent.params.toToken !== allowed.params.toToken) return false;
    if (parseFloat(intent.params.amount) > parseFloat(allowed.params.maxAmount)) return false;
    if (intent.params.expectedSlippage && intent.params.expectedSlippage > allowed.params.maxSlippage) return false;
    if (allowed.params.strategyTag && intent.params.strategyTag !== allowed.params.strategyTag) return false;
    return true;
  }

  private calculateRiskScore(intent: TransactionIntent, allowed: AllowedIntent): number {
    let score = 0;
    // 占模板额度的比例
    const usageRatio = parseFloat(intent.params.amount) / parseFloat(allowed.params.maxAmount);
    score += usageRatio * 40;  // 最多 40 分
    // 滑点
    if (intent.params.expectedSlippage) {
      const slippageRatio = intent.params.expectedSlippage / allowed.params.maxSlippage;
      score += slippageRatio * 30;  // 最多 30 分
    }
    // 策略标签风险权重
    const tagRisk: Record<string, number> = {
      dca: 10,
      rebalance: 20,
      arbitrage: 30,
      yield: 25
    };
    score += (tagRisk[intent.params.strategyTag!] || 15);
    return Math.min(score, 100);
  }
}
```

### 3.6 审计日志扩展

```sql
-- policy_evaluations 表新增字段
ALTER TABLE policy_evaluations ADD COLUMN intent_json TEXT;
ALTER TABLE policy_evaluations ADD COLUMN intent_matched BOOLEAN;
ALTER TABLE policy_evaluations ADD COLUMN intent_template_id TEXT;
ALTER TABLE policy_evaluations ADD COLUMN intent_risk_score INTEGER;
```

### 3.7 Dashboard 新功能

- **交易策略模板页面**：可视化定义允许的 intent 类型 + 参数范围
- **意图审计视图**：每次交易的 intent、匹配结果、风险评分
- **意图不符告警**：Agent 发送未授权意图时通知用户

### 3.8 v1.5 最小实现

1. 支持 `swap` 和 `transfer` 两种 intent 类型
2. 仅支持链上资产白名单（fromToken / toToken）
3. 最大金额 + 最大滑点限制
4. 不匹配时直接 deny（无 warn 中间态）
5. 向后兼容：无 intent 的请求仍可通过静态策略检查

---

## 4. 与上层产品联动

### 4.1 Reputation 闭环

```
Agent 在 AgentM 执行任务
    ↓
Judge 评分 → 更新 Gradience Reputation
    ↓
Wallet 策略引擎下次评估时
    ↓
高 Reputation → spend_limit_multiplier > 1.0
低 Reputation → spend_limit_multiplier < 1.0
    ↓
Agent 下次任务权限自动调整
```

### 4.2 产品差异化

| 竞品 | 策略能力 | 缺失 |
|---|---|---|
| Fireblocks | 静态规则 + 审批流 | 无 Intent、无动态调整 |
| OKX Agentic Wallet | TEE + 风险检测 | 无用户自定义策略 |
| Coinbase Smart Wallet | Passkey + 基本限额 | 无多 Agent 编排 |
| **Gradience** | **静态 + 动态 + Intent + Reputation** | **四者合一** |

---

## 5. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|---|---|---|
| 外部信号 API 不可用 | 动态策略无法评估 | fallback = deny + 缓存上次有效值（带 TTL） |
| Intent 伪造 | Agent 发送虚假 intent | 引擎层用 simulation 验证 intent 与实际 TX 是否一致 |
| 策略复杂度爆炸 | 用户不会配置 | 提供预设模板（保守/标准/激进）+ AI 辅助配置 |
| 性能退化 | 每次评估需调多个 API | 信号缓存 + 并行拉取 + 异步评估 |

---

*本文为预研草案，待 Phase 3 Technical Spec 阶段正式采用。与 `appendix-policy-engine-draft.md` 配合使用。*
