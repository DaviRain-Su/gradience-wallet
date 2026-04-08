use gradience_core::ows::adapter::Transaction;
use gradience_core::policy::engine::{PolicyEngine, EvalContext, Decision, Policy};
use serde_json::json;

fn block_on_async<F, T>(f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    match tokio::runtime::Handle::try_current() {
        Ok(handle) if handle.metrics().num_workers() > 1 => {
            tokio::task::block_in_place(|| handle.block_on(f))
        }
        _ => {
            tokio::runtime::Runtime::new().expect("create tokio runtime").block_on(f)
        }
    }
}

fn get_vault_config() -> anyhow::Result<(String, std::path::PathBuf)> {
    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience")
        });
    let session_path = data_dir.join(".session");
    let passphrase = std::fs::read_to_string(&session_path)?;
    let passphrase = passphrase.trim().to_string();
    if passphrase.len() < 12 {
        anyhow::bail!("passphrase too short");
    }
    Ok((passphrase, data_dir.join("vault")))
}

fn build_unsigned_tx(
    nonce: u64,
    gas_price: u128,
    to: &str,
    value: u128,
    data: &[u8],
    chain_id: u64,
) -> String {
    let to_bytes = hex::decode(to.trim_start_matches("0x")).unwrap_or_default();
    let mut rlp = rlp::RlpStream::new_list(9);
    rlp.append(&nonce);
    rlp.append(&gas_price);
    rlp.append(&21000u64);
    rlp.append(&to_bytes);
    rlp.append(&value);
    rlp.append(&data);
    rlp.append(&chain_id);
    rlp.append(&0u8);
    rlp.append(&0u8);
    format!("0x{}", hex::encode(&rlp.out()))
}

fn resolve_rpc(chain_id: &str) -> &str {
    gradience_core::chain::resolve_rpc(chain_id)
}

