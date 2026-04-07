# Phase 1: PRD — 需求定义

> **Project:** Gradience Wallet — Agent 钱包编排平台
> **Status:** Draft v2.0 (Enhanced)
> **Date:** 2026-04-07
> **Version History:** v1.0 (初始) → v2.0 (新增动态策略、交易意图、完整风险与扩展路线图)

## 1. 背景

### 1.1 问题陈述

AI Agent 已成为链上活动的一等公民，可自主执行交易、支付服务费、管理金库等。但当前 Agent 钱包方案存在严重碎片化：

- **密钥散落、各工具独立管理**，明文私钥常见于环境变量。
- **权限缺乏精细隔离**：要么全权，要么不可用。
- **操作不可审计**，用户难以追溯与控制。
- **缺乏统一开放标准**：OKX Agentic Wallet 等方案封闭且互不兼容。

### 1.2 市场机会

- **Open Wallet Standard (OWS)** 提供链无关、local-first、policy-gated signing 的开放标准 (v1.2.4, MIT License)，获 PayPal、Ethereum Foundation、Solana Foundation 等 15+ 机构支持。
- **Passkey 技术成熟**，提供无助记词的现代身份认证。
- **Agent 经济快速爆发**，个人与企业均需**标准化、可治理的 Agent 钱包编排平台**，支持多租户、精细权限与合规审计。

---

## 2. 产品定位

### 2.1 一句话定义

> Gradience Wallet 是一个面向个人与企业的 **Agent 钱包编排平台** —— 以 Passkey 管理主身份、以 OWS 管理钱包、以智能策略引擎管理权限，支持 Agent 安全自主交易与支付。

### 2.2 产品形态

| 组件 | 形态 | 作用 |
|---|---|---|
| **Web Dashboard** | React SPA (TypeScript) | 用户/团队可视化管理界面 |
| **CLI 工具** | Rust 二进制 | 开发者快速操作与脚本集成 |
| **MCP Server** | Rust 后台服务 | Agent 通过标准协议访问钱包 |
| **Core Engine** | Rust 库 (`gradience-core`) | 策略评估、钱包编排、支付路由 |
| **SDK** | Rust / Node.js (NAPI) / Python (PyO3) | 第三方 Agent 与应用集成 |

### 2.3 目标市场

- **ToC（个人）**：个人自动化 DeFi、日常 Agent 任务。
- **ToB（企业）**：团队级 Agent 钱包池管理、权限分层、合规审计与多租户治理。

### 2.4 竞争定位：Gradience vs Tempo Wallet

Tempo Wallet 是一款优秀的**单生态 Passkey 钱包**：用户通过 Passkey + 邮箱绑定获得一个 Tempo 账户，CLI 通过浏览器授权即可访问。但它本质上是**生态入口**——钱包、链、协议都由 Tempo 自己定义，Agent 一旦获得授权，权限控制相对粗放。

Gradience Wallet 在同样的 Passkey + 邮箱 + CLI Device Auth 体验之上，做成了**Agent 钱包编排平台**，核心差异体现在三方面：

| 维度 | Tempo Wallet | Gradience Wallet |
|---|---|---|
| **钱包标准** | Tempo 自有单生态钱包 | **OWS 开放标准多链钱包**（BIP-39 HD，本地 vault） |
| **Agent 权限** | 基础 spending limit | **多层 Policy Engine**：限额 + 合约/操作/时间/模型白名单 + 意图风险 + 动态信号 |
| **交互协议** | Tempo 私有协议 | **MCP (Model Context Protocol)** — 任何 LLM/Agent 都能标准化接入 |
| **审计与合规** | 基础日志 | **HMAC-chained audit log + Merkle tree 上链 anchoring**， tamper-evident |
| **部署形态** | Tempo 托管 SaaS | **Local-first 单二进制** + 可选自托管云部署 |

一句话差异：
> **Tempo 是“给 Agent 一个钱包”；Gradience 是“让用户真正拥有自己的钱包，并精确编排 Agent 能做什么”。**

---

## 3. 核心功能模块

### 3.1 身份管理

