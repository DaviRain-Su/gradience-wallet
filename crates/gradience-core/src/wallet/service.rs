use crate::wallet::manager::{WalletDescriptor, AccountDescriptor};
use crate::ows::adapter::{OwsAdapter, DerivationParams};
use crate::ows::vault::VaultHandle;
use crate::error::{GradienceError, Result};
use sqlx::{Pool, Sqlite};

pub struct WalletManagerService;

impl WalletManagerService {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_wallet<A: OwsAdapter>(
        &self,
        adapter: &A,
        vault: &VaultHandle,
        name: &str,
    ) -> Result<WalletDescriptor> {
        if name.trim().is_empty() {
            return Err(GradienceError::InvalidCredential("wallet name cannot be empty".into()));
        }
        adapter.create_wallet(vault, name, DerivationParams::default()).await
    }

    pub async fn activate_wallet(&self, db: &Pool<Sqlite>, wallet_id: &str) -> Result<()> {
        gradience_db::queries::update_wallet_status(db, wallet_id, "active")
            .await
            .map_err(|e| GradienceError::Database(e.to_string()))
    }

    pub async fn suspend_wallet(&self, db: &Pool<Sqlite>, wallet_id: &str) -> Result<()> {
        gradience_db::queries::update_wallet_status(db, wallet_id, "suspended")
            .await
            .map_err(|e| GradienceError::Database(e.to_string()))
    }

    pub async fn revoke_wallet(&self, db: &Pool<Sqlite>, wallet_id: &str) -> Result<()> {
        gradience_db::queries::update_wallet_status(db, wallet_id, "revoked")
            .await
            .map_err(|e| GradienceError::Database(e.to_string()))
    }

    pub async fn derive_hd_account<A: OwsAdapter>(
        &self,
        adapter: &A,
        db: &Pool<Sqlite>,
        vault: &VaultHandle,
        wallet_id: &str,
        chain: &str,
        derivation_path: &str,
    ) -> Result<AccountDescriptor> {
        self.require_status_active(db, wallet_id).await?;
        let account = adapter.derive_account(vault, wallet_id, chain, derivation_path).await?;
        let addr_id = uuid::Uuid::new_v4().to_string();
        gradience_db::queries::create_wallet_address(
            db,
            &addr_id,
            wallet_id,
            chain,
            &account.address,
            derivation_path,
        )
        .await
        .map_err(|e| GradienceError::Database(e.to_string()))?;
        Ok(account)
    }

    pub async fn require_status_active(
        &self,
        db: &Pool<Sqlite>,
        wallet_id: &str,
    ) -> Result<()> {
        let wallet = gradience_db::queries::get_wallet_by_id(db, wallet_id)
            .await
            .map_err(|e| GradienceError::Database(e.to_string()))?;
        match wallet {
            Some(w) if w.status == "active" => Ok(()),
            Some(w) => Err(GradienceError::InvalidCredential(format!(
                "wallet status is '{}', operation not allowed",
                w.status
            ))),
            None => Err(GradienceError::WalletNotFound(format!(
                "wallet {} not found",
                wallet_id
            ))),
        }
    }
}
