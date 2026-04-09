use crate::error::{GradienceError, Result};
use chrono::Timelike;
use serde::{Deserialize, Serialize};

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
    SpendLimit {
        max: String,
        token: String,
    },
    DailyLimit {
        max: String,
        token: String,
    },
    MonthlyLimit {
        max: String,
        token: String,
    },
    ChainWhitelist {
        chain_ids: Vec<String>,
    },
    ContractWhitelist {
        contracts: Vec<String>,
    },
    OperationType {
        allowed: Vec<String>,
    },
    TimeWindow {
        start_hour: u8,
        end_hour: u8,
        timezone: String,
    },
    MaxTokensPerCall {
        limit: u64,
    },
    ModelWhitelist {
        models: Vec<String>,
    },
    IntentRisk {
        max_risk: f64,
    },
    DynamicRisk {
        max_forta: f64,
        max_chainalysis: f64,
    },
    SharedBudget {
        max: String,
        token: String,
        period: String,
    },
}

#[derive(Debug, Clone, Default)]
pub struct DynamicSignals {
    pub forta_score: Option<f64>,
    pub chainalysis_score: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct EvalContext {
    pub wallet_id: String,
    pub api_key_id: String,
    pub chain_id: String,
    pub transaction: crate::ows::adapter::Transaction,
    pub intent: Option<Intent>,
    pub timestamp_ms: u64,
    pub dynamic_signals: Option<DynamicSignals>,
    pub max_tokens: Option<u64>,
    pub model: Option<String>,
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
pub struct DynamicAdjustment {
    pub source: String,
    pub multiplier: f64,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct EvalResult {
    pub decision: Decision,
    pub reasons: Vec<String>,
    pub matched_intent: Option<Intent>,
    pub dynamic_adjustments: Vec<DynamicAdjustment>,
}

impl Policy {
    pub fn try_from_db(db_policy: &gradience_db::models::Policy) -> Result<Self> {
        let value: serde_json::Value = serde_json::from_str(&db_policy.rules_json)
            .map_err(|e| GradienceError::Validation(format!("invalid policy json: {}", e)))?;
        let rules: Vec<Rule> =
            serde_json::from_value(value.get("rules").cloned().unwrap_or(serde_json::json!([])))
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

fn resolve_timezone_now(timezone: &str) -> chrono::DateTime<chrono::FixedOffset> {
    if timezone.eq_ignore_ascii_case("UTC") {
        return chrono::Utc::now().into();
    }
    // Try to parse as ISO-8601 offset e.g. +08:00 or -05:00
    let parts: Vec<&str> = timezone.split(':').collect();
    if parts.len() == 2 {
        let hours: i32 = parts[0].parse().unwrap_or(0);
        let mins: i32 = parts[1].parse().unwrap_or(0);
        let total_seconds = hours.abs() * 3600 + mins * 60;
        let total_seconds = if hours < 0 || timezone.starts_with('-') {
            -total_seconds
        } else {
            total_seconds
        };
        if let Some(offset) = chrono::FixedOffset::east_opt(total_seconds) {
            return chrono::Utc::now().with_timezone(&offset);
        }
    }
    chrono::Utc::now().into()
}

impl PolicyEngine {
    pub fn evaluate(&self, ctx: EvalContext, policies: Vec<&Policy>) -> Result<EvalResult> {
        let mut deny_reasons = Vec::new();
        let mut warn_reasons = Vec::new();
        let mut adjustments = Vec::new();

        for policy in policies {
            if policy.status != "active" {
                continue;
            }

            for rule in &policy.rules {
                match rule {
                    Rule::ChainWhitelist { chain_ids } => {
                        if !chain_ids.contains(&ctx.chain_id) {
                            deny_reasons.push(format!("chain {} not in whitelist", ctx.chain_id));
                        }
                    }
                    Rule::ContractWhitelist { contracts } => {
                        let to = ctx.transaction.to.as_deref().unwrap_or("");
                        if !to.is_empty() && !contracts.iter().any(|c| c.eq_ignore_ascii_case(to)) {
                            deny_reasons.push(format!("contract {} not in whitelist", to));
                        }
                    }
                    Rule::OperationType { allowed } => {
                        let op = ctx
                            .intent
                            .as_ref()
                            .map(|i| i.intent_type.as_str())
                            .unwrap_or("unknown");
                        if !allowed.iter().any(|a| a.eq_ignore_ascii_case(op)) {
                            deny_reasons.push(format!("operation type '{}' not allowed", op));
                        }
                    }
                    Rule::TimeWindow {
                        start_hour,
                        end_hour,
                        timezone,
                    } => {
                        let now = resolve_timezone_now(timezone);
                        let hour = now.hour() as u8;
                        if start_hour <= end_hour {
                            if hour < *start_hour || hour > *end_hour {
                                deny_reasons.push(format!(
                                    "current time {} outside allowed window {}-{} {}",
                                    hour, start_hour, end_hour, timezone
                                ));
                            }
                        } else {
                            // window crosses midnight
                            if hour < *start_hour && hour > *end_hour {
                                deny_reasons.push(format!(
                                    "current time {} outside allowed window {}-{} {}",
                                    hour, start_hour, end_hour, timezone
                                ));
                            }
                        }
                    }
                    Rule::MaxTokensPerCall { limit } => {
                        if let Some(tokens) = ctx.max_tokens {
                            if tokens > *limit {
                                deny_reasons.push(format!(
                                    "max tokens per call exceeded: {} > {}",
                                    tokens, limit
                                ));
                            } else if tokens > *limit / 5 * 4 {
                                warn_reasons.push(format!(
                                    "token usage near limit: {} / {}",
                                    tokens, limit
                                ));
                            }
                        }
                    }
                    Rule::ModelWhitelist { models } => {
                        if let Some(ref model) = ctx.model {
                            if !models.iter().any(|m| m.eq_ignore_ascii_case(model)) {
                                deny_reasons.push(format!("model '{}' not in whitelist", model));
                            }
                        }
                    }
                    Rule::SpendLimit { max, .. } => {
                        let val = ctx
                            .transaction
                            .value
                            .parse::<u128>()
                            .or_else(|_| crate::eth_to_wei(&ctx.transaction.value))
                            .unwrap_or(0);
                        let limit = max.parse::<u128>().unwrap_or(u128::MAX);
                        if val > limit {
                            deny_reasons.push("spend limit exceeded".into());
                        } else if val > limit / 5 * 4 {
                            warn_reasons.push("spend limit threshold warning (80%)".into());
                        }
                    }
                    Rule::IntentRisk { max_risk } => {
                        if let Some(ref intent) = ctx.intent {
                            if let Some(risk) = intent.risk_score {
                                if risk > *max_risk {
                                    deny_reasons.push(format!(
                                        "intent risk {} exceeds max {}",
                                        risk, max_risk
                                    ));
                                } else if risk > max_risk * 0.8 {
                                    warn_reasons.push(format!(
                                        "intent risk {} is high (threshold {})",
                                        risk, max_risk
                                    ));
                                }
                            }
                        }
                    }
                    Rule::DynamicRisk {
                        max_forta,
                        max_chainalysis,
                    } => {
                        if let Some(ref signals) = ctx.dynamic_signals {
                            if let Some(score) = signals.forta_score {
                                if score > *max_forta {
                                    deny_reasons.push(format!(
                                        "Forta risk score {} exceeds threshold {}",
                                        score, max_forta
                                    ));
                                }
                            }
                            if let Some(score) = signals.chainalysis_score {
                                if score > *max_chainalysis {
                                    deny_reasons.push(format!(
                                        "Chainalysis risk score {} exceeds threshold {}",
                                        score, max_chainalysis
                                    ));
                                }
                            }
                        }
                        // Dynamic adjustment: tighten by 20% if any signal is above 50% of threshold
                        if let Some(ref signals) = ctx.dynamic_signals {
                            let forta_high = signals
                                .forta_score
                                .map(|s| s > max_forta * 0.5)
                                .unwrap_or(false);
                            let chainalysis_high = signals
                                .chainalysis_score
                                .map(|s| s > max_chainalysis * 0.5)
                                .unwrap_or(false);
                            if forta_high || chainalysis_high {
                                adjustments.push(DynamicAdjustment {
                                    source: "dynamic_risk".into(),
                                    multiplier: 0.8,
                                    reason:
                                        "elevated risk signals detected, tightening limits by 20%"
                                            .into(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Deduplicate reasons while preserving order
        fn dedup_strings(vec: Vec<String>) -> Vec<String> {
            let mut seen = std::collections::HashSet::new();
            vec.into_iter().filter(|s| seen.insert(s.clone())).collect()
        }
        fn dedup_adjustments(vec: Vec<DynamicAdjustment>) -> Vec<DynamicAdjustment> {
            let mut seen = std::collections::HashSet::new();
            vec.into_iter()
                .filter(|a| seen.insert((a.source.clone(), a.reason.clone())))
                .collect()
        }
        let deny_reasons = dedup_strings(deny_reasons);
        let warn_reasons = dedup_strings(warn_reasons);
        let adjustments = dedup_adjustments(adjustments);

        if !deny_reasons.is_empty() {
            return Ok(EvalResult {
                decision: Decision::Deny,
                reasons: deny_reasons,
                matched_intent: ctx.intent.clone(),
                dynamic_adjustments: adjustments,
            });
        }

        if !warn_reasons.is_empty() {
            return Ok(EvalResult {
                decision: Decision::Warn,
                reasons: warn_reasons,
                matched_intent: ctx.intent.clone(),
                dynamic_adjustments: adjustments,
            });
        }

        Ok(EvalResult {
            decision: Decision::Allow,
            reasons: Vec::new(),
            matched_intent: ctx.intent.clone(),
            dynamic_adjustments: adjustments,
        })
    }
}