- **Passkey 主认证**（WebAuthn），无助记词。
- **邮箱绑定**：注册时可选填写真实邮箱，与 Passkey 凭证绑定到同一用户账户。
- **跨设备恢复（Email Recovery）**：
  1. 在新设备点击 "Forgot Passkey?"
  2. 输入用户名 → 系统发送 **recovery code** 到邮箱（当前 demo 输出到 console，生产接 SendGrid/AWS SES）
  3. 验证 code → 获得短期 **recovery_token**
  4. 在新设备直接调用 WebAuthn `create()` **重新注册一枚新 Passkey**
  5. 新 Passkey 自动绑定到原账户，旧 Passkey 被替换
  6. 输入原 **Vault Passphrase** → 解锁同一组 OWS 钱包
- 支持 MFA 与未来 OAuth（Google/GitHub）。
- 关键操作二次确认。

### 3.2 Agent 钱包管理

- 为每个 Agent 创建独立子钱包（HD 派生）。
- 多链地址支持（EVM、Solana、BTC、Sui、TON、Tron、Cosmos 等 OWS 支持链）。
- API Key 发放与绑定。
- 钱包生命周期（active / suspended / revoked）。
- 聚合多链资金概览。

### 3.3 策略引擎（Policy Engine）—— 产品核心中枢

策略引擎是 Gradience Wallet 的**安全大脑与守门人**，所有交易/支付在 pre-signing 阶段完成评估，充分利用 OWS 原生 policy-gated signing。

**静态规则**（v1.0 基础）：
- 单笔/日/月限额、链/合约白名单、操作类型限制、时间窗口、审批模式（allow / warn / deny）。

**动态策略**（v1.5+）：
- 根据外部信号实时调整规则：Gradience Reputation 分数、市场风险（Forta/Chainalysis）、Agent 行为 profiling、团队预算剩余等。
- 示例：高 Reputation Agent 自动提升限额 20%；高风险市场环境下收紧滑点保护并转为 warn 模式。

**交易意图分析**（v1.5+）：
- Agent 提交结构化 Intent（swap / transfer / bridge 等，含 strategyTag、expectedSlippage 等）。
- 引擎先解析意图是否匹配用户预设的"交易策略模板"，再结合动态风险评分进行最终决策。
- 输出清晰 reason，提升可解释性。

**审计闭环**：每次评估记录 context、intent、dynamic factors 与结果。

### 3.4 Web Dashboard

- **首页**：多链资产总览、Agent 状态、近期活动与告警。
- **钱包管理**：Agent 钱包列表、创建/删除/编辑、多链地址展示、余额查询。
- **策略管理**：可视化编辑器 + 模板（保守/标准/开放）、自定义策略、策略生效/暂停/删除。
- **审计日志**：时间线 + 导出。
- **团队管理**：成员邀请/权限分配、角色（Owner / Admin / Viewer）、团队级策略。
- **新增**：交易意图模板配置、动态策略监控仪表盘。

### 3.5 DEX 聚合（内建）

- 支持 Uniswap、SushiSwap、Curve（EVM）、Jupiter（Solana）、Cetus 等。
- 滑点保护、限价单、MEV 保护（私密 RPC），全部受策略引擎控制。

### 3.6 支付协议集成

- **x402**（即时支付）、**MPP**（高频微支付）。
- 协议无关抽象层 + 预算管理 + 策略联动 + 完整审计。

### 3.7 团队管理（多租户）

- Workspace 隔离。
- 角色：Owner / Admin / Member / Viewer。
- 团队级策略与全局预算控制。
- 成员邀请/退出 + 未来 SSO（Enterprise）。

### 3.8 CLI 工具

```bash
# 身份
gradience auth login          # Browser-based Device Auth (opens wallet URL, polls for approval)
gradience auth whoami         # Show remote API auth status + local vault status
gradience auth logout

# 钱包
gradience agent create        # 创建 Agent 钱包
gradience agent list
gradience agent fund          # 入金
gradience agent balance

# 策略
gradience policy set
gradience policy list
gradience policy delete
gradience policy test --tx "0x..." --intent '{"type":"swap",...}'  # 模拟评估

# DEX
gradience swap
gradience quote

# 审计
gradience audit
gradience export

# 团队
gradience team invite
gradience team list
gradience team role
```

