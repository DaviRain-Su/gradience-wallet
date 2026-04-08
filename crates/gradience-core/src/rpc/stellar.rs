use crate::error::{GradienceError, Result};

const STELLAR_DECIMALS: u8 = 7;

pub struct StellarHorizonClient {
    endpoint: String,
}

impl StellarHorizonClient {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    /// Get native XLM balance for a Stellar address (G...).
    pub async fn get_balance(&self, address: &str) -> Result<u64> {
        let url = format!("{}/accounts/{}", self.endpoint, address);
        let resp: serde_json::Value = reqwest::get(&url)
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?
            .json()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        let balances = resp.get("balances").and_then(|b| b.as_array()).cloned().unwrap_or_default();
        let native = balances.iter().find_map(|b| {
            if b.get("asset_type")?.as_str()? == "native" {
                b.get("balance")?.as_str()?.parse::<f64>().ok()
            } else {
                None
            }
        });

        // Convert to stroops (1 XLM = 10^7 stroops)
        let stroops = native.map(|x| (x * 10f64.powi(STELLAR_DECIMALS as i32)) as u64).unwrap_or(0);
        Ok(stroops)
    }
}

/// Convert stroops to a human-readable XLM string.
pub fn stroops_to_xlm(stroops: u64) -> String {
    let base = 10u64.pow(STELLAR_DECIMALS as u32);
    let integer = stroops / base;
    let fractional = stroops % base;
    format!("{}.{:0>7}", integer, fractional)
}
