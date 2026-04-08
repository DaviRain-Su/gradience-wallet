use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use gradience_core::ows::adapter::{DerivationParams, OwsAdapter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};
use webauthn_rs::prelude::*;

use crate::state::*;
use crate::middleware::*;

#[derive(Deserialize)]
pub struct RegisterStartReq {
    username: String,
}

#[derive(Serialize)]
pub struct RegisterStartResp {
    challenge: CreationChallengeResponse,
}

#[derive(Deserialize)]
pub struct RegisterFinishReq {
    username: String,
    credential: RegisterPublicKeyCredential,
    passphrase: String,
    email: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginStartReq {
    username: String,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Serialize)]
pub struct LoginStartResp {
    challenge: RequestChallengeResponse,
}

#[derive(Deserialize)]
pub struct LoginFinishReq {
    username: String,
    #[serde(default)]
    email: Option<String>,
    credential: PublicKeyCredential,
}

#[derive(Serialize)]
pub struct TokenResp {
    token: String,
}

#[derive(Deserialize)]
pub struct UnlockReq {
    passphrase: String,
}

#[derive(Deserialize)]
pub struct EmailSendCodeReq {
    email: String,
}

#[derive(Deserialize)]
pub struct EmailVerifyReq {
    email: String,
    code: String,
}

pub async fn send_email_via_resend(email: &str, code: &str) -> Result<(), String> {
    let api_key = std::env::var("RESEND_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        warn!("RESEND_API_KEY not set; cannot send email");
        return Err("RESEND_API_KEY not configured".into());
    }
    let client = reqwest::Client::new();
    let from = std::env::var("RESEND_FROM_EMAIL")
        .unwrap_or_else(|_| "Gradience <noreply@gradiences.xyz>".to_string());
    let res = client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "from": from,
            "to": [email],
            "subject": "Your Gradience verification code",
            "html": format!("<p>Your verification code is: <strong>{}</strong></p><p>This code expires in 10 minutes.</p>", code)
        }))
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        warn!("Resend API error: {}", body);
        return Err(format!("Resend API error: {}", body));
    }
    Ok(())
}

pub fn generate_otp_code() -> String {
    format!("{:06}", rand::random::<u32>() % 1_000_000)
}