**CLI Device Auth 流程说明**：
1. 用户运行 `gradience auth login`
2. CLI 请求 API `/api/auth/device/initiate`，生成 `user_code` (如 `ABCD-EFGH`) 和验证 URL
3. CLI 自动打开浏览器（或打印 URL），引导用户访问钱包网站的 `/device?code=ABCD-EFGH`
4. 用户在已登录的 Web 钱包中点击 **Approve Device**
5. CLI 轮询 `/api/auth/device/poll`，拿到 `token` 后保存到 `~/.gradience/.cli_token`
6. 后续 CLI 命令携带该 token 调用远程 API

---

## 4. 技术架构

### 4.1 技术栈

| 层 | 技术 | 理由 |
|---|---|---|
| **前端 (Web Dashboard)** | React + TypeScript + Tailwind | 主流 Web 技术 |
| **后端核心 (Core Engine)** | **Rust** | OWS 核心是 Rust，零 FFI 损耗，内存安全，性能最优 |
| **API 服务层** | Rust (Axum / Actix) | 与核心同语言，无需跨语言调用 |
| **CLI 工具** | Rust (clap) | 直接链接 `ows-core`，无 NPM 依赖链 |
| **数据库** | SQLite (本地 via rusqlite) / PostgreSQL (云 via sqlx) | 灵活部署，强类型 ORM |
| **钱包标准** | `ows-core` Rust crate (v1.2+) | OWS 原生实现，直接依赖 |
| **加密** | `ring` / `sodiumoxide` | Rust 生态标准加密库 |
| **认证** | WebAuthn (passkey) via `webauthn-rs` | Rust 原生 WebAuthn 实现 |
| **SDK** | Rust crate + Node.js (NAPI-RS) + Python (PyO3) | 多语言覆盖，底层统一 Rust |
| **部署** | 本地优先 (single binary) + 可选云服务 (Docker) | 单二进制部署极简 |

### 4.2 Rust 核心模块划分

```
gradience-wallet/
├── gradience-core/          # 核心库 (Rust)
│   ├── src/
│   │   ├── identity/        # Passkey/WebAuthn 身份管理
│   │   ├── wallet/          # Agent 钱包创建/派生/HDM
│   │   ├── policy/          # 策略引擎 (静态+动态+Intent)
│   │   ├── dex/             # DEX 聚合路由
│   │   ├── payment/         # x402 / MPP 支付适配器
│   │   ├── audit/           # 审计日志
│   │   └── ows_adapter/     # OWS 集成隔离层
│
├── gradience-cli/           # CLI 应用 (Rust, 链接 core)
│   └── src/main.rs
│
├── gradience-mcp/           # MCP Server (Rust, 链接 core)
│   └── src/main.rs
│
├── gradience-api/           # API Server (Rust Axum, 链接 core)
│   └── src/main.rs
│
└── gradience-sdk/
    ├── gradience-sdk-node/   # NAPI-RS Node.js 绑定
    └── gradience-sdk-python/ # PyO3 Python 绑定
```

### 4.3 架构图

