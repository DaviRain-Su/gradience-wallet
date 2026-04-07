use serde::{Deserialize, Serialize};
use crate::error::{GradienceError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub wallet_id: Option<String>,
    pub workspace_id: Option<String>,
    pub rules: Vec<Rule>,
    pub priority: i32,
    pub status: String,
    pub version: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Rule {
    SpendLimit { max: String, token: String },
    DailyLimit { max: String, token: String },
    MonthlyLimit { max: String, token: String },
    ChainWhitelist { chain_ids: Vec<String> },
    ContractWhitelist { contracts: Vec<String> },
    OperationType { allowed: Vec<String> },
    TimeWindow { start_hour: u8, end_hour: u8, timezone: String },
    MaxTokensPerCall { limit: u64 },
    ModelWhitelist { models: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct EvalContext {
    pub wallet_id: String,
    pub api_key_id: String,
    pub chain_id: String,
    pub transaction: crate::ows::adapter::Transaction,
    pub intent: Option<Intent>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub intent_type: String,
    pub from_token: Option<String>,
    pub to_token: Option<String>,
    pub estimated_value_usd: Option<f64>,
    pub target_protocol: Option<String>,
    pub risk_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Deny,
    Warn,
}

#[derive(Debug, Clone)]
pub struct EvalResult {
    pub decision: Decision,
    pub reasons: Vec<String>,
}

impl Policy {
    pub fn try_from_db(db_policy: &gradience_db::models::Policy) -> Result<Self> {
        let value: serde_json::Value = serde_json::from_str(&db_policy.rules_json)
            .map_err(|e| GradienceError::Validation(format!("invalid policy json: {}", e)))?;
        let rules: Vec<Rule> = serde_json::from_value(value.get("rules").cloned().unwrap_or(serde_json::json!([])))
            .map_err(|e| GradienceError::Validation(format!("invalid rules: {}", e)))?;
        Ok(Self {
            id: db_policy.id.clone(),
            name: db_policy.name.clone(),
            wallet_id: db_policy.wallet_id.clone(),
            workspace_id: db_policy.workspace_id.clone(),
            rules,
            priority: db_policy.priority,
            status: db_policy.status.clone(),
            version: db_policy.version,
            created_at: db_policy.created_at.to_rfc3339(),
            updated_at: db_policy.updated_at.to_rfc3339(),
        })
    }
}

pub struct PolicyEngine;

impl PolicyEngine {
    pub fn evaluate(&self,
        ctx: EvalContext,
        policies: Vec<&Policy>,
    ) -> Result<EvalResult> {
        let mut reasons = Vec::new();
        for policy in policies {
            if policy.status != "active" {
                continue;
            }
            for rule in &policy.rules {
                match rule {
                    Rule::ChainWhitelist { chain_ids } => {
                        if !chain_ids.contains(&ctx.chain_id) {
                            reasons.push(format!("chain {} not in whitelist", ctx.chain_id));
                            return Ok(EvalResult { decision: Decision::Deny, reasons });
                        }
                    }
                    Rule::SpendLimit { max, .. } => {
                        let val = ctx.transaction.value.parse::<u64>().unwrap_or(0);
                        let limit = max.parse::<u64>().unwrap_or(u64::MAX);
                        if val > limit {
                            reasons.push("spend limit exceeded".into());
                            return Ok(EvalResult { decision: Decision::Deny, reasons });
                        }
                        if val > limit / 5 * 4 {
                            reasons.push("spend limit threshold warning (80%)".into());
                            return Ok(EvalResult { decision: Decision::Warn, reasons });
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(EvalResult { decision: Decision::Allow, reasons })
    }
}
