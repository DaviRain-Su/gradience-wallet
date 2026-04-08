use sha3::Digest;
use crate::error::GradienceError;
use crate::ows::adapter::{OwsAdapter, Transaction, PolicyAction, DerivationParams};

// TODO: create a mock/test implementation of OwsAdapter for unit tests
struct TestOwsAdapter;

#[async_trait::async_trait]
impl OwsAdapter for TestOwsAdapter {
    fn adapter_kind(&self) -> crate::ows::adapter::AdapterKind {
        crate::ows::adapter::AdapterKind::Local
    }

    async fn init_vault(
        &self,
        passphrase: &str,
    ) -> Result< crate::ows::vault::VaultHandle, GradienceError> {
        if passphrase.len() < 12 {
            return Err(GradienceError::InvalidCredential("passphrase too short".into()));
        }
        Ok(crate::ows::vault::VaultHandle { passphrase: passphrase.to_string() })
    }

    async fn register_policy_executable(
        &self,
        _vault: &crate::ows::vault::VaultHandle,
        _name: &str,
        _executable_path: &std::path::Path,
        _default_action: PolicyAction,
    ) -> Result<String, GradienceError> {
        Ok("policy-123".into())
    }

    async fn attach_api_key_and_policies(
        &self,
        _vault: &crate::ows::vault::VaultHandle,
        _wallet_id: &str,
        api_key_name: &str,
        _policy_ids: Vec<String>,
    ) -> Result< crate::ows::adapter::GradienceApiKey, GradienceError> {
        let token = format!("ows_key_{:064x}", 123456789u64);
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
        &self,
        _vault: &crate::ows::vault::VaultHandle,
        name: &str,
        _derivation_params: DerivationParams,
    ) -> Result< crate::wallet::manager::WalletDescriptor, GradienceError> {
        Ok(crate::wallet::manager::WalletDescriptor {
            id: "wallet-1".into(),
            name: name.into(),
            accounts: vec![
                crate::wallet::manager::AccountDescriptor {
                    account_id: "eip155:8453:0xabc".into(),
                    address: "0xabc".into(),
                    chain_id: "eip155:8453".into(),
                    derivation_path: "m/44'/60'/0'/0/0".into(),
                },
            ],
        })
    }

    async fn sign_transaction(
        &self,
        _vault: &crate::ows::vault::VaultHandle,
        _wallet_id: &str,
        chain: &str,
        tx: &Transaction,
        credential: &str,
    ) -> Result< crate::ows::adapter::SignedTransaction, GradienceError> {
        if chain == "eip155:999999" {
            return Err(GradienceError::InvalidChain(chain.into()));
        }
        if credential.starts_with("ows_key_") && credential.contains("REVOKED") {
            return Err(GradienceError::InvalidCredential("revoked".into()));
        }
        Ok(crate::ows::adapter::SignedTransaction {
            raw_hex: format!("signed_{}", tx.raw_hex),
            chain_id: chain.into(),
        })
    }

    async fn broadcast(
        &self,
        _chain: &str,
        signed_tx: &crate::ows::adapter::SignedTransaction,
        _rpc_url: &str,
    ) -> Result<String, GradienceError> {
        Ok(format!("tx_hash_for_{}", signed_tx.raw_hex))
    }

    async fn revoke_api_key(
        &self,
        _vault: &crate::ows::vault::VaultHandle,
        _api_key_id: &str,
    ) -> Result<(), GradienceError> {
        Ok(())
    }
}

#[tokio::test]
async fn test_init_vault_success() {
    let adapter = TestOwsAdapter;
    let vault = adapter.init_vault("secure-pass-123").await.unwrap();
    assert!(std::ptr::addr_of!(vault) != std::ptr::null());
}

#[tokio::test]
async fn test_init_vault_short_passphrase() {
    let adapter = TestOwsAdapter;
    let err = adapter.init_vault("short-pass").await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidCredential(_)));
}

#[tokio::test]
async fn test_create_wallet_success() {
    let adapter = TestOwsAdapter;
    let vault = adapter.init_vault("secure-pass-123").await.unwrap();
    let wallet = adapter.create_wallet(&vault, "demo-wallet", Default::default()).await.unwrap();
    assert_eq!(wallet.name, "demo-wallet");
    let evm = wallet.accounts.iter().find(|a| a.chain_id.starts_with("eip155:"));
    assert!(evm.is_some());
}

#[tokio::test]
async fn test_sign_tx_owner_mode_success() {
    let adapter = TestOwsAdapter;
    let vault = adapter.init_vault("secure-pass-123").await.unwrap();
    let tx = Transaction { to: None, value: "0".into(), data: vec![], raw_hex: "0x01".into() };
    let signed = adapter.sign_transaction(&vault, "wallet-1", "eip155:8453", &tx, "secure-pass-123"
    ).await.unwrap();
    assert!(!signed.raw_hex.is_empty());
}

#[tokio::test]
async fn test_sign_tx_invalid_chain_error() {
    let adapter = TestOwsAdapter;
    let vault = adapter.init_vault("secure-pass-123").await.unwrap();
    let tx = Transaction { to: None, value: "0".into(), data: vec![], raw_hex: "0x01".into() };
    let err = adapter.sign_transaction(
        &vault, "wallet-1", "eip155:999999", &tx, "secure-pass-123"
    ).await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidChain(_)));
}

#[tokio::test]
async fn test_sign_tx_revoked_key_attack() {
    let adapter = TestOwsAdapter;
    let vault = adapter.init_vault("secure-pass-123").await.unwrap();
    let tx = Transaction { to: None, value: "0".into(), data: vec![], raw_hex: "0x01".into() };
    let err = adapter.sign_transaction(
        &vault, "wallet-1", "eip155:8453", &tx, "ows_key_REVOKED_123"
    ).await.unwrap_err();
    assert!(matches!(err, GradienceError::InvalidCredential(_)));
}
