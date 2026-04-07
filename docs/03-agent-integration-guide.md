# Agent 集成指南：让 AI Agent 使用 Gradience Wallet

> 本文档描述用户如何将自己的 AI Agent（Claude Code、Codex、OpenClaw 等）连接到 Gradience Wallet，实现安全的链上操作。

---

## 核心架构回顾

```
用户 (Owner)                    AI Agent
    │                              │
    │ 1. 创建 Master Wallet        │
    │    (Passkey 保护)             │
    │                              │
    │ 2. 创建 API Key + Policy     │────── 3. 分发 API Token ──→ │
    │    (限额、链限制、过期)       │                              │
    │                              │                              │
    │ 4. 保持 Owner 权限           │         5. 使用 Token 请求签名 │
    │    (可以绕过 Policy)          │    ◄────── sign/pay/swap ────── │
    │                              │         (Policy 先评估)         │
    │                              │                              │
    │ 6. 审计所有操作               │    ◄────── 审计日志 ────────── │
```

---

## OWS 支持的链（10 条链族）

OWS v1.2.4 原生支持以下链族，**单一条 mnemonic 可派生所有链的地址**：

| 链族 | Curve | Coin Type | 代表链 | CAIP-2 命名空间 |
|---|---|---|---|---|
| **EVM** | secp256k1 | 60 | Ethereum, Base, Arbitrum, OP, BSC, Polygon, Avalanche… | `eip155` |
| Solana | ed25519 | 501 | Solana Mainnet | `solana` |
| Bitcoin | secp256k1 | 0 | Bitcoin (Bech32 native segwit) | `bip122` |
| Cosmos | secp256k1 | 118 | Cosmos Hub | `cosmos` |
| Tron | secp256k1 | 195 | Tron Mainnet | `tron` |
| TON | ed25519 | 607 | TON Mainnet | `ton` |
| Sui | ed25519 | 784 | Sui Mainnet | `sui` |
| XRPL | secp256k1 | 144 | XRP Ledger | `xrpl` |
| Spark | secp256k1 | 8797555 | Spark | `spark` |
| Filecoin | secp256k1 | 461 | Filecoin | `fil` |

**重要**: 增加新链只需 5 步（定义 CAIP-2 ID、派生路径、地址编码、签名行为、序列化规则），无需修改 OWS 核心。

---

## 用户完整操作路径（4 步）

### Step 1: 安装 + 创建钱包

```bash
# 安装 Gradience + OWS
curl -fsSL https://docs.openwallet.sh/install.sh | bash

# 创建 Master Wallet（Passkey 保护）
gradience auth login

# 创建钱包
ows wallet create --name agent-treasury --show-mnemonic
```

输出示例：
```
Created wallet 3198bc9c-6672-5ab3-d995-1234567890ab
  eip155:1                              0xab16...   m/44'/60'/0'/0/0
  solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp  7Kz9...    m/44'/501'/0'/0'
  sui:mainnet                              0x...      m/44'/784'/0'/0'/0'
  bip122:000000000019d6689c085ae165831e93   bc1q...    m/84'/0'/0'/0/0
```

一条 mnemonic，自动派生 10 条链的地址。

### Step 2: 定义策略

```bash
# 策略 1: 只允许在 Base 和 BSC 上操作
cat > base-bsc-only.json << 'EOF'
{
  "id": "base-bsc-only",
  "name": "Base + BSC 访问限制",
  "version": 1,
  "created_at": "2026-04-07T00:00:00Z",
  "rules": [
    { "type": "allowed_chains", "chain_ids": ["eip155:8453", "eip155:56"] }
  ],
  "action": "deny"
}
EOF
ows policy create --file base-bsc-only.json

# 策略 2: 自定义 executable（限额控制 + 意图分析 → Gradience Policy Engine）
cat > spending-limits.json << 'EOF'
{
  "id": "spending-limits",
  "name": "单笔不超过 100 USDC，每日累计不超过 1000 USDC",
  "version": 1,
  "created_at": "2026-04-07T00:00:00Z",
  "executable": "/path/to/gradience-core/target/release/gradience-policy-exec",
  "config": {
    "max_per_tx_usdc": 100,
    "max_daily_usdc": 1000,
    "allowed_contracts": ["0xPancakeSwap", "0xUniswap"]
  },
  "action": "deny"
}
EOF
ows policy create --file spending-limits.json
```

