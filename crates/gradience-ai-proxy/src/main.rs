use axum::{
    routing::{any, delete, get, post},
    Router,
};
use std::sync::Arc;
use tracing::info;

mod handlers;
mod middleware;
mod state;

use gradience_core::ows::local_adapter::LocalOwsAdapter;
use state::{AppState, SessionStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let db_path = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./gradience.db?mode=rwc".to_string());
    let db = sqlx::SqlitePool::connect(&db_path).await?;

    let data_dir = std::env::var("GRADIENCE_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    let vault_dir = data_dir.join("vault");
    std::fs::create_dir_all(&vault_dir)?;

    let state = Arc::new(AppState {
        db: db.clone(),
        sessions: SessionStore::new(db),
        ows: Arc::new(LocalOwsAdapter::new(vault_dir.clone())),
        vault_dir,
    });

    let app = Router::new()
        .route(
            "/api/ai/proxy-keys",
            post(handlers::create_ai_proxy_key).get(handlers::list_ai_proxy_keys),
        )
        .route(
            "/api/ai/proxy-keys/:key_id",
            delete(handlers::delete_ai_proxy_key),
        )
        .route("/v1/proxy/:provider/*path", any(handlers::ai_proxy_handler))
        .route("/health", get(|| async { "OK" }))
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
        .with_state(Arc::clone(&state));

    let bind_addr = std::env::var("AI_PROXY_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".into());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    info!("============================================================");
    info!("Gradience AI Proxy v{}", std::env!("CARGO_PKG_VERSION"));
    info!("============================================================");
    info!("Listening on        : {}", listener.local_addr()?);
    info!("DATABASE_URL        : {}", db_path);
    info!("GRADIENCE_DATA_DIR  : {}", data_dir.display());
    info!("CORS                : allow_any=true");
    info!("============================================================");

    axum::serve(listener, app).await?;
    Ok(())
}
