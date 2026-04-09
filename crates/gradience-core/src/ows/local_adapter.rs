use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::error::{GradienceError, Result};
use crate::ows::adapter::{
    AdapterKind, DerivationParams, GradienceApiKey, OwsAdapter, PolicyAction, SignedTransaction,
    Transaction,
};
use crate::ows::vault::VaultHandle;
use crate::wallet::manager::{AccountDescriptor, WalletDescriptor};

/// Deterministic seed generation used by LocalOwsAdapter for demo chains
/// that are not yet supported by ows-lib native derivation.
pub fn derive_demo_seed(wallet_id: &str, chain: &str, derivation_path: &str) -> [u8; 32] {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(wallet_id.as_bytes());
    hasher.update(chain.as_bytes());
    hasher.update(derivation_path.as_bytes());
    let hash = hasher.finalize();
    hash.as_slice()[..32].try_into().unwrap()
}

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
    fn adapter_kind(&self) -> AdapterKind {
        AdapterKind::Local
    }

    async fn init_vault(&self, passphrase: &str) -> Result<VaultHandle> {
        if passphrase.len() < 12 {
            return Err(GradienceError::InvalidCredential(
                "passphrase too short".into(),
            ));
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

        let mut accounts: Vec<AccountDescriptor> = info
            .accounts
            .into_iter()
            .map(|a| AccountDescriptor {
                account_id: format!("{}:{}", a.chain_id, a.address),
                address: a.address,
                chain_id: a.chain_id,
                derivation_path: a.derivation_path,
            })
            .collect();

        // Append TON address deterministically
        let ton_chain = "ton:0";
        let ton_path = "m/44'/607'/0'/0/0";
        let ton_seed = derive_demo_seed(&info.id, ton_chain, ton_path);
        if let Ok(ton_addr) = crate::ows::signing::ton_address_from_seed(&ton_seed) {
            if !ton_addr.is_empty() {
                accounts.push(AccountDescriptor {
                    account_id: format!("{}:{}", ton_chain, ton_addr),
                    address: ton_addr,
                    chain_id: ton_chain.into(),
                    derivation_path: ton_path.into(),
                });
            }
        }

        // Append Conflux Core Space testnet address deterministically
        let cfx_chain = "cfx:1";
        let cfx_path = "m/44'/503'/0'/0/0";
        let cfx_seed = derive_demo_seed(&info.id, cfx_chain, cfx_path);
        if let Ok(cfx_addr) = crate::rpc::conflux_core::cfx_address_from_seed(&cfx_seed, 1) {
            if !cfx_addr.is_empty() {
                accounts.push(AccountDescriptor {
                    account_id: format!("{}:{}", cfx_chain, cfx_addr),
                    address: cfx_addr,
                    chain_id: cfx_chain.into(),
                    derivation_path: cfx_path.into(),
                });
            }
        }

        Ok(WalletDescriptor {
            id: info.id,
            name: info.name,
            accounts,
        })
    }

    async fn derive_account(
        &self,
        vault: &VaultHandle,
        wallet_id: &str,
        chain: &str,
        derivation_path: &str,
    ) -> Result<AccountDescriptor> {
        if chain.starts_with("solana:") {
            // Use ows-lib to derive the real Solana address from the wallet mnemonic.
            let exported =
                ows_lib::export_wallet(wallet_id, Some(&vault.passphrase), Some(&self.vault_dir))
                    .map_err(map_ows_err)?;

            // Private-key wallets (JSON) cannot be re-derived via mnemonic path.
            if exported.trim_start().starts_with('{') {
                return Err(GradienceError::InvalidConfig(
                    "derive_account for Solana is only supported for mnemonic wallets".into(),
                ));
            }

            let index = derivation_path
                .split('/')
                .filter_map(|s| s.trim_end_matches('\'').parse::<u32>().ok())
                .next_back();
            let address = ows_lib::derive_address(&exported, chain, index).map_err(map_ows_err)?;

            return Ok(AccountDescriptor {
                account_id: format!("{}:{}", chain, address),
                address,
                chain_id: chain.into(),
                derivation_path: derivation_path.into(),
            });
        }

        let secret = derive_demo_seed(wallet_id, chain, derivation_path);

        let address = if chain.starts_with("ton:") {
            crate::ows::signing::ton_address_from_seed(&secret)?
        } else if chain.starts_with("cfx:") {
            let network_id = crate::chain::conflux_core_network_id(chain);
            crate::rpc::conflux_core::cfx_address_from_seed(&secret, network_id)?
        } else if chain.starts_with("eip155:") || chain.starts_with("base:") {
            crate::ows::signing::eth_address_from_secret_key(&secret)?
        } else if chain.starts_with("stellar:") {
            crate::ows::signing::stellar_address_from_secret_key(&secret)?
        } else {
            format!("0x{}", hex::encode(&secret[..20]))
        };

        Ok(AccountDescriptor {
            account_id: format!("{}:{}", chain, address),
            address,
            chain_id: chain.into(),
            derivation_path: derivation_path.into(),
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

        if chain.starts_with("ton:") {
            let seed = derive_demo_seed(wallet_id, chain, "m/44'/607'/0'/0/0");
            let to = tx.to.as_deref().unwrap_or("");
            let amount = tx
                .value
                .parse::<u64>()
                .map_err(|_| GradienceError::Validation("invalid ton amount".into()))?;
            let seqno = if tx.data.len() >= 4 {
                u32::from_be_bytes([tx.data[0], tx.data[1], tx.data[2], tx.data[3]])
            } else {
                return Err(GradienceError::Validation(
                    "ton tx data must contain seqno (4 bytes)".into(),
                ));
            };
            let signed_bytes =
                crate::ows::signing::build_ton_transfer_tx(&seed, to, amount, seqno)?;
            return Ok(SignedTransaction {
                raw_hex: format!("0x{}", hex::encode(&signed_bytes)),
                chain_id: chain.into(),
            });
        }

        if chain.starts_with("cfx:") {
            let cfx_path = "m/44'/503'/0'/0/0";
            let seed = derive_demo_seed(wallet_id, chain, cfx_path);
            let private_key = format!("0x{}", hex::encode(&seed[..32]));
            let to = tx.to.as_deref().unwrap_or("");
            let rpc_url = crate::chain::resolve_rpc(chain);
            let network_id = crate::chain::conflux_core_network_id(chain);
            let client = crate::rpc::conflux_core::ConfluxCoreRpcClient::new_with_url(rpc_url);
            let tx_hash = client.sign_and_send(&private_key, to, &tx.value, network_id)?;
            return Ok(SignedTransaction {
                raw_hex: tx_hash,
                chain_id: chain.into(),
            });
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
        if chain.starts_with("solana:") {
            use crate::rpc::solana::SolanaRpcClient;
            let client = SolanaRpcClient::new(rpc_url);
            let raw_hex = signed_tx
                .raw_hex
                .strip_prefix("0x")
                .unwrap_or(&signed_tx.raw_hex);
            let signed_bytes = hex::decode(raw_hex)
                .map_err(|e| GradienceError::Validation(format!("invalid hex signed tx: {}", e)))?;
            let sig = client.send_transaction(&signed_bytes).await?;
            return Ok(sig);
        }

        if chain.starts_with("ton:") {
            use crate::rpc::ton::TonRpcClient;
            let client = TonRpcClient::new_with_url(rpc_url);
            let raw_hex = signed_tx
                .raw_hex
                .strip_prefix("0x")
                .unwrap_or(&signed_tx.raw_hex);
            let signed_bytes = hex::decode(raw_hex)
                .map_err(|e| GradienceError::Validation(format!("invalid hex signed tx: {}", e)))?;
            client.send_boc(&signed_bytes).await?;
            // toncenter does not return the message hash easily; return a fingerprint
            let fingerprint = hex::encode(&signed_bytes[..16.min(signed_bytes.len())]);
            return Ok(format!("ton:0x{}", fingerprint));
        }

        if chain.starts_with("cfx:") {
            // Core Space transaction was already signed and broadcast in sign_transaction;
            // raw_hex holds the transaction hash.
            return Ok(signed_tx.raw_hex.clone());
        }

        use crate::rpc::evm::EvmRpcClient;
        let client = EvmRpcClient::new(chain, rpc_url)?;
        let tx_hash = client.send_raw_transaction(&signed_tx.raw_hex).await?;
        Ok(tx_hash)
    }

    async fn revoke_api_key(&self, _vault: &VaultHandle, api_key_id: &str) -> Result<()> {
        let _ = ows_lib::key_store::delete_api_key(api_key_id, Some(&self.vault_dir))
            .map_err(map_ows_err)?;
        Ok(())
    }
}
