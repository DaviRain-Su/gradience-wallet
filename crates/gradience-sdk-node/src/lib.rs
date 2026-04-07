use napi::bindgen_prelude::*;
use napi_derive::napi;
use gradience_core::ows::adapter::OwsAdapter;

#[napi]
pub struct WalletDescriptor {
    pub id: String,
    pub name: String,
}

#[napi]
pub struct BalanceResult {
    pub chain_id: String,
    pub address: String,
    pub balance: String,
}

/// Create a new wallet inside the local OWS vault.
/// `passphrase` unlocks the vault; `name` is the wallet label.
#[napi]
pub async fn create_wallet(passphrase: String, name: String) -> Result<WalletDescriptor> {
    if passphrase.len() < 12 {
        return Err(Error::from_reason("passphrase too short"));
    }

    let vault_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".gradience")
        .join("vault");
    std::fs::create_dir_all(&vault_dir).map_err(|e| Error::from_reason(e.to_string()))?;

    let adapter = gradience_core::ows::local_adapter::LocalOwsAdapter::new(vault_dir.clone());
    let vault = adapter.init_vault(&passphrase).await
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let wallet = adapter.create_wallet(&vault, &name, gradience_core::ows::adapter::DerivationParams::default())
        .await
        .map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(WalletDescriptor {
        id: wallet.id,
        name: wallet.name,
    })
}

/// Sign an unsigned EVM transaction hex.
#[napi]
pub fn sign_transaction(
    wallet_id: String,
    chain: String,
    unsigned_tx_hex: String,
    passphrase: String,
) -> Result<String> {
    let vault_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".gradience")
        .join("vault");

    let result = ows_lib::sign_and_send(
        &wallet_id,
        &chain,
        &unsigned_tx_hex,
        Some(&passphrase),
        None,
        None,
        Some(&vault_dir),
    ).map_err(|e| Error::from_reason(e.to_string()))?;

    Ok(result.tx_hash)
}

/// Query the native balance of a wallet on a given chain.
#[napi]
pub async fn get_balance(wallet_id: String, chain: String) -> Result<Vec<BalanceResult>> {
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".gradience");
    let db_path = format!("sqlite:/{}/gradience.db?mode=rwc", data_dir.display());
    let db = sqlx::SqlitePool::connect(&db_path).await
        .map_err(|e| Error::from_reason(e.to_string()))?;

    let addrs = gradience_db::queries::list_wallet_addresses(&db, &wallet_id).await
        .map_err(|e| Error::from_reason(e.to_string()))?;

    let rpc_url = if chain == "base" {
        "https://mainnet.base.org"
    } else {
        "https://eth.llamarpc.com"
    };
    let client = gradience_core::rpc::evm::EvmRpcClient::new("evm", rpc_url)
        .map_err(|e| Error::from_reason(e.to_string()))?;

    let mut results = Vec::new();
    for a in addrs {
        if a.chain_id.starts_with("eip155:") {
            let bal = client.get_balance(&a.address).await.unwrap_or_default();
            results.push(BalanceResult {
                chain_id: a.chain_id,
                address: a.address,
                balance: bal,
            });
        }
    }

    Ok(results)
}
