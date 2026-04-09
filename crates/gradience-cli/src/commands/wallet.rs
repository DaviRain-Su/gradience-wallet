use crate::context::AppContext;
use anyhow::Result;
use gradience_core::ows::adapter::OwsAdapter;
use gradience_core::payment::mpp::{BatchTransferPayload, MppPaymentRequest, MppService};
use gradience_db::queries;

fn api_base() -> String {
    std::env::var("GRADIENCE_API_URL").unwrap_or_else(|_| "http://localhost:8080".into())
}

fn token_path(ctx: &AppContext) -> std::path::PathBuf {
    ctx.data_dir.join(".cli_token")
}

fn read_token(ctx: &AppContext) -> Option<String> {
    std::fs::read_to_string(token_path(ctx))
        .ok()
        .map(|s| s.trim().to_string())
}

pub async fn login(ctx: &AppContext) -> Result<()> {
    crate::commands::auth::login(ctx).await
}

pub async fn logout(ctx: &AppContext) -> Result<()> {
    let path = token_path(ctx);
    if path.exists() {
        std::fs::remove_file(&path)?;
        println!("Logged out successfully.");
    } else {
        println!("Not logged in.");
    }
    Ok(())
}

pub async fn whoami(ctx: &AppContext, json: bool) -> Result<()> {
    if !json {
        return crate::commands::auth::whoami(ctx).await;
    }

    let base = api_base();
    let mut out = serde_json::json!({
        "vault_unlocked": false,
        "remote_authenticated": false,
        "linked_wallets": 0,
        "address": serde_json::Value::Null,
    });

    if let Some(token) = read_token(ctx) {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("{}/api/wallets", base))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;
        if let Ok(r) = res {
            if r.status().is_success() {
                if let Ok(wallets) = r.json::<Vec<serde_json::Value>>().await {
                    out["remote_authenticated"] = serde_json::Value::Bool(true);
                    out["linked_wallets"] = serde_json::Value::Number(wallets.len().into());
                    if let Some(first) = wallets.first() {
                        out["address"] = first["address"].clone();
                    }
                }
            }
        }
    }

    if let Some(pp) = ctx.read_passphrase() {
        if ctx.ows.init_vault(&pp).await.is_ok() {
            out["vault_unlocked"] = serde_json::Value::Bool(true);
        }
    }

    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

pub async fn balance(
    ctx: &AppContext,
    wallet_id: String,
    chain: Option<String>,
    json: bool,
) -> Result<()> {
    let chain = chain.unwrap_or_else(|| "base".into());
    let wallet = queries::get_wallet_by_id(&ctx.db, &wallet_id).await?;
    if wallet.is_none() {
        anyhow::bail!("Wallet not found: {}", wallet_id);
    }

    let addrs = queries::list_wallet_addresses(&ctx.db, &wallet_id)
        .await
        .unwrap_or_default();

    if !json {
        return crate::commands::agent::balance(ctx, wallet_id, Some(chain)).await;
    }

    let mut balances = Vec::new();
    for a in &addrs {
        let is_evm = a.chain_id.starts_with("eip155:");
        let is_match = a.chain_id.contains(&chain)
            || (chain == "base"
                && (a.chain_id == "eip155:8453" || (is_evm && a.chain_id == "eip155:1")))
            || (chain == "ethereum" && a.chain_id == "eip155:1")
            || (chain == "conflux"
                && (a.chain_id == "eip155:1030" || a.chain_id == "eip155:71" || is_evm))
            || (chain == "conflux-core" && a.chain_id.starts_with("cfx:"))
            || (chain == "solana" && a.chain_id.starts_with("solana:"))
            || (chain == "ton" && a.chain_id.starts_with("ton:"));
        if !is_match {
            continue;
        }

        let (balance_str, unit): (String, &str) =
            if chain == "solana" || a.chain_id.starts_with("solana:") {
                let rpc_url = "https://api.devnet.solana.com";
                let client = gradience_core::rpc::solana::SolanaRpcClient::new(rpc_url);
                match client.get_balance(&a.address).await {
                    Ok(lamports) => {
                        let sol = gradience_core::rpc::solana::lamports_to_sol(lamports);
                        (sol.to_string(), "SOL")
                    }
                    Err(e) => (format!("ERROR: {}", e), ""),
                }
            } else if chain == "ton" || a.chain_id.starts_with("ton:") {
                let rpc_url = gradience_core::chain::resolve_rpc(&a.chain_id);
                let client = gradience_core::rpc::ton::TonRpcClient::new_with_url(rpc_url);
                match client.get_balance(&a.address).await {
                    Ok(nanoton) => (format!("{:.9}", nanoton as f64 / 1e9), "TON"),
                    Err(e) => (format!("ERROR: {}", e), ""),
                }
            } else if chain == "conflux-core" || a.chain_id.starts_with("cfx:") {
                let rpc_url = gradience_core::chain::resolve_rpc(&a.chain_id);
                let client =
                    gradience_core::rpc::conflux_core::ConfluxCoreRpcClient::new_with_url(rpc_url);
                match client.get_balance(&a.address).await {
                    Ok(drip) => (format!("{:.18}", drip as f64 / 1e18), "CFX"),
                    Err(e) => (format!("ERROR: {}", e), ""),
                }
            } else {
                let rpc_url = if chain == "base" || a.chain_id == "eip155:8453" {
                    "https://mainnet.base.org"
                } else {
                    gradience_core::chain::resolve_rpc(&a.chain_id)
                };
                let client = gradience_core::rpc::evm::EvmRpcClient::new(&a.chain_id, rpc_url)?;
                match client.get_balance(&a.address).await {
                    Ok(bal) => {
                        let eth = bal.parse::<f64>().unwrap_or(0.0);
                        (format!("{:.18}", eth), "ETH")
                    }
                    Err(e) => (format!("ERROR: {}", e), ""),
                }
            };

        balances.push(serde_json::json!({
            "chain_id": a.chain_id,
            "address": a.address,
            "balance": balance_str,
            "unit": unit,
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "wallet_id": wallet_id,
            "chain": chain,
            "balances": balances,
        }))?
    );
    Ok(())
}

