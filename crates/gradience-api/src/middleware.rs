use axum::http::{header::AUTHORIZATION, StatusCode};
use std::sync::Arc;
use crate::state::{AppState, Session};

pub fn auth_token(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

pub async fn get_session(state: &AppState, token: &str) -> Option<Session> {
    state.sessions.get(token).await
}

pub async fn require_wallet_owner(
    state: &AppState,
    session: &Session,
    wallet_id: &str,
) -> Result<gradience_db::models::Wallet, StatusCode> {
    let wallet = gradience_db::queries::get_wallet_by_id(&state.db, wallet_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    if wallet.owner_id != session.user_id {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(wallet)
}

pub async fn require_workspace_member(
    state: &AppState,
    user_id: &str,
    workspace_id: &str,
) -> Result<String, StatusCode> {
    let members = gradience_db::queries::list_workspace_members(&state.db, workspace_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let m = members.into_iter().find(|m| m.user_id == user_id).ok_or(StatusCode::FORBIDDEN)?;
    Ok(m.role)
}