pub fn handle_sign_transaction(args: crate::args::SignTxArgs) -> anyhow::Result<serde_json::Value> {
    let wallet_id = &args.wallet_id;
    let chain_id = &args.chain_id;
    let to = &args.transaction.to;
    let value = &args.transaction.value;
    let data_hex = args.transaction.data.as_deref().unwrap_or("0x");
    let data = hex::decode(data_hex.trim_start_matches("0x")).unwrap_or_default();

    let tx = Transaction {
        to: Some(to.into()),
        value: value.into(),
        data: data.clone(),
        raw_hex: data_hex.into(),
    };

    let is_solana = chain_id.starts_with("solana:");

    let (policies, nonce, gas_price) = block_on_async(async {
        let data_dir = std::env::var("GRADIENCE_DATA_DIR")
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
        let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);
        let db = match sqlx::SqlitePool::connect(&db_path).await {
            Ok(db) => db,
            Err(_) => return anyhow::Result::<_>::Err(anyhow::anyhow!("db connect failed")),
        };
        let db_policies = gradience_db::queries::list_active_policies_by_wallet(&db, wallet_id).await.unwrap_or_default();
        let policies: Vec<gradience_core::policy::engine::Policy> = db_policies.iter()
            .filter_map(|p| gradience_core::policy::engine::Policy::try_from_db(p).ok())
            .collect();

        let wm = gradience_core::wallet::service::WalletManagerService::new();
        if let Err(e) = wm.require_status_active(&db, wallet_id).await {
            return anyhow::Result::<_>::Err(anyhow::anyhow!("wallet status check failed: {}", e));
        }

        if is_solana {
            return anyhow::Result::Ok((policies, 0u64, 0u128));
        }

        let rpc_url = resolve_rpc(chain_id);
        let addrs = gradience_db::queries::list_wallet_addresses(&db, wallet_id).await.unwrap_or_default();
        let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
            .map_err(|e| anyhow::anyhow!("rpc client: {}", e))?;
        let mut from_addr = None;
        for a in &addrs {
            if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
                from_addr = Some(a.address.clone());
                break;
            }
        }
        let addr = from_addr.unwrap_or_default();
        let nonce = client.get_transaction_count(&addr).await
            .map_err(|e| anyhow::anyhow!("nonce fetch failed: {}", e))?;
        let gp_hex = client.get_gas_price().await
            .map_err(|e| anyhow::anyhow!("gas price fetch failed: {}", e))?;
        let gp = u128::from_str_radix(gp_hex.trim_start_matches("0x"), 16)
            .map_err(|e| anyhow::anyhow!("bad gas price: {}", e))?;
        anyhow::Result::Ok((policies, nonce, gp))
    })?;

    let parser = gradience_core::policy::intent::IntentParser::new();
    let intent = parser.parse(&tx, chain_id).ok();

    let engine = PolicyEngine;
    let ctx = EvalContext {
        wallet_id: wallet_id.into(),
        api_key_id: "mcp-key".into(),
        chain_id: chain_id.into(),
        transaction: tx,
        intent,
        timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        dynamic_signals: None,
        max_tokens: None,
        model: None,
    };

    let policy_refs: Vec<&Policy> = policies.iter().collect();
    let result = engine.evaluate(ctx, policy_refs)?;

    match result.decision {
        Decision::Allow => {
            let (passphrase, vault_dir) = get_vault_config()?;

            let tx_hex = if is_solana {
                let tx_bytes: Vec<u8> = block_on_async(async {
                    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
                        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
                    let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);
                    let db = sqlx::SqlitePool::connect(&db_path).await.ok()
                        .ok_or_else(|| anyhow::anyhow!("db connect failed"))?;
                    let addrs = gradience_db::queries::list_wallet_addresses(&db, wallet_id).await.ok()
                        .ok_or_else(|| anyhow::anyhow!("no addresses"))?;
                    let from = addrs.iter()
                        .find(|a| a.chain_id.starts_with("solana:"))
                        .map(|a| a.address.clone())
                        .ok_or_else(|| anyhow::anyhow!("solana address not found"))?;
                    let rpc_url = resolve_rpc(chain_id);
                    let client = gradience_core::rpc::solana::SolanaRpcClient::new(rpc_url);
                    let blockhash = client.get_latest_blockhash().await
                        .map_err(|e| anyhow::anyhow!("blockhash fetch failed: {}", e))?;
                    let lamports: u64 = value.parse().unwrap_or(0);
                    gradience_core::ows::signing::build_solana_transfer_tx(&from, to, lamports, &blockhash)
                        .map_err(|e| anyhow::anyhow!("build solana tx failed: {}", e))
                })?;
                format!("0x{}", hex::encode(&tx_bytes))
            } else {
                let chain_num = gradience_core::chain::evm_chain_num(chain_id);
                let value_raw: u128 = value.parse().unwrap_or(0);
                build_unsigned_tx(nonce, gas_price, to, value_raw, &data, chain_num)
            };

            let sign_result = ows_lib::sign_transaction(
                wallet_id,
                chain_id,
                &tx_hex,
                Some(&passphrase),
                None,
                Some(&vault_dir),
            ).map_err(|e| anyhow::anyhow!("ows sign failed: {}", e))?;

            Ok(json!({
                "signature": format!("0x{}", sign_result.signature),
                "decision": "allowed",
                "chainId": chain_id,
            }))
        }
        Decision::Deny => {
            Err(anyhow::anyhow!("POLICY_DENIED: {}", result.reasons.join(", ")))
        }
        Decision::Warn => {
            Ok(json!({
                "signature": null,
                "decision": "warned",
                "reasons": result.reasons,
            }))
        }
    }
}