```
┌────────────────────────────────────────────────────────────┐
│                    Web Dashboard (React)                    │
│  首页 | 钱包 | 策略 | 审计 | 团队 | DEX | 设置              │
└──────────────────────┬─────────────────────────────────────┘
                       │ HTTPS
┌──────────────────────▼─────────────────────────────────────┐
│              gradience-api (Rust Axum)                      │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────────┐   │
│  │ 认证服务 │ │ 钱包服务 │ │ 策略服务 │ │ 审计日志服务 │   │
│  │WebAuthn  │ │OWS Vault │ │Policy    │ │  事件存储   │   │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └──────┬──────┘   │
│       │            │            │               │          │
└───────┼────────────┼────────────┼───────────────┼──────────┘
        │            │            │               │
┌───────▼────────────▼────────────▼───────────────▼──────────┐
│              gradience-core (Rust 核心库)                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │
│  │ 密钥管理 │ │ 策略引擎 │ │ DEX 路由 │ │ 支付协议适配 │   │
│  │ HD 派生  │ │ 静态+动态│ │ 最优路径 │ │ x402 / MPP   │   │
│  │          │ │ Intent解析│ │          │ │              │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘  │
│  ┌────────────────┐  ┌────────────────────────────────┐    │
│  │ OWS Adapter    │  │ 动态信号适配器 (HTTP client)   │    │
│  │ (ows-core FFI) │  │ Forta / Chainalysis / Rep API  │    │
│  └────────────────┘  └────────────────────────────────┘    │
└──────────────┬───────────────────┬──────────────────────────┘
               │                   │
        ┌──────▼──────┐    ┌──────▼──────┐
        │gradience-cli│    │ gradience   │
        │  (Rust CLI) │    │  MCP Server │
        │  (clap)     │    │  (Rust)     │
        └─────────────┘    └─────────────┘
               │                   │
               └────────┬──────────┘
                        │
               ┌────────▼────────┐
               │   gradience     │
               │   SDK           │
               │  (NAPI/PyO3)    │
               └────────┬────────┘
                        │
               ┌────────▼────────┐
               │   ows-core      │
               │   (Rust crate)  │
               └────────┬────────┘
                        │
                    ┌───▼─────┐
                    │ 链上 RPC │
                    │EVM/SVM… │
                    └─────────┘
```

### 4.4 数据流

```
用户操作 (Web/CLI/Agent MCP)
    ↓
Rust API Server (WebAuthn 认证 + 权限检查)
    ↓
gradience-core
    ├── Policy Engine (Rust) 判断：允许？
    │   ├── 静态规则评估
    │   ├── 动态信号拉取 (Reputation / 市场风险)
    │   └── Intent 匹配 (如有)
    ├── 如果是交易：DEX Router 获取最优路径
    ├── 如果是支付：支付协议路由
    └── ows-core 签名
    ↓
广播到链上 RPC
    ↓
结果返回 + 审计日志记录 (SQLite/PostgreSQL)
    ↓
Web Dashboard 实时更新 (WebSocket/ SSE)
```

---

## 5. 经济模型

### 5.1 核心原则

**不发 Token，只做现金流业务。**

### 5.2 收入模式

| 模式 | 说明 | 定价参考 |
|---|---|---|
| **免费层** | 1 用户, 3 Agent 钱包, 基础策略 | 吸引用户 |
| **Pro 订阅** | $19/月 — 无限 Agent, 高级策略, 审计导出 | 个人/小团队 |
| **Team 订阅** | $99/月 — 多租户, 角色管理, 团队策略 | 中型团队 |
| **Enterprise** | 定制报价 — SLA, 私有部署, 合规支持, 动态风险评估 | 大型企业 |
| **DEX 手续费** | 交易额的 0.05%（从 DEX 返佣中取） | 透明无额外成本 |
| **支付协议服务费** | x402/MPP 交易额的 0.1% | 可选关闭 |

### 5.3 用户激励

- **返佣计划**：邀请好友获得 Pro 月卡
- **Volume 奖励**：月度交易量大自动降费率
- **开源贡献**：贡献代码/翻译获得订阅折扣

### 5.4 成本结构

| 项目 | 说明 |
|---|---|
| 云服务 (可选) | Vercel / AWS — 按量付费 |
| RPC 节点 | Alchemy / QuickNode — 免费额度 + 付费升级 |
| Passkey 服务 | 自建 WebAuthn — 零成本 |
| 人工 | 开发 + 客服 (早期团队 2-3 人) |

---

## 6. 版本规划

