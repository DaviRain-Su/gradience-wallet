use async_trait::async_trait;
use std::path::Path;

use super::vault::VaultHandle;
use crate::error::GradienceError;
use crate::wallet::manager::{AccountDescriptor, WalletDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Local,
    Cloud,
    Hardware,
    Remote,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub to: Option<String>,
    pub value: String,
    pub data: Vec<u8>,
    pub raw_hex: String,
}

#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub raw_hex: String,
    pub chain_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct DerivationParams {
    pub account_index: u32,
}

#[derive(Debug, Clone)]
pub struct GradienceApiKey {
    pub id: String,
    pub name: String,
    pub raw_token: Option<String>,
    pub token_hash: String,
    pub wallet_ids: Vec<String>,
    pub policy_ids: Vec<String>,
    pub expires_at: Option<String>,
}

#[async_trait]
pub trait OwsAdapter: Send + Sync {
    fn adapter_kind(&self) -> AdapterKind;

    async fn init_vault(&self, passphrase: &str) -> Result<VaultHandle, GradienceError>;

    async fn register_policy_executable(
        &self,
        vault: &VaultHandle,
        name: &str,
        executable_path: &Path,
        default_action: PolicyAction,
    ) -> Result<String, GradienceError>;

    async fn attach_api_key_and_policies(
        &self,
        vault: &VaultHandle,
        wallet_id: &str,
        api_key_name: &str,
        policy_ids: Vec<String>,
    ) -> Result<GradienceApiKey, GradienceError>;

    async fn create_wallet(
        &self,
        vault: &VaultHandle,
        name: &str,
        derivation_params: DerivationParams,
    ) -> Result<WalletDescriptor, GradienceError>;

    /// Derive a new HD account for an existing wallet.
    /// Default implementation returns an error (adapters that don't support HD derivation).
    async fn derive_account(
        &self,
        _vault: &VaultHandle,
        _wallet_id: &str,
        _chain: &str,
        _derivation_path: &str,
    ) -> Result<AccountDescriptor, GradienceError> {
        Err(GradienceError::InvalidConfig(
            "derive_account not supported by this adapter".into(),
        ))
    }

    async fn sign_transaction(
        &self,
        vault: &VaultHandle,
        wallet_id: &str,
        chain: &str,
        tx: &Transaction,
        credential: &str,
    ) -> Result<SignedTransaction, GradienceError>;

    async fn broadcast(
        &self,
        chain: &str,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<String, GradienceError>;

    async fn revoke_api_key(
        &self,
        vault: &VaultHandle,
        api_key_id: &str,
    ) -> Result<(), GradienceError>;
}
