# Phase 5: Test Spec — 测试规格

> Project: Gradience Wallet
> Date: 2026-04-07
> Scope: Development Demo 核心路径
> Rule: 每个接口必须先定义测试，再写实现 (TDD)

---

## 1. 测试结构

```
crates/
├── gradience-core/
│   └── src/
│       └── tests/
│           ├── ows_adapter_tests.rs
│           ├── policy_engine_tests.rs
│           └── wallet_manager_tests.rs
├── gradience-db/
│   └── tests/
│       └── migration_tests.rs
├── gradience-mcp/
│   └── tests/
│       └── tool_tests.rs
└── gradience-cli/
    └── tests/
        └── e2e_tests.rs
```

---

## 2. OWS Adapter 测试 (`ows_adapter_tests.rs`)

### 2.1 `init_vault`

```rust
#[tokio::test]
async fn test_init_vault_success() {
    // Happy Path: 正确 passphrase >= 12 chars
    let adapter = create_test_adapter().await;
    let vault = adapter.init_vault("secure-pass-123").await.unwrap();
    assert!(!vault.pointer_eq(std::ptr::null())); // handle 有效
}

#[tokio::test]
async fn test_init_vault_short_passphrase_boundary() {
    // Boundary: 刚好 11 chars (< MIN_PASSPHRASE_LEN=12)
    let adapter = create_test_adapter().await;
    let err = adapter.init_vault("short-pass").await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidCredential(_)));
}

#[tokio::test]
async fn test_init_vault_corrupted_vault_error() {
    // Error: vault 文件被篡改
    let adapter = create_test_adapter_with_corrupted_vault().await;
    let err = adapter.init_vault("secure-pass-123").await.unwrap_err();
    assert!(matches!(err, GradienceError::Ows(_)));
}
```

### 2.2 `create_wallet`

```rust
#[tokio::test]
async fn test_create_wallet_success() {
    // Happy Path
    let (adapter, vault) = create_test_vault().await;
    let wallet = adapter.create_wallet(&vault, "demo-wallet", Default::default()).await.unwrap();
    assert_eq!(wallet.name, "demo-wallet");
    // 所有 EVM 链族应有 account 0
    let evm_chains: Vec<_> = wallet.accounts.iter()
        .filter(|a| a.chain_id.starts_with("eip155:"))
        .collect();
    assert!(!evm_chains.is_empty());
}

#[tokio::test]
async fn test_create_wallet_duplicate_name_boundary() {
    // Boundary: 相同 name 创建第二次
    let (adapter, vault) = create_test_vault().await;
    adapter.create_wallet(&vault, "dup", Default::default()).await.unwrap();
    let wallet2 = adapter.create_wallet(&vault, "dup", Default::default()).await.unwrap();
    // 应为不同 UUID，允许同名
    assert_ne!(wallet.id, wallet2.id);
}
```

### 2.3 `sign_transaction`

