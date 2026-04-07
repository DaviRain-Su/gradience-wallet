use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct SignTxArgs {
    pub wallet_id: String,
    pub chain_id: String,
    pub transaction: TxBody,
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

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AiModelsArgs {}
