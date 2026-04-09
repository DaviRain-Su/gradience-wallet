use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::middleware::*;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateAiProxyKeyReq {
    wallet_id: String,
    name: String,
}

pub async fn create_ai_proxy_key(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateAiProxyKeyReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &body.wallet_id).await?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let key_id = format!("grd_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let raw_token = format!("grd_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let key_hash = ring::digest::digest(&ring::digest::SHA256, raw_token.as_bytes());

    gradience_db::queries::create_api_key(
        &state.db,
        &key_id,
        &body.wallet_id,
        name,
        key_hash.as_ref(),
        "ai_proxy",
        Some(chrono::Utc::now() + chrono::Duration::hours(24)),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db,
        &body.wallet_id,
        None,
        "create_ai_proxy_key",
        &serde_json::json!({"key_id": key_id, "name": name}).to_string(),
        "allowed",
    )
    .await;

    Ok((
        StatusCode::CREATED,
        axum::Json(serde_json::json!({
            "id": key_id,
            "name": name,
            "raw_token": raw_token,
            "permissions": "ai_proxy",
            "expires_at": (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339(),
        })),
    ))
}

pub async fn list_ai_proxy_keys(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let wallet_id = params.get("wallet_id").cloned().unwrap_or_default();
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let rows = gradience_db::queries::list_api_keys_by_wallet_and_permission(
        &state.db, &wallet_id, "ai_proxy",
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    #[derive(Serialize)]
    struct KeyRow {
        id: String,
        name: String,
        permissions: String,
        expires_at: Option<String>,
        created_at: String,
    }

    let keys: Vec<_> = rows
        .into_iter()
        .map(|k| KeyRow {
            id: k.id,
            name: k.name,
            permissions: k.permissions,
            expires_at: k.expires_at.map(|t| t.to_rfc3339()),
            created_at: k.created_at.to_rfc3339(),
        })
        .collect();

    Ok((StatusCode::OK, axum::Json(keys)))
}

pub async fn delete_ai_proxy_key(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(key_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let key_row = gradience_db::queries::get_api_key_by_id(&state.db, &key_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let _wallet = require_wallet_owner(&state, &session, &key_row.wallet_id).await?;

    gradience_db::queries::revoke_api_key(&state.db, &key_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db,
        &key_row.wallet_id,
        None,
        "delete_ai_proxy_key",
        &serde_json::json!({"key_id": key_id}).to_string(),
        "allowed",
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn ai_proxy_handler(
    State(state): State<Arc<AppState>>,
    Path((provider, path)): Path<(String, String)>,
    req: axum::extract::Request,
) -> Result<axum::response::Response, StatusCode> {
    // 1. Authenticate via Bearer token
    let token = auth_token(req.headers()).ok_or(StatusCode::UNAUTHORIZED)?;
    let key_hash = ring::digest::digest(&ring::digest::SHA256, token.as_bytes());
    let key_row = gradience_db::queries::get_api_key_by_hash(&state.db, key_hash.as_ref())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !key_row.permissions.contains("ai_proxy") {
        return Err(StatusCode::FORBIDDEN);
    }

    let wallet_id = key_row.wallet_id;

    // 2. Resolve provider base URL
    let base_url = match provider.as_str() {
        "openai" => "https://openai.mpp.tempo.xyz",
        "anthropic" => "https://anthropic.mpp.tempo.xyz",
        "openrouter" => "https://openrouter.mpp.tempo.xyz",
        "gemini" => "https://gemini.mpp.tempo.xyz",
        "groq" => "https://groq.mpp.paywithlocus.com",
        "mistral" => "https://mistral.mpp.paywithlocus.com",
        "deepseek" => "https://deepseek.mpp.paywithlocus.com",
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let mut target_url = format!("{}/{}", base_url, path);
    if let Some(q) = req.uri().query() {
        target_url.push('?');
        target_url.push_str(q);
    }

    // 3. Read body
    let (parts, body) = req.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 4. Build reqwest request
    let http_client = reqwest::Client::new();
    let reqwest_method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
        .unwrap_or(reqwest::Method::GET);
    let mut req_builder = http_client
        .request(reqwest_method, &target_url)
        .body(body_bytes.to_vec());

    for (k, v) in &parts.headers {
        let k_str = k.as_str().to_lowercase();
        if k_str != "host" && k_str != "connection" && k_str != "authorization" {
            if let Ok(v_str) = v.to_str() {
                req_builder = req_builder.header(k.as_str(), v_str);
            }
        }
    }

    // 5. Build MppClient with demo seed signers
    use gradience_core::payment::mpp_client::{GradienceMppProvider, MppClient};
    use gradience_core::payment::router::PaymentRouter;

    let router = PaymentRouter::default();
    let mut mpp_provider = GradienceMppProvider::new(&wallet_id, router);

    use gradience_core::payment::mpp_client::EvmChargeConfig;

    let evm_path = "m/44'/60'/0'/0/0";
    let evm_seed =
        gradience_core::ows::local_adapter::derive_demo_seed(&wallet_id, "eip155:8453", evm_path);
    // Register multiple EVM chains for MPP charge
    mpp_provider = mpp_provider
        .with_evm_chain(EvmChargeConfig::new(
            8453,
            "https://mainnet.base.org",
            evm_seed,
        ))
        .with_evm_chain(EvmChargeConfig::new(
            56,
            "https://bsc-dataseed.binance.org",
            evm_seed,
        ))
        .with_evm_chain(EvmChargeConfig::new(
            1030,
            "https://evm.confluxrpc.com",
            evm_seed,
        ))
        .with_evm_chain(EvmChargeConfig::new(
            196,
            "https://rpc.xlayer.tech",
            evm_seed,
        ))
        .with_evm_chain(EvmChargeConfig::new(
            42161,
            "https://arb1.arbitrum.io/rpc",
            evm_seed,
        ))
        .with_evm_chain(EvmChargeConfig::new(
            137,
            "https://polygon-rpc.com",
            evm_seed,
        ))
        .with_evm_chain(EvmChargeConfig::new(
            10,
            "https://mainnet.optimism.io",
            evm_seed,
        ));

    let solana_path = "m/44'/501'/0'/0";
    let solana_seed = gradience_core::ows::local_adapter::derive_demo_seed(
        &wallet_id,
        "solana:mainnet",
        solana_path,
    );
    mpp_provider = mpp_provider
        .with_solana_secret(solana_seed)
        .with_solana_rpc("https://api.mainnet-beta.solana.com");

    // Register TON mainnet
    let ton_path = "m/44'/607'/0'/0";
    let ton_seed =
        gradience_core::ows::local_adapter::derive_demo_seed(&wallet_id, "ton:mainnet", ton_path);
    mpp_provider = mpp_provider.with_ton_seed(ton_seed).with_ton_mainnet(true);

    let client = MppClient::new(mpp_provider);

    let resp = client.send(req_builder).await.map_err(|e| {
        tracing::error!("ai_proxy_handler failed: {}", e);
        StatusCode::BAD_GATEWAY
    })?;

    // 6. Convert reqwest response to axum response (supports streaming)
    let axum_status = axum::http::StatusCode::from_u16(resp.status().as_u16())
        .unwrap_or(axum::http::StatusCode::OK);
    let headers_to_copy: Vec<(String, String)> = resp
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|v2| (k.as_str().to_string(), v2.to_string()))
        })
        .collect();
    let mut response = axum::response::Response::builder()
        .status(axum_status)
        .body(axum::body::Body::from_stream(resp.bytes_stream()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for (k, v) in headers_to_copy {
        if let (Ok(name), Ok(hv)) = (
            axum::http::HeaderName::from_bytes(k.as_bytes()),
            axum::http::HeaderValue::from_str(&v),
        ) {
            response.headers_mut().insert(name, hv);
        }
    }

    // 7. Audit log
    let provider_name = provider.clone();
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db,
        &wallet_id,
        None,
        "ai_proxy",
        &serde_json::json!({"provider": provider_name, "path": path, "status": axum_status.as_u16()}).to_string(),
        "allowed",
    ).await;

    Ok(response)
}
