use crate::error::{GradienceError, Result};
use serde::Deserialize;

const LIFI_QUOTE_URL: &str = "https://li.quest/v1/quote";

#[derive(Debug, Clone, Deserialize)]
pub struct LiFiQuote {
    pub estimate: LiFiEstimate,
    #[serde(default, rename = "transactionRequest")]
    pub transaction_request: Option<LiFiTxRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LiFiEstimate {
    #[serde(rename = "toAmount")]
    pub to_amount: String,
    #[serde(rename = "toAmountMin")]
    pub to_amount_min: String,
    #[serde(rename = "approvalAddress")]
    pub approval_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LiFiTxRequest {
    pub to: String,
    pub data: String,
    pub value: String,
}

pub struct LiFiClient;

impl Default for LiFiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LiFiClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn quote(
        &self,
        from_chain_id: u64,
        to_chain_id: u64,
        from_token: &str,
        to_token: &str,
        from_amount: &str,
        from_address: &str,
        slippage: f64,
    ) -> Result<LiFiQuote> {
        let url = format!(
            "{}?fromChain={}&toChain={}&fromToken={}&toToken={}&fromAmount={}&fromAddress={}&slippage={:.2}",
            LIFI_QUOTE_URL,
            lifi_chain_id(from_chain_id),
            lifi_chain_id(to_chain_id),
            from_token,
            to_token,
            from_amount,
            from_address,
            slippage
        );
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(8))
            .send()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;
        if !status.is_success() {
            return Err(GradienceError::Http(format!(
                "LI.FI quote error ({}): {}",
                status, text
            )));
        }

        let quote: LiFiQuote = serde_json::from_str(&text)
            .map_err(|e| GradienceError::Http(format!("LI.FI decode error: {}", e)))?;
        Ok(quote)
    }
}

fn lifi_chain_id(chain_num: u64) -> u64 {
    match chain_num {
        1 => 1,                  // ETH
        137 => 137,              // Polygon
        42161 => 42161,          // Arbitrum
        10 => 10,                // Optimism
        56 => 56,                // BSC
        8453 => 8453,            // Base
        101 => 1151111081099710, // Solana (LI.FI internal id)
        _ => chain_num,
    }
}

/// Map common token symbols to their canonical addresses for LI.FI.
pub fn resolve_token_address(chain_num: u64, symbol_or_addr: &str) -> String {
    let upper = symbol_or_addr.to_uppercase();
    if upper.starts_with("0X") || upper.len() > 20 {
        return symbol_or_addr.into();
    }
    match chain_num {
        1 => match upper.as_str() {
            "ETH" => "0x0000000000000000000000000000000000000000".into(),
            "USDC" => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606EB48".into(),
            "USDT" => "0xdAC17F958D2ee523a2206206994597C13D831ec7".into(),
            _ => symbol_or_addr.into(),
        },
        8453 => match upper.as_str() {
            "ETH" => "0x0000000000000000000000000000000000000000".into(),
            "USDC" => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".into(),
            _ => symbol_or_addr.into(),
        },
        56 => match upper.as_str() {
            "BNB" => "0x0000000000000000000000000000000000000000".into(),
            "USDC" => "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d".into(),
            _ => symbol_or_addr.into(),
        },
        _ => symbol_or_addr.into(),
    }
}
