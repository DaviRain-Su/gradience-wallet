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

    /// Get the latest blockhash for constructing transactions.
    pub async fn get_latest_blockhash(&self) -> Result<String> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getLatestBlockhash",
            "params": [{"commitment": "finalized"}],
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

        resp["result"]["value"]["blockhash"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| GradienceError::Http("missing blockhash in response".into()))
    }

    /// Get account info for a base58-encoded address.
    /// Returns (owner, data_base64, lamports) if the account exists.
    pub async fn get_account_info(&self, address: &str) -> Result<Option<(String, String, u64)>> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getAccountInfo",
            "params": [address, {"encoding": "base64"}],
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

        if let Some(err) = resp.get("error") {
            return Err(GradienceError::Http(format!(
                "getAccountInfo error: {}",
                err
            )));
        }

        let result = resp.get("result").and_then(|r| r.get("value"));
        match result {
            Some(val) if val.is_null() => Ok(None),
            Some(val) => {
                let owner = val["owner"].as_str().unwrap_or("").to_string();
                let data = val["data"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let lamports = val["lamports"].as_u64().unwrap_or(0);
                Ok(Some((owner, data, lamports)))
            }
            None => Ok(None),
        }
    }

    /// Get token accounts by owner for a given mint (optional).
    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &str,
        mint: Option<&str>,
    ) -> Result<Vec<(String, String)>> {
        let client = reqwest::Client::new();
        let filter = match mint {
            Some(m) => serde_json::json!({"mint": m}),
            None => serde_json::json!({"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"}),
        };
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenAccountsByOwner",
            "params": [owner, filter, {"encoding": "base64"}],
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

        if let Some(err) = resp.get("error") {
            return Err(GradienceError::Http(format!(
                "getTokenAccountsByOwner error: {}",
                err
            )));
        }

        let mut accounts = Vec::new();
        if let Some(arr) = resp["result"]["value"].as_array() {
            for item in arr {
                let addr = item["pubkey"].as_str().unwrap_or("").to_string();
                let data = item["account"]["data"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                accounts.push((addr, data));
            }
        }
        Ok(accounts)
    }

    /// Broadcast a signed Solana transaction (bytes) via sendTransaction RPC.
    /// Returns the transaction signature string.
    pub async fn send_transaction(&self, signed_tx_bytes: &[u8]) -> Result<String> {
        use base64::Engine;
        let b64_tx = base64::engine::general_purpose::STANDARD.encode(signed_tx_bytes);
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": [b64_tx, {"encoding": "base64", "skipPreflight": false, "preflightCommitment": "confirmed"}],
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

        if let Some(err) = resp.get("error") {
            return Err(GradienceError::Http(format!(
                "sendTransaction error: {}",
                err
            )));
        }

        resp["result"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                GradienceError::Http(format!(
                    "missing result in sendTransaction response: {}",
                    resp
                ))
            })
    }
}

/// Convert lamports to a human-readable SOL string.
pub fn lamports_to_sol(lamports: u64) -> String {
    let base = 10u64.pow(SOLANA_DECIMALS as u32);
    let integer = lamports / base;
    let fractional = lamports % base;
    format!("{}.{:0>9}", integer, fractional)
}
