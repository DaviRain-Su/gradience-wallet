# Gradience Wallet：Passkey / FaceID 替代 Passphrase 技术方案

## 1. 目标
用户在 iOS / Android 原生应用中使用 FaceID / TouchID / 指纹直接解锁 OWS Vault，无需手动输入 passphrase。Web 端保留现有 passphrase 输入作为 fallback。

## 2. 技术栈选型：Next.js PWA + Capacitor

| 方案 | 优点 | 缺点 | 结论 |
|------|------|------|------|
| **PWA + Capacitor** | 现有 Next.js 代码零重写；直接调用 iOS Keychain / Android Keystore | 需要维护 iOS/Android 工程 | **✅ 采用** |
| Tauri | 桌面端成熟 | 移动端仍处早期，插件生态弱 | ❌ 排除 |
| React Native | 原生体验最好 | 需重写所有 UI 页面 | ❌ 排除 |

**为什么选择 Capacitor**：它把 `web/` 目录的 Next.js build 产物直接包进原生壳，前端代码（React hooks、API 调用、页面路由）完全复用。只需增加一个轻量的原生插件来打通 FaceID ↔ Keychain。

---

## 3. 安全架构：Master Key 模型

当前 OWS Vault 用用户 passphrase 加密私钥。要接入 FaceID，必须引入一个**中间层**：

```
┌─────────────────────────────────────────────────────────────┐
│  User (FaceID / TouchID / 指纹)                              │
└────────────┬────────────────────────────────────────────────┘
             │  生物识别成功
             ▼
┌────────────────────────┐
│  iOS Keychain          │   ← 设备绑定，操作系统级保护
│  Android Keystore      │
│  (存储 32-byte Master  │
│   Key)                 │
└────────────┬───────────┘
             │ 释放 Master Key
             ▼
┌────────────────────────┐
│  App 用 Master Key     │   ← 对 OWS 透明
│  作为 passphrase       │
│  解密 Vault            │
└────────────────────────┘
```

**Master Key 的生命周期**：
1. **生成**：首次设置 passphrase 时，由 `crypto.getRandomValues` 生成 32 字节随机数，base64/hex 编码。
2. **存储**：立刻交给自定义 Capacitor 插件 `SecureVault`，写入 Keychain / Keystore，并标记 **biometricRequired**。
3. **读取**：App 启动/刷新时调用 `SecureVault.retrieveKey()`，系统自动弹出生物识别，成功则返回 Master Key。
4. **Fallback**：生物识别失败、换设备、或用户在 Web 浏览器访问时，走原有的 passphrase 输入流。

---

## 4. 自定义原生插件：SecureVault

我们将手写一个极简 Capacitor 插件，前端接口如下：

```typescript
interface SecureVaultPlugin {
  isAvailable(): Promise<{ value: boolean }>;
  storeKey(options: { key: string }): Promise<void>;   // key: base64
  retrieveKey(): Promise<{ key: string }>;
  deleteKey(): Promise<void>;
}
```

### iOS (Swift) 实现要点
- 使用 `SecItemAdd` / `SecItemCopyMatching` 写入 Keychain。
- `kSecAccessControl` 设为 `kSecAccessControlBiometryCurrentSet`（或 `BiometryAny`，宽松一些）。
- `retrieveKey()` 触发时，系统会自动弹出 FaceID/TouchID 授权弹窗。

### Android (Kotlin) 实现要点
- 使用 `AndroidKeyStore` 生成受生物识别保护的 AES 密钥。
- `KeyGenParameterSpec.Builder` 设置：
  ```kotlin
  setUserAuthenticationRequired(true)
  setInvalidatedByBiometricEnrollment(true)
  ```
- 用该密钥加密 Master Key，密文存到 EncryptedSharedPreferences。
- `retrieveKey()` 时先调 `BiometricPrompt`，成功后再用 `Cipher` 解密密文。

---

## 5. 前端解锁流程改动

`web/app/dashboard/page.tsx` 的 unlock 逻辑需要适配：

```typescript
async function autoUnlock() {
  if (Capacitor.isNativePlatform()) {
    try {
      const { key } = await SecureVault.retrieveKey();
      await apiPost("/api/auth/unlock", { passphrase: key });
      setNeedsPassphrase(false);
      return;
    } catch (e) {
      // 生物识别失败，继续 fallback
    }
  }
  // Web 端 或 原生失败 → 弹出原有 passphrase modal
  setNeedsPassphrase(true);
}
```