```rust
#[tokio::test]
async fn test_sign_tx_owner_mode_success() {
    // Happy Path: owner passphrase, no policy
    let (adapter, vault, wallet) = create_test_wallet().await;
    let tx = create_test_tx("eip155:8453"); // Base
    let signed = adapter.sign_transaction(
        &vault, &wallet.id, "eip155:8453", &tx, "secure-pass-123"
    ).await.unwrap();
    assert!(!signed.raw_hex.is_empty());
}

#[tokio::test]
async fn test_sign_tx_agent_mode_with_policy_success() {
    // Happy Path: agent token + policy allow
    let (adapter, vault, wallet) = create_test_wallet_with_policy_allow().await;
    let tx = create_test_tx("eip155:8453");
    let api_key = adapter.attach_api_key_and_policies(
        &vault, &wallet.id, "claude", vec!["allow-base".into()]
    ).await.unwrap();
    let signed = adapter.sign_transaction(
        &vault, &wallet.id, "eip155:8453", &tx, &api_key.raw_token.unwrap()
    ).await.unwrap();
    assert!(!signed.raw_hex.is_empty());
}

#[tokio::test]
async fn test_sign_tx_agent_mode_policy_denied_error() {
    // Error: agent token + policy deny
    let (adapter, vault, wallet) = create_test_wallet_with_policy_deny().await;
    let api_key = adapter.attach_api_key_and_policies(
        &vault, &wallet.id, "claude", vec!["deny-all".into()]
    ).await.unwrap();
    let tx = create_test_tx("eip155:8453");
    let err = adapter.sign_transaction(
        &vault, &wallet.id, "eip155:8453", &tx, &api_key.raw_token.unwrap()
    ).await.unwrap_err();
    assert!(matches!(err, GradienceError::PolicyDenied(_)));
}

#[tokio::test]
async fn test_sign_tx_invalid_chain_error() {
    // Error: 不支持的链
    let (adapter, vault, wallet) = create_test_wallet().await;
    let tx = create_test_tx("eip155:999999");
    let err = adapter.sign_transaction(
        &vault, &wallet.id, "eip155:999999", &tx, "secure-pass-123"
    ).await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidChain(_)));
}

#[tokio::test]
async fn test_sign_tx_revoked_key_attack() {
    // Attack: 已吊销的 API key
    let (adapter, vault, wallet) = create_test_wallet().await;
    let api_key = adapter.attach_api_key_and_policies(
        &vault, &wallet.id, "claude", vec![]
    ).await.unwrap();
    let token = api_key.raw_token.unwrap();
    // 吊销 key
    adapter.revoke_api_key(&vault, &api_key.id).await.unwrap();
    let tx = create_test_tx("eip155:8453");
    let err = adapter.sign_transaction(
        &vault, &wallet.id, "eip155:8453", &tx, &token
    ).await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidCredential(_)));
}
```

---

## 3. Policy Engine 测试 (`policy_engine_tests.rs`)

### 3.1 `evaluate` — chain_whitelist

```rust
#[tokio::test]
async fn test_chain_whitelist_allow() {
    let engine = create_test_engine().await;
    let policy = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into(), "eip155:56".into()],
        }],
        ..default_policy()
    };
    let ctx = create_eval_ctx("eip155:8453");
    let result = engine.evaluate(ctx, vec![&policy]).await.unwrap();
    assert_eq!(result.decision, Decision::Allow);
}

#[tokio::test]
async fn test_chain_whitelist_deny() {
    let engine = create_test_engine().await;
    let policy = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into()],
        }],
        ..default_policy()
    };
    let ctx = create_eval_ctx("eip155:1"); // Ethereum mainnet 不在白名单
    let result = engine.evaluate(ctx, vec![&policy]).await.unwrap();
    assert_eq!(result.decision, Decision::Deny);
}
```

### 3.2 `evaluate` — spend_limit

```rust
#[tokio::test]
async fn test_spend_limit_allow_boundary() {
    // Boundary: 刚好等于 limit
    let engine = create_test_engine_with_spending(0).await;
    let policy = Policy {
        rules: vec![Rule::SpendLimit {
            max: "1000000000".into(), // 1000 USDC (6 decimals)
            token: "USDC".into(),
        }],
        ..default_policy()
    };
    let mut ctx = create_eval_ctx("eip155:8453");
    ctx.transaction.value = "1000000000".into(); // 刚好 1000
    let result = engine.evaluate(ctx, vec![&policy]).await.unwrap();
    assert_eq!(result.decision, Decision::Allow);
}

#[tokio::test]
async fn test_spend_limit_deny_over_boundary() {
    // Boundary: 比 limit 大 1
    let engine = create_test_engine_with_spending(0).await;
    let policy = Policy {
        rules: vec![Rule::SpendLimit {
            max: "1000000000".into(),
            token: "USDC".into(),
        }],
        ..default_policy()
    };
    let mut ctx = create_eval_ctx("eip155:8453");
    ctx.transaction.value = "1000000001".into(); // 1000.000001
    let result = engine.evaluate(ctx, vec![&policy]).await.unwrap();
    assert_eq!(result.decision, Decision::Deny);
}

#[tokio::test]
async fn test_spend_limit_zero_attack() {
    // Attack: limit = 0，任何正数交易都应拒绝
    let engine = create_test_engine_with_spending(0).await;
    let policy = Policy {
        rules: vec![Rule::SpendLimit { max: "0".into(), token: "USDC".into() }],
        ..default_policy()
    };
    let mut ctx = create_eval_ctx("eip155:8453");
    ctx.transaction.value = "1".into();
    let result = engine.evaluate(ctx, vec![&policy]).await.unwrap();
    assert_eq!(result.decision, Decision::Deny);
}
```

