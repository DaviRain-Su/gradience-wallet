---
theme: default
background: https://images.unsplash.com/photo-1451187580459-43490279c0fa?q=80&w=2072&auto=format&fit=crop
class: text-center
highlighter: shiki
---

# Gradience Wallet

## Agent 钱包编排平台

<p class="text-xl opacity-80 mt-4">
以 Passkey 管理身份 · 以 OWS 管理本地多链钱包 · 以 Policy Engine 精确编排 Agent 权限
</p>

<div class="mt-12 text-sm opacity-60">
HashKey Chain Horizon Hackathon 2026
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
Gradience Wallet 是一个面向个人与企业的 <span class="text-blue-600">Agent 钱包编排平台</span> —— 以 Passkey 管理主身份、以 OWS 管理钱包、以智能策略引擎管理权限，支持 Agent 安全自主交易与支付。
</div>

<div class="grid grid-cols-3 gap-4 mt-8">
  <div class="p-4 rounded bg-gray-50">
    <div class="font-bold text-lg">🗝️ Passkey</div>
    <div class="text-sm opacity-80">无助记词现代身份认证</div>
  </div>
  <div class="p-4 rounded bg-gray-50">
    <div class="font-bold text-lg">🔐 OWS</div>
    <div class="text-sm opacity-80">本地 first 多链钱包标准</div>
  </div>
  <div class="p-4 rounded bg-gray-50">
    <div class="font-bold text-lg">🛡️ Policy</div>
    <div class="text-sm opacity-80">签名前自动评估与拦截</div>
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
  E --> G[Agent 1 请求 Swap]
  F --> H[Agent 2 请求转账]
  G --> I{Policy Engine}
  H --> I
  I -->|Allow| J[自动签名执行]
  I -->|Deny| K[拒绝并返回原因]
```

<div class="mt-4 text-sm">
<strong>关键：</strong>Agent 不持有私钥。每一步操作都必须先通过 Policy Engine 的 pre-signing 评估。
</div>

---
layout: default
---

# 竞争壁垒：Gradience vs Tempo

| 维度 | Tempo Wallet | <span class="text-blue-600 font-bold">Gradience Wallet</span> |
|:---|:---|:---|
| **钱包标准** | Tempo 自有单生态 | **OWS 开放标准多链钱包**（BIP-39，本地 vault） |
| **Agent 权限** | 基础 spending limit | **多层 Policy Engine**：限额 + 合约/操作/时间/模型白名单 + 意图风险 + 动态信号 |
| **交互协议** | 私有协议 | **MCP (Model Context Protocol)** — 任何 LLM/Agent 标准接入 |
| **安全审计** | 基础日志 | **HMAC-chained audit log + Merkle tree 上链 anchoring** |
| **部署形态** | Tempo 托管 SaaS | **Local-first 单二进制** + 可选自托管云部署 |

<div class="mt-6 p-4 border-l-4 border-purple-500 bg-purple-50 rounded">
<strong>核心差异：</strong>Tempo 是“给 Agent 一个钱包”；Gradience 是“让用户真正拥有自己的钱包，并精确编排 Agent 能做什么”。
</div>

---
layout: default
---

# 已落地的产品矩阵

<div class="grid grid-cols-2 gap-6">

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-2">🌐 Web Dashboard</div>
<div class="text-sm">钱包管理、余额、Swap、Fund、API Key、策略配置</div>
<div class="text-xs text-green-600 mt-1 font-mono">STATUS: ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-2">🔌 MCP Server</div>
<div class="text-sm">7 个标准 tool，schemars 自动生成 JSON Schema</div>
<div class="text-xs text-green-600 mt-1 font-mono">STATUS: ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-2">💻 CLI</div>
<div class="text-sm">Device auth 浏览器登录、本地/远程双模式</div>
<div class="text-xs text-green-600 mt-1 font-mono">STATUS: ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-2">🖼️ Embedded Wallet</div>
<div class="text-sm">/embed iframe + postMessage，第三方 dApp 可嵌入</div>
<div class="text-xs text-green-600 mt-1 font-mono">STATUS: ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-2">🎬 Examples & Playgrounds</div>
<div class="text-sm">4 个独立 demo + 一键启动脚本 (run-all.sh)</div>
<div class="text-xs text-green-600 mt-1 font-mono">STATUS: ONLINE</div>
</div>

<div class="p-4 rounded border">
<div class="font-bold text-lg mb-2">🚀 Deployment Ready</div>
<div class="text-sm">Dockerfile + DEPLOY.md (Vercel + Railway/Fly.io)</div>
<div class="text-xs text-green-600 mt-1 font-mono">STATUS: READY</div>
</div>

</div>

---
layout: default
class: text-center
---

# Demo 流程

## 现场 5 分钟演示

<div class="text-left max-w-3xl mx-auto mt-8 space-y-4">

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">1</div>
  <div>
    <div class="font-semibold">注册 & 登录</div>
    <div class="text-sm opacity-80">Web 端 Passkey 注册 → 创建 Wallet → 查看 Balance & Swap</div>
  </div>
</div>

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">2</div>
  <div>
    <div class="font-semibold">邮箱恢复 Passkey（跨设备）</div>
    <div class="text-sm opacity-80">Forgot Passkey → 输入 recovery code → 新设备重新注册 Passkey → 同一钱包恢复</div>
  </div>
</div>

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">3</div>
  <div>
    <div class="font-semibold">CLI Device Auth</div>
    <div class="text-sm opacity-80"><code>gradience auth login</code> → 浏览器确认 → CLI 自动拿到 token → <code>gradience auth whoami</code></div>
  </div>
</div>

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">4</div>
  <div>
    <div class="font-semibold">MCP Client 演示</div>
    <div class="text-sm opacity-80">Node.js MCP 客户端 spawn gradience-mcp → tools/list → tools/call (get_balance)</div>
  </div>
</div>

<div class="flex items-start gap-3">
  <div class="w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold shrink-0">5</div>
  <div>
    <div class="font-semibold">嵌入式钱包</div>
    <div class="text-sm opacity-80">第三方 dApp demo 通过 iframe 请求签名 → 用户在 embed 页面 Approve/Reject</div>
  </div>
</div>

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
GitHub: github.com/your-org/gradience-wallet<br>
Demo: localhost:3000 | CLI: gradience auth login
</div>
