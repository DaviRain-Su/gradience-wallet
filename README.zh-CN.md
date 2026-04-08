# Gradience Wallet

基于 [Open Wallet Standard (OWS)](https://github.com/open-wallet-standard/core) 构建的 **Agent 钱包编排平台**。

Gradience 让用户能够创建 Passkey 身份、在本地管理多链钱包，并通过标准化的 MCP（Model Context Protocol）接口将细粒度、策略受控的访问权限委托给 AI Agent。

---

## 核心特性

- **OWS 原生 Vault**：与 `ows-lib` 和 `ows-signer` 真实集成，实现本地助记词生成、加密钱包存储与多链签名。
- **策略引擎**：多层策略体系 —— 花费限额、意图分析、动态风险信号、时间窗口、链/合约白名单。
- **Web UI + Passkey**：基于 Next.js 的前端，支持 WebAuthn Passkey 注册/登录，本地优先架构。
- **DEX 集成**：真实的 1inch Swap API + Uniswap V3 降级方案，可通过 Web UI、CLI 和 MCP 执行。
- **MCP Server**：JSON-RPC MCP 服务器，暴露 `sign_transaction`、`sign_message`、`sign_and_send`、`get_balance`、`swap`、`pay`、`llm_generate`、`ai_balance`、`ai_models`、`verify_api_key` 等工具。
- **AI 网关**：真实的 Anthropic Messages API 集成，支持预付费余额、成本追踪与模型白名单对账。
- **审计与完整性**：基于 HMAC 链的审计日志，结合 Merkle 树锚定实现篡改检测。
- **x402 支付**：真实的 OWS 签名 x402 结算，支持 Base/Ethereum 上的 ERC-20 转账。
- **共享预算**：Workspace 级别团队预算，通过 `shared_budget` 策略规则实现跨钱包花费追踪。
- **多平台 SDK**：Python、TypeScript、Go、Java、Ruby SDK。
- **Telegram Mini App**：TWA 钱包 UI，支持 Bot Webhook。
- **本地优先**：SQLite + 本地 Vault；所有数据保留在设备上，完全可自托管。

---

## 快速开始

### 环境要求

- Rust 1.80+ / Cargo
- Node.js 18+ / npm

### 构建

```bash
cargo build --workspace
```

### 启动 Web UI（本地优先）

最简单的方式是通过一条命令同时启动 API 服务器和 Web UI。

**方式 A — Shell 脚本**
```bash
./start-local.sh
```

**方式 B — Rust CLI**
```bash
cargo run --bin gradience -- start
```

两者都会：
1. 在 `http://localhost:8080` 启动 API 服务器
2. 在 `http://localhost:3000` 启动 Next.js 开发服务器
3. 自动打开浏览器

然后使用 Passkey 注册/登录，通过 Web UI 创建钱包、充值、兑换并锚定交易。

### CLI 使用

```bash
cargo run --bin gradience -- --help

# 创建钱包
cargo run --bin gradience -- agent create --name demo

# 查询 Base 余额
cargo run --bin gradience -- agent balance <wallet-id> --chain base

# 执行真实 DEX 兑换
cargo run --bin gradience -- dex swap <wallet-id> --from 0x8335... --to 0x4200... --amount 1

# 导出审计日志
cargo run --bin gradience -- audit export <wallet-id> --format json
```

### SDK 使用

**Python SDK**
```bash
pip install ./sdk/python
```
```python
from gradience_sdk import GradienceClient

client = GradienceClient("http://localhost:8080", api_token="YOUR_TOKEN")
wallet = client.create_wallet("demo")
balance = client.get_balance(wallet["id"])
```

**TypeScript SDK**
```bash
npm install ./sdk/typescript
```
```typescript
import { GradienceClient } from "@gradience/sdk";

const client = new GradienceClient("http://localhost:8080", { apiToken: "YOUR_TOKEN" });
const wallet = await client.createWallet("demo");
const balance = await client.getBalance(wallet.id);
```

查看 [`docs/06-sdk-guide.md`](docs/06-sdk-guide.md) 获取完整 SDK 开发指南。

### 运行 MCP 服务器

```bash
cargo run --bin gradience-mcp
```

### 运行测试

```bash
cargo test --workspace
```

---

## 项目结构

```
gradience-wallet/
├── crates/
│   ├── gradience-core/      # 领域逻辑：OWS 适配器、策略引擎、审计、签名、RPC、DEX、HD、团队
│   ├── gradience-cli/       # 命令行钱包 (clap)
│   ├── gradience-db/        # SQLite/PostgreSQL 层 (sqlx)
│   ├── gradience-api/       # Axum REST API 服务器
│   ├── gradience-mcp/       # MCP stdio 服务器与工具处理
│   └── gradience-sdk-node/  # Node.js NAPI 绑定
├── contracts/               # Solidity 合约（Merkle 锚定）
├── docs/                    # PRD、架构、技术规范、测试规范、SDK 指南
├── sdk/
│   ├── python/              # Python SDK
│   ├── typescript/          # TypeScript SDK
│   ├── go/                  # Go SDK
│   ├── java/                # Java SDK
│   └── ruby/                # Ruby SDK
├── web/                     # Next.js 前端
├── start-local.sh           # 一键本地启动脚本（macOS/Linux）
├── start-local.ps1          # 一键本地启动脚本（Windows）
└── .sqlx/                   # sqlx 离线查询元数据
```

---

## 架构

1. **OWS 适配器 (`gradience-core`)**：`LocalOwsAdapter` 通过 git 依赖将所有钱包创建、签名和 API 密钥管理委托给官方 `ows-lib` crate。
2. **数据库层 (`gradience-db`)**：15 张表的 Schema，涵盖用户、钱包、地址、策略、API 密钥、工作区、审计日志和支付。
3. **策略引擎**：静态 JSON 策略评估，采用最严格合并语义支持多策略叠加。
4. **MCP 网关**：基于 stdio 的 JSON-RPC 2.0，兼容任意 MCP 宿主（Claude、Cursor 等）。

---

## 文档

- [`docs/01-prd.md`](docs/01-prd.md) — 产品需求与路线图
- [`docs/02-architecture.md`](docs/02-architecture.md) — 系统架构与架构决策记录
- [`docs/03-technical-spec.md`](docs/03-technical-spec.md) — 接口、数据库 Schema、算法
- [`docs/04-task-breakdown.md`](docs/04-task-breakdown.md) — 开发计划与里程碑
- [`docs/05-test-spec.md`](docs/05-test-spec.md) — TDD 测试定义
- [`docs/06-sdk-guide.md`](docs/06-sdk-guide.md) — SDK 开发指南与路线图

---

## 技术栈

- **语言**：Rust
- **CLI**：`clap`
- **Web**：Next.js + TypeScript + Tailwind CSS
- **数据库**：`sqlx` + SQLite（本地）/ PostgreSQL（云端）
- **加密**：`ows-lib` / `ows-signer`（OWS 原生）、`secp256k1`、`rlp`
- **网络**：`reqwest`、`axum`
- **MCP**：自定义 JSON-RPC stdio 服务器
- **SDK**：Python `requests`、TypeScript `fetch`、Go `net/http`、Java OkHttp、Ruby `net/http`、`napi-rs`（Node.js 原生）

---

## 开发状态

核心平台功能已完成。所有后端 API、MCP 工具、前端页面、策略引擎、审计、共享预算、HD 派生和多链支持均已实现。SDK 覆盖 Python、TypeScript、Go、Java、Ruby。

---

## 许可证

MIT（或由仓库所有者另行指定）

---

[English](README.md) | 简体中文
