use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use webauthn_rs::prelude::*;

#[derive(Clone)]
pub struct RecoverySession {
    pub user_id: String,
    pub username: String,
}

#[derive(Clone)]
pub struct DeviceAuth {
    pub user_code: String,
    pub token: Option<String>,
}

#[derive(Clone)]
pub struct Session {
    pub user_id: String,
    pub username: String,
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

    pub async fn insert(
        &self,
        token: String,
        session: Session,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) {
        let _ = gradience_db::queries::create_session(
            &self.db,
            &token,
            &session.user_id,
            &session.username,
            session.passphrase.as_deref(),
            expires_at,
        )
        .await;
        self.cache.lock().await.insert(token, session);
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
        self.cache.lock().await.insert(token.to_string(), session.clone());
        Some(session)
    }

    pub async fn update_passphrase(&self, token: &str, passphrase: String) -> bool {
        if gradience_db::queries::update_session_passphrase(&self.db, token, &passphrase)
            .await
            .is_err()
        {
            return false;
        }
        if let Some(s) = self.cache.lock().await.get_mut(token) {
            s.passphrase = Some(passphrase);
            true
        } else {
            false
        }
    }

    pub async fn remove(&self, token: &str) {
        let _ = gradience_db::queries::delete_session(&self.db, token).await;
        self.cache.lock().await.remove(token);
    }
}

pub struct AppState {
    pub db: Pool<Sqlite>,
    pub webauthn: Webauthn,
    pub ows: Arc<gradience_core::ows::local_adapter::LocalOwsAdapter>,
    pub vault_dir: std::path::PathBuf,
    pub reg_challenges: Mutex<HashMap<String, PasskeyRegistration>>,
    pub auth_challenges: Mutex<HashMap<String, PasskeyAuthentication>>,
    pub credentials: Mutex<HashMap<String, Passkey>>,
    pub sessions: SessionStore,
    pub recovery_sessions: Mutex<HashMap<String, RecoverySession>>,
    pub device_auths: Mutex<HashMap<String, DeviceAuth>>,
    pub risk_cache: gradience_core::policy::dynamic::RiskSignalCache,
}