| 阶段 | 交付物 | 关键特性 | 预计周期 |
|---|---|---|---|
| **v0.1 Alpha** | CLI | 身份、Agent 钱包创建、基础策略 | 2-3 周 |
| **v0.2 Beta** | Web Dashboard | 完整前端、审计日志、DEX 聚合 | 4-6 周 |
| **v1.0 GA** | 完整产品 | x402 支付、Pro 订阅、CLI 完善 | 6-8 周 |
| **v1.5** | 动态策略 | Reputation 联动、交易意图、Agent profiling | 4-6 周 |
| **v2.0** | Enterprise | MPP、SSO、审批流、规则版本控制 | 8-10 周 |
| **v2.5+** | 生态闭环 | AI 策略建议、策略市场、AgentM 集成 | 持续 |

---

## 7. 验收标准（当前实现状态）

### 7.1 v0.1 Alpha — 已完成 ✅

- [x] CLI 可以安装并运行 (`cargo build --bin gradience`)
- [x] 用户可以通过 Passkey 注册/登录
- [x] 用户可以创建 Agent 钱包并看到多链地址
- [x] 用户可以设置基础 Policy (限额、白名单、时间窗口)
- [x] MCP Agent 可以通过 OWS 接口签名和发送交易
- [x] Policy 在实际签名前生效
- [x] 支持 OWS EVM 链（Base、Ethereum 等）
- [x] 代码开源，有完整 README

### 7.2 v0.2 Beta — 已完成 ✅

- [x] Web Dashboard 可访问 (`/dashboard`)
- [x] 资产总览页面（原生余额 + 代币资产）
- [x] 钱包管理界面（创建、地址展示、余额查询、转账、Swap、Anchor）
- [x] API Key 管理界面
- [x] 审计日志时间线（最近交易列表）
- [x] DEX 聚合可用（1inch Swap API + Uniswap V3 fallback，Base 链）
- [x] CLI 与 Web 数据同步（共享同一 SQLite 数据库 + OWS Vault）
- [x] **新增**：邮箱恢复 Passkey 重注册完整 flow
- [x] **新增**：CLI Device Authorization（浏览器确认登录）
- [x] **新增**：Examples & Playgrounds 矩阵（4 个独立 demo）
- [x] **新增**：嵌入式 iframe 钱包（`/embed` + Messenger）

### 7.3 v0.3 Pre-GA — 当前冲刺中 🚧

- [x] x402 支付协议集成（MCP `pay` tool + AI Gateway balance）
- [ ] Pro 订阅付费流程
- [x] 审计导出基础（日志含 intent 与 dynamic factors）
- [x] **生产环境部署准备**：Dockerfile + `DEPLOY.md`
- [x] 完整文档 + API 文档（MCP、Examples、Architecture）
- [x] **MCP 类型安全层**：schemars 自动生成 JSON Schema

### 7.4 v1.0 GA — 4 月目标

- [ ] 动态策略上线（Reputation 评分 + 市场风险信号集成）
- [ ] 交易意图模板库（swap、DCA、rebalance）
- [ ] 自定义域名 + Vercel/Railway 一键部署验证
- [ ] 团队级审批流（warn 交易多级审批）

### 7.5 v1.5 — 后续扩展

- [ ] 动态策略支持实时 Forta/Chainalysis 联动
- [ ] Agent 行为 profiling 自动调参
- [ ] 跨 Agent 预算共享与 Workspace 总预算控制
- [ ] 策略市场与 AI 辅助策略生成

---

## 8. 风险 & 后续扩展

### 8.1 主要风险及缓解措施

