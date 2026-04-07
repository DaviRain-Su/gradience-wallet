use anyhow::Result;
use sqlx::{Pool, Sqlite};

pub struct AiBalanceService;

impl AiBalanceService {
    pub fn new() -> Self {
        Self
    }

    /// Top up an AI balance for a wallet.
    pub async fn topup(&self, pool: &Pool<Sqlite>, wallet_id: &str, token: &str, amount_raw: &str) -> Result<()> {
        let current = gradience_db::queries::get_ai_balance(pool, wallet_id, token).await?;
        let new_balance = if let Some(b) = current {
            let cur: u128 = b.balance_raw.parse().unwrap_or(0);
            let add: u128 = amount_raw.parse().unwrap_or(0);
            (cur + add).to_string()
        } else {
            amount_raw.to_string()
        };
        gradience_db::queries::upsert_ai_balance(pool, wallet_id, token, &new_balance).await?;
        Ok(())
    }

    /// Get current balance.
    pub async fn get_balance(&self, pool: &Pool<Sqlite>, wallet_id: &str, token: &str) -> Result<String> {
        let bal = gradience_db::queries::get_ai_balance(pool, wallet_id, token).await?;
        Ok(bal.map(|b| b.balance_raw).unwrap_or_else(|| "0".into()))
    }

    /// Deduct balance if sufficient.
    pub async fn deduct(&self, pool: &Pool<Sqlite>, wallet_id: &str, token: &str, amount_raw: &str) -> Result<bool> {
        let bal = gradience_db::queries::get_ai_balance(pool, wallet_id, token).await?;
        let cur: u128 = bal.as_ref().map(|b| b.balance_raw.parse().unwrap_or(0)).unwrap_or(0);
        let deduct: u128 = amount_raw.parse().unwrap_or(0);
        if cur < deduct {
            return Ok(false);
        }
        gradience_db::queries::upsert_ai_balance(pool, wallet_id, token, &(cur - deduct).to_string()).await?;
        Ok(true)
    }
}
