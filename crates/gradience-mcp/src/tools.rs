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
    if chain_id.contains("8453") {
        "https://mainnet.base.org"
    } else {
        "https://eth.llamarpc.com"
    }
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

            let chain_num = if chain_id.contains("8453") { 8453u64 } else { 1u64 };
            let value_raw: u128 = value.parse().unwrap_or(0);
            let tx_hex = build_unsigned_tx(nonce, gas_price, to, value_raw, &data, chain_num);

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
        let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url).ok()?;
        for a in addrs {
            if a.chain_id.starts_with("eip155:") {
                return client.get_balance(&a.address).await.ok();
            }
        }
        None
    });

    Ok(json!({
        "walletId": wallet_id,
        "chainId": chain_id,
        "native": {
            "symbol": "ETH",
            "balance": balance.unwrap_or_else(|| "0x0".into()),
            "decimals": 18,
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
    let chain_num = if chain.contains("8453") || chain == "base" { 8453u64 } else { 1u64 };
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
        let tx = dex.build_swap_tx(&addr, from_token, to_token, amount, chain_num).await
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

    let tx_hash = block_on_async(async {
        let db = sqlx::SqlitePool::connect(&db_path).await.ok()?;
        let addrs = gradience_db::queries::list_wallet_addresses(&db, wallet_id).await.ok()?;
        let mut addr = None;
        for a in addrs {
            if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
                addr = Some(a.address.clone());
                break;
            }
        }
        let from_addr = addr?;

        let svc = gradience_core::payment::x402::X402Service::new();
        let deadline = (std::time::SystemTime::now() + std::time::Duration::from_secs(3600))
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let req = svc.create_requirement(recipient, amount, token, deadline).ok()?;
        let sig = "dummy-signature-for-demo";
        let mut payment = svc.sign_payment(req, sig).ok()?;
        svc.settle_payment(&mut payment, wallet_id, &from_addr, chain, &passphrase, &vault_dir).await.ok()
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