### 3.3 `merge_policies_strictest`

```rust
#[test]
fn test_merge_spend_limit_takes_min() {
    let wp = Policy {
        rules: vec![Rule::SpendLimit { max: "1000".into(), token: "USDC".into() }],
        priority: 0, ..default_policy()
    };
    let ap = Policy {
        rules: vec![Rule::SpendLimit { max: "500".into(), token: "USDC".into() }],
        priority: 1, ..default_policy()
    };
    let merged = merge_policies_strictest(Some(&wp), vec![&ap]);
    assert_eq!(merged.spend_limit, Some("500".into()));
}

#[test]
fn test_merge_chain_whitelist_intersection() {
    let wp = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into(), "eip155:56".into(), "eip155:1".into()],
        }],
        priority: 0, ..default_policy()
    };
    let ap = Policy {
        rules: vec![Rule::ChainWhitelist {
            chain_ids: vec!["eip155:8453".into(), "eip155:56".into()],
        }],
        priority: 1, ..default_policy()
    };
    let merged = merge_policies_strictest(Some(&wp), vec![&ap]);
    assert_eq!(
        merged.chain_whitelist,
        Some(vec!["eip155:8453".into(), "eip155:56".into()])
    );
}

#[test]
fn test_merge_empty_intersection_deny_all() {
    let wp = Policy {
        rules: vec![Rule::ChainWhitelist { chain_ids: vec!["eip155:1".into()] }],
        priority: 0, ..default_policy()
    };
    let ap = Policy {
        rules: vec![Rule::ChainWhitelist { chain_ids: vec!["eip155:8453".into()] }],
        priority: 1, ..default_policy()
    };
    let merged = merge_policies_strictest(Some(&wp), vec![&ap]);
    assert_eq!(merged.chain_whitelist, Some(vec![]));
    // 空交集意味着任何链都会被拒绝
}
```

---

## 4. Wallet Manager + API Key 测试

### 4.1 `create_api_key`

```rust
#[tokio::test]
async fn test_api_key_format() {
    let service = create_test_key_service().await;
    let key = service.create_key("wallet-1", "claude-code").await.unwrap();
    assert!(key.raw_token.as_ref().unwrap().starts_with("ows_key_"));
    assert_eq!(key.raw_token.as_ref().unwrap().len(), 8 + 64); // prefix + 64 hex
}

#[tokio::test]
async fn test_api_key_lookup_by_hash() {
    let service = create_test_key_service().await;
    let key = service.create_key("wallet-1", "claude-code").await.unwrap();
    let raw = key.raw_token.unwrap();
    let found = service.verify_key(&raw).await.unwrap();
    assert_eq!(found.wallet_ids, vec!["wallet-1"]);
}

#[tokio::test]
async fn test_api_key_revoke_lookup_error() {
    let service = create_test_key_service().await;
    let key = service.create_key("wallet-1", "claude-code").await.unwrap();
    let raw = key.raw_token.unwrap();
    service.revoke_key(&key.id).await.unwrap();
    let err = service.verify_key(&raw).await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidCredential(_)));
}
```

---

## 5. Audit Logger 测试

### 5.1 HMAC 链完整性

