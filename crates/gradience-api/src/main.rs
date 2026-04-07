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
    db: &Pool<Sqlite>,
    wallet_id: &str,
    chain_id: &str,
    transaction: gradience_core::ows::adapter::Transaction,
) -> Result<gradience_core::policy::engine::EvalResult, StatusCode> {
    use gradience_core::policy::engine::{PolicyEngine, EvalContext};

    let policies = gradience_db::queries::list_active_policies_by_wallet(db, wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let engine = PolicyEngine;
    let ctx = EvalContext {
        wallet_id: wallet_id.into(),
        api_key_id: "web".into(),
        chain_id: chain_id.into(),
        transaction,
        intent: None,
        timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
    };

    let core_policies: Vec<_> = policies.iter()
        .filter_map(|p| gradience_core::policy::engine::Policy::try_from_db(p).ok())
        .collect();
    let policy_refs: Vec<_> = core_policies.iter().collect();
    engine.evaluate(ctx, policy_refs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
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
    let eval = evaluate_wallet_policy(
        &state.db, &wallet_id, &format!("eip155:{}", chain_num), tx).await?;
    if eval.decision == gradience_core::policy::engine::Decision::Deny {
        return Ok((StatusCode::FORBIDDEN, axum::Json(serde_json::json!({"error": eval.reasons.join(", ")}))));
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

    Ok((StatusCode::OK, axum::Json(serde_json::json!({ "tx_hash": result.tx_hash }))))
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
    let _from_addr = addr.ok_or(StatusCode::NOT_FOUND)?;

    let wei = gradience_core::eth_to_wei(&body.amount)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let rpc_url = if body.chain == "base" {
        "https://mainnet.base.org"
    } else {
        "https://eth.llamarpc.com"
    };

    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let nonce = client.get_transaction_count(&body.to).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    let eval = evaluate_wallet_policy(
        &state.db, &wallet_id, &format!("eip155:{}", chain_num), tx).await?;
    if eval.decision == gradience_core::policy::engine::Decision::Deny {
        return Ok((StatusCode::FORBIDDEN, axum::Json(serde_json::json!({"error": eval.reasons.join(", ")}))));
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

    #[derive(Serialize)]
    struct SignResp {
        tx_hash: String,
    }
    Ok((StatusCode::OK, axum::Json(serde_json::json!({ "tx_hash": result.tx_hash }))))
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

    #[derive(Serialize)]
    struct ApiKeyResp {
        id: String,
        name: String,
        raw_token: Option<String>,
    }

    Ok((StatusCode::CREATED, axum::Json(ApiKeyResp {
        id: key.id,
        name: name.into(),
        raw_token: key.raw_token,
    })))
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

    #[derive(Serialize)]
    struct GenerateResp {
        content: String,
        input_tokens: i64,
        output_tokens: i64,
        cost_raw: String,
        status: String,
    }

    Ok((StatusCode::OK, axum::Json(GenerateResp {
        content: resp.content,
        input_tokens: resp.input_tokens,
        output_tokens: resp.output_tokens,
        cost_raw: resp.cost_raw,
        status: resp.status,
    })))
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

    let state = Arc::new(AppState {
        db,
        webauthn,
        ows: Arc::new(LocalOwsAdapter::new(vault_dir.clone())),
        vault_dir,
        reg_challenges: Mutex::new(HashMap::new()),
        auth_challenges: Mutex::new(HashMap::new()),
        credentials: Mutex::new(HashMap::new()),
        sessions: Mutex::new(HashMap::new()),
    });

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
        .route("/api/wallets/:id/transactions", get(wallet_transactions))
        .route("/api/wallets/:id/anchor", post(wallet_anchor))
        .route("/api/wallets/:id/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api/wallets/:id/policies", post(create_policy))
        .route("/api/ai/topup", post(ai_topup))
        .route("/api/ai/balance/:wallet_id", get(ai_balance))
        .route("/api/ai/generate", post(ai_generate))
        .route("/api/mcp/sign_transaction", post(mcp_sign_transaction))
        .route("/api/mcp/get_balance", post(mcp_get_balance))
        .route("/health", get(|| async { "ok" }))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("Gradience API listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
