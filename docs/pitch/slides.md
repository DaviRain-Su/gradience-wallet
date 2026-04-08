---
theme: default
background: https://images.unsplash.com/photo-1451187580459-43490279c0fa?q=80&w=2072&auto=format&fit=crop
class: text-center
highlighter: shiki
---

# Gradience Wallet

## Agent Wallet Orchestration Platform

<p class="text-xl opacity-80 mt-4">
Passkey Identity · OWS Multi-Chain Vault · Policy-Gated Agent Access via MCP
</p>

<div class="mt-12 text-sm opacity-60">
  Open Source · Local-First · MCP-Native
</div>

<style>
h1 {
  background-color: #2B51B1;
  background-image: linear-gradient(45deg, #4F80F0 10%, #8B5CF6 100%);
  background-size: 100%;
  -webkit-background-clip: text;
  -moz-background-clip: text;
  -webkit-text-fill-color: transparent;
  -moz-text-fill-color: transparent;
}
</style>

---
layout: default
---

# 一句话定义

<div class="text-2xl font-semibold text-center py-8 px-4 border-l-4 border-blue-500 bg-gray-50 rounded">
Gradience 是一个面向个人与团队的 <span class="text-blue-600">Agent 钱包编排平台</span> —— 用 Passkey 接管身份、用 OWS 标准托管本地多链资产、用可编程策略引擎精确控制 AI Agent 的每一笔交易与支付。
</div>

<div class="grid grid-cols-3 gap-4 mt-8">
  <div class="p-4 rounded bg-gray-50">
    <div class="font-bold text-lg">🗝️ Passkey</div>
    <div class="text-sm opacity-80">无助助词现代身份认证 + 设备恢复</div>
  </div>
  <div class="p-4 rounded bg-gray-50">
    <div class="font-bold text-lg">🔐 OWS</div>
    <div class="text-sm opacity-80">本地优先多链钱包（BIP-39 + HD 派生）</div>
  </div>
  <div class="p-4 rounded bg-gray-50">
    <div class="font-bold text-lg">🛡️ Policy</div>
    <div class="text-sm opacity-80">签名前自动评估：限额 / 合约 / 时间 / 风险信号</div>
  </div>
</div>

---
layout: default
---

# 核心机制：Agent 怎么用钱包？

```mermaid {scale: 0.8}
graph LR
  A[用户] -->|Passkey 认证| B[OWS Vault]
  B --> C[Wallet A]
  B --> D[Wallet B]
  C --> E[API Key A + Policy]
  D --> F[API Key B + Policy]
  E --> G[Agent 请求 Swap]
  F --> H[Agent 请求转账]
  G --> I{Policy Engine}
  H --> I
  I -->|Allow| J[自动签名执行]
  I -->|Deny| K[拒绝并返回原因]
```

<div class="mt-4 text-sm">
<strong>关键：</strong>Agent 不持有私钥。每一笔操作都必须先通过 Policy Engine 的 pre-signing 评估，才能触发 OWS 本地签名。
</div>

---
layout: default
---

# 竞争壁垒：Gradience vs 现有方案

| 维度 | Tempo / 托管钱包 | <span class="text-blue-600 font-bold">Gradience Wallet</span> |
|:---|:---|:---|
| **钱包标准** | 私有单生态 | **OWS 开放标准**（BIP-39，本地 vault，多链 HD） |
| **Agent 权限** | 基础 spending limit | **多层 Policy Engine**：限额 + 合约/操作/时间/模型白名单 + 意图风险 + 动态信号 |
| **交互协议** | 私有协议 | **MCP (Model Context Protocol)** — Claude / Cursor 等任意 Host 标准接入 |
| **支付协议** | 传统审批 | **x402 链上支付**：OWS 签名 + ERC-20 settlement on Base/Ethereum |
| **团队预算** | 无 | **Shared Budget**：Workspace 级别跨钱包预算与实时对账 |
| **审计溯源** | 基础日志 | **HMAC-chained audit + Merkle tree on-chain anchoring** |
| **部署形态** | 托管 SaaS | **Local-first 单二进制** + 自托管云 + Telegram Mini App |

<div class="mt-4 p-4 border-l-4 border-purple-500 bg-purple-50 rounded">
<strong>核心差异：</strong>别人是“给 Agent 一个钱包”；Gradience 是“让用户真正拥有自己的钱包，并精确编排 Agent 能做什么”。
</div>

---
layout: default
---

# 已落地的产品矩阵

<div class="grid grid-cols-3 gap-4">

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">🌐 Web App</div>
<div class="text-sm">Landing Page + Passkey Login + Dashboard + Swap/Fund/Policy</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">🔌 MCP Server</div>
<div class="text-sm">10 个标准 tool：sign_tx / sign_msg / swap / pay / llm_generate / ai_balance / verify_api_key 等</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">💻 CLI</div>
<div class="text-sm">Device auth 浏览器登录、local-unlock、agent create、dex swap、audit export</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">🤖 AI Gateway</div>
<div class="text-sm">真实 Anthropic Messages API 集成，预付费余额、成本追踪、模型白名单</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">💰 Shared Budget</div>
<div class="text-sm">Workspace team budgets + cross-wallet spending tracking + policy enforcement</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">⚡ x402 Payments</div>
<div class="text-sm">真实 OWS 签名 x402 结算，支持 Base / Ethereum ERC-20 transfer</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">🖼️ Embedded Wallet</div>
<div class="text-sm">/embed iframe + postMessage，第三方 dApp 可直接集成</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">✈️ Telegram Mini App</div>
<div class="text-sm">TWA 钱包 UI + Bot webhook，支持移动端 Agent 交互</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">🪐 Solana + Jupiter</div>
<div class="text-sm">真实 Solana ed25519 签名 + Legacy Message 序列化 + Jupiter v6 Swap（devnet 已验证）</div>
<div class="text-xs text-green-600 mt-1 font-mono">ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-1">🚀 Deployment Ready</div>
<div class="text-sm">Dockerfile + DEPLOY.md + start-local.sh（Vercel + Railway / Fly.io）</div>
<div class="text-xs text-green-600 mt-1 font-mono">READY</div>
</div>

</div>

---
layout: default
---

# 开发里程碑：从 0 到全平台

<div class="space-y-3 text-sm">

<div class="flex items-center gap-3 p-3 rounded bg-green-50 border border-green-200">
  <span class="text-green-600 font-bold">✅ T00–T03</span>
  <span>核心 OWS Vault、Policy Engine、MCP Server、Axum API、Next.js Dashboard、Passkey Auth</span>
</div>

<div class="flex items-center gap-3 p-3 rounded bg-green-50 border border-green-200">
  <span class="text-green-600 font-bold">✅ T04–T06</span>
  <span>Wallet Lifecycle（create/close/pause）、Multi-chain Support、Real DEX Swap（1inch + Uniswap V3 fallback）</span>
</div>

<div class="flex items-center gap-3 p-3 rounded bg-green-50 border border-green-200">
  <span class="text-green-600 font-bold">✅ T07–T10</span>
  <span>Audit & Integrity（HMAC chain + Merkle anchor）、x402 Payments、Shared Budget Team Workspaces、Advanced Policy Engine（intent + risk signal）</span>
</div>

<div class="flex items-center gap-3 p-3 rounded bg-green-50 border border-green-200">
  <span class="text-green-600 font-bold">✅ T11–T16</span>
  <span>Identity Recovery（email + Passkey re-register）、CLI Device Auth、Telegram Mini App、5-language SDKs、Landing Page、Pitch Deck</span>
</div>

<div class="flex items-center gap-3 p-3 rounded bg-green-50 border border-green-200">
  <span class="text-green-600 font-bold">✅ T17</span>
  <span>Solana 签名链路接入：真实 base58 地址派生、devnet 余额查询、SOL 转账签名广播、Jupiter DEX Swap 集成</span>
</div>

</div>

<div class="mt-6 text-center text-lg font-semibold text-blue-600">
核心平台完成度 ≈ 99% · 后端 API / MCP / 前端页面 / SDK / 多链签名 全部可用
</div>

---
layout: default
class: text-center
---

# Demo 流程

## 现场 4 分钟演示

<div class="text-left max-w-3xl mx-auto mt-6 space-y-3">

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">1</div>
  <div>
    <div class="font-semibold">Landing Page → 注册 & 登录</div>
    <div class="text-sm opacity-80">产品页 CTA → <code>/login</code> Passkey 注册 → Dashboard 创建 Wallet → Balance & Swap</div>
  </div>
</div>

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">2</div>
  <div>
    <div class="font-semibold">策略 & 团队预算</div>
    <div class="text-sm opacity-80">配置 Policy（spend limit + contract whitelist）→ 创建 Workspace Shared Budget → 跨钱包实时追踪</div>
  </div>
</div>

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">3</div>
  <div>
    <div class="font-semibold">Solana 签名 Demo（Devnet）</div>
    <div class="text-sm opacity-80"><code>gradience agent create --name sol-test</code> → 生成真实 base58 地址 → 查 devnet 余额 → <code>agent fund ... --chain solana</code> 签名并广播转账</div>
  </div>
</div>

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">4</div>
  <div>
    <div class="font-semibold">CLI Device Auth + MCP</div>
    <div class="text-sm opacity-80"><code>gradience auth login</code> → 浏览器授权 → CLI 自动拿到 token → spawn gradience-mcp → tools/call get_balance / sign_transaction</div>
  </div>
</div>

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">5</div>
  <div>
    <div class="font-semibold">恢复 & 多平台</div>
    <div class="text-sm opacity-80">Forgot Passkey → Recovery code → 新设备重注册 Passkey → Telegram Mini App 查看同一钱包</div>
  </div>
</div>

</div>

---
layout: default
---

# Why Now / Go-to-Market

<div class="grid grid-cols-2 gap-6">

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-2">🌊 市场时机</div>
<ul class="text-sm list-disc pl-4 space-y-1">
<li>AI Agent 数量激增，但 99% 没有安全的钱包托管方案</li>
<li>$200B+ 的链上 Agent Economy 需要“Autonomy with Guardrails”</li>
<li>MCP 正在成为 LLM 调用外部工具的事实标准</li>
</ul>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-2">🚀 落地路径</div>
<ul class="text-sm list-disc pl-4 space-y-1">
<li><strong>开发者</strong>：通过 MCP + 5-language SDK 快速集成</li>
<li><strong>企业</strong>：Shared Budget + Audit + x402 满足合规与支付需求</li>
<li><strong>终端用户</strong>：Telegram Mini App + Embedded Wallet 降低使用门槛</li>
</ul>
</div>

</div>

<div class="mt-6 p-4 rounded bg-blue-50 text-center">
<div class="font-semibold text-blue-700">商业模式</div>
<div class="text-sm mt-1">Cloud 托管版 SaaS 订阅 + MCP API 调用按量计费 + 未来协议层手续费抽取</div>
</div>

---
layout: default
class: text-center
---

# 谢谢

<div class="text-2xl font-semibold mt-12">
Autonomy with guardrails — that's the only way Agentic Economy scales.
</div>

<div class="mt-8 text-sm opacity-60">
GitHub: github.com/open-wallet-standard/gradience-wallet<br>
Live Demo: localhost:3000 &nbsp;|&nbsp; CLI: <code>cargo run --bin gradience -- start</code>
</div>
