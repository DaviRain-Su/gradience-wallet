use axum::{
    response::IntoResponse,
    routing::{delete, get, post},
    Router,
};
use rust_embed::RustEmbed;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use webauthn_rs::prelude::*;

mod handlers;
mod middleware;
mod state;

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
struct Assets;

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html"
    } else if path.ends_with(".js") {
        "application/javascript"
    } else if path.ends_with(".css") {
        "text/css"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".woff") || path.ends_with(".woff2") {
        "font/woff2"
    } else {
        "application/octet-stream"
    }
}

async fn static_handler(uri: axum::http::Uri) -> axum::response::Response {
    let path = uri.path().trim_start_matches('/');

    let (body, ct) = if let Some(file) = Assets::get(path) {
        (axum::body::Body::from(file.data.into_owned()), content_type(path))
    } else {
        let html_path = format!("{}.html", path);
        if let Some(file) = Assets::get(&html_path) {
            (axum::body::Body::from(file.data.into_owned()), "text/html")
        } else if let Some(file) = Assets::get("index.html") {
            (axum::body::Body::from(file.data.into_owned()), "text/html")
        } else {
            return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response();
        }
    };

    axum::response::Response::builder()
        .header("content-type", ct)
        .body(body)
        .unwrap()
}

use gradience_core::ows::local_adapter::LocalOwsAdapter;
use state::{AppState, Session, SessionStore};