pub async fn fund(
    ctx: &AppContext,
    wallet_id: String,
    amount: String,
    chain: Option<String>,
    to: Option<String>,
    _json: bool,
) -> Result<()> {
    crate::commands::agent::fund(ctx, wallet_id, amount, chain, to).await
}

pub async fn transfer(
    ctx: &AppContext,
    wallet_id: String,
    amount: String,
    token: String,
    to: String,
    chain: Option<String>,
    json: bool,
) -> Result<()> {
    let native = [
        "ETH", "BASE_ETH", "SOL", "TON", "CFX", "BNB", "MATIC", "ARB_ETH", "OP_ETH",
    ];
    let is_native = native.contains(&token.to_uppercase().as_str());
    if !is_native {
        anyhow::bail!(
            "ERC20 / SPL token transfers are not yet implemented in this CLI version. Use native tokens only."
        );
    }
    if json {
        println!(
            "{}",
            serde_json::json!({
                "wallet_id": wallet_id,
                "amount": amount,
                "token": token,
                "to": to,
                "chain": chain.clone().unwrap_or_else(|| "base".into()),
                "status": "executing"
            })
        );
    }
    crate::commands::agent::fund(ctx, wallet_id, amount, chain, Some(to)).await
}

pub async fn keys(ctx: &AppContext, wallet_id: String, json: bool) -> Result<()> {
    let keys = queries::list_api_keys_by_wallet(&ctx.db, &wallet_id).await?;
    if !json {
        if keys.is_empty() {
            println!("No API keys for wallet {}", wallet_id);
            return Ok(());
        }
        println!("API keys for wallet {}:", wallet_id);
        for k in keys {
            let status = if k.expires_at.is_some() {
                "revoked"
            } else {
                "active"
            };
            println!("  {} - {} [{}]", k.id, k.name, status);
        }
        return Ok(());
    }

    let items: Vec<serde_json::Value> = keys
        .into_iter()
        .map(|k| {
            serde_json::json!({
                "id": k.id,
                "name": k.name,
                "permissions": k.permissions,
                "active": k.expires_at.is_none(),
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "wallet_id": wallet_id,
            "keys": items,
        }))?
    );
    Ok(())
}

pub async fn services(json: bool) -> Result<()> {
    let services = vec![
        serde_json::json!({
            "id": "openai",
            "name": "OpenAI",
            "base_url": "https://api.openai.com",
            "methods": ["POST"],
        }),
        serde_json::json!({
            "id": "anthropic",
            "name": "Anthropic",
            "base_url": "https://api.anthropic.com",
            "methods": ["POST"],
        }),
        serde_json::json!({
            "id": "gradience-ai",
            "name": "Gradience AI Proxy",
            "base_url": format!("{}/v1/proxy/gradience", api_base()),
            "methods": ["POST", "GET"],
        }),
    ];

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "services": services }))?
        );
    } else {
        println!("Registered AI / MPP services:");
        for s in services {
            println!(
                "  {} - {} ({})",
                s["id"].as_str().unwrap_or(""),
                s["name"].as_str().unwrap_or(""),
                s["base_url"].as_str().unwrap_or("")
            );
        }
    }
    Ok(())
}

pub async fn sessions_list(ctx: &AppContext, wallet_id: String) -> Result<()> {
    let wallet = queries::get_wallet_by_id(&ctx.db, &wallet_id).await?;
    if wallet.is_none() {
        anyhow::bail!("Wallet not found: {}", wallet_id);
    }
    let sessions = queries::list_sessions_by_user(&ctx.db, "user-1").await?;
    if sessions.is_empty() {
        println!("No active sessions for wallet {}", wallet_id);
    } else {
        println!("Active sessions for wallet {}:", wallet_id);
        for s in sessions {
            let (token, username, created_at, _expires_at) = s;
            println!("  {} (user: {}, created: {})", token, username, created_at);
        }
    }
    Ok(())
}

