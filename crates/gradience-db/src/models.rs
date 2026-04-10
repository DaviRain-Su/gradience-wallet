use chrono::{DateTime, Utc};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PasskeyCredential {
    pub id: String,
    pub user_id: String,
    pub credential_id: Vec<u8>,
    pub credential_pk: Vec<u8>,
    pub counter: i64,
    pub transports: String,
    pub device_name: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub plan: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkspaceMember {
    pub workspace_id: String,
    pub user_id: String,
    pub role: String,
    pub invited_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Wallet {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub workspace_id: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct WalletAddress {
    pub id: String,
    pub wallet_id: String,
    pub chain_id: String,
    pub address: String,
    pub derivation_path: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ApiKey {
    pub id: String,
    pub wallet_id: String,
    pub name: String,
    pub key_hash: Vec<u8>,
    pub permissions: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub wallet_id: Option<String>,
    pub workspace_id: Option<String>,
    pub rules_json: String,
    pub priority: i32,
    pub status: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SpendingTracker {
    pub wallet_id: String,
    pub workspace_id: Option<String>,
    pub rule_type: String,
    pub token_address: String,
    pub chain_id: String,
    pub period: String,
    pub spent_amount: String,
    pub reset_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PolicyApproval {
    pub id: String,
    pub policy_id: String,
    pub wallet_id: String,
    pub request_json: String,
    pub status: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuditLog {
    pub id: i64,
    pub wallet_id: String,
    pub api_key_id: Option<String>,
    pub action: String,
    pub context_json: String,
    pub intent_json: Option<String>,
    pub decision: String,
    pub decision_reason: Option<String>,
    pub dynamic_factors: Option<String>,
    pub tx_hash: Option<String>,
    pub anchor_tx_hash: Option<String>,
    pub anchor_root: Option<String>,
    pub anchor_leaf_index: Option<i64>,
    pub prev_hash: String,
    pub current_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AnchorBatch {
    pub id: i64,
    pub root: String,
    pub prev_root: Option<String>,
    pub log_start_index: i64,
    pub log_end_index: i64,
    pub leaf_count: i32,
    pub tx_hash: String,
    pub block_number: Option<i64>,
    pub anchored_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PaymentRecord {
    pub id: String,
    pub wallet_id: String,
    pub protocol: String,
    pub amount: String,
    pub token: String,
    pub recipient: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AiBalance {
    pub wallet_id: String,
    pub token: String,
    pub balance_raw: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LlmCallLog {
    pub id: i64,
    pub wallet_id: String,
    pub api_key_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: Option<i64>,
    pub cost_raw: String,
    pub duration_ms: Option<i32>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModelPricing {
    pub id: i64,
    pub provider: String,
    pub model: String,
    pub input_per_m: i64,
    pub output_per_m: i64,
    pub cache_per_m: i64,
    pub currency: String,
    pub effective_from: DateTime<Utc>,
    pub effective_to: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RecoveryCode {
    pub id: String,
    pub user_id: String,
    pub code: String,
    pub purpose: String,
    pub used_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OAuthIdentity {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub provider_user_id: String,
    pub email: Option<String>,
    pub metadata_json: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WalletPaymentRoute {
    pub id: String,
    pub wallet_id: String,
    pub chain_id: String,
    pub token_address: String,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SharedBudgetTracker {
    pub workspace_id: String,
    pub token_address: String,
    pub chain_id: String,
    pub period: String,
    pub spent_amount: String,
    pub total_amount: String,
    pub reset_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentSession {
    pub id: String,
    pub wallet_id: String,
    pub name: String,
    pub session_type: String,
    pub agent_key_hash: Option<String>,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentSessionLimit {
    pub session_id: String,
    pub limit_type: String,
    pub token: String,
    pub amount_raw: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentSessionUsage {
    pub session_id: String,
    pub token: String,
    pub usage_date: chrono::NaiveDate,
    pub spent_raw: String,
}