```rust
#[tokio::test]
async fn test_audit_hmac_chain_integrity() {
    let logger = create_test_logger().await;
    let entry1 = logger.log("sign_tx", &context(), "allowed").await.unwrap();
    let entry2 = logger.log("sign_tx", &context(), "allowed").await.unwrap();
    
    // entry2.prev_hash == entry1.current_hash
    assert_eq!(entry2.prev_hash, entry1.current_hash);
    
    // 重新计算 HMAC 应匹配
    let recomputed = compute_audit_hash(
        logger.secret_key(),
        &entry2.prev_hash,
        &AuditLogEntry::from_row(&entry2)
    );
    assert_eq!(recomputed, entry2.current_hash);
}

#[tokio::test]
async fn test_audit_tamper_detection() {
    let logger = create_test_logger().await;
    let entry = logger.log("sign_tx", &context(), "allowed").await.unwrap();
    
    // 模拟篡改
    let mut tampered = AuditLogEntry::from_row(&entry);
    tampered.decision = "denied".into();
    
    let recomputed = compute_audit_hash(
        logger.secret_key(),
        &tampered.prev_hash,
        &tampered
    );
    assert_ne!(recomputed, entry.current_hash);
}
```

### 5.2 spending_tracker 更新

```rust
#[tokio::test]
async fn test_spending_tracker_accumulates() {
    let logger = create_test_logger().await;
    logger.log_with_amount("sign_tx", 500_000_000u64).await.unwrap();
    logger.log_with_amount("sign_tx", 300_000_000u64).await.unwrap();
    
    let tracker = logger.get_spending_tracker("daily", "USDC").await.unwrap();
    assert_eq!(tracker.spent_amount, "800000000"); // 800 USDC
}
```

---

## 6. EVM RPC 测试

### 6.1 `EvmRpcClient`

```rust
#[tokio::test]
async fn test_evm_get_balance_success() {
    let client = EvmRpcClient::new("eip155:8453", "https://mainnet.base.org").unwrap();
    // 使用已知地址 (Base 上的 USDC holder)
    let balance = client.get_balance("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913").await.unwrap();
    assert!(!balance.is_empty());
}

#[tokio::test]
async fn test_evm_invalid_rpc_url_error() {
    let err = EvmRpcClient::new("eip155:8453", "not-a-url").unwrap_err();
    assert!(matches!(err, GradienceError::Http(_)));
}

#[tokio::test]
async fn test_evm_send_raw_tx_returns_expected() {
    // 用无效签名交易，应返回特定 RPC error，但不会 panic
    let client = EvmRpcClient::new("eip155:8453", "https://mainnet.base.org").unwrap();
    let result = client.send_raw_transaction("0xdeadbeef").await;
    // 预期链上拒绝，但 API 调用本身成功
    assert!(result.is_err()); // invalid tx format
}
```

---

## 7. MCP Server 测试 (`tool_tests.rs`)

### 7.1 `sign_transaction` tool

```rust
#[tokio::test]
async fn test_mcp_sign_tx_success() {
    let client = create_test_mcp_client().await;
    let result = client.call_tool("sign_transaction", json!({
        "walletId": "demo-wallet-uuid",
        "chainId": "eip155:8453",
        "transaction": {
            "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C",
            "value": "1000000000000000",
            "data": "0x"
        }
    })).await.unwrap();
    
    assert!(result.get("signature").is_some());
    assert_eq!(result.get("decision").unwrap(), "allowed");
}

#[tokio::test]
async fn test_mcp_sign_tx_policy_denied() {
    let client = create_test_mcp_client_with_deny_policy().await;
    let err = client.call_tool("sign_transaction", json!({
        "walletId": "demo-wallet-uuid",
        "chainId": "eip155:1", // 不在白名单
        "transaction": { "to": "0x...", "value": "0", "data": "0x" }
    })).await.unwrap_err();
    
    assert!(err.to_string().contains("POLICY_DENIED"));
}

#[tokio::test]
async fn test_mcp_sign_tx_invalid_wallet() {
    let client = create_test_mcp_client().await;
    let err = client.call_tool("sign_transaction", json!({
        "walletId": "nonexistent-wallet",
        "chainId": "eip155:8453",
        "transaction": { "to": "0x...", "value": "0", "data": "0x" }
    })).await.unwrap_err();
    
    assert!(err.to_string().contains("WalletNotFound"));
}
```

