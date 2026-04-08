# Gradience Wallet – 5-Minute Demo Script

> 场景：Hackathon Demo / 投资人 Pitch（2026-04-08 版本，支持 EVM + Solana）

---

## 1. Hook（30s）

**[讲者]**
"大多数钱包还在让用户背助记词、下载插件、手动切链。Gradience 的核心假设是：钱包应该像解锁手机一样简单，并且一个钱包同时管 EVM 和 Solana。"

**[动作]**
打开 Web Dashboard（`http://localhost:3000/dashboard`）。

---

## 2. 创建钱包（30s）

**[讲者]**
"我们不用助记词，直接用 Passkey 创建确定性钱包。"

**[动作]**
- 输入钱包名称，点击 **Create Wallet**。
- 系统弹出 Passkey 注册/验证（Face ID / Touch ID / YubiKey）。
- 钱包创建成功后，展开 WalletCard。

**[展示点]**
- 地址列表里同时出现 `eip155:8453`（Base）、`solana:103`（Solana devnet）和 `ton:0`（TON testnet）三个地址。
- 说明：同一个 Passkey，同一套派生参数，跨 EVM / Solana / TON deterministic 生成。

---

## 3. 查看 Solana 余额（30s）

**[讲者]**
"现在我们看这个 Solana 地址的实时余额。"

**[动作]**
- 在 WalletCard 的 Balances 区域，可以看到 `solana:103` 卡片显示 `0.01 SOL`（提前用 `solana transfer` 空投 0.01 SOL 到该地址）。
- 如果是刚创建的钱包，也可以现场执行 CLI 空投后再刷新页面：
  ```bash
  solana transfer <wallet_solana_address> 0.01 --allow-unfunded-recipient
  ```

**[展示点]**
- Dashboard 直接调用后端 API 查询 Solana RPC，余额以 SOL（非 lamports）展示。
- Telegram Mini App 也能看到同样的余额和地址。

---

## 4. Solana 转账 – Fund（1min）

**[讲者]**
"接下来我们在 Web 上直接给另一个地址发 Solana，全程不用私钥文件、不用 Phantom 插件。"

**[动作]**
- 点击 WalletCard 上的 **Fund**。
- 在 Chain 下拉框选择 **Solana**。
- 输入目标地址（可以是自己另一个 devnet 钱包地址，也可以设为空转回自己）。
- 输入金额 `0.005`，点击 **Send**。
- 系统会再次要求 Passkey 解锁（或已通过 session passphrase 解锁），随后签名并广播。

**[展示点]**
- 几秒后弹出提示：`Funded! Tx: 5jTT2zYN...`
- 刷新 Balances，SOL 余额减少 `0.005` + 手续费。
- 解释：底层调用 `ows_lib::sign_and_send`，Solana 交易由 OWS Core 本地签名，raw bytes 通过 `sendTransaction` 直接上链 devnet。

---

## 4.5 TON 转账（30s）

**[讲者]**
"同样的逻辑也跑在 TON 上。我们选一个 TON 地址，直接发 testnet TON。"

**[动作]**
- 在 WalletCard 上点击 **Fund**。
- Chain 选择 **TON**。
- To 地址留空（默认转回自己），或填另一个 TON testnet 地址；金额填 `0.001`。
- 点击 **Send**。

**[展示点]**
- 提示 `Funded! Tx: ton:0x...`。
- 余额从 `0 TON` 变为扣除 `0.001` + 手续费后的值。
- 解释：TON 是基于 WalletV4R2 合约的地址，`sign_transaction` 里构造了 BoC 外部消息，`broadcast` 调用 toncenter `/sendBoc` 上链 testnet。

---

## 5. Solana Swap – Jupiter DEX（1min）

**[讲者]**
"不仅是转账，Solana 上的 DEX Swap 也集成好了。"

**[动作]**
- 点击 WalletCard 上的 **Swap**。
- Chain 选择 **Solana**。
- From token 默认是 `SOL`，To token 默认是 USDC mint（`EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`）。
- 输入金额 `0.001`，点击 **Swap**。

**[展示点]**
- 后端调用 Jupiter v6 API：`/quote` -> `/swap`，返回 base64 `swapTransaction`。
- OWS Core 将 unsigned bytes 签名后 broadcast 到 Solana devnet。
- 提示 `Swapped! Tx: <signature>`。
- *备注*：如果现场 Jupiter API 因网络 SSL 问题不可用，可提前录屏或在 slides 里放成功截图，并说明代码路径已打通。

---

## 6. MCP / AI Agent 控制（1min）

**[讲者]**
"这不仅仅是 Web UI。我们把所有链上能力暴露成了 Model Context Protocol（MCP）工具，AI Agent 可以直接调用钱包。"

**[动作]**
- 打开终端，启动 MCP Inspector 或直接在 Cursor/Claude 里加载 `gradience-mcp`。
- 示例 Prompt：
  ```
  Check my Solana balance for wallet <wallet_id>,
  then transfer 0.001 SOL to <address>.
  ```
- Agent 会依次调用 `get_balance` -> `sign_and_send`（Solana 分支）。

**[展示点]**
- MCP Tools 列表里包含：
  - `get_balance`
  - `sign_transaction`
  - `sign_and_send`
  - `transfer_spl_token`
  - `delegate_stake`
  - `deactivate_stake`
- 强调：AI 不需要私钥，只需要通过 policy engine 的审批就能执行操作。

---

## 7. Policy + Audit 防线（30s）

**[讲者]**
"关键动作都会经过策略引擎和审计日志。"

**[动作]**
- 打开 `/policies` 页面，展示一条示例规则：
  - "单笔转账超过 0.01 SOL 需要审批"
- 尝试再发一次 `0.02 SOL`，系统弹出 `Swap denied` 或进入 Pending Approvals。
- 打开 `/approvals` 页面，批准或拒绝该请求。
- 回到 WalletCard，查看 Recent Transactions，所有操作（允许/拒绝）都有链上 tx hash 或决策记录。

**[展示点]**
- 企业级安全：允许 + 审计 = 合规；拒绝 + 审批流 = 风险可控。

---

## 8. Closing（30s）

**[讲者]**
"Gradience 把助记词时代终结了：一个 Passkey，一个钱包，跨 EVM、Solana 和 TON，能做人机转账、DEX Swap，也能被 AI Agent 安全调用。"

**[收尾动作]**
- 展示 Pitch Deck 最后一页：Roadmap（T00–T17）。
- 留下 GitHub 和 Telegram Mini App 二维码。

---

## 备用 CLI 指令（Demo 前准备）

```bash
# 1. 启动本地环境
./start-local.sh

# 2. 创建钱包（CLI 备用）
gradience agent create --name demo-wallet

# 3. 查看 Solana 地址
gradience agent list-addresses --name demo-wallet

# 4. 给该地址空投 SOL
solana transfer <address> 0.01 --allow-unfunded-recipient

# 5. 确认余额
gradience agent balance --chain solana --name demo-wallet
gradience agent balance --chain ton --name demo-wallet
gradience agent balance --chain conflux --name demo-wallet

# 6. 转账/换币（CLI 备用）
gradience agent fund --chain solana --to <address> --amount 0.005 --name demo-wallet
gradience agent fund --chain ton --to <address> --amount 0.001 --name demo-wallet
gradience agent fund --chain conflux --to <address> --amount 0.001 --name demo-wallet
gradience dex swap --chain solana --from SOL --to USDC --amount 0.001 --name demo-wallet
```