pub fn handle_get_balance(args: crate::args::GetBalanceArgs) -> anyhow::Result<serde_json::Value> {
    let wallet_id = &args.wallet_id;
    let chain_id = args.chain_id.as_deref().unwrap_or("eip155:8453");

    let rpc_url = resolve_rpc(chain_id);
    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
    let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);

    let balance = block_on_async(async {
        let db = sqlx::SqlitePool::connect(&db_path).await.ok()?;
        let addrs = gradience_db::queries::list_wallet_addresses(&db, wallet_id).await.ok()?;

        if chain_id.starts_with("solana:") {
            let client = gradience_core::rpc::solana::SolanaRpcClient::new(rpc_url);
            for a in addrs {
                if a.chain_id.starts_with("solana:") {
                    let lamports = client.get_balance(&a.address).await.ok()?;
                    return Some(gradience_core::rpc::solana::lamports_to_sol(lamports));
                }
            }
            return None;
        }

        if chain_id.starts_with("stellar:") {
            let client = gradience_core::rpc::stellar::StellarHorizonClient::new(rpc_url);
            for a in addrs {
                if a.chain_id.starts_with("stellar:") {
                    let stroops = client.get_balance(&a.address).await.ok()?;
                    return Some(gradience_core::rpc::stellar::stroops_to_xlm(stroops));
                }
            }
            return None;
        }

        let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url).ok()?;
        for a in addrs {
            if a.chain_id.starts_with("eip155:") {
                return client.get_balance(&a.address).await.ok();
            }
        }
        None
    });

    let (symbol, decimals) = if chain_id.starts_with("solana:") {
        ("SOL", 9)
    } else if chain_id.starts_with("stellar:") {
        ("XLM", 7)
    } else {
        ("ETH", 18)
    };

    Ok(json!({
        "walletId": wallet_id,
        "chainId": chain_id,
        "native": {
            "symbol": symbol,
            "balance": balance.unwrap_or_else(|| "0".into()),
            "decimals": decimals,
        },
        "tokens": []
    }))
}

pub fn handle_swap(args: crate::args::SwapArgs) -> anyhow::Result<serde_json::Value> {
    let wallet_id = &args.wallet_id;
    let from_token = &args.from;
    let to_token = &args.to;
    let amount = &args.amount;
    let chain = args.chain.as_deref().unwrap_or("base");
    let chain_num = gradience_core::chain::evm_chain_num(chain);
    let chain_id_str = format!("eip155:{}", chain_num);

    let (passphrase, vault_dir) = get_vault_config()?;
    let rpc_url = resolve_rpc(&chain_id_str);

    let result = block_on_async(async {
        let data_dir = std::env::var("GRADIENCE_DATA_DIR")
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
        let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);
        let db = match sqlx::SqlitePool::connect(&db_path).await {
            Ok(db) => db,
            Err(_) => return anyhow::Result::<_>::Err(anyhow::anyhow!("db connect failed")),
        };

        let wm = gradience_core::wallet::service::WalletManagerService::new();
        if let Err(e) = wm.require_status_active(&db, wallet_id).await {
            return anyhow::Result::<_>::Err(anyhow::anyhow!("wallet status check failed: {}", e));
        }

        let addrs = gradience_db::queries::list_wallet_addresses(&db, wallet_id).await.unwrap_or_default();
        let mut from_addr = None;
        for a in &addrs {
            if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
                from_addr = Some(a.address.clone());
                break;
            }
        }
        let addr = from_addr.ok_or_else(|| anyhow::anyhow!("wallet address not found"))?;

        let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
            .map_err(|e| anyhow::anyhow!("rpc client: {}", e))?;
        let nonce = client.get_transaction_count(&addr).await
            .map_err(|e| anyhow::anyhow!("nonce fetch failed: {}", e))?;
        let gp_hex = client.get_gas_price().await
            .map_err(|e| anyhow::anyhow!("gas price fetch failed: {}", e))?;
        let gas_price = u128::from_str_radix(gp_hex.trim_start_matches("0x"), 16)
            .map_err(|_| anyhow::anyhow!("bad gas price"))?;

        let dex = gradience_core::dex::service::DexService::new();
        let tx = dex.build_swap_tx(&addr, from_token, to_token, amount, chain_num, 50).await
            .map_err(|e| anyhow::anyhow!("build swap tx failed: {}", e))?;

        let to = tx.to.as_deref().unwrap_or("");
        let value = tx.value.parse::<u128>().unwrap_or(0);
        let data = tx.data;
        let tx_hex = build_unsigned_tx(nonce, gas_price, to, value, &data, chain_num);

        let res = ows_lib::sign_and_send(
            wallet_id, chain, &tx_hex, Some(&passphrase), None, Some(rpc_url), Some(&vault_dir)
        ).map_err(|e| anyhow::anyhow!("sign_and_send failed: {}", e))?;

        anyhow::Result::Ok(res)
    });

    match result {
        Ok(res) => Ok(json!({"txHash": res.tx_hash})),
        Err(e) => Err(e),
    }
}