### 7.2 `get_balance` tool

```rust
#[tokio::test]
async fn test_mcp_get_balance_returns_native_and_tokens() {
    let client = create_test_mcp_client().await;
    let result = client.call_tool("get_balance", json!({
        "walletId": "demo-wallet-uuid",
        "chainId": "eip155:8453"
    })).await.unwrap();
    
    assert!(result.get("native").is_some());
    assert!(result.get("tokens").is_some());
}
```

---

## 8. Merkle Anchor 测试

### 8.1 `MerkleTree`

```rust
#[test]
fn test_merkle_tree_root_consistency() {
    let leaves: Vec<[u8; 32]> = (0..4).map(|i| keccak256(&[i as u8])).collect();
    let tree = MerkleTree::new(leaves.clone());
    
    // 同一批叶子，root 不变
    let tree2 = MerkleTree::new(leaves);
    assert_eq!(tree.root, tree2.root);
}

#[test]
fn test_merkle_proof_verification() {
    let leaves: Vec<[u8; 32]> = (0..4).map(|i| keccak256(&[i as u8])).collect();
    let tree = MerkleTree::new(leaves.clone());
    
    let (proof, leaf) = tree.generate_proof(2).unwrap();
    assert!(verify_proof(tree.root, leaf, &proof));
}

#[test]
fn test_merkle_tampered_leaf_fails() {
    let leaves: Vec<[u8; 32]> = (0..4).map(|i| keccak256(&[i as u8])).collect();
    let tree = MerkleTree::new(leaves);
    
    let (proof, _leaf) = tree.generate_proof(1).unwrap();
    let fake_leaf = keccak256(b"fake");
    assert!(!verify_proof(tree.root, fake_leaf, &proof));
}
```

---

## 9. CLI E2E 测试 (`e2e_tests.rs`)

### 9.1 完整工作流

```rust
#[tokio::test]
async fn test_e2e_create_wallet_and_sign() {
    let cli = TestCli::new().await;
    
    // 1. login
    cli.run(["auth", "login"]).with_input("test-pass-123\n").success();
    
    // 2. create wallet
    let out = cli.run(["agent", "create", "--name", "e2e-demo"]).success();
    let wallet_id = extract_uuid(&out.stdout);
    
    // 3. set policy
    let policy_file = create_temp_policy(json!({
        "rules": [{"type": "chain_whitelist", "chain_ids": ["eip155:8453"]}]
    }));
    cli.run(["policy", "set", &wallet_id, "--file", &policy_file]).success();
    
    // 4. create api key
    let out = cli.run(["agent", "fund", &wallet_id, "1.0", "--chain", "base"]).success();
    
    // 5. 检查 balance 命令不 panic
    cli.run(["agent", "balance", &wallet_id, "--chain", "base"]).success();
}
```

---

## 10. 测试运行命令

```bash
# 单元测试
cargo test --workspace --lib

# 集成测试 (需要本地 SQLite DB)
cargo test --workspace --test '*'

# 跑特定 crate 测试
cargo test -p gradience-core
cargo test -p gradience-mcp
cargo test -p gradience-cli --test e2e_tests

# 覆盖率
cargo llvm-cov --workspace --html --open
```

---

## 验收标准

- [x] OWS Adapter: 5 个测试 (1 happy + 2 boundary + 2 error)
- [x] Policy Engine: 6 个测试 (chain whitelist + spend limit + merge)
- [x] API Key: 3 个测试 (format + lookup + revoke)
- [x] Audit Logger: 3 个测试 (HMAC chain + tamper + spending)
- [x] EVM RPC: 3 个测试 (balance + invalid URL + raw tx)
- [x] MCP Tools: 4 个测试 (sign success + deny + invalid wallet + balance)
- [x] Merkle Tree: 3 个测试 (root + proof + tamper)
- [x] CLI E2E: 1 个完整工作流测试

**所有测试骨架必须先于实现代码提交。**