pub async fn email_send_code(
    State(state): State<Arc<AppState>>,
    Json(body): Json<EmailSendCodeReq>,
) -> Result<StatusCode, StatusCode> {
    let email = body.email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }

    if let Ok(Some((last_sent, count))) = gradience_db::queries::get_email_send_limit(&state.db, &email).await {
        let since_last = chrono::Utc::now() - last_sent;
        if since_last < chrono::Duration::seconds(60) {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        if since_last < chrono::Duration::hours(1) && count >= 5 {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        if since_last >= chrono::Duration::hours(1) {
            let _ = gradience_db::queries::reset_email_send_limit(&state.db, &email).await;
        } else {
            let _ = gradience_db::queries::record_email_send(&state.db, &email).await;
        }
    } else {
        let _ = gradience_db::queries::record_email_send(&state.db, &email).await;
    }

    let code = generate_otp_code();
    if let Err(e) = send_email_via_resend(&email, &code).await {
        warn!("Failed to send verification email to {}: {}", email, e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(10);
    gradience_db::queries::upsert_email_verification(&state.db, &email, &code, expires_at
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    info!("Verification code sent to {}", email);
    Ok(StatusCode::OK)
}

pub async fn email_verify_code(
    State(state): State<Arc<AppState>>,
    Json(body): Json<EmailVerifyReq>,
) -> Result<Json<TokenResp>, StatusCode> {
    let email = body.email.trim().to_lowercase();
    let code = body.code.trim();
    if email.is_empty() || code.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (stored_code, expires_at, attempts) = gradience_db::queries::get_email_verification(&state.db, &email
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::BAD_REQUEST)?;

    if chrono::Utc::now() > expires_at {
        let _ = gradience_db::queries::delete_email_verification(&state.db, &email).await;
        return Err(StatusCode::GONE);
    }

    if attempts >= 5 {
        let _ = gradience_db::queries::delete_email_verification(&state.db, &email).await;
        return Err(StatusCode::FORBIDDEN);
    }

    if stored_code != code {
        gradience_db::queries::increment_email_verification_attempts(&state.db, &email
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Err(StatusCode::BAD_REQUEST);
    }

    gradience_db::queries::delete_email_verification(&state.db, &email
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = gradience_db::queries::get_user_by_email(&state.db, &email
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_id = if let Some(u) = user {
        u.id
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        gradience_db::queries::create_user(&state.db, &new_id, &email
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        new_id
    };

    let username = email.split('@').next().unwrap_or(&email).to_string();
    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    state.sessions.insert(token.clone(), Session {
        user_id,
        username,
        passphrase: None,
    }, expires_at).await;
    info!("Email login success for {}", email);
    Ok(Json(TokenResp { token }))
}

// ==================== Auth Handlers ====================

pub async fn register_start(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterStartReq>,
) -> Result<Json<RegisterStartResp>, StatusCode> {
    let username = body.username.trim().to_lowercase();
    if username.len() < 3 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let user_id = Uuid::new_v4();

    let creds = state.credentials.lock().await;
    let exclude = if let Some(pk) = creds.get(&username) {
        vec![pk.cred_id().clone()]
    } else {
        let email = format!("{}@gradience.local", username);
        let row = sqlx::query_as::<_, (Vec<u8>,)>(
            "SELECT pc.credential_id FROM passkey_credentials pc JOIN users u ON u.id = pc.user_id WHERE u.email = ?"
        )
        .bind(&email)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            warn!("db load credential_id error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        if let Some((cred_id,)) = row {
            vec![webauthn_rs::prelude::CredentialID::from(cred_id)]
        } else {
            vec![]
        }
    };
    drop(creds);

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

pub async fn register_finish(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterFinishReq>,
) -> Result<Json<TokenResp>, StatusCode> {
    let username = body.username.trim().to_lowercase();
    if body.passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check for existing username conflict
    let default_email = format!("{}@gradience.local", username);
    if let Some(existing) = gradience_db::queries::get_user_by_email(&state.db, &default_email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        // Also verify custom email conflict if provided
        let custom_email = body.email.as_deref().and_then(|e| {
            let trimmed = e.trim();
            if trimmed.is_empty() { None } else { Some(trimmed.to_lowercase()) }
        });
        let conflict = if let Some(ref custom) = custom_email {
            gradience_db::queries::get_user_by_email(&state.db, custom)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .is_some()
        } else {
            true // default_email already exists
        };
        if conflict {
            warn!("register conflict for username={} existing_email={}", username, existing.email);
            return Err(StatusCode::CONFLICT);
        }
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
    let email = body
        .email
        .as_deref()
        .and_then(|e| {
            let trimmed = e.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_lowercase())
            }
        })
        .unwrap_or_else(|| format!("{}@gradience.local", username));

    let existing_user = gradience_db::queries::get_user_by_email(&state.db, &email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_id = if let Some(user) = existing_user {
        user.id
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        gradience_db::queries::create_user(&state.db, &new_id, &email)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        new_id
    };

    // Replace passkey credential for this user
    let _ = sqlx::query("DELETE FROM passkey_credentials WHERE user_id = ?")
        .bind(&user_id)
        .execute(&state.db)
        .await;

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
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    state.sessions.insert(token.clone(), Session {
        user_id: user_id.clone(),
        username,
        passphrase: Some(body.passphrase),
    }, expires_at).await;

    Ok(Json(TokenResp { token }))
}

pub fn resolve_email(username: &str, email: Option<&str>) -> String {
    email
        .and_then(|e| {
            let trimmed = e.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_lowercase())
            }
        })
        .unwrap_or_else(|| format!("{}@gradience.local", username))
}

pub async fn login_start(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginStartReq>,
) -> Result<Json<LoginStartResp>, StatusCode> {
    let username = body.username.trim().to_lowercase();
    let email = resolve_email(&username, body.email.as_deref());

    let mut creds = state.credentials.lock().await;
    let allowed = if let Some(pk) = creds.get(&username) {
        vec![pk.clone()]
    } else {
        let row = sqlx::query_as::<_, (Vec<u8>,)>(
            "SELECT pc.credential_pk FROM passkey_credentials pc JOIN users u ON u.id = pc.user_id WHERE u.email = ?"
        )
        .bind(&email)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            warn!("db load passkey error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if let Some((cred_pk,)) = row {
            let pk: webauthn_rs::prelude::Passkey = serde_json::from_slice(&cred_pk)
                .map_err(|e| {
                    warn!("passkey deserialize error: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            creds.insert(username.clone(), pk.clone());
            vec![pk]
        } else {
            vec![]
        }
    };
    drop(creds);

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

pub async fn login_finish(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginFinishReq>,
) -> Result<Json<TokenResp>, StatusCode> {
    let username = body.username.trim().to_lowercase();
    let email = resolve_email(&username, body.email.as_deref());
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
    let user = gradience_db::queries::get_user_by_email(&state.db, &email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user = user.ok_or(StatusCode::NOT_FOUND)?;

    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    state.sessions.insert(token.clone(), Session {
        user_id: user.id,
        username,
        passphrase: None,
    }, expires_at).await;
    Ok(Json(TokenResp { token }))
}

pub async fn unlock(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<UnlockReq>,
) -> Result<StatusCode, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    if body.passphrase.len() < 12 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let vault = state.ows.init_vault(&body.passphrase).await.map_err(|_| StatusCode::UNAUTHORIZED)?;
    drop(vault);
    if !state.sessions.update_passphrase(&token, body.passphrase).await {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(StatusCode::OK)
}

#[derive(Serialize)]
pub struct AuthMeResp {
    user_id: String,
    username: String,
    has_passphrase: bool,
}

pub async fn auth_me(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<AuthMeResp>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    Ok(Json(AuthMeResp {
        user_id: session.user_id,
        username: session.username,
        has_passphrase: session.passphrase.is_some(),
    }))
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    state.sessions.remove(&token).await;
    Ok(StatusCode::OK)
}

// ==================== Recovery ====================

#[derive(Deserialize)]
pub struct RecoverInitiateReq {
    username: String,
}

pub async fn recover_initiate(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RecoverInitiateReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let username = body.username.trim().to_lowercase();
    let email = format!("{}@gradience.local", username);

    let user = gradience_db::queries::get_user_by_email(&state.db, &email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = user.ok_or(StatusCode::NOT_FOUND)?;
    let code = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let id = uuid::Uuid::new_v4().to_string();
    gradience_db::queries::create_recovery_code(&state.db, &id, &user.id, &code, "passkey_recovery")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Local-only recovery code delivery (production would use an email/SMS provider)
    info!("[RECOVERY] To: {} | Recovery code: {}", email, code);

    Ok((StatusCode::OK, axum::Json(serde_json::json!({"sent": true, "local": true}))))
}

#[derive(Deserialize)]
pub struct RecoverVerifyReq {
    username: String,
    code: String,
}

pub async fn recover_verify(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RecoverVerifyReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let username = body.username.trim().to_lowercase();
    let email = format!("{}@gradience.local", username);

    let user = gradience_db::queries::get_user_by_email(&state.db, &email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = user.ok_or(StatusCode::NOT_FOUND)?;
    let row = gradience_db::queries::get_valid_recovery_code(&state.db, &user.id, &body.code, "passkey_recovery")
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(rc) = row {
        gradience_db::queries::mark_recovery_code_used(&state.db, &rc.id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let token = uuid::Uuid::new_v4().to_string();
        state.recovery_sessions.lock().await.insert(token.clone(), RecoverySession {
            user_id: user.id,
            username,
        });
        return Ok((StatusCode::OK, axum::Json(serde_json::json!({"recovery_token": token, "recovered": true}))));
    }

    Err(StatusCode::BAD_REQUEST)
}

#[derive(Deserialize)]
pub struct RecoverRegisterReq {
    recovery_token: String,
    credential: serde_json::Value,
}

pub async fn recover_register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RecoverRegisterReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let rec = state.recovery_sessions.lock().await.remove(&body.recovery_token);
    let rec = rec.ok_or(StatusCode::UNAUTHORIZED)?;

    let pk: Passkey = serde_json::from_value(body.credential)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let cred_id = pk.cred_id().as_ref().to_vec();
    let cred_json = serde_json::to_vec(&pk).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = sqlx::query("DELETE FROM passkey_credentials WHERE user_id = ?")
        .bind(&rec.user_id)
        .execute(&state.db)
        .await;

    sqlx::query(
        "INSERT INTO passkey_credentials (id, user_id, credential_id, credential_pk, counter, transports, device_name) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&rec.user_id)
    .bind(&rec.user_id)
    .bind(&cred_id)
    .bind(&cred_json)
    .bind(0i64)
    .bind("internal")
    .bind("Passkey")
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let token = uuid::Uuid::new_v4().to_string();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
    state.sessions.insert(token.clone(), Session {
        user_id: rec.user_id,
        username: rec.username,
        passphrase: None,
    }, expires_at).await;

    Ok((StatusCode::OK, axum::Json(serde_json::json!({"token": token, "registered": true}))))
}


#[derive(Deserialize)]
pub struct DeviceInitiateReq {
    client_name: Option<String>,
}

#[derive(Serialize)]
pub struct DeviceInitiateResp {
    device_code: String,
    user_code: String,
    verification_url: String,
}

pub async fn device_initiate() -> Result<Json<DeviceInitiateResp>, StatusCode> {
    let device_code = uuid::Uuid::new_v4().to_string();
    let user_code = format!("{:04}-{:04}", rand::random::<u32>() % 10000, rand::random::<u32>() % 10000);
    let origin = std::env::var("ORIGIN").unwrap_or_else(|_| "http://localhost:3000".into());
    Ok(Json(DeviceInitiateResp {
        device_code: device_code.clone(),
        user_code: user_code.clone(),
        verification_url: format!("{}/device?code={}", origin, user_code),
    }))
}

#[derive(Deserialize)]
pub struct DevicePollReq {
    device_code: String,
}

pub async fn device_poll(
    State(state): State<Arc<AppState>>,
    Json(body): Json<DevicePollReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let auths = state.device_auths.lock().await;
    if let Some(auth) = auths.get(&body.device_code) {
        if let Some(token) = &auth.token {
            return Ok((StatusCode::OK, axum::Json(serde_json::json!({"token": token, "authorized": true}))));
        }
        return Ok((StatusCode::OK, axum::Json(serde_json::json!({"authorized": false}))));
    }
    Err(StatusCode::NOT_FOUND)
}

#[derive(Deserialize)]
pub struct DeviceAuthorizeReq {
    user_code: String,
}

pub async fn device_authorize(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<DeviceAuthorizeReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let _session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let mut auths = state.device_auths.lock().await;
    for (device_code, auth) in auths.iter_mut() {
        if auth.user_code == body.user_code {
            auth.token = Some(token);
            return Ok((StatusCode::OK, axum::Json(serde_json::json!({"device_code": device_code, "authorized": true}))));
        }
    }
    Err(StatusCode::NOT_FOUND)
}

// ==================== OAuth (Skeleton) ====================


pub async fn oauth_start(
    axum::extract::Path(provider): axum::extract::Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let auth_url = match provider.as_str() {
        "google" => "https://accounts.google.com/o/oauth2/auth?client_id=YOUR_CLIENT_ID&redirect_uri=http://localhost:3000/api/auth/oauth/google/callback&response_type=code&scope=openid%20email",
        "github" => "https://github.com/login/oauth/authorize?client_id=YOUR_CLIENT_ID&redirect_uri=http://localhost:3000/api/auth/oauth/github/callback&scope=user:email",
        _ => return Err(StatusCode::NOT_FOUND),
    };
    Ok((StatusCode::TEMPORARY_REDIRECT, [(axum::http::header::LOCATION, auth_url)]))
}

pub async fn oauth_callback(
    axum::extract::Path(provider): axum::extract::Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let _code = params.get("code").cloned().unwrap_or_default();
    info!("OAuth callback from {} with code {}", provider, _code);
    // In production, exchange code for token, fetch user info, link/create user.
    Ok((StatusCode::OK, axum::Json(serde_json::json!({
        "provider": provider,
        "status": "skeleton",
        "note": "Configure client_id/secret in production"
    }))))
}

// ==================== Wallet DTOs ====================

#[derive(Deserialize)]
pub struct CreateWalletReq {
    name: String,
}

#[derive(Serialize)]
pub struct WalletResp {
    id: String,
    name: String,
    owner_id: String,
    workspace_id: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
}

// ==================== Wallet Handlers ====================

pub async fn list_wallets(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<WalletResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let rows = gradience_db::queries::list_wallets_by_owner(&state.db, &session.user_id)
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

pub async fn create_wallet(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateWalletReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.clone().ok_or(StatusCode::FORBIDDEN)?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let vault = state.ows.init_vault(&passphrase).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let wallet = state.ows.create_wallet(&vault, name, DerivationParams::default()).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    gradience_db::queries::create_wallet(&state.db, &wallet.id, &wallet.name, &session.user_id, None)
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
pub struct BalanceResp {
    chain_id: String,
    address: String,
    balance: String,
}

pub async fn wallet_balance(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<Json<Vec<BalanceResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

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
        } else if a.chain_id.starts_with("solana:") {
            let sol_client = gradience_core::rpc::solana::SolanaRpcClient::new("https://api.devnet.solana.com");
            let lamports = sol_client.get_balance(&a.address).await.unwrap_or(0);
            let hex_bal = format!("0x{:x}", lamports);
            balances.push(BalanceResp {
                chain_id: a.chain_id,
                address: a.address,
                balance: hex_bal,
            });
        } else if a.chain_id.starts_with("ton:") {
            let rpc_url = gradience_core::chain::resolve_rpc(&a.chain_id);
            let ton_client = gradience_core::rpc::ton::TonRpcClient::new_with_url(rpc_url);
            let nanoton = ton_client.get_balance(&a.address).await.unwrap_or(0);
            let hex_bal = format!("0x{:x}", nanoton);
            balances.push(BalanceResp {
                chain_id: a.chain_id,
                address: a.address,
                balance: hex_bal,
            });
        } else if a.chain_id.starts_with("cfx:") {
            let rpc_url = gradience_core::chain::resolve_rpc(&a.chain_id);
            let cfx_client = gradience_core::rpc::conflux_core::ConfluxCoreRpcClient::new_with_url(rpc_url);
            let drip = cfx_client.get_balance(&a.address).await.unwrap_or(0);
            let hex_bal = format!("0x{:x}", drip);
            balances.push(BalanceResp {
                chain_id: a.chain_id,
                address: a.address,
                balance: hex_bal,
            });
        } else {
            balances.push(BalanceResp {
                chain_id: a.chain_id,
                address: a.address,
                balance: "unsupported".into(),
            });
        }
    }

    Ok(Json(balances))
}

#[derive(Serialize)]
pub struct AddressResp {
    chain_id: String,
    address: String,
}

pub async fn wallet_addresses(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<Json<Vec<AddressResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result: Vec<_> = addrs.into_iter().map(|a| AddressResp {
        chain_id: a.chain_id,
        address: a.address,
    }).collect();

    Ok(Json(result))
}

#[derive(Serialize)]
pub struct PortfolioResp {
    chain_id: String,
    address: String,
    native_balance: String,
    assets: Vec<gradience_core::portfolio::discovery::TokenAsset>,
}

pub async fn wallet_portfolio(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<Json<Vec<PortfolioResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut result = Vec::new();
    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", "https://mainnet.base.org")
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let svc = gradience_core::portfolio::discovery::TokenDiscoveryService::new();

    for a in addrs {
        if a.chain_id.starts_with("eip155:") {
            let native = client.get_balance(&a.address).await.unwrap_or_default();
            let assets = svc.discover(&a.chain_id, &a.address).await.unwrap_or_default();
            result.push(PortfolioResp {
                chain_id: a.chain_id,
                address: a.address,
                native_balance: native,
                assets,
            });
        } else if a.chain_id.starts_with("solana:") {
            let sol_client = gradience_core::rpc::solana::SolanaRpcClient::new("https://api.devnet.solana.com");
            let lamports = sol_client.get_balance(&a.address).await.unwrap_or(0);
            let native_balance = format!("0x{:x}", lamports);
            result.push(PortfolioResp {
                chain_id: a.chain_id,
                address: a.address,
                native_balance,
                assets: vec![],
            });
        } else if a.chain_id.starts_with("ton:") {
            let rpc_url = gradience_core::chain::resolve_rpc(&a.chain_id);
            let ton_client = gradience_core::rpc::ton::TonRpcClient::new_with_url(rpc_url);
            let nanoton = ton_client.get_balance(&a.address).await.unwrap_or(0);
            let native_balance = format!("0x{:x}", nanoton);
            result.push(PortfolioResp {
                chain_id: a.chain_id,
                address: a.address,
                native_balance,
                assets: vec![],
            });
        } else {
            result.push(PortfolioResp {
                chain_id: a.chain_id,
                address: a.address,
                native_balance: "0x0".into(),
                assets: vec![],
            });
        }
    }

    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct FundReq {
    to: String,
    amount: String,
    chain: Option<String>,
}

pub async fn evaluate_wallet_policy(
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

    // Build dynamic signals snapshot from cache
    let dynamic_signals = gradience_core::policy::engine::DynamicSignals {
        forta_score: state.risk_cache.get("*", "forta").map(|s| s.score),
        chainalysis_score: state.risk_cache.get("*", "chainalysis").map(|s| s.score),
    };

    let engine = PolicyEngine;
    let ctx = EvalContext {
        wallet_id: wallet_id.into(),
        api_key_id: "web".into(),
        chain_id: chain_id.into(),
        transaction: transaction.clone(),
        intent,
        timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        dynamic_signals: Some(dynamic_signals),
        max_tokens: None,
        model: None,
    };

    let policy_refs: Vec<_> = core_policies.iter().collect();
    let mut result = engine.evaluate(ctx, policy_refs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.decision == gradience_core::policy::engine::Decision::Allow {
        let amount_wei = gradience_core::eth_to_wei(&transaction.value).unwrap_or(0);
        let wallet = gradience_db::queries::get_wallet_by_id(&state.db, wallet_id
        ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let workspace_id = wallet.and_then(|w| w.workspace_id);
        let spend_eval = gradience_core::policy::spending::evaluate_spending_limits(
            &state.db, wallet_id, workspace_id.as_deref(), chain_id, amount_wei, &core_policies
        ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if spend_eval.decision == gradience_core::policy::engine::Decision::Deny {
            result = spend_eval;
        }
    }

    Ok((result, core_policies))
}

pub async fn wallet_fund(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<FundReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.clone().ok_or(StatusCode::FORBIDDEN)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let wm = gradience_core::wallet::service::WalletManagerService::new();
    wm.require_status_active(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let chain = body.chain.unwrap_or_else(|| "base".into());
    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // ---------- Solana branch ----------
    if chain == "solana" {
        let mut sol_addr = None;
        for a in &addrs {
            if a.chain_id.starts_with("solana:") {
                sol_addr = Some(a.address.clone());
                break;
            }
        }
        let from_addr = sol_addr.ok_or(StatusCode::BAD_REQUEST)?;
        let to_addr = if body.to.trim().is_empty() { from_addr.clone() } else { body.to.clone() };

        let sol_amount: f64 = body.amount.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
        let lamports = (sol_amount * 1_000_000_000.0) as u64;

        let rpc_url = "https://api.devnet.solana.com";
        let sol_client = gradience_core::rpc::solana::SolanaRpcClient::new(rpc_url);
        let blockhash = sol_client.get_latest_blockhash().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let tx_bytes = gradience_core::ows::signing::build_solana_transfer_tx(
            &from_addr,
            &to_addr,
            lamports,
            &blockhash,
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

        let tx = gradience_core::ows::adapter::Transaction {
            to: Some(to_addr.clone()),
            value: lamports.to_string(),
            data: vec![],
            raw_hex: tx_hex.clone(),
        };
        let (eval, core_policies) = evaluate_wallet_policy(
            &state, &wallet_id, "solana:103", tx.clone()).await?;

        let mut approval_id = None;
        if eval.decision == gradience_core::policy::engine::Decision::Deny {
            let _ = gradience_core::audit::service::log_wallet_action(
                &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": to_addr, "amount": body.amount}).to_string(), "denied",
            ).await;
            return Ok((StatusCode::FORBIDDEN, axum::Json(serde_json::json!({"error": eval.reasons.join(", ")}))));
        }
        if eval.decision == gradience_core::policy::engine::Decision::Warn {
            let aid = uuid::Uuid::new_v4().to_string();
            let policy_id = core_policies.first().map(|p| p.id.clone()).unwrap_or_default();
            let request_json = serde_json::json!({
                "action": "fund",
                "wallet_id": wallet_id,
                "to": to_addr,
                "amount": body.amount,
                "chain": chain,
            }).to_string();
            let _ = gradience_db::queries::create_policy_approval(
                &state.db, &aid, &policy_id, &wallet_id, &request_json
            ).await;
            approval_id = Some(aid);
        }

        let result = ows_lib::sign_and_send(
            &wallet_id,
            &chain,
            &tx_hex,
            Some(&passphrase),
            None,
            Some(rpc_url),
            Some(&state.vault_dir),
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let _ = gradience_core::audit::service::log_wallet_action(
            &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": to_addr, "amount": body.amount, "tx_hash": result.tx_hash}).to_string(), "allowed",
        ).await;

        let mut resp = serde_json::json!({ "tx_hash": result.tx_hash });
        if let Some(aid) = approval_id {
            resp["approval_id"] = aid.into();
            resp["warning"] = true.into();
            resp["reasons"] = eval.reasons.into();
        }
        return Ok((StatusCode::OK, axum::Json(resp)));
    }

    // ---------- TON branch ----------
    if chain == "ton" || chain == "toncoin" {
        let mut ton_addr = None;
        for a in &addrs {
            if a.chain_id.starts_with("ton:") {
                ton_addr = Some(a.address.clone());
                break;
            }
        }
        let from_addr = ton_addr.ok_or(StatusCode::BAD_REQUEST)?;
        let to_addr = if body.to.trim().is_empty() { from_addr.clone() } else { body.to.clone() };

        let ton_amount: f64 = body.amount.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
        let nanoton = (ton_amount * 1_000_000_000.0) as u64;

        let rpc_url = gradience_core::chain::resolve_rpc(&chain);
        let ton_client = gradience_core::rpc::ton::TonRpcClient::new_with_url(rpc_url);
        let seqno = ton_client.get_seqno(&from_addr).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let tx = gradience_core::ows::adapter::Transaction {
            to: Some(to_addr.clone()),
            value: nanoton.to_string(),
            data: seqno.to_be_bytes().to_vec(),
            raw_hex: "".into(),
        };
        let (eval, core_policies) = evaluate_wallet_policy(
            &state, &wallet_id, "ton:0", tx.clone()).await?;

        let mut approval_id = None;
        if eval.decision == gradience_core::policy::engine::Decision::Deny {
            let _ = gradience_core::audit::service::log_wallet_action(
                &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": to_addr, "amount": body.amount}).to_string(), "denied",
            ).await;
            return Ok((StatusCode::FORBIDDEN, axum::Json(serde_json::json!({"error": eval.reasons.join(", ")}))));
        }
        if eval.decision == gradience_core::policy::engine::Decision::Warn {
            let aid = uuid::Uuid::new_v4().to_string();
            let policy_id = core_policies.first().map(|p| p.id.clone()).unwrap_or_default();
            let request_json = serde_json::json!({
                "action": "fund",
                "wallet_id": wallet_id,
                "to": to_addr,
                "amount": body.amount,
                "chain": chain,
            }).to_string();
            let _ = gradience_db::queries::create_policy_approval(
                &state.db, &aid, &policy_id, &wallet_id, &request_json
            ).await;
            approval_id = Some(aid);
        }

        let vault = state.ows.init_vault(&passphrase).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let signed = state.ows.sign_transaction(&vault, &wallet_id, "ton:0", &tx, &passphrase).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let result = state.ows.broadcast("ton:0", &signed, rpc_url).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let _ = gradience_core::audit::service::log_wallet_action(
            &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": to_addr, "amount": body.amount, "tx_hash": result}).to_string(), "allowed",
        ).await;

        let mut resp = serde_json::json!({ "tx_hash": result });
        if let Some(aid) = approval_id {
            resp["approval_id"] = aid.into();
            resp["warning"] = true.into();
            resp["reasons"] = eval.reasons.into();
        }
        return Ok((StatusCode::OK, axum::Json(resp)));
    }

    // ---------- Conflux Core Space branch ----------
    if chain == "conflux-core" || chain.starts_with("cfx:") {
        let mut cfx_addr = None;
        for a in &addrs {
            if a.chain_id.starts_with("cfx:") {
                cfx_addr = Some(a.address.clone());
                break;
            }
        }
        let from_addr = cfx_addr.ok_or(StatusCode::BAD_REQUEST)?;
        let to_addr = if body.to.trim().is_empty() { from_addr.clone() } else { body.to.clone() };

        let amount_cfx: f64 = body.amount.parse().map_err(|_| StatusCode::BAD_REQUEST)?;
        let drip = (amount_cfx * 1_000_000_000_000_000_000.0) as u128;
        let value_hex = format!("0x{:x}", drip);

        let tx = gradience_core::ows::adapter::Transaction {
            to: Some(to_addr.clone()),
            value: value_hex,
            data: vec![],
            raw_hex: "".into(),
        };
        let (eval, core_policies) = evaluate_wallet_policy(
            &state, &wallet_id, "cfx:1", tx.clone()).await?;

        let mut approval_id = None;
        if eval.decision == gradience_core::policy::engine::Decision::Deny {
            let _ = gradience_core::audit::service::log_wallet_action(
                &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": to_addr, "amount": body.amount}).to_string(), "denied",
            ).await;
            return Ok((StatusCode::FORBIDDEN, axum::Json(serde_json::json!({"error": eval.reasons.join(", ")}))));
        }
        if eval.decision == gradience_core::policy::engine::Decision::Warn {
            let aid = uuid::Uuid::new_v4().to_string();
            let policy_id = core_policies.first().map(|p| p.id.clone()).unwrap_or_default();
            let request_json = serde_json::json!({
                "action": "fund",
                "wallet_id": wallet_id,
                "to": to_addr,
                "amount": body.amount,
                "chain": chain,
            }).to_string();
            let _ = gradience_db::queries::create_policy_approval(
                &state.db, &aid, &policy_id, &wallet_id, &request_json
            ).await;
            approval_id = Some(aid);
        }

        let vault = state.ows.init_vault(&passphrase).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let signed = state.ows.sign_transaction(&vault, &wallet_id, "cfx:1", &tx, &passphrase).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let result = state.ows.broadcast("cfx:1", &signed, "").await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let _ = gradience_core::audit::service::log_wallet_action(
            &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": to_addr, "amount": body.amount, "tx_hash": result}).to_string(), "allowed",
        ).await;

        let mut resp = serde_json::json!({ "tx_hash": result });
        if let Some(aid) = approval_id {
            resp["approval_id"] = aid.into();
            resp["warning"] = true.into();
            resp["reasons"] = eval.reasons.into();
        }
        return Ok((StatusCode::OK, axum::Json(resp)));
    }

    // ---------- EVM branch ----------
    let mut from_addr = None;
    for a in &addrs {
        if a.chain_id.starts_with("eip155:") {
            from_addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = from_addr.ok_or(StatusCode::BAD_REQUEST)?;

    let wei = gradience_core::eth_to_wei(&body.amount)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let chain_num = gradience_core::chain::evm_chain_num(&chain);
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
            &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": body.to, "amount": body.amount}).to_string(), "denied",
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

    let rpc_url = gradience_core::chain::resolve_rpc(&chain);

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

    let wallet_ws = gradience_db::queries::get_wallet_by_id(&state.db, &wallet_id)
        .await.ok().flatten().and_then(|w| w.workspace_id);
    let _ = gradience_core::policy::spending::record_spending(
        &state.db, &wallet_id, wallet_ws.as_deref(), &format!("eip155:{}", chain_num), wei, &core_policies,
    ).await;
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "fund", &serde_json::json!({"to": body.to, "amount": body.amount, "tx_hash": result.tx_hash}).to_string(), "allowed",
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
pub struct SignReq {
    chain: String,
    to: String,
    amount: String,
    data: Option<String>,
}

pub async fn wallet_sign(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<SignReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.clone().ok_or(StatusCode::FORBIDDEN)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let wm = gradience_core::wallet::service::WalletManagerService::new();
    wm.require_status_active(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut addr = None;
    for a in &addrs {
        if a.chain_id.starts_with("eip155:") {
            addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = match addr {
        Some(a) => a,
        None => {
            return Ok((StatusCode::BAD_REQUEST, axum::Json(serde_json::json!({
                "error": "Solana/Stellar transaction signing is on the roadmap — please use an EVM chain (Base, Ethereum, BNB) for live transactions."
            }))));
        }
    };

    let wei = gradience_core::eth_to_wei(&body.amount)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let rpc_url = gradience_core::chain::resolve_rpc(&body.chain);

    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let nonce = client.get_transaction_count(&from_addr).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let gas_price_hex = client.get_gas_price().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let gas_price = u128::from_str_radix(gas_price_hex.trim_start_matches("0x"), 16)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let chain_num = gradience_core::chain::evm_chain_num(&body.chain);
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
            &state.db, &wallet_id, None, "sign", &serde_json::json!({"to": body.to, "amount": body.amount}).to_string(), "denied",
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

    let wallet_ws = gradience_db::queries::get_wallet_by_id(
        &state.db, &wallet_id)
        .await.ok().flatten().and_then(|w| w.workspace_id);
    let _ = gradience_core::policy::spending::record_spending(
        &state.db, &wallet_id, wallet_ws.as_deref(), &format!("eip155:{}", chain_num), wei, &core_policies,
    ).await;
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "sign", &serde_json::json!({"to": body.to, "amount": body.amount, "tx_hash": result.tx_hash}).to_string(), "allowed",
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
pub struct SwapReq {
    chain: String,
    from_token: String,
    to_token: String,
    amount: String,
    slippage_bps: Option<u16>,
}

#[derive(Deserialize)]
pub struct SwapQuoteReq {
    chain: String,
    from_token: String,
    to_token: String,
    amount: String,
}

pub async fn swap_quote(
    Json(body): Json<SwapQuoteReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let chain_num = if body.chain == "solana" { 101u64 } else { gradience_core::chain::evm_chain_num(&body.chain) };
    let dex = gradience_core::dex::service::DexService::new();
    let quote = dex.get_quote("", &body.from_token, &body.to_token, &body.amount, chain_num).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, axum::Json(serde_json::json!({
        "from_token": quote.from_token,
        "to_token": quote.to_token,
        "from_amount": quote.from_amount,
        "to_amount": quote.to_amount,
        "price_impact": quote.price_impact,
        "provider": quote.provider,
    }))))
}

pub async fn wallet_swap(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<SwapReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.clone().ok_or(StatusCode::FORBIDDEN)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let wm = gradience_core::wallet::service::WalletManagerService::new();
    wm.require_status_active(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let addrs = gradience_db::queries::list_wallet_addresses(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // ---------- Solana branch ----------
    if body.chain == "solana" {
        let mut sol_addr = None;
        for a in &addrs {
            if a.chain_id.starts_with("solana:") {
                sol_addr = Some(a.address.clone());
                break;
            }
        }
        let from_addr = sol_addr.ok_or(StatusCode::BAD_REQUEST)?;

        let dex = gradience_core::dex::service::DexService::new();
        let tx = dex.build_swap_tx(&from_addr, &body.from_token, &body.to_token, &body.amount, 101u64, body.slippage_bps.unwrap_or(50))
            .await.map_err(|_| StatusCode::BAD_REQUEST)?;

        let (eval, core_policies) = evaluate_wallet_policy(
            &state, &wallet_id, "solana:103", tx.clone()).await?;

        let mut approval_id = None;
        if eval.decision == gradience_core::policy::engine::Decision::Deny {
            let _ = gradience_core::audit::service::log_wallet_action(
                &state.db, &wallet_id, None, "swap",
                &serde_json::json!({"from_token": body.from_token, "to_token": body.to_token, "amount": body.amount}).to_string(), "denied",
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

        let rpc_url = "https://api.devnet.solana.com";
        let result = ows_lib::sign_and_send(
            &wallet_id,
            "solana",
            &tx.raw_hex,
            Some(&passphrase),
            None,
            Some(rpc_url),
            Some(&state.vault_dir),
        ).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let _ = gradience_core::audit::service::log_wallet_action(
            &state.db, &wallet_id, None, "swap",
            &serde_json::json!({"from_token": body.from_token, "to_token": body.to_token, "amount": body.amount, "tx_hash": result.tx_hash}).to_string(), "allowed",
        ).await;

        let mut resp = serde_json::json!({ "tx_hash": result.tx_hash });
        if let Some(aid) = approval_id {
            resp["approval_id"] = aid.into();
            resp["warning"] = true.into();
            resp["reasons"] = eval.reasons.into();
        }
        return Ok((StatusCode::OK, axum::Json(resp)));
    }

    // ---------- EVM branch ----------
    let mut addr = None;
    for a in &addrs {
        if a.chain_id.starts_with("eip155:") {
            addr = Some(a.address.clone());
            break;
        }
    }
    let from_addr = addr.ok_or(StatusCode::BAD_REQUEST)?;

    let chain_num = gradience_core::chain::evm_chain_num(&body.chain);
    let rpc_url = gradience_core::chain::resolve_rpc(&body.chain);

    let dex = gradience_core::dex::service::DexService::new();
    let tx = dex.build_swap_tx(
        &from_addr,
        &body.from_token,
        &body.to_token,
        &body.amount,
        chain_num,
        body.slippage_bps.unwrap_or(50),
    ).await.map_err(|_| StatusCode::BAD_REQUEST)?;

    let (eval, core_policies) = evaluate_wallet_policy(
        &state, &wallet_id, &format!("eip155:{}", chain_num), tx.clone()).await?;

    let mut approval_id = None;
    if eval.decision == gradience_core::policy::engine::Decision::Deny {
        let _ = gradience_core::audit::service::log_wallet_action(
            &state.db, &wallet_id, None, "swap",
            &serde_json::json!({"from_token": body.from_token, "to_token": body.to_token, "amount": body.amount}).to_string(), "denied",
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
    rlp.append(&300000u64);
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

    let wallet_ws = gradience_db::queries::get_wallet_by_id(
        &state.db, &wallet_id)
        .await.ok().flatten().and_then(|w| w.workspace_id);
    let _ = gradience_core::policy::spending::record_spending(
        &state.db, &wallet_id, wallet_ws.as_deref(), &format!("eip155:{}", chain_num), value_wei, &core_policies,
    ).await;
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "swap",
        &serde_json::json!({"from_token": body.from_token, "to_token": body.to_token, "amount": body.amount, "tx_hash": result.tx_hash}).to_string(), "allowed",
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

pub async fn list_policy_approvals(
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

pub async fn approve_policy_approval(
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

pub async fn reject_policy_approval(
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
pub struct TxResp {
    id: i64,
    action: String,
    decision: String,
    tx_hash: Option<String>,
    created_at: String,
}

pub async fn wallet_transactions(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<Json<Vec<TxResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

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

#[derive(Deserialize)]
pub struct VerifyProofReq {
    root: String,
    leaf: String,
    proof: Vec<String>,
}

pub async fn verify_audit_proof(
    Json(body): Json<VerifyProofReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let root = hex::decode(body.root.trim_start_matches("0x"))
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let leaf = hex::decode(body.leaf.trim_start_matches("0x"))
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let proof: Vec<[u8; 32]> = body
        .proof
        .iter()
        .filter_map(|p| hex::decode(p.trim_start_matches("0x")).ok().and_then(|v| v.try_into().ok()))
        .collect();
    if proof.len() != body.proof.len() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let valid = gradience_core::audit::merkle::verify_proof(root, leaf, &proof);
    Ok((StatusCode::OK, axum::Json(serde_json::json!({ "valid": valid }))))
}

pub async fn audit_export(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let format = params.get("format").cloned().unwrap_or_else(|| "json".into());
    let logs = gradience_db::queries::list_audit_logs_by_wallet(&state.db, &wallet_id, i64::MAX)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if format == "csv" {
        let mut csv = String::from("id,wallet_id,action,decision,tx_hash,created_at\n");
        for l in logs {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                l.id,
                l.wallet_id,
                l.action,
                l.decision,
                l.tx_hash.as_deref().unwrap_or(""),
                l.created_at.to_rfc3339()
            ));
        }
        let mut resp = csv.into_response();
        resp.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("text/csv"),
        );
        *resp.status_mut() = StatusCode::OK;
        return Ok(resp);
    }

    let json: Vec<_> = logs.into_iter().map(|l| serde_json::json!({
        "id": l.id,
        "wallet_id": l.wallet_id,
        "action": l.action,
        "decision": l.decision,
        "tx_hash": l.tx_hash,
        "created_at": l.created_at.to_rfc3339(),
    })).collect();
    Ok((StatusCode::OK, axum::Json(json)).into_response())
}

pub async fn wallet_audit_proof(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let log_id = params
        .get("log_id")
        .and_then(|v| v.parse::<i64>().ok())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let (proof, leaf, root) = gradience_core::audit::service::generate_merkle_proof_for_log(
        &state.db,
        &wallet_id,
        log_id,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(axum::Json(serde_json::json!({
        "wallet_id": wallet_id,
        "log_id": log_id,
        "root": root,
        "leaf": leaf,
        "proof": proof,
    })))
}

// ==================== API Key Handlers ====================

#[derive(Deserialize)]
pub struct CreateApiKeyReq {
    name: String,
}

pub async fn create_api_key(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<CreateApiKeyReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let passphrase = session.passphrase.clone().ok_or(StatusCode::FORBIDDEN)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

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
        &state.db, &wallet_id, None, "create_api_key", &serde_json::json!({"key_id": key.id, "name": name}).to_string(), "allowed",
    ).await;

    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({
        "id": key.id,
        "name": name,
        "raw_token": key.raw_token,
    }))))
}

pub async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

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
pub struct CreatePolicyReq {
    content: String,
}

pub async fn create_policy(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<CreatePolicyReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

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
        &state.db, &wallet_id, None, "create_policy", &serde_json::json!({"policy_id": policy_id}).to_string(), "allowed",
    ).await;

    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({ "policy_id": policy_id }))))
}

#[derive(Serialize)]
pub struct PolicyResp {
    id: String,
    name: String,
    wallet_id: Option<String>,
    workspace_id: Option<String>,
    rules_json: String,
    status: String,
    created_at: String,
}

pub async fn create_workspace_policy(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(workspace_id): Path<String>,
    Json(body): Json<CreatePolicyReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _ = require_workspace_member(&state, &session.user_id, &workspace_id).await?;

    let policy_id = gradience_core::policy::service::create_policy_sync(
        &state.db,
        None,
        Some(&workspace_id),
        &body.content,
        Some(&state.vault_dir),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({ "policy_id": policy_id }))))
}

pub async fn list_workspace_policies(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<PolicyResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _ = require_workspace_member(&state, &session.user_id, &workspace_id).await?;

    let rows = gradience_db::queries::list_active_policies_by_workspace(&state.db, &workspace_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let policies = rows.into_iter().map(|p| PolicyResp {
        id: p.id,
        name: p.name,
        wallet_id: p.wallet_id,
        workspace_id: p.workspace_id,
        rules_json: p.rules_json,
        status: p.status,
        created_at: p.created_at.to_rfc3339(),
    }).collect();

    Ok(Json(policies))
}

pub async fn list_wallet_policies(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<Json<Vec<PolicyResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let rows = gradience_db::queries::list_active_policies_by_wallet(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let policies = rows.into_iter().map(|p| PolicyResp {
        id: p.id,
        name: p.name,
        wallet_id: p.wallet_id,
        workspace_id: p.workspace_id,
        rules_json: p.rules_json,
        status: p.status,
        created_at: p.created_at.to_rfc3339(),
    }).collect();

    Ok(Json(policies))
}

// ==================== Payment Routes Handlers ====================

#[derive(Deserialize)]
pub struct SetPaymentRoutesReq {
    routes: Vec<PaymentRouteItem>,
}

#[derive(Deserialize, Serialize)]
pub struct PaymentRouteItem {
    chain_id: String,
    token_address: String,
    priority: u32,
}

#[derive(Serialize)]
pub struct PaymentRouteResp {
    chain_id: String,
    token_address: String,
    priority: u32,
}

pub async fn set_payment_routes(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    Json(body): Json<SetPaymentRoutesReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    gradience_db::queries::clear_payment_routes_by_wallet(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for r in &body.routes {
        let id = uuid::Uuid::new_v4().to_string();
        gradience_db::queries::create_payment_route(
            &state.db,
            &id,
            &wallet_id,
            &r.chain_id,
            &r.token_address,
            r.priority as i32,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db,
        &wallet_id,
        None,
        "set_payment_routes",
        &serde_json::json!({"count": body.routes.len()}).to_string(),
        "allowed",
    )
    .await;

    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({"saved": body.routes.len()}))))
}

pub async fn list_payment_routes(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<Json<Vec<PaymentRouteResp>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let rows = gradience_db::queries::list_payment_routes_by_wallet(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let routes = rows.into_iter().map(|r| PaymentRouteResp {
        chain_id: r.chain_id,
        token_address: r.token_address,
        priority: r.priority as u32,
    }).collect();

    Ok(Json(routes))
}

// ==================== AI Gateway Handlers ====================

#[derive(Deserialize)]
pub struct TopupReq {
    wallet_id: String,
    token: String,
    amount_raw: String,
}

pub async fn ai_topup(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<TopupReq>,
) -> Result<StatusCode, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &body.wallet_id).await?;

    let svc = gradience_core::ai::gateway::AiGatewayService::new();
    svc.topup(&state.db, &body.wallet_id, &body.token, &body.amount_raw)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

pub async fn ai_balance(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let token_sym = params.get("token").map(|s| s.as_str()).unwrap_or("USDC");
    let svc = gradience_core::ai::gateway::AiGatewayService::new();
    let bal = svc.get_balance(&state.db, &wallet_id, token_sym)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, axum::Json(serde_json::json!({ "balance_raw": bal }))))
}

#[derive(Deserialize)]
pub struct GenerateReq {
    wallet_id: String,
    provider: String,
    model: String,
    prompt: String,
}

pub async fn ai_generate(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<GenerateReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &body.wallet_id).await?;

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
        &state.db, &body.wallet_id, None, "ai_generate", &serde_json::json!({"provider": body.provider, "model": body.model, "cost_raw": resp.cost_raw}).to_string(), "allowed",
    ).await;

    Ok((StatusCode::OK, axum::Json(serde_json::json!({
        "content": resp.content,
        "input_tokens": resp.input_tokens,
        "output_tokens": resp.output_tokens,
        "cost_raw": resp.cost_raw,
        "status": resp.status,
    }))))
}

pub async fn ai_models(
    State(state): State<Arc<AppState>>,
    _headers: axum::http::HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let rows = gradience_db::queries::get_all_model_pricing(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let models: Vec<_> = rows.into_iter().map(|m| serde_json::json!({
        "provider": m.provider,
        "model": m.model,
        "input_per_m": m.input_per_m,
        "output_per_m": m.output_per_m,
        "cache_per_m": m.cache_per_m,
        "currency": m.currency,
    })).collect();
    Ok((StatusCode::OK, axum::Json(models)))
}

pub async fn list_payments(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let wallet_id = params.get("wallet_id").cloned().unwrap_or_default();
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let rows = gradience_db::queries::list_payment_records_by_wallet(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let payments: Vec<_> = rows.into_iter().map(|p| serde_json::json!({
        "id": p.id,
        "wallet_id": p.wallet_id,
        "protocol": p.protocol,
        "amount": p.amount,
        "token": p.token,
        "recipient": p.recipient,
        "status": p.status,
        "tx_hash": p.tx_hash,
        "created_at": p.created_at.to_rfc3339(),
    })).collect();
    Ok((StatusCode::OK, axum::Json(payments)))
}

pub async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

pub async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    use axum::extract::ws::Message;

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        let count: i64 = gradience_db::queries::list_all_pending_policy_approvals(&state.db)
            .await
            .map(|v| v.len() as i64)
            .unwrap_or(0);
        let payload = serde_json::json!({ "pendingApprovals": count });
        if socket.send(Message::Text(payload.to_string())).await.is_err() {
            break;
        }
    }
}

pub async fn wallet_anchor(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _wallet = require_wallet_owner(&state, &session, &wallet_id).await?;

    let wm = gradience_core::wallet::service::WalletManagerService::new();
    wm.require_status_active(&state.db, &wallet_id)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let svc = gradience_core::audit::anchor::AnchorService::from_env()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(svc) = svc else {
        return Ok((StatusCode::SERVICE_UNAVAILABLE, axum::Json(serde_json::json!({"error": "ANCHOR_RPC_URL not configured"}))));
    };

    let tx_hash = svc.anchor_unanchored_logs(&state.db, &wallet_id, 100)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let decision = if tx_hash.is_some() { "allowed" } else { "allowed" };
    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, &wallet_id, None, "anchor", &serde_json::json!({"tx_hash": tx_hash}).to_string(), decision,
    ).await;

    match tx_hash {
        Some(hash) => Ok((StatusCode::OK, axum::Json(serde_json::json!({ "tx_hash": hash })))),
        None => Ok((StatusCode::OK, axum::Json(serde_json::json!({ "message": "No unanchored logs" })))),
    }
}

pub async fn mcp_sign_transaction(
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, StatusCode> {
    let args: gradience_mcp::args::SignTxArgs = serde_json::from_value(body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let resp = gradience_mcp::tools::handle_sign_transaction(args)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, axum::Json(resp)))
}

pub async fn mcp_get_balance(
    Json(body): Json<serde_json::Value>,
) -> Result<impl IntoResponse, StatusCode> {
    let args: gradience_mcp::args::GetBalanceArgs = serde_json::from_value(body)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let resp = gradience_mcp::tools::handle_get_balance(args)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok((StatusCode::OK, axum::Json(resp)))
}

pub async fn health_check(
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
pub struct CreateWorkspaceReq {
    name: String,
}

pub async fn create_workspace(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<CreateWorkspaceReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let name = body.name.trim();
    if name.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let svc = gradience_core::team::workspace::WorkspaceService::new();
    let workspace_id = svc.create_workspace(&state.db, name, &session.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = gradience_core::audit::service::log_wallet_action(
        &state.db, "system", None, "create_workspace", &serde_json::json!({"workspace_id": workspace_id}).to_string(), "allowed",
    ).await;

    Ok((StatusCode::CREATED, axum::Json(serde_json::json!({ "workspace_id": workspace_id }))))
}

pub async fn list_workspaces(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;

    let rows = gradience_db::queries::list_workspaces_by_owner(&state.db, &session.user_id)
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
pub struct InviteMemberReq {
    email: String,
    role: String,
}

pub async fn invite_workspace_member(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(workspace_id): Path<String>,
    Json(body): Json<InviteMemberReq>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _role = require_workspace_member(&state, &session.user_id, &workspace_id).await?;

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

pub async fn list_workspace_members(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let token = auth_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;
    let session = get_session(&state, &token).await.ok_or(StatusCode::UNAUTHORIZED)?;
    let _role = require_workspace_member(&state, &session.user_id, &workspace_id).await?;

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
pub struct TgWebhookReq {
    #[allow(dead_code)]
    update_id: i64,
    message: Option<TgMessage>,
}

#[derive(Deserialize)]
pub struct TgMessage {
    #[allow(dead_code)]
    message_id: i64,
    chat: TgChat,
    text: Option<String>,
}

#[derive(Deserialize)]
pub struct TgChat {
    id: i64,
}

pub async fn tg_webhook(
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

// ==================== MPP Demo Mock ====================

#[derive(Deserialize)]
pub struct MppDemoReq {
    prompt: String,
}

pub async fn mpp_demo(
    headers: axum::http::HeaderMap,
    Json(body): Json<MppDemoReq>,
) -> Result<impl IntoResponse, StatusCode> {
    if let Some(auth) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if auth.starts_with("Payment ") {
            info!("MPP demo credential received");
            let body = axum::Json(serde_json::json!({
                "content": format!("Echo: {}", body.prompt),
                "method": "mpp",
                "status": "paid"
            }));
            return Ok(body.into_response());
        }
    }

    let challenge_id = uuid::Uuid::new_v4().to_string();
    let request_json = serde_json::json!({
        "amount": "0.01",
        "currency": "0x20c0000000000000000000000000000000000000",
        "recipient": "0x742d35Cc6634C0532925a3b844Bc9e7595f1B0F2"
    });
    let request_b64 = mpp::base64url_encode(request_json.to_string().as_bytes());

    let www_auth = format!(
        r#"Payment id="{}", method="tempo", intent="charge", request="{}""#,
        challenge_id, request_b64
    );

    let body = axum::Json(serde_json::json!({
        "error": "payment required",
        "type": "https://paymentauth.org/problems/payment-required"
    }));
    let mut resp = body.into_response();
    *resp.status_mut() = StatusCode::PAYMENT_REQUIRED;
    resp.headers_mut().insert(
        axum::http::header::WWW_AUTHENTICATE,
        www_auth.parse().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok(resp)
}