pub fn handle_pay(args: crate::args::PayArgs) -> anyhow::Result<serde_json::Value> {
    let wallet_id = &args.wallet_id;
    let recipient = &args.recipient;
    let amount = &args.amount;
    let token = args.token.as_deref().unwrap_or("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
    let chain = args.chain.as_deref().unwrap_or("base");

    let (passphrase, vault_dir) = get_vault_config()?;
    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
    let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);

    // If recipient looks like an HTTP URL, attempt MPP 402 payment flow.
    if recipient.starts_with("http://") || recipient.starts_with("https://") {
        let mpp_result = block_on_async(async {
            use gradience_core::payment::router::{PaymentRoutePreference, PaymentRouter};
            use gradience_core::payment::mpp_client::{GradienceMppProvider, MppClient};

            let db = sqlx::SqlitePool::connect(&db_path).await.ok()?;
            let routes = gradience_db::queries::list_payment_routes_by_wallet(&db, wallet_id).await.ok()?;
            let preferences: Vec<PaymentRoutePreference> = routes.into_iter().map(|r| PaymentRoutePreference {
                chain_id: r.chain_id,
                token_address: r.token_address,
                priority: r.priority as u32,
            }).collect();

            let router = if preferences.is_empty() {
                PaymentRouter::new(vec![
                    PaymentRoutePreference { chain_id: "eip155:8453".into(), token_address: token.into(), priority: 1 },
                    PaymentRoutePreference { chain_id: "eip155:1".into(), token_address: "".into(), priority: 2 },
                ])
            } else {
                PaymentRouter::new(preferences)
            };

            let provider = GradienceMppProvider::new(wallet_id, router);
            let client = MppClient::new(provider);
            client.send(reqwest::Client::new().get(recipient)).await.ok()
        });

        if let Some(resp) = mpp_result {
            return Ok(json!({"mppStatus": resp.status().as_u16(), "mppUrl": recipient}));
        }
        return Err(anyhow::anyhow!("MPP payment failed (Tempo wallet may not be configured)"));
    }

    // Otherwise fall back to direct on-chain transfer via payment router.
    let tx_hash = block_on_async(async {
        let db = sqlx::SqlitePool::connect(&db_path).await.ok()?;
        let routes = gradience_db::queries::list_payment_routes_by_wallet(&db, wallet_id).await.ok()?;
        let addrs = gradience_db::queries::list_wallet_addresses(&db, wallet_id).await.ok()?;

        let selected_chain = if let Some(first) = routes.first() {
            first.chain_id.clone()
        } else {
            format!("eip155:{}", if chain == "base" { 8453 } else { 1 })
        };

        let mut addr = None;
        for a in addrs {
            if a.chain_id == selected_chain || a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
                addr = Some((a.address.clone(), a.chain_id.clone()));
                break;
            }
        }
        let (from_addr, used_chain) = addr?;

        let svc = gradience_core::payment::x402::X402Service::new();
        let deadline = (std::time::SystemTime::now() + std::time::Duration::from_secs(3600))
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let req = svc.create_requirement(recipient, amount, token, deadline, Some(chain)).ok()?;
        let sign_payload = format!("x402:{}:{}:{}", recipient, amount, deadline);
        let sign_result = ows_lib::sign_message(
            wallet_id,
            "eip155:1",
            &sign_payload,
            Some(&passphrase),
            Some("utf8"),
            None,
            Some(&vault_dir),
        ).ok()?;
        let mut payment = svc.sign_payment(req, &sign_result.signature).ok()?;
        let chain_str = if used_chain.contains("8453") { "base" } else { "ethereum" };
        svc.settle_payment(&mut payment, wallet_id, &from_addr, chain_str, &passphrase, &vault_dir).await.ok()
    });

    match tx_hash {
        Some(hash) => Ok(json!({"txHash": hash})),
        None => Err(anyhow::anyhow!("x402 settlement failed")),
    }
}

