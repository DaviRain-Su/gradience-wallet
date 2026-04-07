use crate::error::{GradienceError, Result};
use crate::policy::engine::{Decision, EvalResult, Policy, Rule};
use sqlx::{Pool, Sqlite};

/// Check daily/monthly spending limits for a wallet and update trackers.
/// Returns Deny if any limit is exceeded.
pub async fn evaluate_spending_limits(
    db: &Pool<Sqlite>,
    wallet_id: &str,
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
            let (limit_wei, period) = match rule {
                Rule::DailyLimit { max, .. } => {
                    let wei = max.parse::<u128>().unwrap_or(u128::MAX);
                    (wei, "daily")
                }
                Rule::MonthlyLimit { max, .. } => {
                    let wei = max.parse::<u128>().unwrap_or(u128::MAX);
                    (wei, "monthly")
                }
                _ => continue,
            };

            let now = chrono::Utc::now();
            let reset_at = if period == "daily" {
                now + chrono::Duration::days(1)
            } else {
                now + chrono::Duration::days(30)
            };

            let current = gradience_db::queries::get_spending(
                db, wallet_id, period, token_address, chain_id, period,
            )
            .await
            .ok()
            .flatten();

            let mut spent = amount_eth_wei;
            if let Some(ref tracker) = current {
                if now < tracker.reset_at {
                    spent = spent.saturating_add(tracker.spent_amount.parse::<u128>().unwrap_or(0));
                }
            }

            if spent > limit_wei {
                return Ok(EvalResult {
                    decision: Decision::Deny,
                    reasons: vec![format!("{} limit exceeded", period)],
                    matched_intent: None,
                    dynamic_adjustments: vec![],
                });
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
            let period = match rule {
                Rule::DailyLimit { .. } => "daily",
                Rule::MonthlyLimit { .. } => "monthly",
                _ => continue,
            };

            let reset_at = if period == "daily" {
                now + chrono::Duration::days(1)
            } else {
                now + chrono::Duration::days(30)
            };

            let current = gradience_db::queries::get_spending(
                db, wallet_id, period, token_address, chain_id, period,
            )
            .await
            .ok()
            .flatten();

            let new_spent = if let Some(ref tracker) = current {
                if now < tracker.reset_at {
                    amount_eth_wei.saturating_add(tracker.spent_amount.parse::<u128>().unwrap_or(0))
                } else {
                    amount_eth_wei
                }
            } else {
                amount_eth_wei
            };

            gradience_db::queries::upsert_spending(
                db,
                wallet_id,
                period,
                token_address,
                chain_id,
                period,
                &new_spent.to_string(),
                reset_at,
            )
            .await
            .map_err(|e| GradienceError::Database(e.to_string()))?;
        }
    }

    Ok(())
}
