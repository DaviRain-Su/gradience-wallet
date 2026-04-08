use crate::error::{GradienceError, Result};
use chrono::Utc;
use sqlx::{Pool, Sqlite};

pub struct SharedBudgetService;

impl SharedBudgetService {
    pub fn new() -> Self {
        Self
    }

    /// Allocate (or update) a total budget for a workspace.
    pub async fn allocate_workspace_budget(
        &self,
        db: &Pool<Sqlite>,
        workspace_id: &str,
        total_amount_wei: u128,
        token: &str,
        chain_id: &str,
        period: &str,
    ) -> Result<()> {
        let total_str = total_amount_wei.to_string();
        let reset_at = Utc::now() + chrono::Duration::days(30);
        gradience_db::queries::upsert_shared_budget_total(
            db,
            workspace_id,
            token,
            chain_id,
            period,
            &total_str,
            reset_at,
        )
        .await
        .map_err(|e| GradienceError::Database(e.to_string()))?;
        Ok(())
    }

    /// Get remaining budget for a workspace.
    pub async fn get_remaining_budget(
        &self,
        db: &Pool<Sqlite>,
        workspace_id: &str,
        token: &str,
        chain_id: &str,
        period: &str,
    ) -> Result<u128> {
        let tracker = gradience_db::queries::get_shared_budget_spending(
            db, workspace_id, token, chain_id, period,
        )
        .await
        .map_err(|e| GradienceError::Database(e.to_string()))?;

        match tracker {
            Some(t) => {
                let total = t.total_amount.parse::<u128>().unwrap_or(0);
                let spent = t.spent_amount.parse::<u128>().unwrap_or(0);
                Ok(total.saturating_sub(spent))
            }
            None => Ok(0),
        }
    }
}