pub fn handle_llm_generate(args: crate::args::LlmGenerateArgs) -> anyhow::Result<serde_json::Value> {
    let wallet_id = &args.wallet_id;
    let provider = args.provider.as_deref().unwrap_or("anthropic");
    let model = args.model.as_deref().unwrap_or("claude-3-5-sonnet");
    let prompt = &args.prompt;

    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
    let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);

    let result = block_on_async(async {
        let db = sqlx::SqlitePool::connect(&db_path).await.ok()?;
        let svc = gradience_core::ai::gateway::AiGatewayService::new();
        svc.llm_generate(&db, wallet_id, None, provider, model, prompt).await.ok()
    });

    match result {
        Some(resp) => Ok(json!({
            "content": resp.content,
            "inputTokens": resp.input_tokens,
            "outputTokens": resp.output_tokens,
            "costRaw": resp.cost_raw,
            "status": resp.status,
        })),
        None => Err(anyhow::anyhow!("llm generate failed")),
    }
}

pub fn handle_ai_balance(args: crate::args::AiBalanceArgs) -> anyhow::Result<serde_json::Value> {
    let wallet_id = &args.wallet_id;
    let token = args.token.as_deref().unwrap_or("USDC");

    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
    let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);

    let balance = block_on_async(async {
        let db = sqlx::SqlitePool::connect(&db_path).await.ok()?;
        let svc = gradience_core::ai::gateway::AiGatewayService::new();
        svc.get_balance(&db, wallet_id, token).await.ok()
    });

    Ok(json!({
        "walletId": wallet_id,
        "token": token,
        "balanceRaw": balance.unwrap_or_else(|| "0".into()),
    }))
}

pub fn handle_ai_models() -> anyhow::Result<serde_json::Value> {
    Ok(json!({
        "models": [
            { "provider": "anthropic", "model": "claude-3-5-sonnet", "priceInput": "3000000", "priceOutput": "15000000" },
            { "provider": "openai", "model": "gpt-4o", "priceInput": "2500000", "priceOutput": "10000000" },
        ]
    }))
}

pub fn handle_sign_message(args: crate::args::SignMessageArgs) -> anyhow::Result<serde_json::Value> {
    let wallet_id = &args.wallet_id;
    let message = &args.message;
    let (passphrase, vault_dir) = get_vault_config()?;

    let result = ows_lib::sign_message(
        wallet_id,
        "eip155:1",
        message,
        Some(&passphrase),
        Some("utf8"),
        None,
        Some(&vault_dir),
    ).map_err(|e| anyhow::anyhow!("ows sign_message failed: {}", e))?;

    Ok(json!({
        "signature": result.signature,
        "walletId": wallet_id,
    }))
}

