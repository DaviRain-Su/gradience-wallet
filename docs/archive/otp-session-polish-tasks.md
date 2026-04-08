# OTP Login & Session Persistence — Polish Tasks

> 当前邮箱验证码（OTP）登录流程与 Session 持久化已完成并部署。以下是需要进一步改进完善的任务清单，按优先级排列。

---

## P0 — 影响核心可用性与安全

### 1. Resend 域名验证
- **问题**：Resend 免费/ Starter 账户向未验证域名发送邮件时，邮件可能被拦截或进入垃圾箱。
- **任务**：
  1. 登录 [resend.com/domains](https://resend.com/domains)；
  2. 添加域名 `gradiences.xyz`；
  3. 按提示在 DNS（如 Cloudflare）添加 TXT 记录并验证；
  4. 获取并配置 `RESEND_FROM_EMAIL`（如 `noreply@gradiences.xyz`）。
- **交付**：任意邮箱均可稳定接收 Gradience OTP 邮件。

### 2. Vault Passphrase 强制设置引导
- **问题**：当前邮箱验证码登录后直接跳 Dashboard，但 `session.passphrase` 为 `None`，OWS 本地 vault 未初始化。用户在 Dashboard 创建 wallet 或签名时底层可能报错。
- **任务**：
  1. 登录成功后，后端返回 `passphrase_exists` 标志；
  2. 前端 Dashboard 初始化时检测该标志；
  3. 若 `false`，弹出不可关闭的模态框：
     - 输入 passphrase（≥12 字符）
     - 确认 passphrase
     - 调用 `/api/auth/unlock` 初始化 vault
  4. 成功后关闭弹窗，正常进入 Dashboard。
- **交付**：新用户首次登录后必须完成 passphrase 设置才能继续使用。

### 3. API CORS 生产环境收紧
- **问题**：`tower_http::cors::CorsLayer::new().allow_origin(Any)` 允许任意跨域来源，存在 CSRF 与资产滥用风险。
- **任务**：
  1. 修改 `crates/gradience-api/src/main.rs`；
  2. 将 `allow_origin(Any)` 替换为只允许 `ORIGIN` 环境变量中的域名（默认 `https://wallets.gradiences.xyz`）；
  3. 保留开发环境的 `http://localhost:3000` 回退。
- **交付**：生产 API 仅接受前端域名请求。

---

## P1 — 体验优化

### 4. Logout（登出）功能
- **问题**：用户无法主动登出或切换账号，只能手动清浏览器缓存。
- **任务**：
  1. 后端新增 `POST /api/auth/logout`：
     - 从 `Authorization` header 读取 token；
     - 调用 `delete_session` 删除数据库中的 session；
  2. 前端 Dashboard 右上角增加用户菜单：
     - 显示当前用户邮箱/用户名
     - 点击 "Log out" 调用 `/api/auth/logout` → 清 localStorage → 跳转 `/login`。
- **交付**：用户可以一键安全登出。

### 5. 验证码发送后端限流
- **问题**：`POST /api/auth/email/send-code` 前端有 60s 倒计时，但后端没有全局/IP/邮箱维度的限流，存在被刷邮件的成本风险。
- **任务**：
  1. 在 `email_send_code` handler 中加入限流逻辑（无需 Redis，直接存 SQLite）：
     - 新增 `email_rate_limits` 表 `(email TEXT PRIMARY KEY, last_sent DATETIME, count_1h INTEGER)`
     - 同一邮箱 60 秒内禁止重发；
     - 同一邮箱 1 小时内最多发送 5 次；
     - 超出限制返回 `429 Too Many Requests`。
  2. 前端显示友好的 "发送过于频繁，请稍后再试" 提示。
- **交付**：有效防止邮件接口被刷。

### 6. 移除 Dashboard API Base 调试 Banner
- **问题**：Dashboard 页面顶部显示 "Connected to: ..." 的 API Base 地址，对终端用户无意义。
- **任务**：
  1. 在 `web/app/dashboard/page.tsx` 中移除/隐藏该调试横幅；
  2. 仅在 dev 模式或本地运行时才显示（可选）。
- **交付**：Dashboard 界面更干净。

---

## P2 — 锦上添花

### 7. Passkey 绑定引导（Dashboard 设置页）
- **问题**：用户希望保留 Passkey 用于保护 OWS vault value，但目前只有邮箱 OTP 登录，没有后续绑定 Passkey 的入口。
- **任务**：
  1. Dashboard 增加 "安全设置" 页面；
  2. 已登录用户可点击 "绑定 Passkey"；
  3. 调用现有 Passkey 注册流程，将 Passkey 凭证关联到当前用户；
  4. 下次解锁 vault 时可选使用 Passkey 验证。
- **交付**：高安全需求用户可无缝升级至 Passkey 保护。

### 8. 品牌化 404 / 错误页面
- **问题**：访问不存在路由时显示 Next.js 默认 404 页面，与品牌不一致。
- **任务**：
  1. 新建 `web/app/not-found.tsx`；
  2. 使用 Gradience 配色与导航，提供 "返回 Dashboard" / "返回 Home" 按钮。
- **交付**：全站错误页统一品牌风格。

---

## 执行顺序建议

**第一批次（MVP 锁死可用）**：任务 1 → 任务 2 → 任务 3  
**第二批次（体验补全）**：任务 4 → 任务 5 → 任务 6  
**第三批次（增值功能）**：任务 7 → 任务 8  