**首次启用 FaceID 的引导**：
- 用户在 Dashboard 点击 "Bind Passkey / FaceID"（复用你已有的按钮位置）
- 弹出说明：是否启用 FaceID 解锁？
- 用户确认后：
  1. 输入原有 passphrase → 验证通过
  2. 生成 Master Key → 用其重新 unlock vault（对 OWS 而言就是把 Master Key 当 passphrase 用）
  3. 调用 `SecureVault.storeKey(masterKey)`
  4. **显示 Recovery Phrase（12 词助记词）** → 要求用户抄写保存
  5. 完成。此后该设备只需 FaceID。

---

## 6. Recovery & 跨设备

**核心风险**：Keychain/Keystore 是**设备绑定**的。换手机、重装 App、恢复出厂设置后 Master Key 会丢失。

因此必须提供恢复机制：

| 场景 | 恢复方式 |
|------|----------|
| 同一设备，生物识别失效 | Fallback 到原 passphrase 输入 |
| 新设备 / 重装 App | **Recovery Phrase**（助记词）→ 重建 Master Key |
| 助记词也丢了 | 只能重新初始化 Vault（数据清空） |

**Recovery Phrase 生成**：
- 用 BIP39 把 Master Key 编码为 12 个英文助记词。
- 用户抄写后，新设备上输入助记词即可还原 Master Key，再存入新设备的 Keychain。

> 如果不做助记词，也可以简单 fallback 到原有 passphrase。但 passphrase 本身不具备跨设备恢复能力（因为它只存在用户脑子里，Vault 数据如果本地丢了也无法恢复）。对于钱包产品，**助记词是行业标准**，强烈建议加上。

---

## 7. 实施阶段（Roadmap）

### 阶段 1：PWA 基础（1-2 天）
- 添加 `manifest.json`、Service Worker、`apple-touch-icon`、`theme-color`
- `next.config.mjs` 配置 `output: 'export'`（如果需要把静态产物给 Capacitor）
- 验证 Web 可"添加到主屏幕"

### 阶段 2：Capacitor 工程初始化（2-3 天）
- 安装 `@capacitor/core`、`@capacitor/cli`、`@capacitor/ios`、`@capacitor/android`
- 初始化 `capacitor.config.ts`，`webDir` 指向 Next.js `dist` 或 `out`
- 运行 `npx cap add ios` 和 `npx cap add android`
- 验证现有页面在模拟器中正常渲染

### 阶段 3：SecureVault 原生插件（4-6 天）
- 开发 iOS Swift 端（Keychain + FaceID/TouchID）
- 开发 Android Kotlin 端（Keystore + BiometricPrompt）
- 前端 Typescript 定义 + 桥接测试
- **关键：处理生物识别变更/失效的边界情况**

### 阶段 4：前端解锁逻辑整合（3-5 天）
- 修改 Dashboard unlock flow（`page.tsx`）
- 首次启用 FaceID 的引导 UI
- Recovery Phrase 生成、显示、验证 UI
- Web fallback 保证浏览器用户不受影响

### 阶段 5：测试与分发（3-5 天）
- iOS Simulator / 真机测试
- Android Emulator / 真机测试
- 上传 TestFlight（iOS）和 Google Play Console 内测（Android）

**总预估：2-3 周（全职一个人）**

---

## 8. 风险与限制

1. **设备绑定**：FaceID/TouchID 不跨设备。必须教育用户保存 Recovery Phrase。
2. **生物识别变更**：
   - iOS 上如果用户**删除并重新录入** FaceID/TouchID，某些 Keychain 配置会导致旧的 Keychain item 失效。
   - 建议用 `kSecAccessControlBiometryAny` 而不是 `CurrentSet`，避免因指纹变更导致锁定。
3. **Web 用户割裂**：原生 App 用户不再需要 passphrase，但 Web 浏览器用户仍然需要。两套 UX 需要清晰区分。
4. **OWS 兼容性**：需要确认 OWS `init_vault` / `unlock` 的 passphrase 参数是否支持**任意长字符串**（Master Key hex 可能是 64 字符）。如果不支持，需要调整 OWS 接口或做一层 wrapping。

---

## 9. 平台覆盖策略

| 平台 | 交付形态 | 解锁方式 |
|------|----------|----------|
| iOS | Capacitor App（App Store / TestFlight） | FaceID / TouchID + Keychain |
| Android | Capacitor App（Google Play） | 指纹 / Face unlock + Keystore |
| Web（桌面/手机） | PWA / 浏览器 | 保留现有 passphrase 输入 |
| 桌面端（macOS/Windows） | 当前 Web 版 | 保留现有 passphrase 输入（未来可接 Tauri） |

---

## 10. 待确认清单

- [ ] OWS vault 的 passphrase 是否允许 64 位 hex 字符串？（待测试验证）
- [x] Recovery Phrase 采用 **12 词** BIP39 助记词（用户确认）
- [x] Capacitor App 采用 **严格原生发版**（用户确认）
- [x] 两平台 **iOS + Android 同时推进**（用户确认）
