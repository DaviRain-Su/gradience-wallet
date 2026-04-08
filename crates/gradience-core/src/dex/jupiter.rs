use crate::error::{GradienceError, Result};
use serde::Deserialize;

const JUPITER_QUOTE_URL: &str = "https://quote-api.jup.ag/v6/quote";

#[derive(Debug, Clone, Deserialize)]
pub struct JupiterQuote {
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

pub struct JupiterClient;

impl JupiterClient {
    pub fn new() -> Self {
        Self
    }

    /// Fetch a Jupiter quote for a Solana swap.
    /// `amount` is in the smallest token unit (lamports for SOL, raw integer for SPL).
    pub async fn quote(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount: &str,
        slippage_bps: u16,
    ) -> Result<JupiterQuote> {
        let url = format!(
            "{}?inputMint={}&outputMint={}&amount={}&slippageBps={}&onlyDirectRoutes=false",
            JUPITER_QUOTE_URL, input_mint, output_mint, amount, slippage_bps
        );
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| GradienceError::Http(e.to_string()))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| GradienceError::Http(e.to_string()))?;
        if !status.is_success() {
            return Err(GradienceError::Http(format!(
                "Jupiter quote error ({}): {}",
                status, text
            )));
        }

        let quote: JupiterQuote = serde_json::from_str(&text)
            .map_err(|e| GradienceError::Http(format!("Jupiter decode error: {}", e)))?;

        if let Some(err) = quote.error {
            return Err(GradienceError::Http(format!("Jupiter API error: {}", err)));
        }
        Ok(quote)
    }
}

/// Resolve a common token symbol to its Solana mint address.
/// Falls back to returning the input if it already looks like a 32-byte base58 address.
pub fn resolve_solana_mint(symbol_or_mint: &str) -> String {
    let upper = symbol_or_mint.to_uppercase();
    match upper.as_str() {
        "SOL" | "WSOL" => "So11111111111111111111111111111111111111112".into(),
        "USDC" => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".into(),
        "USDT" => "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".into(),
        "BONK" => "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".into(),
        _ => symbol_or_mint.into(),
    }
}