pub async fn run() -> anyhow::Result<()> {
    let origin = std::env::var("ORIGIN").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let rp_id = std::env::var("RP_ID").unwrap_or_else(|_| "localhost".to_string());
    let origin_url: url::Url = origin.parse()?;
    let webauthn = WebauthnBuilder::new(&rp_id, &origin_url)?.build()?;

    let db_path = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./gradience.db?mode=rwc".to_string());
    let db = sqlx::SqlitePool::connect(&db_path).await?;

    let migrations_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/gradience-db/migrations");
    let migrator = sqlx::migrate::Migrator::new(migrations_path).await?;
    migrator.run(&db).await?;

    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    let vault_dir = data_dir.join("vault");
    std::fs::create_dir_all(&vault_dir)?;

    let risk_cache = gradience_core::policy::dynamic::RiskSignalCache::new();
    let db_clone = db.clone();
    let state = Arc::new(AppState {
        db,
        webauthn,
        ows: Arc::new(LocalOwsAdapter::new(vault_dir.clone())),
        vault_dir,
        reg_challenges: Mutex::new(HashMap::new()),
        auth_challenges: Mutex::new(HashMap::new()),
        credentials: Mutex::new(HashMap::new()),
        sessions: SessionStore::new(db_clone),
        recovery_sessions: Mutex::new(HashMap::new()),
        device_auths: Mutex::new(HashMap::new()),
        risk_cache: risk_cache.clone(),
    });

    if let Ok(count) = gradience_db::queries::delete_expired_sessions(&state.db).await {
        if count > 0 {
            info!("Cleaned up {} expired sessions", count);
        }
    }

    let use_mock_risk = std::env::var("USE_MOCK_RISK")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if use_mock_risk {
        tokio::spawn(gradience_core::policy::dynamic::mock_fetch_signals(
            risk_cache,
            std::env::var("RISK_FETCH_INTERVAL_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
        ));
    } else {
        tokio::spawn(gradience_core::policy::dynamic::fetch_signals(
            risk_cache,
            std::env::var("RISK_FETCH_INTERVAL_SEC")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
        ));
    }

    let app = Router::new()
        .route("/api/auth/email/send-code", post(handlers::email_send_code))
        .route("/api/auth/email/verify", post(handlers::email_verify_code))
        .route(
            "/api/auth/passkey/register/start",
            post(handlers::register_start),
        )
        .route(
            "/api/auth/passkey/register/finish",
            post(handlers::register_finish),
        )
        .route("/api/auth/passkey/login/start", post(handlers::login_start))
        .route(
            "/api/auth/passkey/login/finish",
            post(handlers::login_finish),
        )
        .route("/api/auth/unlock", post(handlers::unlock))
        .route("/api/auth/me", get(handlers::auth_me))
        .route("/api/auth/me/sessions", get(handlers::list_sessions))
        .route("/api/auth/sessions", delete(handlers::revoke_session))
        .route("/api/auth/logout", post(handlers::logout))
        .route(
            "/api/auth/recover/initiate",
            post(handlers::recover_initiate),
        )
        .route("/api/auth/recover/verify", post(handlers::recover_verify))
        .route(
            "/api/auth/recover/register",
            post(handlers::recover_register),
        )
        .route("/api/auth/device/initiate", post(handlers::device_initiate))
        .route("/api/auth/device/poll", post(handlers::device_poll))
        .route(
            "/api/auth/device/authorize",
            post(handlers::device_authorize),
        )
        .route(
            "/api/auth/oauth/:provider/start",
            get(handlers::oauth_start),
        )
        .route(
            "/api/auth/oauth/:provider/callback",
            get(handlers::oauth_callback),
        )
        .route(
            "/api/wallets",
            get(handlers::list_wallets).post(handlers::create_wallet),
        )
        .route("/api/wallets/:id/balance", get(handlers::wallet_balance))
        .route(
            "/api/wallets/:id/addresses",
            get(handlers::wallet_addresses),
        )
        .route(
            "/api/wallets/:id/portfolio",
            get(handlers::wallet_portfolio),
        )
        .route("/api/wallets/:id/fund", post(handlers::wallet_fund))
        .route("/api/wallets/:id/sign", post(handlers::wallet_sign))
        .route("/api/wallets/:id/swap", post(handlers::wallet_swap))
        .route(
            "/api/wallets/:id/transactions",
            get(handlers::wallet_transactions),
        )
        .route("/api/wallets/:id/audit/export", get(handlers::audit_export))
        .route(
            "/api/wallets/:id/audit/proof",
            get(handlers::wallet_audit_proof),
        )
        .route("/api/audit/verify", post(handlers::verify_audit_proof))
        .route("/api/wallets/:id/anchor", post(handlers::wallet_anchor))
        .route(
            "/api/wallets/:id/api-keys",
            get(handlers::list_api_keys).post(handlers::create_api_key),
        )
        .route(
            "/api/wallets/:id/api-keys/:key_id",
            delete(handlers::revoke_api_key),
        )
        .route(
            "/api/wallets/:id/policies",
            get(handlers::list_wallet_policies).post(handlers::create_policy),
        )
        .route(
            "/api/wallets/:id/payment-routes",
            get(handlers::list_payment_routes).post(handlers::set_payment_routes),
        )
        .route("/api/swap/quote", post(handlers::swap_quote))
        .route("/api/ai/generate", post(handlers::ai_generate))
        .route("/api/ai/balance/:wallet_id", get(handlers::ai_balance))
        .route("/api/payments", get(handlers::list_payments))
        .route("/api/ws", get(handlers::ws_handler))
        .route("/api/mpp/demo", post(handlers::mpp_demo))
        .route(
            "/api/mcp/sign_transaction",
            post(handlers::mcp_sign_transaction),
        )
        .route("/api/mcp/get_balance", post(handlers::mcp_get_balance))
        .route(
            "/api/workspaces",
            get(handlers::list_workspaces).post(handlers::create_workspace),
        )
        .route(
            "/api/workspaces/:id/policies",
            get(handlers::list_workspace_policies).post(handlers::create_workspace_policy),
        )
        .route(
            "/api/workspaces/:id/members",
            get(handlers::list_workspace_members).post(handlers::invite_workspace_member),
        )
        .route(
            "/api/policy-approvals",
            get(handlers::list_policy_approvals),
        )
        .route(
            "/api/policy-approvals/:id/approve",
            post(handlers::approve_policy_approval),
        )
        .route(
            "/api/policy-approvals/:id/reject",
            post(handlers::reject_policy_approval),
        )
        .route("/api/tg/webhook", post(handlers::tg_webhook))
        .route("/health", get(handlers::health_check))
        .layer({
            let origin = std::env::var("ORIGIN")
                .unwrap_or_else(|_| "https://wallets.gradiences.xyz".to_string());
            let allowed_headers = vec![
                axum::http::header::CONTENT_TYPE,
                axum::http::header::AUTHORIZATION,
                axum::http::header::ACCEPT,
            ];
            if origin.trim() == "*" {
                tower_http::cors::CorsLayer::new()
                    .allow_origin(tower_http::cors::Any)
                    .allow_methods(tower_http::cors::Any)
                    .allow_headers(allowed_headers)
            } else {
                let origins: Vec<axum::http::HeaderValue> = vec![
                    origin
                        .parse()
                        .unwrap_or_else(|_| "https://wallets.gradiences.xyz".parse().unwrap()),
                    "http://localhost:3000".parse().unwrap(),
                ];
                tower_http::cors::CorsLayer::new()
                    .allow_origin(tower_http::cors::AllowOrigin::list(origins))
                    .allow_methods(tower_http::cors::Any)
                    .allow_headers(allowed_headers)
            }
        })
        .fallback(static_handler)
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
                    match gradience_db::queries::list_unanchored_logs(&anchor_state.db, 1000).await
                    {
                        Ok(logs) if !logs.is_empty() => {
                            let mut seen = std::collections::HashSet::new();
                            for log in logs {
                                if seen.insert(log.wallet_id.clone()) {
                                    match svc
                                        .anchor_unanchored_logs(
                                            &anchor_state.db,
                                            &log.wallet_id,
                                            100,
                                        )
                                        .await
                                    {
                                        Ok(Some(tx_hash)) => info!(
                                            "Auto-anchored wallet {} tx {}",
                                            log.wallet_id, tx_hash
                                        ),
                                        Ok(None) => {}
                                        Err(e) => {
                                            warn!("Auto-anchor failed for {}: {}", log.wallet_id, e)
                                        }
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
    info!(
        "ANCHOR_INTERVAL_SEC : {}s",
        std::env::var("ANCHOR_INTERVAL_SEC").unwrap_or_else(|_| "300".into())
    );
    match gradience_core::audit::anchor::AnchorService::from_env() {
        Ok(Some(_)) => info!("Anchor Service      : enabled (contract ready)"),
        Ok(None) => warn!("Anchor Service      : disabled (missing ANCHOR_RPC_URL)"),
        Err(e) => warn!("Anchor Service      : config error ({})", e),
    }
    info!("CORS                : allow_any=true");
    info!("============================================================");

    // SECURITY NOTE: GRADIENCE_DEMO_TOKEN is for local demonstration only.
    // It injects a hardcoded session into memory and must NEVER be enabled in production.
    if let Ok(demo_token) = std::env::var("GRADIENCE_DEMO_TOKEN") {
        warn!("DEMO MODE ENABLED — do not deploy with GRADIENCE_DEMO_TOKEN in production");
        let demo_pass = std::env::var("GRADIENCE_DEMO_PASSPHRASE")
            .unwrap_or_else(|_| "demo-passphrase-123".into());
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);
        state
            .sessions
            .insert(
                demo_token,
                Session {
                    user_id: "user-1".into(),
                    username: "demo@gradience.io".into(),
                    passphrase: Some(demo_pass),
                },
                expires_at,
            )
            .await;
        info!("Demo session        : injected for user-1");
    }

    axum::serve(listener, app).await?;
    Ok(())
}
