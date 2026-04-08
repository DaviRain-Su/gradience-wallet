use crate::error::{GradienceError, Result};

const SOLANA_DECIMALS: u8 = 9;

pub struct SolanaRpcClient {
    endpoint: String,
}

impl SolanaRpcClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    /// Get SOL balance for a base58-encoded address.
    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [address],
        });
        let resp: serde_json::Value = client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        let lamports = resp
            .get("result")
            .and_then(|r| r.get("value"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        Ok(lamports)
    }
}

/// Convert lamports to a human-readable SOL string.
pub fn lamports_to_sol(lamports: u64) -> String {
    let base = 10u64.pow(SOLANA_DECIMALS as u32);
    let integer = lamports / base;
    let fractional = lamports % base;
    format!("{}.{:0>9}", integer, fractional)
}