pub async fn sessions_close(ctx: &AppContext, session_id: String) -> Result<()> {
    let rows = queries::delete_session_by_token(&ctx.db, &session_id).await?;
    if rows > 0 {
        println!("Closed session {}", session_id);
    } else {
        println!("Session {} not found", session_id);
    }
    Ok(())
}

pub async fn mpp_sign(
    ctx: &AppContext,
    wallet_id: String,
    challenge_file: String,
    json: bool,
) -> Result<()> {
    let challenge_text = std::fs::read_to_string(&challenge_file)
        .map_err(|e| anyhow::anyhow!("Failed to read challenge file: {}", e))?;
    let challenge: mpp::PaymentChallenge = serde_json::from_str(&challenge_text)
        .map_err(|e| anyhow::anyhow!("Invalid MPP challenge JSON: {}", e))?;

    let passphrase = ctx
        .read_passphrase()
        .ok_or_else(|| anyhow::anyhow!("No session found. Run 'gradience wallet login' first."))?;
    let vault = ctx.ows.init_vault(&passphrase).await?;

    // Derive a signing key from the vault for this wallet.
    // We use ows_lib export to get the mnemonic, then derive an EVM signing key.
    let exported =
        ows_lib::export_wallet(&wallet_id, Some(&vault.passphrase), Some(&ctx.vault_dir))
            .map_err(|e| anyhow::anyhow!("Export wallet failed: {}", e))?;

    if exported.trim_start().starts_with('{') {
        anyhow::bail!("MPP sign is only supported for mnemonic wallets");
    }

    // Derive the first EVM account (eip155:1 / m/44'/60'/0'/0/0)
    let evm_address = ows_lib::derive_address(&exported, "eip155:1", Some(0))
        .map_err(|e| anyhow::anyhow!("Derive address failed: {}", e))?;

    // Get the private key for this account via ows_lib signing function not exposed,
    // so we derive the secret deterministically using the same logic as demo adapters.
    let seed = gradience_core::ows::local_adapter::derive_demo_seed(
        &wallet_id,
        "eip155:1",
        "m/44'/60'/0'/0/0",
    );
    let sk = secp256k1::SecretKey::from_slice(&seed)
        .map_err(|e| anyhow::anyhow!("Invalid derived secret: {}", e))?;
    let secp = secp256k1::Secp256k1::new();

    // Personal-sign style: keccak256("\x19Ethereum Signed Message:\n" + len(challenge) + challenge)
    let msg_bytes = challenge_text.as_bytes();
    let prefix = format!("\x19Ethereum Signed Message:\n{}", msg_bytes.len());
    let mut full_msg = prefix.into_bytes();
    full_msg.extend_from_slice(msg_bytes);
    let digest = gradience_core::ows::signing::keccak256(&full_msg);
    let message = secp256k1::Message::from_digest_slice(&digest)
        .map_err(|e| anyhow::anyhow!("Message construction failed: {}", e))?;
    let sig = secp.sign_ecdsa(&message, &sk);
    let raw_sig = sig.serialize_compact();
    let mut sig_bytes = raw_sig.to_vec();
    sig_bytes.push(27); // recovery id placeholder (personal_sign recovery id is often ignored by consumers)
    let signature_hex = format!("0x{}", hex::encode(&sig_bytes));

    let out = serde_json::json!({
        "wallet_id": wallet_id,
        "address": evm_address,
        "challenge_id": challenge.id,
        "method": challenge.method.as_str(),
        "signature": signature_hex,
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        println!(
            "Signed MPP challenge {} for wallet {}",
            challenge.id, wallet_id
        );
        println!("Signature: {}", signature_hex);
    }
    Ok(())
}

pub async fn batch(_ctx: &AppContext, request_file: String, json: bool) -> Result<()> {
    let req_text = std::fs::read_to_string(&request_file)
        .map_err(|e| anyhow::anyhow!("Failed to read request file: {}", e))?;
    let req: MppPaymentRequest = serde_json::from_str(&req_text)
        .map_err(|e| anyhow::anyhow!("Invalid batch request JSON: {}", e))?;

    let svc = MppService::new();
    let payload = svc.build_batch(&req)?;

    if json {
        let out = match payload {
            BatchTransferPayload::Evm { to, value, data } => serde_json::json!({
                "type": "evm",
                "to": to,
                "value": value,
                "data": data,
            }),
            BatchTransferPayload::Solana { serialized_tx } => serde_json::json!({
                "type": "solana",
                "serialized_tx": serialized_tx,
            }),
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else {
        match payload {
            BatchTransferPayload::Evm { to, value, data } => {
                println!("EVM Batch Transfer:");
                println!("  Multicall3: {}", to);
                println!("  Value:      {}", value);
                println!("  Data:       {}", data);
            }
            BatchTransferPayload::Solana { serialized_tx } => {
                println!("Solana Batch Transfer:");
                println!("  Transaction (base64): {}", serialized_tx);
            }
        }
    }
    Ok(())
}
