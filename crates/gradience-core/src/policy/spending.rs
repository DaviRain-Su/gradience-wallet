use crate::error::{GradienceError, Result};
use crate::policy::engine::{Decision, EvalResult, Policy, Rule};
use sqlx::{Pool, Sqlite};

fn resolve_reset_at(
    period: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> chrono::DateTime<chrono::Utc> {
    match period {
        "daily" => now + chrono::Duration::days(1),
        "weekly" => now + chrono::Duration::weeks(1),
        "monthly" => now + chrono::Duration::days(30),
        _ => now + chrono::Duration::days(1),
    }
}

/// Check daily/monthly spending limits for a wallet and update trackers.
/// Returns Deny if any limit is exceeded.
pub async fn evaluate_spending_limits(
    db: &Pool<Sqlite>,
    wallet_id: &str,
    workspace_id: Option<&str>,
    chain_id: &str,
    amount_eth_wei: u128,
    policies: &[Policy],
) -> Result<EvalResult> {
    let token_address = ""; // native token placeholder

    for policy in policies {
        if policy.status != "active" {
            continue;
        }
        for rule in &policy.rules {
            match rule {
                Rule::DailyLimit { max, .. } => {
                    let limit_wei = max.parse::<u128>().unwrap_or(u128::MAX);
                    let now = chrono::Utc::now();
                    let _reset_at = resolve_reset_at("daily", now);
                    let current = gradience_db::queries::get_spending(
                        db,
                        wallet_id,
                        "daily",
                        token_address,
                        chain_id,
                        "daily",
                    )
                    .await
                    .ok()
                    .flatten();
                    let mut spent = amount_eth_wei;
                    if let Some(ref tracker) = current {
                        if now < tracker.reset_at {
                            spent = spent
                                .saturating_add(tracker.spent_amount.parse::<u128>().unwrap_or(0));
                        }
                    }
                    if spent > limit_wei {
                        return Ok(EvalResult {
                            decision: Decision::Deny,
                            reasons: vec!["daily limit exceeded".into()],
                            matched_intent: None,
                            dynamic_adjustments: vec![],
                        });
                    }
                }
                Rule::MonthlyLimit { max, .. } => {
                    let limit_wei = max.parse::<u128>().unwrap_or(u128::MAX);
                    let now = chrono::Utc::now();
                    let _reset_at = resolve_reset_at("monthly", now);
                    let current = gradience_db::queries::get_spending(
                        db,
                        wallet_id,
                        "monthly",
                        token_address,
                        chain_id,
                        "monthly",
                    )
                    .await
                    .ok()
                    .flatten();
                    let mut spent = amount_eth_wei;
                    if let Some(ref tracker) = current {
                        if now < tracker.reset_at {
                            spent = spent
                                .saturating_add(tracker.spent_amount.parse::<u128>().unwrap_or(0));
                        }
                    }
                    if spent > limit_wei {
                        return Ok(EvalResult {
                            decision: Decision::Deny,
                            reasons: vec!["monthly limit exceeded".into()],
                            matched_intent: None,
                            dynamic_adjustments: vec![],
                        });
                    }
                }
                Rule::SharedBudget { max, token, period } => {
                    let ws_id = match workspace_id {
                        Some(id) => id,
                        None => continue,
                    };
                    let limit_wei = max.parse::<u128>().unwrap_or(u128::MAX);
                    let now = chrono::Utc::now();
                    let current = gradience_db::queries::get_shared_budget_spending(
                        db, ws_id, token, chain_id, period,
                    )
                    .await
                    .ok()
                    .flatten();
                    let mut spent = amount_eth_wei;
                    if let Some(ref tracker) = current {
                        if now < tracker.reset_at {
                            spent = spent
                                .saturating_add(tracker.spent_amount.parse::<u128>().unwrap_or(0));
                        }
                    }
                    if spent > limit_wei {
                        return Ok(EvalResult {
                            decision: Decision::Deny,
                            reasons: vec![format!(
                                "shared budget exceeded for workspace {}",
                                ws_id
                            )],
                            matched_intent: None,
                            dynamic_adjustments: vec![],
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Ok(EvalResult {
        decision: Decision::Allow,
        reasons: vec![],
        matched_intent: None,
        dynamic_adjustments: vec![],
    })
}

/// Update spending trackers after a successful approved transaction.
pub async fn record_spending(
    db: &Pool<Sqlite>,
    wallet_id: &str,
    workspace_id: Option<&str>,
    chain_id: &str,
    amount_eth_wei: u128,
    policies: &[Policy],
) -> Result<()> {
    let token_address = ""; // native token placeholder
    let now = chrono::Utc::now();

    for policy in policies {
        if policy.status != "active" {
            continue;
        }
        for rule in &policy.rules {
            match rule {
                Rule::DailyLimit { .. } => {
                    let reset_at = resolve_reset_at("daily", now);
                    let current = gradience_db::queries::get_spending(
                        db,
                        wallet_id,
                        "daily",
                        token_address,
                        chain_id,
                        "daily",
                    )
                    .await
                    .ok()
                    .flatten();
                    let new_spent = if let Some(ref tracker) = current {
                        if now < tracker.reset_at {
                            amount_eth_wei
                                .saturating_add(tracker.spent_amount.parse::<u128>().unwrap_or(0))
                        } else {
                            amount_eth_wei
                        }
                    } else {
                        amount_eth_wei
                    };
                    gradience_db::queries::upsert_spending(
                        db,
                        wallet_id,
                        "daily",
                        token_address,
                        chain_id,
                        "daily",
                        &new_spent.to_string(),
                        reset_at,
                    )
                    .await
                    .map_err(|e| GradienceError::Database(e.to_string()))?;
                }
                Rule::MonthlyLimit { .. } => {
                    let reset_at = resolve_reset_at("monthly", now);
                    let current = gradience_db::queries::get_spending(
                        db,
                        wallet_id,
                        "monthly",
                        token_address,
                        chain_id,
                        "monthly",
                    )
                    .await
                    .ok()
                    .flatten();
                    let new_spent = if let Some(ref tracker) = current {
                        if now < tracker.reset_at {
                            amount_eth_wei
                                .saturating_add(tracker.spent_amount.parse::<u128>().unwrap_or(0))
                        } else {
                            amount_eth_wei
                        }
                    } else {
                        amount_eth_wei
                    };
                    gradience_db::queries::upsert_spending(
                        db,
                        wallet_id,
                        "monthly",
                        token_address,
                        chain_id,
                        "monthly",
                        &new_spent.to_string(),
                        reset_at,
                    )
                    .await
                    .map_err(|e| GradienceError::Database(e.to_string()))?;
                }
                Rule::SharedBudget { token, period, .. } => {
                    let ws_id = match workspace_id {
                        Some(id) => id,
                        None => continue,
                    };
                    let reset_at = resolve_reset_at(period, now);
                    let current = gradience_db::queries::get_shared_budget_spending(
                        db, ws_id, token, chain_id, period,
                    )
                    .await
                    .ok()
                    .flatten();
                    let new_spent = if let Some(ref tracker) = current {
                        if now < tracker.reset_at {
                            amount_eth_wei
                                .saturating_add(tracker.spent_amount.parse::<u128>().unwrap_or(0))
                        } else {
                            amount_eth_wei
                        }
                    } else {
                        amount_eth_wei
                    };
                    gradience_db::queries::upsert_shared_budget_spending(
                        db,
                        ws_id,
                        token,
                        chain_id,
                        period,
                        &new_spent.to_string(),
                        reset_at,
                    )
                    .await
                    .map_err(|e| GradienceError::Database(e.to_string()))?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