### Step 3: 为每个 Agent 创建 API Key

```bash
# 给 Claude Code 创建 API Key
ows key create --name claude-code \
  --wallet agent-treasury \
  --policy base-bsc-only \
  --policy spending-limits

# 输出: ows_key_a1b2c3d4e5f6789012345678901234567890abcdef...
# ⚠️ TOKEN 只显示一次，必须保存

# 给 Codex 创建另一个 API Key（不同策略）
ows key create --name codex-trader \
  --wallet agent-treasury \
  --policy base-bsc-only

# 给 OpenClaw 创建
ows key create --name openclaw \
  --wallet agent-treasury \
  --policy base-bsc-only \
  --policy spending-limits
```

**安全模型**: 每个 Agent 一个 Key，策略独立。撤销某个 Key 不影响其他 Agent 和用户本身。

### Step 4: 分发 API Token 给 Agent

---

## Agent 集成方式（3 种）

### 方式 1: Gradience MCP Server（推荐，体验最佳）

**架构**:
```
Claude Code / Codex / OpenClaw
         │
         │ MCP Protocol (stdio or HTTP)
         ▼
gradience-mcp (本地或云端)
    │  → 策略评估 (Gradience Policy Engine)
    │  → OWS 签名 (本地，私钥不离开)
    │  → 链上广播
    ▼
区块链 (Base / BSC / 任意链)
```

**用户配置** (以 Claude Code 为例):

```bash
# 启动 Gradience MCP Server (默认 localhost)
gradience mcp start --port 3000
```

Claude Code 的 `.claude/settings.json`:
```json
{
  "mcpServers": {
    "gradience": {
      "command": "gradience",
      "args": ["mcp", "stdio"],
      "env": {
        "GRADIENCE_API_TOKEN": "ows_key_a1b2c3d4e5f6..."
      }
    }
  }
}
```

**Agent 使用**:
```
用户: "帮我用 Claude 在 Base 上把 50 USDC 换为 ETH"
Claude Code: MCP 调用 gradience.swap({
  wallet: "agent-treasury",
  from: "eip155:8453:native:0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
  to: "eip155:8453:native",
  amount: "50",
  dex: "pancakeswap"
})
→ Gradience 策略引擎: allowed_chains ✓, spending limit ✓ (50 < 100)
→ Gradience OWS adapter: 签名 + 广播
→ 交易完成
```

### 方式 2: OWS CLI 直接调用

适用于有 shell 访问权限的 Agent（Claude Code, OpenClaw, Cursor 等）。

**用户配置**:
```bash
# 设置环境变量
export OWS_PASSPHRASE="ows_key_a1b2c3d4e5f6789012345678901234567890abcdef..."
```

**Agent 使用**:
```bash
# Agent 直接调用 OWS 签名
OWS_PASSPHRASE="ows_key_..." \
  ows sign tx --wallet agent-treasury --chain base --tx 0x02f8...

# x402 自动支付
OWS_PASSPHRASE="ows_key_..." \
  ows pay request "https://api.example.com/data" --wallet agent-treasury

# 余额查询
OWS_PASSPHRASE="ows_key_..." \
  ows fund balance --wallet agent-treasury --chain base
```

### 方式 3: Node.js / Python SDK

适用于自定义 Agent 框架。

**Node.js**:
```javascript
import { OWS } from "@open-wallet/ows";
const ows = new OWS({ passphrase: process.env.OWS_PASSPHRASE });

const wallet = await ows.getWallet("agent-treasury");
const signature = await ows.signTx({
  wallet: "agent-treasury",
  chain: "eip155:8453",
  transaction: rawTx,
});
```

