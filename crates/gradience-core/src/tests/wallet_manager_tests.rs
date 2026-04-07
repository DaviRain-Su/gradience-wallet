use crate::error::GradienceError;
use crate::identity::api_key::{ApiKeyService, ApiKeyDescriptor};
use crate::wallet::service::WalletManagerService;
use crate::ows::adapter::{OwsAdapter, Transaction, PolicyAction, DerivationParams};
use crate::ows::vault::VaultHandle;
use std::path::Path;

struct TestOwsAdapter;

#[async_trait::async_trait]
impl OwsAdapter for TestOwsAdapter {
    async fn init_vault(
        &self, _passphrase: &str,
    ) -> Result<VaultHandle, GradienceError> {
        Ok(VaultHandle { passphrase: _passphrase.to_string() })
    }

    async fn register_policy_executable(
        &self, _vault: &VaultHandle, _name: &str, _path: &Path, _action: PolicyAction,
    ) -> Result<String, GradienceError> {
        Ok("policy-123".into())
    }

    async fn attach_api_key_and_policies(
        &self, _vault: &VaultHandle, _wallet_id: &str, api_key_name: &str, _policy_ids: Vec<String>,
    ) -> Result<crate::ows::adapter::GradienceApiKey, GradienceError> {
        let token = format!("ows_key_{:064x}", 123456789u64);
        use sha3::Digest;
        Ok(crate::ows::adapter::GradienceApiKey {
            id: "key-1".into(),
            name: api_key_name.into(),
            raw_token: Some(token.clone()),
            token_hash: format!("{:x}", sha3::Sha3_256::digest(token.as_bytes())),
            wallet_ids: vec!["wallet-1".into()],
            policy_ids: vec![],
            expires_at: None,
        })
    }

    async fn create_wallet(
        &self, _vault: &VaultHandle, name: &str, _params: DerivationParams,
    ) -> Result<crate::wallet::manager::WalletDescriptor, GradienceError> {
        Ok(crate::wallet::manager::WalletDescriptor {
            id: "wallet-1".into(),
            name: name.into(),
            accounts: vec![crate::wallet::manager::AccountDescriptor {
                account_id: "acc-1".into(),
                address: "0xabc".into(),
                chain_id: "eip155:8453".into(),
                derivation_path: "m/44'/60'/0'/0/0".into(),
            }],
        })
    }

    async fn sign_transaction(
        &self, _vault: &VaultHandle, _wallet_id: &str, chain: &str, tx: &Transaction, _credential: &str,
    ) -> Result<crate::ows::adapter::SignedTransaction, GradienceError> {
        Ok(crate::ows::adapter::SignedTransaction {
            raw_hex: format!("signed_{}", tx.raw_hex),
            chain_id: chain.into(),
        })
    }

    async fn broadcast(
        &self, _chain: &str, signed_tx: &crate::ows::adapter::SignedTransaction, _rpc_url: &str,
    ) -> Result<String, GradienceError> {
        Ok(format!("tx_hash_for_{}", signed_tx.raw_hex))
    }

    async fn revoke_api_key(
        &self, _vault: &VaultHandle, _api_key_id: &str,
    ) -> Result<(), GradienceError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_wallet_manager_create_success() {
    let svc = WalletManagerService::new();
    let adapter = TestOwsAdapter;
    let vault = adapter.init_vault("test-pass-123").await.unwrap();
    let wallet = svc.create_wallet(&adapter, &vault, "my-wallet").await.unwrap();
    assert_eq!(wallet.name, "my-wallet");
    assert_eq!(wallet.id, "wallet-1");
}

#[tokio::test]
async fn test_wallet_manager_empty_name_error() {
    let svc = WalletManagerService::new();
    let adapter = TestOwsAdapter;
    let vault = adapter.init_vault("test-pass-123").await.unwrap();
    let err = svc.create_wallet(&adapter, &vault, "  ").await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidCredential(_)));
}

#[tokio::test]
async fn test_api_key_format() {
    let svc = ApiKeyService::new();
    let key = svc.create_key("wallet-1", "claude-code").await.unwrap();
    let token = key.raw_token.as_ref().unwrap();
    assert!(token.starts_with("ows_key_"));
    assert_eq!(token.len(), 8 + 32); // prefix + 32 hex chars from uuid simple
}

#[tokio::test]
async fn test_api_key_verify_hash_success() {
    let svc = ApiKeyService::new();
    let key = svc.create_key("wallet-1", "claude-code").await.unwrap();
    let raw = key.raw_token.as_ref().unwrap();
    let ok = svc.verify_key(raw, &key).await.unwrap();
    assert!(ok);
}

#[tokio::test]
async fn test_api_key_verify_wrong_token_fails() {
    let svc = ApiKeyService::new();
    let key = svc.create_key("wallet-1", "claude-code").await.unwrap();
    let ok = svc.verify_key("wrong-token", &key).await.unwrap();
    assert!(!ok);
}

#[tokio::test]
async fn test_api_key_empty_name_error() {
    let svc = ApiKeyService::new();
    let err = svc.create_key("wallet-1", "  ").await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidCredential(_)));
}
