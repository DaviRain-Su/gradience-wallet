use crate::error::{GradienceError, Result};
use serde::Deserialize;

fn blockscout_base_url(chain_id: &str) -> Option<&'static str> {
    match chain_id {
        "eip155:8453" => Some("https://base.blockscout.com"),
        "eip155:1" => Some("https://eth.blockscout.com"),
        "eip155:137" => Some("https://polygon.blockscout.com"),
        "eip155:42161" => Some("https://arbitrum.blockscout.com"),
        "eip155:10" => Some("https://optimism.blockscout.com"),
        "eip155:56" => Some("https://bnb.blockscout.com"),
        "eip155:43114" => Some("https://avalanche.blockscout.com"),
        "eip155:250" => Some("https://ftm.blockscout.com"),
        "eip155:100" => Some("https://gnosis.blockscout.com"),
        _ => None,
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenBalance {
    pub token: TokenMeta,
    pub value: String,
    pub token_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenMeta {
    pub address_hash: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<String>,
    pub exchange_rate: Option<String>,
    pub icon_url: Option<String>,
    #[serde(rename = "type")]
    pub token_type: Option<String>,
}

pub async fn fetch_token_balances(chain_id: &str, address: &str) -> Result<Vec<TokenBalance>> {
    let base = blockscout_base_url(chain_id)
        .ok_or_else(|| GradienceError::Http(format!("blockscout not available for {}", chain_id)))?;
    let url = format!("{}/api/v2/addresses/{}/token-balances", base, address);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| GradienceError::Http(e.to_string()))?;

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| GradienceError::Http(format!("blockscout request failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(GradienceError::Http(format!(
            "blockscout error: HTTP {}",
            resp.status()
        )));
    }

    let list: Vec<TokenBalance> = resp
        .json()
        .await
        .map_err(|e| GradienceError::Http(format!("blockscout json parse failed: {}", e)))?;

    // Filter to ERC-20 only (exclude NFTs with token_id)
    let filtered: Vec<_> = list
        .into_iter()
        .filter(|item| item.token_id.is_none())
        .filter(|item| item.token.token_type.as_deref().unwrap_or("") == "ERC-20")
        .collect();

    Ok(filtered)
}
