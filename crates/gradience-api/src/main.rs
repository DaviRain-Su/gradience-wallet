use axum::{
    extract::{Json, Path, State},
    http::{header::AUTHORIZATION, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use gradience_core::ows::adapter::{DerivationParams, OwsAdapter};
use gradience_core::ows::local_adapter::LocalOwsAdapter;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use webauthn_rs::prelude::*;

#[derive(Clone)]
struct Session {
    username: String,
    passphrase: Option<String>,
}

struct AppState {
    db: Pool<Sqlite>,
    webauthn: Webauthn,
    ows: Arc<LocalOwsAdapter>,
    vault_dir: std::path::PathBuf,
    reg_challenges: Mutex<HashMap<String, PasskeyRegistration>>,
    auth_challenges: Mutex<HashMap<String, PasskeyAuthentication>>,
    credentials: Mutex<HashMap<String, Passkey>>,
    sessions: Mutex<HashMap<String, Session>>,
    risk_cache: gradience_core::policy::dynamic::RiskSignalCache,
}

fn auth_token(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

async fn get_session(state: &AppState, token: &str) -> Option<Session> {
    state.sessions.lock().await.get(token).cloned()
}

// ==================== Auth DTOs ====================

#[derive(Deserialize)]
struct RegisterStartReq {
    username: String,
}

#[derive(Serialize)]
struct RegisterStartResp {
    challenge: CreationChallengeResponse,
}

#[derive(Deserialize)]
struct RegisterFinishReq {
    username: String,
    credential: RegisterPublicKeyCredential,
    passphrase: String,
}

#[derive(Deserialize)]
struct LoginStartReq {
    username: String,
}

#[derive(Serialize)]
struct LoginStartResp {
    challenge: RequestChallengeResponse,
}

#[derive(Deserialize)]
struct LoginFinishReq {
    username: String,
    credential: PublicKeyCredential,
}

#[derive(Serialize)]
struct TokenResp {
    token: String,
}

#[derive(Deserialize)]
struct UnlockReq {
    passphrase: String,
}

// ==================== Auth Handlers ====================

async fn register_start(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterStartReq>,
) -> Result<Json<RegisterStartResp>, StatusCode> {
    let username = body.username.trim().to_lowercase();
    if username.len() < 3 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let user_id = Uuid::new_v4();
    let exclude = state
        .credentials
        .lock()
        .await
        .get(&username)
        .map(|pk| vec![pk.cred_id().clone()])
        .unwrap_or_default();

    let (ccr, reg_state) = state
        .webauthn
        .start_passkey_registration(user_id, &username, &username, Some(exclude))
        .map_err(|e| {
            warn!("webauthn start reg error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state.reg_challenges.lock().await.insert(username.clone(), reg_state);
    info!("Passkey register started for {}", username);
    Ok(Json(RegisterStartResp { challenge: ccr }))
}

async fn register_finish(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterFinishReq>,
) -> Result<Json<TokenResp>, StatusCode> {
    let username = body.username.trim().to_lowercase();
    if body.passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let reg_state = state
        .reg_challenges
        .lock()
        .await
        .remove(&username)
        .ok_or(StatusCode::BAD_REQUEST)?;

    let pk = state
        .webauthn
        .finish_passkey_registration(&body.credential, &reg_state)
        .map_err(|e| {
            warn!("webauthn finish reg error: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    let cred_json = serde_json::to_vec(&pk).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user_id = uuid::Uuid::new_v4().to_string();
    gradience_db::queries::create_user(&state.db, &user_id, &format!("{}@gradience.local", username))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    sqlx::query(
        "INSERT INTO passkey_credentials (id, user_id, credential_id, credential_pk, counter, transports, device_name) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&user_id)
    .bind(&user_id)
    .bind(pk.cred_id().as_ref())
    .bind(&cred_json)
    .bind(0i64)
    .bind("internal")
    .bind("Passkey")
    .execute(&state.db)
    .await
    .map_err(|e| {
        warn!("db insert passkey error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    state.credentials.lock().await.insert(username.clone(), pk);

    let token = uuid::Uuid::new_v4().to_string();
    state.sessions.lock().await.insert(token.clone(), Session {
        username,
        passphrase: Some(body.passphrase),
    });

    Ok(Json(TokenResp { token }))
}

async fn login_start(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginStartReq>,
) -> Result<Json<LoginStartResp>, StatusCode> {
    let username = body.username.trim().to_lowercase();
    let allowed = state
        .credentials
        .lock()
        .await
        .get(&username)
        .map(|pk| vec![pk.clone()])
        .unwrap_or_default();

    if allowed.is_empty() {
        return Err(StatusCode::NOT_FOUND);
    }

    let (rcr, auth_state) = state
        .webauthn
        .start_passkey_authentication(&allowed)
        .map_err(|e| {
            warn!("webauthn start auth error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    state.auth_challenges.lock().await.insert(username, auth_state);
    Ok(Json(LoginStartResp { challenge: rcr }))
}

async fn login_finish(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginFinishReq>,
) -> Result<Json<TokenResp>, StatusCode> {
    let username = body.username.trim().to_lowercase();
    let auth_state = state
        .auth_challenges
        .lock()
        .await
        .remove(&username)
        .ok_or(StatusCode::BAD_REQUEST)?;

    let _auth_result = state
        .webauthn
        .finish_passkey_authentication(&body.credential, &auth_state)
        .map_err(|e| {
            warn!("webauthn finish auth error: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    info!("Passkey login success for {}", username);
    let token = uuid::Uuid::new_v4().to_string();
    state.sessions.lock().await.insert(token.clone(), Session {
        username,
        passphrase: None,
    });
    Ok(Json(TokenResp { token }))
}

async fn unlock(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<UnlockReq>,
) -> Result<StatusCode, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let mut sessions = state.sessions.lock().await;
    let session = sessions.get_mut(&token).ok_or(StatusCode::UNAUTHORIZED)?;
    if body.passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let vault = state.ows.init_vault(&body.passphrase).await.map_err(|_| StatusCode::UNAUTHORIZED)?;
    drop(vault);
    session.passphrase = Some(body.passphrase);
    Ok(StatusCode::OK)
}

// ==================== Wallet DTOs ====================

#[derive(Deserialize)]
struct CreateWalletReq {
    name: String,
}

#[derive(Serialize)]
struct WalletResp {
    id: String,
    name: String,
    owner_id: String,
    workspace_id: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
}

// ==================== Wallet Handlers ====================

async fn list_wallets(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<WalletResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let rows = gradience_db::queries::list_wallets_by_owner(&state.db, "user-1")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let wallets = rows.into_iter().map(|w| WalletResp {
        id: w.id,
        name: w.name,
        owner_id: w.owner_id,
        workspace_id: w.workspace_id,
        status: w.status,
        created_at: w.created_at.to_rfc3339(),
        updated_at: w.updated_at.to_rfc3339(),
    }).collect();

    Ok(Json(wallets))
}

async fn create_wallet(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateWalletReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.ok_or(StatusCode::FORBIDDEN)?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Ensure demo user exists
    let _ = gradience_db::queries::create_user(&state.db, "user-1", "demo@gradience.io").await;

    let vault = state.ows.init_vault(&passphrase).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let wallet = state.ows.create_wallet(&vault, name, DerivationParams::default()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    gradience_db::queries::create_wallet(&state.db, &wallet.id, &wallet.name, "user-1", None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for acc in &wallet.accounts {
        gradience_db::queries::create_wallet_address(
            &state.db,
            &uuid::Uuid::new_v4().to_string(),
            &wallet.id,
            &acc.chain_id,
            &acc.address,
            &acc.derivation_path,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::CREATED)
}

#[derive(Serialize)]
struct BalanceResp {
    chain_id: String,
    address: String,
    balance: String,
}

async fn wallet_balance(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<Json<Vec<BalanceResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut balances = Vec::new();
    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", "https://mainnet.base.org")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for a in addrs {
        if a.chain_id.starts_with("eip155:") {
            let bal = client.get_balance(&a.address).await.unwrap_or_default();
            balances.push(BalanceResp {
                chain_id: a.chain_id,
                address: a.address,
                balance: bal,
            });
        }
    }

    Ok(Json(balances))
}

#[derive(Deserialize)]
struct FundReq {
    to: String,
    amount: String,
    chain: Option<String>,
}

async fn evaluate_wallet_policy(
    state: &AppState,
    wallet_id: &str,
    chain_id: &str,
    transaction: gradience_core::ows::adapter::Transaction,
) -> Result<(gradience_core::policy::engine::EvalResult, Vec<gradience_core::policy::engine::Policy>), StatusCode> {
    use gradience_core::policy::engine::{PolicyEngine, EvalContext};

    let db_policies = gradience_db::queries::list_active_policies_by_wallet(&state.db, wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let core_policies: Vec<_> = db_policies.iter()
        .filter_map(|p| gradience_core::policy::engine::Policy::try_from_db(p).ok())
        .collect();

    let parser = gradience_core::policy::intent::IntentParser::new();
    let intent = parser.parse(&transaction, chain_id).ok();

    let engine = PolicyEngine;
    let ctx = EvalContext {
        wallet_id: wallet_id.into(),
        api_key_id: "web".into(),
        chain_id: chain_id.into(),
        transaction: transaction.clone(),
        intent,
        timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
    };

    let policy_refs: Vec<_> = core_policies.iter().collect();
    let mut result = engine.evaluate(ctx, policy_refs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.decision == gradience_core::policy::engine::Decision::Allow {
        let amount_wei = transaction.value.parse::<u128>().unwrap_or(0);
        let spend_eval = gradience_core::policy::spending::evaluate_spending_limits(
            &state.db, wallet_id, chain_id, amount_wei, &core_policies
        ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if spend_eval.decision == gradience_core::policy::engine::Decision::Deny {
            result = spend_eval;
        }
    }

    if result.decision == gradience_core::policy::engine::Decision::Allow {
        for policy in &core_policies {
            if policy.status != "active" { continue; }
            for rule in &policy.rules {
                if let gradience_core::policy::engine::Rule::DynamicRisk { max_forta, max_chainalysis } = rule {
                    match state.risk_cache.evaluate("*", *max_forta, *max_chainalysis) {
                        Ok((true, reasons)) => {
                            return Ok((gradience_core::policy::engine::EvalResult {
                                decision: gradience_core::policy::engine::Decision::Deny,
                                reasons,
                            }, core_policies));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok((result, core_policies))
}

async fn wallet_fund(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<FundReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.ok_or(StatusCode::FORBIDDEN)?;

    let chain = body.chain.unwrap_or_else(|| "base".into());
    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut from_addr = None;
    for a in &addrs {
        if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
            from_addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = from_addr.ok_or(StatusCode::NOT_FOUND)?;

    let wei = gradience_core::eth_to_wei(&body.amount)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let chain_num = if chain == "base" { 8453u64 } else { 1u64 };
    let tx = gradience_core::ows::adapter::Transaction {
        to: Some(body.to.clone()),
        value: body.amount.clone(),
        data: vec![],
        raw_hex: format!("0x{}", hex::encode(body.to.trim_start_matches("0x"))),
    };
    let (eval, core_policies) = evaluate_wallet_policy(
        &state, &wallet_id, &format!("eip155:{}", chain_num), tx.clone()).await?;

    let mut approval_id = None;
    if eval.decision == gradience_core::policy::engine::Decision::Deny {
        let _ = gradience_core::audit::service::log_wallet_action(
            &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": body.to, "amount": body.amount}).to_string(), "deny",
        ).await;
        return Ok((StatusCode::FORBIDDEN, axum::Json(serde_json::json!({"error": eval.reasons.join(", ")}))));
    }
    if eval.decision == gradience_core::policy::engine::Decision::Warn {
        let aid = uuid::Uuid::new_v4().to_string();
        let policy_id = core_policies.first().map(|p| p.id.clone()).unwrap_or_default();
        let request_json = serde_json::json!({
            "action": "fund",
            "wallet_id": wallet_id,
            "to": body.to,
            "amount": body.amount,
            "chain": chain,
        }).to_string();
        let _ = gradience_db::queries::create_policy_approval(
            &state.db, &aid, &policy_id, &wallet_id, &request_json
        ).await;
        approval_id = Some(aid);
    }

    let rpc_url = if chain == "base" {
        "https://mainnet.base.org"
    } else {
        "https://eth.llamarpc.com"
    };

    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let nonce = client.get_transaction_count(&from_addr).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let gas_price_hex = client.get_gas_price().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let gas_price = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let to_bytes = hex::decode(body.to.trim_start_matches("0x")).unwrap_or_default();
    let mut rlp = rlp::RlpStream::new_list(9);
    rlp.append(&nonce);
    rlp.append(&gas_price);
    rlp.append(&21000u64);
    rlp.append(&to_bytes);
    rlp.append(&wei);
    rlp.append(&Vec::<u8>::new());
    rlp.append(&chain_num);
    rlp.append(&0u8);
    rlp.append(&0u8);
    let tx_hex = format!("0x{}", hex::encode(&rlp.out()));

    let result = ows_lib::sign_and_send(
        &wallet_id,
        &chain,
        &tx_hex,
        Some(&passphrase),
        None,
        Some(rpc_url),
        Some(&state.vault_dir),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::policy::spending::record_spending(
        &state.db, &wallet_id, &format!("eip155:{}", chain_num), wei, &core_policies,
    ).await;
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": body.to, "amount": body.amount, "tx_hash": result.tx_hash}).to_string(), "allow",
    ).await;

    let mut resp = serde_json::json!({ "tx_hash": result.tx_hash });
    if let Some(aid) = approval_id {
        resp["approval_id"] = aid.into();
        resp["warning"] = true.into();
        resp["reasons"] = eval.reasons.into();
    }
    Ok((StatusCode::OK, axum::Json(resp)))
}

#[derive(Deserialize)]
struct SignReq {
    chain: String,
    to: String,
    amount: String,
    data: Option<String>,
}

async fn wallet_sign(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<SignReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.ok_or(StatusCode::FORBIDDEN)?;

    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut addr = None;
    for a in &addrs {
        if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
            addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = addr.ok_or(StatusCode::NOT_FOUND)?;

    let wei = gradience_core::eth_to_wei(&body.amount)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let rpc_url = if body.chain == "base" {
        "https://mainnet.base.org"
    } else {
        "https://eth.llamarpc.com"
    };

    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let nonce = client.get_transaction_count(&from_addr).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let gas_price_hex = client.get_gas_price().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let gas_price = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let chain_num = if body.chain == "base" { 8453u64 } else { 1u64 };
    let to_bytes = hex::decode(body.to.trim_start_matches("0x")).unwrap_or_default();
    let data_bytes = hex::decode(body.data.as_deref().unwrap_or("").trim_start_matches("0x")).unwrap_or_default();

    let tx = gradience_core::ows::adapter::Transaction {
        to: Some(body.to.clone()),
        value: body.amount.clone(),
        data: data_bytes.clone(),
        raw_hex: format!("0x{}", hex::encode(&data_bytes)),
    };
    let (eval, core_policies) = evaluate_wallet_policy(
        &state, &wallet_id, &format!("eip155:{}", chain_num), tx.clone()).await?;

    let mut approval_id = None;
    if eval.decision == gradience_core::policy::engine::Decision::Deny {
        let _ = gradience_core::audit::service::log_wallet_action(
            &state.db, &wallet_id, None, "sign", &serde_json::json!({"to": body.to, "amount": body.amount}).to_string(), "deny",
        ).await;
        return Ok((StatusCode::FORBIDDEN, axum::Json(serde_json::json!({"error": eval.reasons.join(", ")}))));
    }
    if eval.decision == gradience_core::policy::engine::Decision::Warn {
        let aid = uuid::Uuid::new_v4().to_string();
        let policy_id = core_policies.first().map(|p| p.id.clone()).unwrap_or_default();
        let request_json = serde_json::json!({
            "action": "sign",
            "wallet_id": wallet_id,
            "to": body.to,
            "amount": body.amount,
            "data": body.data.as_deref().unwrap_or(""),
            "chain": body.chain,
        }).to_string();
        let _ = gradience_db::queries::create_policy_approval(
            &state.db, &aid, &policy_id, &wallet_id, &request_json
        ).await;
        approval_id = Some(aid);
    }
    let mut rlp = rlp::RlpStream::new_list(9);
    rlp.append(&nonce);
    rlp.append(&gas_price);
    rlp.append(&21000u64);
    rlp.append(&to_bytes);
    rlp.append(&wei);
    rlp.append(&data_bytes);
    rlp.append(&chain_num);
    rlp.append(&0u8);
    rlp.append(&0u8);
    let tx_hex = format!("0x{}", hex::encode(&rlp.out()));

    let result = ows_lib::sign_and_send(
        &wallet_id,
        &body.chain,
        &tx_hex,
        Some(&passphrase),
        None,
        Some(rpc_url),
        Some(&state.vault_dir),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::policy::spending::record_spending(
        &state.db, &wallet_id, &format!("eip155:{}", chain_num), wei, &core_policies,
    ).await;
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "sign", &serde_json::json!({"to": body.to, "amount": body.amount, "tx_hash": result.tx_hash}).to_string(), "allow",
    ).await;

    let mut resp = serde_json::json!({ "tx_hash": result.tx_hash });
    if let Some(aid) = approval_id {
        resp["approval_id"] = aid.into();
        resp["warning"] = true.into();
        resp["reasons"] = eval.reasons.into();
    }
    Ok((StatusCode::OK, axum::Json(resp)))
}

#[derive(Deserialize)]
struct SwapReq {
    chain: String,
    from_token: String,
    to_token: String,
    amount: String,
}

async fn wallet_swap(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<SwapReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.ok_or(StatusCode::FORBIDDEN)?;

    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut addr = None;
    for a in &addrs {
        if a.chain_id == "eip155:8453" || a.chain_id == "eip155:1" {
            addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = addr.ok_or(StatusCode::NOT_FOUND)?;

    let chain_num = if body.chain == "base" { 8453u64 } else { 1u64 };
    let rpc_url = if body.chain == "base" {
        "https://mainnet.base.org"
    } else {
        "https://eth.llamarpc.com"
    };

    let dex = gradience_core::dex::service::DexService::new();
    let tx = dex.build_swap_tx(
        &from_addr,
        &body.from_token,
        &body.to_token,
        &body.amount,
        chain_num,
    ).await.map_err(|_| StatusCode::BAD_REQUEST)?;

    let (eval, core_policies) = evaluate_wallet_policy(
        &state, &wallet_id, &format!("eip155:{}", chain_num), tx.clone()).await?;

    let mut approval_id = None;
    if eval.decision == gradience_core::policy::engine::Decision::Deny {
        let _ = gradience_core::audit::service::log_wallet_action(
            &state.db, &wallet_id, None, "swap",
            &serde_json::json!({"from_token": body.from_token, "to_token": body.to_token, "amount": body.amount}).to_string(), "deny",
        ).await;
        return Ok((StatusCode::FORBIDDEN, axum::Json(serde_json::json!({"error": eval.reasons.join(", ")}))));
    }
    if eval.decision == gradience_core::policy::engine::Decision::Warn {
        let aid = uuid::Uuid::new_v4().to_string();
        let policy_id = core_policies.first().map(|p| p.id.clone()).unwrap_or_default();
        let request_json = serde_json::json!({
            "action": "swap",
            "wallet_id": wallet_id,
            "from_token": body.from_token,
            "to_token": body.to_token,
            "amount": body.amount,
            "chain": body.chain,
        }).to_string();
        let _ = gradience_db::queries::create_policy_approval(
            &state.db, &aid, &policy_id, &wallet_id, &request_json
        ).await;
        approval_id = Some(aid);
    }

    // Estimate gas for the swap tx
    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let nonce = client.get_transaction_count(&from_addr).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let gas_price_hex = client.get_gas_price().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let gas_price = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let to_bytes = hex::decode(tx.to.as_deref().unwrap_or("").trim_start_matches("0x")).unwrap_or_default();
    let data_bytes = tx.data.clone();
    let value_wei = tx.value.parse::<u128>().unwrap_or(0);

    let mut rlp = rlp::RlpStream::new_list(9);
    rlp.append(&nonce);
    rlp.append(&gas_price);
    rlp.append(&300000u64); // higher gas limit for DEX swaps
    rlp.append(&to_bytes);
    rlp.append(&value_wei);
    rlp.append(&data_bytes);
    rlp.append(&chain_num);
    rlp.append(&0u8);
    rlp.append(&0u8);
    let tx_hex = format!("0x{}", hex::encode(&rlp.out()));

    let result = ows_lib::sign_and_send(
        &wallet_id,
        &body.chain,
        &tx_hex,
        Some(&passphrase),
        None,
        Some(rpc_url),
        Some(&state.vault_dir),
    ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::policy::spending::record_spending(
        &state.db, &wallet_id, &format!("eip155:{}", chain_num), value_wei, &core_policies,
    ).await;
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "swap",
        &serde_json::json!({"from_token": body.from_token, "to_token": body.to_token, "amount": body.amount, "tx_hash": result.tx_hash}).to_string(), "allow",
    ).await;

    let mut resp = serde_json::json!({ "tx_hash": result.tx_hash });
    if let Some(aid) = approval_id {
        resp["approval_id"] = aid.into();
        resp["warning"] = true.into();
        resp["reasons"] = eval.reasons.into();
    }
    Ok((StatusCode::OK, axum::Json(resp)))
}

// ==================== Policy Approval Handlers ====================

async fn list_policy_approvals(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    // For demo, list all pending approvals across all wallets
    let rows = gradience_db::queries::list_all_pending_policy_approvals(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let list: Vec<_> = rows.into_iter().map(|a| serde_json::json!({
        "id": a.id,
        "policy_id": a.policy_id,
        "wallet_id": a.wallet_id,
        "status": a.status,
        "request_json": a.request_json,
        "expires_at": a.expires_at.to_rfc3339(),
        "created_at": a.created_at.to_rfc3339(),
    })).collect();

    Ok(Json(list))
}

async fn approve_policy_approval(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(approval_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    gradience_db::queries::update_policy_approval_status(
        &state.db, &approval_id, "approved", Some(&session.username),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

async fn reject_policy_approval(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(approval_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    gradience_db::queries::update_policy_approval_status(
        &state.db, &approval_id, "rejected", Some(&session.username),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

#[derive(Serialize)]
struct TxResp {
    id: i64,
    action: String,
    decision: String,
    tx_hash: Option<String>,
    created_at: String,
}

async fn wallet_transactions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<Json<Vec<TxResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let logs = gradience_db::queries::list_audit_logs_by_wallet(&state.db, &wallet_id, 50)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let txs = logs.into_iter().map(|l| TxResp {
        id: l.id,
        action: l.action,
        decision: l.decision,
        tx_hash: l.tx_hash,
        created_at: l.created_at.to_rfc3339(),
    }).collect();

    Ok(Json(txs))
}

// ==================== API Key Handlers ====================

#[derive(Deserialize)]
struct CreateApiKeyReq {
    name: String,
}

async fn create_api_key(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<CreateApiKeyReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.ok_or(StatusCode::FORBIDDEN)?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let vault = state.ows.init_vault(&passphrase).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let key = state.ows.attach_api_key_and_policies(&vault, &wallet_id, name, vec![])
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let key_hash = hex::decode(&key.token_hash).unwrap_or_default();
    gradience_db::queries::create_api_key(&state.db, &key.id, &wallet_id, name, &key_hash, "sign,read", None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "create_api_key", &serde_json::json!({"key_id": key.id, "name": name}).to_string(), "allow",
    ).await;

    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({
        "id": key.id,
        "name": name,
        "raw_token": key.raw_token,
    }))))
}

async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let rows = gradience_db::queries::list_api_keys_by_wallet(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    #[derive(Serialize)]
    struct KeyRow {
        id: String,
        name: String,
        permissions: String,
        expired: bool,
    }

    let keys: Vec<_> = rows.into_iter().map(|k| KeyRow {
        id: k.id,
        name: k.name,
        permissions: k.permissions,
        expired: k.expires_at.is_some(),
    }).collect();

    Ok((StatusCode::OK, axum::Json(keys)))
}

// ==================== Policy Handlers ====================

#[derive(Deserialize)]
struct CreatePolicyReq {
    content: String,
}

async fn create_policy(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<CreatePolicyReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let policy_id = gradience_core::policy::service::create_policy_sync(
        &state.db,
        Some(&wallet_id),
        None,
        &body.content,
        Some(&state.vault_dir),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "create_policy", &serde_json::json!({"policy_id": policy_id}).to_string(), "allow",
    ).await;

    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({ "policy_id": policy_id }))))
}

// ==================== AI Gateway Handlers ====================

#[derive(Deserialize)]
struct TopupReq {
    wallet_id: String,
    token: String,
    amount_raw: String,
}

async fn ai_topup(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TopupReq>,
) -> Result<StatusCode, StatusCode> {
    let svc = gradience_core::ai::gateway::AiGatewayService::new();
    svc.topup(&state.db, &body.wallet_id, &body.token, &body.amount_raw)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

async fn ai_balance(
    State(state): State<Arc<AppState>>,
    Path(wallet_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = params.get("token").map(|s| s.as_str()).unwrap_or("USDC");
    let svc = gradience_core::ai::gateway::AiGatewayService::new();
    let bal = svc.get_balance(&state.db, &wallet_id, token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, axum::Json(serde_json::json!({ "balance_raw": bal }))))
}

#[derive(Deserialize)]
struct GenerateReq {
    wallet_id: String,
    provider: String,
    model: String,
    prompt: String,
}

async fn ai_generate(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<GenerateReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let svc = gradience_core::ai::gateway::AiGatewayService::new();
    let resp = svc.llm_generate(
        &state.db,
        &body.wallet_id,
        None,
        &body.provider,
        &body.model,
        &body.prompt,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &body.wallet_id, None, "ai_generate", &serde_json::json!({"provider": body.provider, "model": body.model, "cost_raw": resp.cost_raw}).to_string(), "allow",
    ).await;

    Ok((StatusCode::OK, axum::Json(serde_json::json!({
        "content": resp.content,
        "input_tokens": resp.input_tokens,
        "output_tokens": resp.output_tokens,
        "cost_raw": resp.cost_raw,
        "status": resp.status,
    }))))
}

async fn wallet_anchor(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let svc = gradience_core::audit::anchor::AnchorService::from_env()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(svc) = svc else {
        return Ok((StatusCode::SERVICE_UNAVAILABLE, axum::Json(serde_json::json!({"error": "ANCHOR_RPC_URL not configured"}))));
    };

    let tx_hash = svc.anchor_unanchored_logs(&state.db, &wallet_id, 100)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let decision = if tx_hash.is_some() { "allow" } else { "noop" };
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "anchor", &serde_json::json!({"tx_hash": tx_hash}).to_string(), decision,
    ).await;

    match tx_hash {
        Some(hash) => Ok((StatusCode::OK, axum::Json(serde_json::json!({ "tx_hash": hash })))),
        None => Ok((StatusCode::OK, axum::Json(serde_json::json!({ "message": "No unanchored logs" })))),
    }
}

async fn mcp_sign_transaction(
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, StatusCode> {
    let resp = gradience_mcp::tools::handle_sign_transaction(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, axum::Json(resp)))
}

async fn mcp_get_balance(
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, StatusCode> {
    let resp = gradience_mcp::tools::handle_get_balance(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, axum::Json(resp)))
}

async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let db_healthy = sqlx::query("SELECT 1").fetch_one(&state.db).await.is_ok();
    let vault_exists = state.vault_dir.exists();
    let anchor_ready = gradience_core::audit::anchor::AnchorService::from_env()
        .map(|o| o.is_some())
        .unwrap_or(false);

    let status = if db_healthy && vault_exists { "ok" } else { "degraded" };

    Ok(axum::Json(serde_json::json!({
        "status": status,
        "version": std::env!("CARGO_PKG_VERSION"),
        "checks": {
            "database": db_healthy,
            "vault_dir": vault_exists,
            "anchor_configured": anchor_ready,
        }
    })))
}

// ==================== Workspace Handlers ====================

#[derive(Deserialize)]
struct CreateWorkspaceReq {
    name: String,
}

async fn create_workspace(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateWorkspaceReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let svc = gradience_core::team::workspace::WorkspaceService::new();
    let workspace_id = svc.create_workspace(&state.db, name, "user-1")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, "system", None, "create_workspace", &serde_json::json!({"workspace_id": workspace_id}).to_string(), "allow",
    ).await;

    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({ "workspace_id": workspace_id }))))
}

async fn list_workspaces(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let rows = gradience_db::queries::list_workspaces_by_owner(&state.db, "user-1")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let list: Vec<_> = rows.into_iter().map(|w| serde_json::json!({
        "id": w.id,
        "name": w.name,
        "owner_id": w.owner_id,
        "plan": w.plan,
        "created_at": w.created_at.to_rfc3339(),
    })).collect();

    Ok(Json(list))
}

#[derive(Deserialize)]
struct InviteMemberReq {
    email: String,
    role: String,
}

async fn invite_workspace_member(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(workspace_id): Path<String>,
    Json(body): Json<InviteMemberReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let role = gradience_core::team::workspace::WorkspaceRole::from_str(&body.role)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Ensure user exists
    let user = gradience_db::queries::get_user_by_email(&state.db, &body.email).await.ok().flatten();
    let user_id = match user {
        Some(u) => u.id,
        None => {
            let uid = uuid::Uuid::new_v4().to_string();
            gradience_db::queries::create_user(&state.db, &uid, &body.email).await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            uid
        }
    };

    let svc = gradience_core::team::workspace::WorkspaceService::new();
    svc.add_member(&state.db, &workspace_id, &user_id, role)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

async fn list_workspace_members(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let rows = gradience_db::queries::list_workspace_members(&state.db, &workspace_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let list: Vec<_> = rows.into_iter().map(|m| serde_json::json!({
        "workspace_id": m.workspace_id,
        "user_id": m.user_id,
        "role": m.role,
        "invited_at": m.invited_at.to_rfc3339(),
    })).collect();

    Ok(Json(list))
}

// ==================== Telegram Mini App Webhook ====================

#[derive(Deserialize)]
struct TgWebhookReq {
    #[allow(dead_code)]
    update_id: i64,
    message: Option<TgMessage>,
}

#[derive(Deserialize)]
struct TgMessage {
    #[allow(dead_code)]
    message_id: i64,
    chat: TgChat,
    text: Option<String>,
}

#[derive(Deserialize)]
struct TgChat {
    id: i64,
}

async fn tg_webhook(
    Json(body): Json<TgWebhookReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = std::env::var("TELEGRAM_BOT_TOKEN")
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let origin = std::env::var("ORIGIN")
        .unwrap_or_else(|_| "https://gradience-wallet.example.com".into());

    let chat_id = body.message.as_ref().map(|m| m.chat.id);
    let text = body.message.as_ref().and_then(|m| m.text.clone());

    if let (Some(chat_id), Some(text)) = (chat_id, text) {
        if text.trim().to_lowercase().starts_with("/start") {
            let url = format!("{}/tg", origin.trim_end_matches('/'));
            let payload = serde_json::json!({
                "chat_id": chat_id,
                "text": "Welcome to Gradience Wallet! Tap below to open your wallet.",
                "reply_markup": {
                    "inline_keyboard": [
                        [
                            {
                                "text": "Open Gradience",
                                "web_app": {
                                    "url": url
                                }
                            }
                        ]
                    ]
                }
            });

            let client = reqwest::Client::new();
            let _ = client
                .post(format!("https://api.telegram.org/bot{}/sendMessage", token))
                .json(&payload)
                .send()
                .await;
        }
    }

    Ok(StatusCode::OK)
}

// ==================== Main ====================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let origin = std::env::var("ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());
    let rp_id = std::env::var("RP_ID").unwrap_or_else(|_| "localhost".to_string());
    let origin_url: url::Url = origin.parse()?;
    let webauthn = WebauthnBuilder::new(&rp_id, &origin_url)?.build()?;

    let db_path = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./gradience.db?mode=rwc".to_string());
    let db = sqlx::SqlitePool::connect(&db_path).await?;

    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    let vault_dir = data_dir.join("vault");
    std::fs::create_dir_all(&vault_dir)?;

    let risk_cache = gradience_core::policy::dynamic::RiskSignalCache::new();
    let state = Arc::new(AppState {
        db,
        webauthn,
        ows: Arc::new(LocalOwsAdapter::new(vault_dir.clone())),
        vault_dir,
        reg_challenges: Mutex::new(HashMap::new()),
        auth_challenges: Mutex::new(HashMap::new()),
        credentials: Mutex::new(HashMap::new()),
        sessions: Mutex::new(HashMap::new()),
        risk_cache: risk_cache.clone(),
    });

    tokio::spawn(gradience_core::policy::dynamic::mock_fetch_signals(
        risk_cache,
        std::env::var("RISK_FETCH_INTERVAL_SEC")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60),
    ));

    let app = Router::new()
        .route("/api/auth/passkey/register/start", post(register_start))
        .route("/api/auth/passkey/register/finish", post(register_finish))
        .route("/api/auth/passkey/login/start", post(login_start))
        .route("/api/auth/passkey/login/finish", post(login_finish))
        .route("/api/auth/unlock", post(unlock))
        .route("/api/wallets", get(list_wallets).post(create_wallet))
        .route("/api/wallets/:id/balance", get(wallet_balance))
        .route("/api/wallets/:id/fund", post(wallet_fund))
        .route("/api/wallets/:id/sign", post(wallet_sign))
        .route("/api/wallets/:id/swap", post(wallet_swap))
        .route("/api/wallets/:id/transactions", get(wallet_transactions))
        .route("/api/wallets/:id/anchor", post(wallet_anchor))
        .route("/api/wallets/:id/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api/wallets/:id/policies", post(create_policy))
        .route("/api/ai/topup", post(ai_topup))
        .route("/api/ai/balance/:wallet_id", get(ai_balance))
        .route("/api/ai/generate", post(ai_generate))
        .route("/api/mcp/sign_transaction", post(mcp_sign_transaction))
        .route("/api/mcp/get_balance", post(mcp_get_balance))
        .route("/api/workspaces", get(list_workspaces).post(create_workspace))
        .route("/api/workspaces/:id/members", get(list_workspace_members).post(invite_workspace_member))
        .route("/api/policy-approvals", get(list_policy_approvals))
        .route("/api/policy-approvals/:id/approve", post(approve_policy_approval))
        .route("/api/policy-approvals/:id/reject", post(reject_policy_approval))
        .route("/api/tg/webhook", post(tg_webhook))
        .route("/health", get(health_check))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(Arc::clone(&state));

    let anchor_state = Arc::clone(&state);
    tokio::spawn(async move {
        let interval_sec: u64 = std::env::var("ANCHOR_INTERVAL_SEC")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300);
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_sec));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            match gradience_core::audit::anchor::AnchorService::from_env() {
                Ok(Some(svc)) => {
                    match gradience_db::queries::list_unanchored_logs(&anchor_state.db, 1000).await {
                        Ok(logs) if !logs.is_empty() => {
                            let mut seen = std::collections::HashSet::new();
                            for log in logs {
                                if seen.insert(log.wallet_id.clone()) {
                                    match svc.anchor_unanchored_logs(
                                        &anchor_state.db, &log.wallet_id, 100
                                    ).await {
                                        Ok(Some(tx_hash)) => info!("Auto-anchored wallet {} tx {}", log.wallet_id, tx_hash),
                                        Ok(None) => {},
                                        Err(e) => warn!("Auto-anchor failed for {}: {}", log.wallet_id, e),
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Ok(None) => {}
                Err(e) => warn!("AnchorService init error: {}", e),
            }
        }
    });

    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("============================================================");
    info!("Gradience API v{}", std::env!("CARGO_PKG_VERSION"));
    info!("============================================================");
    info!("Listening on        : {}", listener.local_addr()?);
    info!("DATABASE_URL        : {}", db_path);
    info!("GRADIENCE_DATA_DIR  : {}", data_dir.display());
    info!("ORIGIN / RP_ID      : {} / {}", origin, rp_id);
    info!("ANCHOR_INTERVAL_SEC : {}s", std::env::var("ANCHOR_INTERVAL_SEC").unwrap_or_else(|_| "300".into()));
    match gradience_core::audit::anchor::AnchorService::from_env() {
        Ok(Some(_)) => info!("Anchor Service      : enabled (contract ready)"),
        Ok(None)    => warn!("Anchor Service      : disabled (missing ANCHOR_RPC_URL)"),
        Err(e)      => warn!("Anchor Service      : config error ({})", e),
    }
    info!("CORS                : allow_any=true");
    info!("============================================================");

    axum::serve(listener, app).await?;
    Ok(())
}