**Python**:
```python
from ows import OWS

ows = OWS(passphrase=os.environ["OWS_PASSPHRASE"])
wallet = ows.get_wallet("agent-treasury")
signature = ows.sign_tx(wallet="agent-treasury", chain="eip155:8453", transaction=raw_tx)
```

---

## 各 Agent 平台集成示例

### Claude Code

**集成方式**: MCP Server + CLI
**配置**: `.claude/settings.json` (见上方方式 1)
**用户体验**: Claude Code 直接在对话中执行链上操作，用户可以看到完整的策略评估和交易确认提示。

### Codex (OpenAI Code Execution Agent)

**集成方式**: MCP Server (HTTP) 或环境变量 + CLI
**配置**: 环境变量 `GRADIENCE_API_TOKEN` 或 `OWS_PASSPHRASE`
**用户体验**: Codex 在 sandbox 中安全调用 Gradience API，所有请求经过策略引擎审核。

### OpenClaw

**集成方式**: MCP Server (stdio)
**配置**: OpenClaw 的 `claw.json` 中添加 MCP server 配置
**用户体验**: OpenClaw 的 natural language 接口直接映射到 Gradience MCP tools。

### Cursor

**集成方式**: CLI 或 Node.js SDK
**配置**: 项目环境配置 `.env` 文件 (仅本地开发)
**用户体验**: Cursor Agent 在开发过程中可以直接调用测试链操作。

---

## 安全模型总结

| 角色 | 认证方式 | Policy 评估 | 适合场景 |
|---|---|---|---|
| **用户 (Owner)** | Passphrase | **不评估** (完全控制) | 紧急操作、大额交易 |
| **Agent (API Token)** | `ows_key_...` | **全部评估** (AND 逻辑) | 日常操作、DeFi、支付 |

**核心原则**:
1. **私钥不离开用户设备** — OWS 的 `~/.ows/wallets/*.json` 是加密存储
2. **Agent 只有 Token** — 无法直接访问私钥，只能请求签名
3. **每个请求都过 Policy** — 限额、链限制、合约白名单、意图分析
4. **一键撤销** — `ows key revoke` 立即失效，不影响用户和其他 Agent
5. **完整审计** — 所有签名操作记录到 `~/.ows/logs/audit.jsonl`，可锚定到链上

---

## 用户完整生命周期

```
用户
  │
  ├── 1. 安装 Gradience + OWS
  │
  ├── 2. 注册/Passkey 登录
  │
  ├── 3. 创建 Master Wallet (自动生成 10 链地址)
  │
  ├── 4. 存入资金 (MoonPay / 链上转账)
  │
  ├── 5. 配置策略 (链限制 / 限额 / 合约白名单 / 意图分析)
  │
  ├── 6. 为 Agent 创建 API Key
  │   ├── Claude Code: `ows key create --name claude-code --policy ...`
  │   ├── Codex: `ows key create --name codex-trader --policy ...`
  │   └── OpenClaw: `ows key create --name openclaw --policy ...`
  │
  ├── 7. 分发 Token 给 Agent (环境变量 / MCP 配置)
  │
  ├── 8. Agent 开始工作
  │   ├── Agent 调用 sign/pay/swap
  │   ├── Policy Engine 评估 (通过/拒绝)
  │   └── 审计日志记录 + 可选链上锚定
  │
  └── 9. 用户 Dashboard 查看
      ├── 所有 Agent 操作记录
      ├── 余额/限额状态
      ├── 审计日志完整性验证 (Merkle proof)
      └── 随时撤销/调整策略
```

---

## Gradience Wallet 的定位

> **Gradience Wallet = Passkey 主控钱包 + 策略引擎 + Agent 编排层**
>
> - OWS 是**底层标准** (存储/签名/Policy 接口)
> - Gradience 是**上层产品** (智能策略 + 多 Agent 管理 + 审计 + DEX 聚合 + 支付抽象 + MCP)
>
> **用户通过 Gradience 管理所有 Agent Wallet，每个 Agent Wallet 是一个 OWS API Key + 独特 Policy。**

---

*本文档配套参考: `docs/01-prd.md` (产品需求), `docs/02-architecture.md` (系统架构)*
