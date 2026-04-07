use crate::error::{GradienceError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InchSwapTx {
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas: u64,
    pub gas_price: String,
}

pub struct OneInchClient {
    api_key: String,
    http: reqwest::Client,
}

impl OneInchClient {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            http: reqwest::Client::new(),
        }
    }

    /// Fetch swap transaction data from 1inch Swap API v5.2
    pub async fn swap(
        &self,
        chain_id: u64,
        from_token: &str,
        to_token: &str,
        amount: &str,
        from_addr: &str,
        slippage: f64,
    ) -> Result<InchSwapTx> {
        let url = format!(
            "https://api.1inch.dev/swap/v5.2/{}/swap",
            chain_id
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[
                ("src", from_token),
                ("dst", to_token),
                ("amount", amount),
                ("from", from_addr),
                ("slippage", &format!("{}", slippage)),
                ("disableEstimate", "true"),
            ])
            .send()
            .await
            .map_err(|e| GradienceError::Http(format!("1inch request failed: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GradienceError::Http(format!("1inch error: {}", body)));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GradienceError::Http(format!("1inch invalid json: {}", e)))?;

        let tx = json
            .get("tx")
            .ok_or_else(|| GradienceError::Http("1inch missing tx field".into()))?;

        Ok(InchSwapTx {
            to: tx["to"].as_str().unwrap_or_default().to_string(),
            data: tx["data"].as_str().unwrap_or_default().to_string(),
            value: tx["value"].as_str().unwrap_or("0").to_string(),
            gas: tx["gas"].as_u64().unwrap_or(300000),
            gas_price: tx["gasPrice"].as_str().unwrap_or("0").to_string(),
        })
    }

    /// Fetch quote from 1inch Quote API v5.2
    pub async fn quote(
        &self,
        chain_id: u64,
        from_token: &str,
        to_token: &str,
        amount: &str,
    ) -> Result<InchQuote> {
        let url = format!(
            "https://api.1inch.dev/swap/v5.2/{}/quote",
            chain_id
        );
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&[
                ("src", from_token),
                ("dst", to_token),
                ("amount", amount),
            ])
            .send()
            .await
            .map_err(|e| GradienceError::Http(format!("1inch quote failed: {}", e)))?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(GradienceError::Http(format!("1inch quote error: {}", body)));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| GradienceError::Http(format!("1inch invalid json: {}", e)))?;

        let to_amount = json["toAmount"].as_str().unwrap_or("0").to_string();
        let price_impact = json["estimatedGas"].as_u64().map(|g| format!("{} gas", g)).unwrap_or_else(|| "0.30%".into());

        Ok(InchQuote { to_amount, price_impact })
    }
}

#[derive(Debug, Clone)]
pub struct InchQuote {
    pub to_amount: String,
    pub price_impact: String,
}
