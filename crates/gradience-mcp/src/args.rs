use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SignTxArgs {
    pub wallet_id: String,
    pub chain_id: String,
    pub transaction: TxBody,
    #[serde(default)]
    pub approval_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TxBody {
    pub to: String,
    pub value: String,
    pub data: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GetBalanceArgs {
    pub wallet_id: String,
    pub chain_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SwapArgs {
    pub wallet_id: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub chain: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct PayArgs {
    pub wallet_id: String,
    pub recipient: String,
    pub amount: String,
    pub token: Option<String>,
    pub chain: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct LlmGenerateArgs {
    pub wallet_id: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AiBalanceArgs {
    pub wallet_id: String,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SignMessageArgs {
    pub wallet_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SignAndSendArgs {
    pub wallet_id: String,
    pub chain_id: String,
    pub transaction: TxBody,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TransferSplArgs {
    pub wallet_id: String,
    pub chain_id: String,
    pub mint: String,
    pub to: String,
    pub amount: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DelegateStakeArgs {
    pub wallet_id: String,
    pub chain_id: String,
    pub stake_account: String,
    pub vote_account: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct DeactivateStakeArgs {
    pub wallet_id: String,
    pub chain_id: String,
    pub stake_account: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct VerifyApiKeyArgs {
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct CheckApprovalArgs {
    pub approval_id: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AiModelsArgs {}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct EarnDiscoverArgs {
    pub chain_id: u64,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct EarnPositionsArgs {
    pub wallet_address: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct EarnQuoteArgs {
    pub from_chain: u64,
    pub to_chain: u64,
    pub from_token: String,
    pub to_token: String,
    pub from_address: String,
    pub to_address: String,
    pub from_amount: String,
}
