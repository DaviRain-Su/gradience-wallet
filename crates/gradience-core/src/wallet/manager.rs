use crate::error::GradienceError;

#[derive(Debug, Clone)]
pub struct WalletDescriptor {
    pub id: String,
    pub name: String,
    pub accounts: Vec<AccountDescriptor>,
}

#[derive(Debug, Clone)]
pub struct AccountDescriptor {
    pub account_id: String,
    pub address: String,
    pub chain_id: String,
    pub derivation_path: String,
}