| 风险类别 | 具体风险 | 影响 | 缓解措施 |
|---|---|---|---|
| **标准依赖** | OWS 小版本升级导致兼容性问题 | 高 | 引入 `ows_adapter.rs` 抽象层隔离 `ows-core` API，锁定核心接口版本，定期运行兼容测试；积极参与 OWS 社区贡献 |
| **密钥与身份安全** | Passkey 设备丢失 / 主密钥泄露 | 高 | 多设备绑定 + 邮箱恢复 + 社交恢复机制规划；HD 派生 + libsodium 本地加密 + 未来 HSM 支持 |
| **动态信号可靠性** | 外部 API（Forta/Chainalysis）延迟或不可用 | 中 | 缓存 + fallback 规则（默认收紧策略）；多源信号组合；本地模拟测试模式 |
| **意图解析准确性** | Agent 提交恶意/模糊 intent | 中 | 严格 schema 验证 + simulation 预检查；审计所有 mismatch 案例；提供清晰用户反馈 |
| **合规与监管** | 金融监管政策变化（尤其是 ToB 端） | 高 | 明确"不托管用户资金，仅提供工具"；支持导出合规模块；Enterprise 版提供 SOC2/ISO27001 支持路径 |
| **性能与成本** | 高频交易下策略评估延迟或 RPC 成本上升 | 中 | 规则并行评估 + 缓存；按量付费云资源；免费额度 RPC + 私有节点选项 |
| **竞争** | 大厂（OKX、Coinbase、Fireblocks）快速跟进 | 中 | 强调开源 + Passkey + Reputation 联动 + 交易意图 + 多租户治理差异化；与 OWS 生态深度绑定 |
| **采用门槛** | 开发者/企业集成复杂 | 中 | 提供丰富 SDK、CLI、示例 + 完整文档；社区模板与教程；免费层降低试用门槛 |

### 8.2 后续扩展路线图

**v1.5（中期功能强化）**：
- 动态风险评分：集成 Chainalysis KYT + Forta
- Agent 行为 profiling：基于历史交易模式自动调整策略
- 跨 Agent 预算共享：Workspace 层面总预算控制
- 基础交易意图模板库（swap、DCA、rebalance）

**v2.0（企业级完备）**：
- 规则版本控制：Policy 完整版本历史、diff、回滚
- 审批流：warn 交易可配置多级审批 + 超时自动 deny
- 高级审计：合规友好导出（PDF/CSV）、风险热力图
- SSO / SAML / OIDC + 审计日志不可篡改（可选区块链 anchoring）

**v2.5+（生态与智能进化）**：
- Reputation 闭环：AgentM 任务 → Judge 评分 → 动态策略自动调整
- AI 辅助策略生成：用户描述需求，AI 推荐 Policy + Intent 模板
- 策略市场：社区贡献的高质量模板
- 高级安全：Guardian 机制、紧急暂停（kill switch）、保险集成
- 可观测性：OpenTelemetry 集成 + 实时策略执行监控

**长期愿景**：
Gradience Wallet 成为 Agentic Economy 的"操作系统级"钱包治理层，支持任意交易策略安全执行、Reputation-driven 权限、机构级合规，同时保持开源核心 + 商业增值服务的平衡。

---

## 9. 术语

| 术语 | 定义 |
|---|---|
| **Master Account** | 用户的主身份，由 Passkey 保护 |
| **Agent Wallet** | 从 Master Account 派生的子钱包，供 Agent 使用 |
| **Policy** | 控制 Agent 钱包权限的规则集 |
| **OWS** | Open Wallet Standard，开放钱包标准 |
| **MCP** | Model Context Protocol，AI Agent 的通用工具调用协议 |
| **Vault** | OWS 的加密钱包存储格式 |
| **X402** | 基于 HTTP 402 的即时支付协议 |
| **MPP** | Machine Payments Protocol，Tempo/Stripe 的高频微支付协议 |
| **DEX** | 去中心化交易所 |
| **MEV** | Maximal Extractable Value，最大可提取价值 |
| **Workspace** | 团队工作空间，多租户的隔离单元 |
| **Dynamic Policy** | 基于实时信号调整的策略规则 |
| **Transaction Intent** | Agent 提交的结构化交易意图描述 |

---

## 10. 参考

- [OWS Specification](https://docs.openwallet.sh/)
- [OWS GitHub](https://github.com/open-wallet-standard/core)
- [WebAuthn / Passkey](https://webauthn.guide/)
- [MCP Protocol](https://modelcontextprotocol.io/)
- [X402 Protocol](https://x402.org)
- [MPP - Machine Payments Protocol](https://www.tempo.xyz/mpp)
- [Jupiter DEX Aggregator](https://jup.ag)
- [Uniswap](https://uniswap.org)
- [Chainalysis KYT](https://www.chainalysis.com/products/kyt/)
- [Forta Network](https://forta.org/)

---

*验收通过后进入 Phase 2: Architecture*
