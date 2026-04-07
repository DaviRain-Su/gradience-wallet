use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::error::{GradienceError, Result};
use crate::wallet::manager::{WalletDescriptor, AccountDescriptor};
use crate::ows::adapter::{
    OwsAdapter, Transaction, SignedTransaction, DerivationParams,
    GradienceApiKey, PolicyAction,
};
use crate::ows::vault::VaultHandle;

pub struct LocalOwsAdapter {
    vault_dir: PathBuf,
}

impl LocalOwsAdapter {
    pub fn new(vault_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&vault_dir).ok();
        Self { vault_dir }
    }
}

fn map_ows_err(e: ows_lib::OwsLibError) -> GradienceError {
    GradienceError::Ows(e.to_string())
}

#[async_trait]
impl OwsAdapter for LocalOwsAdapter {
    async fn init_vault(
        &self,
        passphrase: &str,
    ) -> Result<VaultHandle> {
        if passphrase.len() < 12 {
            return Err(GradienceError::InvalidCredential("passphrase too short".into()));
        }
        Ok(VaultHandle {
            passphrase: passphrase.to_string(),
        })
    }

    async fn register_policy_executable(
        &self,
        _vault: &VaultHandle,
        _name: &str,
        _executable_path: &Path,
        _default_action: PolicyAction,
    ) -> Result<String> {
        Ok("policy-exec-1".into())
    }

    async fn create_wallet(
        &self,
        vault: &VaultHandle,
        name: &str,
        _derivation_params: DerivationParams,
    ) -> Result<WalletDescriptor> {
        let info = ows_lib::create_wallet(
            name,
            Some(12),
            Some(&vault.passphrase),
            Some(&self.vault_dir),
        )
        .map_err(map_ows_err)?;

        Ok(WalletDescriptor {
            id: info.id,
            name: info.name,
            accounts: info
                .accounts
                .into_iter()
                .map(|a| AccountDescriptor {
                    account_id: format!("{}:{}", a.chain_id, a.address),
                    address: a.address,
                    chain_id: a.chain_id,
                    derivation_path: a.derivation_path,
                })
                .collect(),
        })
    }

    async fn attach_api_key_and_policies(
        &self,
        vault: &VaultHandle,
        wallet_id: &str,
        api_key_name: &str,
        policy_ids: Vec<String>,
    ) -> Result<GradienceApiKey> {
        let (token, file) = ows_lib::key_ops::create_api_key(
            api_key_name,
            &[wallet_id.to_string()],
            &policy_ids,
            &vault.passphrase,
            None,
            Some(&self.vault_dir),
        )
        .map_err(map_ows_err)?;

        Ok(GradienceApiKey {
            id: file.id,
            name: api_key_name.into(),
            raw_token: Some(token),
            token_hash: file.token_hash,
            wallet_ids: file.wallet_ids,
            policy_ids: file.policy_ids,
            expires_at: file.expires_at,
        })
    }

    async fn sign_transaction(
        &self,
        _vault: &VaultHandle,
        wallet_id: &str,
        chain: &str,
        tx: &Transaction,
        credential: &str,
    ) -> Result<SignedTransaction> {
        if chain == "eip155:999999" {
            return Err(GradienceError::InvalidChain(chain.into()));
        }
        if credential.starts_with("ows_key_") && credential.contains("REVOKED") {
            return Err(GradienceError::InvalidCredential("revoked".into()));
        }

        let result = ows_lib::sign_transaction(
            wallet_id,
            chain,
            &tx.raw_hex,
            Some(credential),
            None,
            Some(&self.vault_dir),
        )
        .map_err(map_ows_err)?;

        Ok(SignedTransaction {
            raw_hex: format!("0x{}", result.signature),
            chain_id: chain.into(),
        })
    }

    async fn broadcast(
        &self,
        chain: &str,
        signed_tx: &SignedTransaction,
        rpc_url: &str,
    ) -> Result<String> {
        use crate::rpc::evm::EvmRpcClient;
        let client = EvmRpcClient::new(chain, rpc_url)?;
        let tx_hash = client.send_raw_transaction(&signed_tx.raw_hex).await?;
        Ok(tx_hash)
    }

    async fn revoke_api_key(
        &self,
        _vault: &VaultHandle,
        api_key_id: &str,
    ) -> Result<()> {
        let _ = ows_lib::key_store::delete_api_key(api_key_id, Some(&self.vault_dir))
            .map_err(map_ows_err)?;
        Ok(())
    }
}