pub fn handle_sign_and_send(args: crate::args::SignAndSendArgs) -> anyhow::Result<serde_json::Value> {
    let wallet_id = &args.wallet_id;
    let chain_id = &args.chain_id;
    let to = &args.transaction.to;
    let value = &args.transaction.value;
    let data_hex = args.transaction.data.as_deref().unwrap_or("0x");
    let data = hex::decode(data_hex.trim_start_matches("0x")).unwrap_or_default();

    let (passphrase, vault_dir) = get_vault_config()?;
    let is_solana = chain_id.starts_with("solana:");

    let result = block_on_async(async {
        let data_dir = std::env::var("GRADIENCE_DATA_DIR")
            .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
        let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);
        let db = match sqlx::SqlitePool::connect(&db_path).await {
            Ok(db) => db,
            Err(_) => return anyhow::Result::<_>::Err(anyhow::anyhow!("db connect failed")),
        };
        let addrs = gradience_db::queries::list_wallet_addresses(&db, wallet_id).await.unwrap_or_default();

        let rpc_url = resolve_rpc(chain_id);
        let tx_hex = if is_solana {
            let from = addrs.iter()
                .find(|a| a.chain_id.starts_with("solana:"))
                .map(|a| a.address.clone())
                .ok_or_else(|| anyhow::anyhow!("solana address not found"))?;
            let client = gradience_core::rpc::solana::SolanaRpcClient::new(rpc_url);
            let blockhash = client.get_latest_blockhash().await
                .map_err(|e| anyhow::anyhow!("blockhash fetch failed: {}", e))?;
            let lamports: u64 = value.parse().unwrap_or(0);
            let tx_bytes = gradience_core::ows::signing::build_solana_transfer_tx(&from, to, lamports, &blockhash)
                .map_err(|e| anyhow::anyhow!("build solana tx failed: {}", e))?;
            format!("0x{}", hex::encode(&tx_bytes))
        } else {
            let chain_num = gradience_core::chain::evm_chain_num(chain_id);
            let value_raw: u128 = value.parse().unwrap_or(0);
            let mut from_addr = None;
            for a in &addrs {
                if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
                    from_addr = Some(a.address.clone());
                    break;
                }
            }
            let addr = from_addr.ok_or_else(|| anyhow::anyhow!("wallet address not found"))?;
            let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
                .map_err(|e| anyhow::anyhow!("rpc client: {}", e))?;
            let nonce = client.get_transaction_count(&addr).await
                .map_err(|e| anyhow::anyhow!("nonce fetch failed: {}", e))?;
            let gp_hex = client.get_gas_price().await
                .map_err(|e| anyhow::anyhow!("gas price fetch failed: {}", e))?;
            let gas_price = u128::from_str_radix(gp_hex.trim_start_matches("0x"), 16)
                .map_err(|_| anyhow::anyhow!("bad gas price"))?;
            build_unsigned_tx(nonce, gas_price, to, value_raw, &data, chain_num)
        };

        let res = ows_lib::sign_and_send(
            wallet_id,
            chain_id,
            &tx_hex,
            Some(&passphrase),
            None,
            Some(rpc_url),
            Some(&vault_dir),
        ).map_err(|e| anyhow::anyhow!("sign_and_send failed: {}", e))?;

        let _ = gradience_core::audit::service::log_wallet_action(
            &db,
            wallet_id,
            None,
            "mcp_sign_and_send",
            &serde_json::json!({"to": to, "value": value, "chain_id": chain_id, "tx_hash": res.tx_hash}).to_string(),
            "allow",
        ).await;

        anyhow::Result::Ok(res)
    });

    match result {
        Ok(res) => Ok(json!({"txHash": res.tx_hash, "walletId": wallet_id})),
        Err(e) => Err(e),
    }
}

pub fn handle_verify_api_key(args: crate::args::VerifyApiKeyArgs) -> anyhow::Result<serde_json::Value> {
    let api_key = &args.api_key;
    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".gradience").to_string_lossy().to_string());
    let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir);

    let valid = block_on_async(async {
        let db = sqlx::SqlitePool::connect(&db_path).await.ok()?;
        let hash = ring::digest::digest(&ring::digest::SHA256, api_key.as_bytes());
        gradience_db::queries::get_api_key_by_hash(&db, hash.as_ref()).await.ok()
    });

    Ok(json!({
        "valid": valid.is_some(),
        "apiKeyPrefix": api_key.get(..8).unwrap_or(""),
    }))
}
