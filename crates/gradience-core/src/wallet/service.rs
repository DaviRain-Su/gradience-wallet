use crate::wallet::manager::{WalletDescriptor, AccountDescriptor};
use crate::ows::adapter::{OwsAdapter, DerivationParams};
use crate::ows::vault::VaultHandle;
use crate::error::{GradienceError, Result};

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
}
