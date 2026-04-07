use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradienceConfig {
    pub vault_path: String,
    pub database_path: String,
    pub log_level: String,
    pub chains: Vec<ChainConfig>,
    pub merkle_anchor: Option<MerkleAnchorConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: String,
    pub rpc_url: String,
    pub native_token: String,
    pub explorer_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleAnchorConfig {
    pub enabled: bool,
    pub hashkey_rpc: String,
    pub contract_address: String,
    pub batch_size: usize,
    pub anchor_interval_secs: u64,
}
