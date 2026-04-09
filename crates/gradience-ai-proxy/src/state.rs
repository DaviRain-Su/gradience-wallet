use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Session {
    pub user_id: String,
    #[allow(dead_code)]
    pub username: String,
    #[allow(dead_code)]
    pub passphrase: Option<String>,
}

#[derive(Clone)]
pub struct SessionStore {
    cache: Arc<Mutex<HashMap<String, Session>>>,
    db: Pool<Sqlite>,
}

impl SessionStore {
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
            db,
        }
    }

    pub async fn get(&self, token: &str) -> Option<Session> {
        if let Some(s) = self.cache.lock().await.get(token).cloned() {
            return Some(s);
        }
        let row = gradience_db::queries::get_session_by_token(&self.db, token)
            .await
            .ok()
            .flatten()?;
        let session = Session {
            user_id: row.0,
            username: row.1,
            passphrase: row.2,
        };
        self.cache
            .lock()
            .await
            .insert(token.to_string(), session.clone());
        Some(session)
    }
}

/// Latest verified state update for a channel (payee-side tracker).
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct VerifiedState {
    pub nonce: u64,
    pub amount: u128,
    pub signature: [u8; 65],
}

pub struct AppState {
    pub db: Pool<Sqlite>,
    pub sessions: SessionStore,
    #[allow(dead_code)]
    pub ows: Arc<gradience_core::ows::local_adapter::LocalOwsAdapter>,
    #[allow(dead_code)]
    pub vault_dir: std::path::PathBuf,
    pub state_channels: Arc<std::sync::Mutex<HashMap<[u8; 32], VerifiedState>>>,
}
